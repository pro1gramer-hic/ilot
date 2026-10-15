//! The link with îlot's browser extension (extension/, installed by the user only, never
//! by îlot). On îlot's request the extension asks the browser for its own
//! picture-in-picture on the Crunchyroll tab (docs/09-DEMO-FINALE.md, 5 bis, step 2): the
//! browser's player keeps showing the video with its rights, îlot never sees a frame.
//!
//! Edge or Chrome starts `ilot.exe chrome-extension://<id>/` as its native messaging host
//! (https://learn.microsoft.com/en-us/microsoft-edge/extensions/developer-guide/native-messaging):
//! each message is a 32-bit length in native byte order, then UTF-8 JSON. That process
//! checks the caller and every message, then relays them as JSON lines to the running
//! îlot through `\\.\pipe\ilot-video-<SID>`: owner-only access list, remote clients
//! refused, first instance or nothing, and served by this very executable (the host
//! checks it). No port, no command run, nothing written to disk.
//!
//! Chromium only opens a picture-in-picture window right after a user gesture
//! (https://developer.mozilla.org/en-US/docs/Web/API/HTMLVideoElement/requestPictureInPicture):
//! a request from îlot can be refused (`gesture`), and the extension's own shortcut, a
//! real key press, then does it.

use crate::locale::Text;
use crate::video;
use ilot_bridge::client::same_path;
use ilot_bridge::identity::{SecurityDescriptor, current_user_sid, owner_only_sddl};
use serde::{Deserialize, Serialize};
use std::ffi::c_void;
use std::io::{self, Read, Write};
use std::os::windows::io::AsRawHandle;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient, NamedPipeServer, PipeMode, ServerOptions};
use tokio::sync::{mpsc, oneshot};
use windows::Win32::Foundation::{CloseHandle, ERROR_PIPE_BUSY, HANDLE};
use windows::Win32::Security::SECURITY_ATTRIBUTES;
use windows::Win32::Storage::FileSystem::SECURITY_IDENTIFICATION;
use windows::Win32::System::Pipes::GetNamedPipeServerProcessId;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::core::PWSTR;

/// The extension's id, fixed by the public `key` of extension/manifest.json. The browser
/// knows this host as `io.github.hic_69.ilot` (extension/background.js and
/// extension/hote/io.github.hic_69.ilot.json); tests/unit/extension.test.ts checks all three.
pub const EXTENSION_ID: &str = "pgannnjdlbpfgibgpenliogidpinjokf";
pub const PROTOCOL_VERSION: u32 = 1;
/// Largest message either side accepts: the real ones are a few dozen bytes.
pub const MAX_MESSAGE: usize = 4 * 1024;
/// How long îlot waits for the browser's answer to a picture-in-picture request.
const PIP_TIMEOUT: Duration = Duration::from_millis(1500);
/// A busy pipe instance (another relay just connected) is tried again this often.
const BUSY_RETRY: Duration = Duration::from_millis(50);
const BUSY_TRIES: u32 = 10;
/// Wait before creating a pipe instance again after a failure.
const RETRY: Duration = Duration::from_millis(500);

const NOT_CONNECTED: Text<'static> = Text::ExtensionNotConnected;
const NO_ANSWER: Text<'static> = Text::BrowserNoAnswer;

pub fn allowed_origin() -> String {
    format!("chrome-extension://{EXTENSION_ID}/")
}

pub fn pipe_name(sid: &str) -> String {
    format!(r"\\.\pipe\ilot-video-{sid}")
}

// ---------------------------------------------------------------------------------------
// Messages (pure, tested with the extension's real messages)

/// îlot to the extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum ToExtension {
    /// Show the Crunchyroll video in the browser's floating window.
    Pip,
    /// Put it back in its tab.
    ExitPip,
}

/// Why the browser did not open its floating window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Refusal {
    /// Chromium wants a user gesture first.
    Gesture,
    /// The site turned picture-in-picture off for this video (`disablePictureInPicture`),
    /// which the extension respects.
    Disabled,
    NoVideo,
    Failed,
}

/// The extension to îlot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum FromExtension {
    Hello { version: u32 },
    /// A Crunchyroll tab holds a video; `pip` while the browser shows it floating.
    State { video: bool, pip: bool },
    /// Answer to `pip`.
    Pip {
        ok: bool,
        #[serde(default)]
        reason: Option<Refusal>,
    },
}

/// A message from the extension, if it is one îlot knows and of a sane size.
pub fn parse_from_extension(bytes: &[u8]) -> Option<FromExtension> {
    (bytes.len() <= MAX_MESSAGE).then(|| serde_json::from_slice(bytes).ok()).flatten()
}

/// A message from îlot, checked again by the relay before it reaches the browser.
pub fn parse_to_extension(bytes: &[u8]) -> Option<ToExtension> {
    (bytes.len() <= MAX_MESSAGE).then(|| serde_json::from_slice(bytes).ok()).flatten()
}

/// One message as the browser reads it: its length, then the JSON. The browser accepts
/// at most 1 MiB from a host; îlot never comes close.
pub fn frame(json: &[u8]) -> io::Result<Vec<u8>> {
    let len = u32::try_from(json.len())
        .ok()
        .filter(|&len| len as usize <= MAX_MESSAGE)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "message too large"))?;
    let mut bytes = Vec::with_capacity(4 + json.len());
    bytes.extend_from_slice(&len.to_ne_bytes());
    bytes.extend_from_slice(json);
    Ok(bytes)
}

/// Reads one message from the browser. `None` when the browser closed the stream between
/// two messages; an error for a cut message or one longer than `max`.
pub fn read_frame(reader: &mut impl Read, max: usize) -> io::Result<Option<Vec<u8>>> {
    let mut len = [0u8; 4];
    let mut got = 0;
    while got < len.len() {
        match reader.read(&mut len[got..])? {
            0 if got == 0 => return Ok(None),
            0 => return Err(io::ErrorKind::UnexpectedEof.into()),
            n => got += n,
        }
    }
    let len = u32::from_ne_bytes(len) as usize;
    if len > max {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "message too large"));
    }
    let mut message = vec![0u8; len];
    reader.read_exact(&mut message)?;
    Ok(Some(message))
}

/// One JSON line of at most `max` bytes (newline excluded); `None` at the end of the stream.
async fn next_line<R: AsyncBufRead + Unpin>(reader: &mut R, max: usize) -> io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    let read = (&mut *reader).take(max as u64 + 1).read_until(b'\n', &mut line).await?;
    if read == 0 {
        return Ok(None);
    }
    if line.pop() != Some(b'\n') {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "line too long or cut"));
    }
    Ok(Some(line))
}

fn line_of<T: Serialize>(message: &T) -> io::Result<Vec<u8>> {
    let mut line = serde_json::to_vec(message).map_err(io::Error::other)?;
    line.push(b'\n');
    Ok(line)
}

// ---------------------------------------------------------------------------------------
// The host process the browser starts

/// True when the browser starts îlot as its native messaging host.
pub fn is_host_launch(args: &[String]) -> bool {
    args.get(1).is_some_and(|arg| arg.starts_with("chrome-extension://"))
}

/// The relay between the browser and the running îlot. Exit code 0 when either side
/// leaves; 1 for a caller that is not îlot's extension.
pub fn run_host(origin: &str) -> i32 {
    if origin != allowed_origin() {
        return 1;
    }
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread().enable_all().build() else { return 1 };
    // îlot not running: the host quits and the extension tries again later.
    let _ = runtime.block_on(relay());
    0
}

async fn relay() -> io::Result<()> {
    let (from_ilot, mut to_ilot) = tokio::io::split(connect().await?);
    let mut from_ilot = BufReader::new(from_ilot);
    // The browser's messages, read on a thread of their own: stdin blocks.
    let (sender, mut browser) = mpsc::channel::<FromExtension>(8);
    std::thread::spawn(move || {
        let mut stdin = io::stdin().lock();
        while let Ok(Some(bytes)) = read_frame(&mut stdin, MAX_MESSAGE) {
            // Anything else is dropped: only checked messages reach îlot.
            if let Some(message) = parse_from_extension(&bytes)
                && sender.blocking_send(message).is_err()
            {
                break;
            }
        }
    });
    let upstream = async {
        while let Some(message) = browser.recv().await {
            to_ilot.write_all(&line_of(&message)?).await?;
        }
        Ok(())
    };
    let downstream = async {
        let mut stdout = io::stdout();
        while let Some(line) = next_line(&mut from_ilot, MAX_MESSAGE).await? {
            if let Some(message) = parse_to_extension(&line) {
                stdout.write_all(&frame(&serde_json::to_vec(&message).map_err(io::Error::other)?)?)?;
                stdout.flush()?;
            }
        }
        Ok(())
    };
    tokio::select! {
        done = upstream => done,
        done = downstream => done,
    }
}

/// Opens îlot's pipe at the identification level, and only if this very executable
/// serves it (no pipe squatting).
async fn connect() -> io::Result<NamedPipeClient> {
    let name = pipe_name(&current_user_sid()?);
    let mut tries = 0;
    let pipe = loop {
        match ClientOptions::new().security_qos_flags(SECURITY_IDENTIFICATION.0).open(&name) {
            Ok(pipe) => break pipe,
            Err(error) if error.raw_os_error() == Some(ERROR_PIPE_BUSY.0 as i32) && tries < BUSY_TRIES => {
                tries += 1;
                tokio::time::sleep(BUSY_RETRY).await;
            }
            Err(error) => return Err(error),
        }
    };
    if same_path(&server_image(&pipe)?, &std::env::current_exe()?) {
        Ok(pipe)
    } else {
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "pipe served by another program"))
    }
}

fn server_image(pipe: &NamedPipeClient) -> io::Result<PathBuf> {
    // SAFETY: the pipe handle is valid while `pipe` lives; the process handle is closed
    // before returning and the buffer is large enough for any Windows path.
    unsafe {
        let mut pid = 0u32;
        GetNamedPipeServerProcessId(HANDLE(pipe.as_raw_handle()), &mut pid)?;
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)?;
        let mut buffer = [0u16; 32 * 1024];
        let mut len = buffer.len() as u32;
        let read = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut len);
        let _ = CloseHandle(process);
        read?;
        Ok(PathBuf::from(String::from_utf16_lossy(&buffer[..len as usize])))
    }
}

// ---------------------------------------------------------------------------------------
// îlot's side: the pipe server and what the extension last said

/// What the browser answered to a picture-in-picture request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipAnswer {
    Shown,
    Refused(Refusal),
}

impl PipAnswer {
    fn of(ok: bool, reason: Option<Refusal>) -> Self {
        if ok { Self::Shown } else { Self::Refused(reason.unwrap_or(Refusal::Failed)) }
    }
}

#[derive(Default)]
struct Link {
    /// Messages to the extension, while one is connected.
    out: Option<mpsc::UnboundedSender<ToExtension>>,
    /// Number of the current connection: an older one that ends clears nothing.
    connection: u64,
    hello: bool,
    video: bool,
    pip: bool,
    answer: Option<oneshot::Sender<PipAnswer>>,
}

/// The extension link, managed by Tauri.
#[derive(Default)]
pub struct Extension(Mutex<Link>);

impl Extension {
    /// The extension said hello on the current connection.
    pub fn connected(&self) -> bool {
        let link = self.0.lock().unwrap();
        link.out.is_some() && link.hello
    }

    /// Connected, and a Crunchyroll tab holds a video.
    pub fn ready(&self) -> bool {
        self.connected() && self.0.lock().unwrap().video
    }

    /// The browser shows the video in its floating window.
    pub fn in_pip(&self) -> bool {
        self.connected() && self.0.lock().unwrap().pip
    }

    fn open(&self, out: mpsc::UnboundedSender<ToExtension>) -> u64 {
        let mut link = self.0.lock().unwrap();
        let connection = link.connection + 1;
        *link = Link { out: Some(out), connection, ..Link::default() };
        connection
    }

    fn close(&self, connection: u64) {
        let mut link = self.0.lock().unwrap();
        if link.connection == connection {
            *link = Link { connection, ..Link::default() };
        }
    }

    /// Records a message; true when the picture-in-picture state changed.
    fn receive(&self, message: FromExtension) -> bool {
        let mut link = self.0.lock().unwrap();
        match message {
            FromExtension::Hello { version } => {
                link.hello = version == PROTOCOL_VERSION;
                false
            }
            FromExtension::State { video, pip } => {
                link.video = video;
                std::mem::replace(&mut link.pip, pip) != pip
            }
            FromExtension::Pip { ok, reason } => {
                if let Some(answer) = link.answer.take() {
                    let _ = answer.send(PipAnswer::of(ok, reason));
                }
                false
            }
        }
    }

    fn send(&self, message: ToExtension) -> bool {
        self.0.lock().unwrap().out.as_ref().is_some_and(|out| out.send(message).is_ok())
    }
}

/// Asks the browser to float the Crunchyroll video, and waits for its answer.
pub async fn request_pip(app: &AppHandle) -> Result<PipAnswer, String> {
    let (sender, answer) = oneshot::channel();
    {
        let extension = app.state::<Extension>();
        if !extension.connected() {
            return Err(NOT_CONNECTED.into());
        }
        extension.0.lock().unwrap().answer = Some(sender);
        if !extension.send(ToExtension::Pip) {
            return Err(NOT_CONNECTED.into());
        }
    }
    match tokio::time::timeout(PIP_TIMEOUT, answer).await {
        Ok(Ok(answer)) => Ok(answer),
        _ => Err(NO_ANSWER.into()),
    }
}

/// Puts the video back in its tab.
pub fn exit_pip(app: &AppHandle) {
    app.state::<Extension>().send(ToExtension::ExitPip);
}

fn create(name: &str, descriptor: &SecurityDescriptor, first: bool) -> io::Result<NamedPipeServer> {
    let mut options = ServerOptions::new();
    options.first_pipe_instance(first).reject_remote_clients(true).pipe_mode(PipeMode::Byte);
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.as_ptr(),
        bInheritHandle: false.into(),
    };
    // SAFETY: the attributes and the descriptor they point to outlive the call.
    unsafe { options.create_with_security_attributes_raw(name, (&raw mut attributes).cast::<c_void>()) }
}

/// Serves the extension's relay for the rest of the run. Without the pipe îlot still
/// runs: Crunchyroll in a browser is then kept on top instead.
pub fn start(app: &AppHandle) -> io::Result<()> {
    let sid = current_user_sid()?;
    let name = pipe_name(&sid);
    let descriptor = SecurityDescriptor::from_sddl(&owner_only_sddl(&sid))?;
    // Built inside the runtime: tokio pipes need its reactor.
    let first = tauri::async_runtime::block_on(async { create(&name, &descriptor, true) })?;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut server = first;
        loop {
            let connected = server.connect().await.is_ok();
            let next = loop {
                match create(&name, &descriptor, false) {
                    Ok(next) => break next,
                    Err(_) => tokio::time::sleep(RETRY).await,
                }
            };
            let pipe = std::mem::replace(&mut server, next);
            if connected {
                tauri::async_runtime::spawn(serve(app.clone(), pipe));
            }
        }
    });
    Ok(())
}

/// One relay: the newest one replaces the previous (its sender is dropped, so it ends).
async fn serve(app: AppHandle, pipe: NamedPipeServer) {
    let (reader, mut writer) = tokio::io::split(pipe);
    let mut reader = BufReader::new(reader);
    let (out, mut outbox) = mpsc::unbounded_channel();
    let connection = app.state::<Extension>().open(out);
    let writing = async {
        while let Some(message) = outbox.recv().await {
            writer.write_all(&line_of(&message)?).await?;
        }
        Ok::<(), io::Error>(())
    };
    let reading = async {
        while let Some(line) = next_line(&mut reader, MAX_MESSAGE).await? {
            let Some(message) = parse_from_extension(&line) else { continue };
            let pip_changed = app.state::<Extension>().receive(message);
            video::extension_changed(&app, pip_changed);
        }
        Ok::<(), io::Error>(())
    };
    let _ = tokio::select! {
        done = writing => done,
        done = reading => done,
    };
    app.state::<Extension>().close(connection);
    video::extension_changed(&app, true);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Messages exactly as extension/background.js and extension/content.js send them.
    #[test]
    fn the_extension_messages_are_understood() {
        assert_eq!(parse_from_extension(br#"{"type":"hello","version":1}"#), Some(FromExtension::Hello { version: 1 }));
        assert_eq!(
            parse_from_extension(br#"{"type":"state","video":true,"pip":false}"#),
            Some(FromExtension::State { video: true, pip: false })
        );
        assert_eq!(parse_from_extension(br#"{"type":"pip","ok":true}"#), Some(FromExtension::Pip { ok: true, reason: None }));
        assert_eq!(
            parse_from_extension(br#"{"type":"pip","ok":false,"reason":"gesture"}"#),
            Some(FromExtension::Pip { ok: false, reason: Some(Refusal::Gesture) })
        );
        assert_eq!(
            parse_from_extension(br#"{"type":"pip","ok":false,"reason":"disabled"}"#),
            Some(FromExtension::Pip { ok: false, reason: Some(Refusal::Disabled) })
        );
    }

    #[test]
    fn anything_else_is_dropped() {
        for bad in [
            &br#"{"type":"run","command":"calc.exe"}"#[..],
            br#"{"type":"state","video":true,"pip":false,"url":"https://evil.example"}"#,
            br#"{"type":"pip","ok":false,"reason":"because"}"#,
            br#"{"type":"hello"}"#,
            br#"not json"#,
            b"",
        ] {
            assert_eq!(parse_from_extension(bad), None, "{}", String::from_utf8_lossy(bad));
        }
        let huge = format!(r#"{{"type":"hello","version":1,"pad":"{}"}}"#, "x".repeat(MAX_MESSAGE));
        assert_eq!(parse_from_extension(huge.as_bytes()), None);
        assert_eq!(parse_to_extension(br#"{"type":"pip"}"#), Some(ToExtension::Pip));
        assert_eq!(parse_to_extension(br#"{"type":"exitPip"}"#), Some(ToExtension::ExitPip));
        assert_eq!(parse_to_extension(br#"{"type":"seek","seconds":3}"#), None);
    }

    #[test]
    fn ilot_sends_only_two_short_messages() {
        assert_eq!(serde_json::to_string(&ToExtension::Pip).unwrap(), r#"{"type":"pip"}"#);
        assert_eq!(serde_json::to_string(&ToExtension::ExitPip).unwrap(), r#"{"type":"exitPip"}"#);
        assert_eq!(line_of(&ToExtension::Pip).unwrap(), b"{\"type\":\"pip\"}\n");
    }

    #[test]
    fn frames_carry_their_length_in_native_byte_order() {
        let bytes = frame(br#"{"type":"pip"}"#).unwrap();
        assert_eq!(&bytes[..4], &14u32.to_ne_bytes());
        assert_eq!(&bytes[4..], br#"{"type":"pip"}"#);
        assert!(frame(&vec![b' '; MAX_MESSAGE + 1]).is_err());
    }

    #[test]
    fn frames_are_read_back_one_by_one() {
        let mut stream = frame(br#"{"type":"hello","version":1}"#).unwrap();
        stream.extend(frame(br#"{"type":"state","video":true,"pip":true}"#).unwrap());
        let mut reader = Cursor::new(stream);
        assert_eq!(read_frame(&mut reader, MAX_MESSAGE).unwrap().unwrap(), br#"{"type":"hello","version":1}"#);
        assert_eq!(read_frame(&mut reader, MAX_MESSAGE).unwrap().unwrap(), br#"{"type":"state","video":true,"pip":true}"#);
        assert_eq!(read_frame(&mut reader, MAX_MESSAGE).unwrap(), None);
    }

    #[test]
    fn cut_or_huge_frames_stop_the_relay() {
        let mut cut = Cursor::new(vec![10, 0]);
        assert!(read_frame(&mut cut, MAX_MESSAGE).is_err());
        let mut short = Cursor::new([&10u32.to_ne_bytes()[..], b"{}"].concat());
        assert!(read_frame(&mut short, MAX_MESSAGE).is_err());
        let mut huge = Cursor::new(((MAX_MESSAGE + 1) as u32).to_ne_bytes().to_vec());
        assert!(read_frame(&mut huge, MAX_MESSAGE).is_err());
    }

    #[test]
    fn lines_are_limited() {
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        runtime.block_on(async {
            let mut two = BufReader::new(&b"{\"type\":\"pip\"}\n{\"type\":\"exitPip\"}\n"[..]);
            assert_eq!(next_line(&mut two, MAX_MESSAGE).await.unwrap().unwrap(), b"{\"type\":\"pip\"}");
            assert_eq!(next_line(&mut two, MAX_MESSAGE).await.unwrap().unwrap(), b"{\"type\":\"exitPip\"}");
            assert_eq!(next_line(&mut two, MAX_MESSAGE).await.unwrap(), None);
            let mut long = BufReader::new(&b"0123456789\n"[..]);
            assert!(next_line(&mut long, 4).await.is_err());
            let mut cut = BufReader::new(&b"{\"type\""[..]);
            assert!(next_line(&mut cut, MAX_MESSAGE).await.is_err());
        });
    }

    #[test]
    fn only_the_extension_may_start_the_host() {
        let host = |arg: &str| is_host_launch(&["ilot.exe".into(), arg.into()]);
        assert!(host("chrome-extension://pgannnjdlbpfgibgpenliogidpinjokf/"));
        assert!(!host("--uninstall-cleanup"));
        assert!(!is_host_launch(&["ilot.exe".into()]));
        assert_eq!(allowed_origin(), "chrome-extension://pgannnjdlbpfgibgpenliogidpinjokf/");
        assert_eq!(run_host("chrome-extension://abcdefghijklmnopabcdefghijklmnop/"), 1);
        assert_eq!(pipe_name("S-1-5-21-1"), r"\\.\pipe\ilot-video-S-1-5-21-1");
    }

    #[test]
    fn the_link_follows_the_newest_relay() {
        let extension = Extension::default();
        let (first, _keep_first) = mpsc::unbounded_channel();
        let one = extension.open(first);
        assert!(!extension.connected(), "no hello yet");
        extension.receive(FromExtension::Hello { version: PROTOCOL_VERSION });
        assert!(extension.connected());
        assert!(!extension.ready(), "no video yet");
        assert!(extension.receive(FromExtension::State { video: true, pip: true }));
        assert!(extension.ready() && extension.in_pip());
        assert!(!extension.receive(FromExtension::State { video: true, pip: true }), "same state");
        let (second, _keep_second) = mpsc::unbounded_channel();
        let two = extension.open(second);
        extension.close(one);
        assert!(!extension.connected(), "the new relay has not said hello");
        extension.receive(FromExtension::Hello { version: 2 });
        assert!(!extension.connected(), "an unknown version is not trusted");
        extension.close(two);
        assert!(!extension.connected());
    }

    #[test]
    fn a_pip_answer_reaches_the_waiting_request() {
        let extension = Extension::default();
        let (out, mut outbox) = mpsc::unbounded_channel();
        extension.open(out);
        let (sender, mut answer) = oneshot::channel();
        extension.0.lock().unwrap().answer = Some(sender);
        assert!(extension.send(ToExtension::Pip));
        assert_eq!(outbox.try_recv().unwrap(), ToExtension::Pip);
        extension.receive(FromExtension::Pip { ok: false, reason: Some(Refusal::Gesture) });
        assert_eq!(answer.try_recv().unwrap(), PipAnswer::Refused(Refusal::Gesture));
        assert_eq!(PipAnswer::of(false, None), PipAnswer::Refused(Refusal::Failed));
        assert_eq!(PipAnswer::of(true, None), PipAnswer::Shown);
    }

    /// The real pipe, the real relay checks: a server made by this test process, a client
    /// that checks it is served by the same executable, one message each way.
    #[test]
    fn a_relay_reaches_a_pipe_served_by_the_same_executable() {
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        runtime.block_on(async {
            let sid = current_user_sid().unwrap();
            let name = format!(r"\\.\pipe\ilot-video-test-{}", std::process::id());
            let descriptor = SecurityDescriptor::from_sddl(&owner_only_sddl(&sid)).unwrap();
            let server = create(&name, &descriptor, true).unwrap();
            assert!(create(&name, &descriptor, true).is_err(), "a second first instance is refused");
            let client = ClientOptions::new().security_qos_flags(SECURITY_IDENTIFICATION.0).open(&name).unwrap();
            server.connect().await.unwrap();
            assert!(same_path(&server_image(&client).unwrap(), &std::env::current_exe().unwrap()));
            let (client_reader, mut client_writer) = tokio::io::split(client);
            let (server_reader, mut server_writer) = tokio::io::split(server);
            client_writer.write_all(&line_of(&FromExtension::Hello { version: 1 }).unwrap()).await.unwrap();
            let mut server_reader = BufReader::new(server_reader);
            let line = next_line(&mut server_reader, MAX_MESSAGE).await.unwrap().unwrap();
            assert_eq!(parse_from_extension(&line), Some(FromExtension::Hello { version: 1 }));
            server_writer.write_all(&line_of(&ToExtension::Pip).unwrap()).await.unwrap();
            let mut client_reader = BufReader::new(client_reader);
            let line = next_line(&mut client_reader, MAX_MESSAGE).await.unwrap().unwrap();
            assert_eq!(parse_to_extension(&line), Some(ToExtension::Pip));
        });
    }
}
