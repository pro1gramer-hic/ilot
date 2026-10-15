//! Pipe client used by the relays. Every failure ends in "no answer", never in a
//! decision: the agent then asks in its own terminal.

use crate::identity::{current_user_sid, pipe_name, token_path};
use crate::protocol::{MAX_MESSAGE_BYTES, PROTOCOL_VERSION, Reply, Request, encode_line, read_line_limited};
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{CloseHandle, ERROR_PIPE_BUSY, HANDLE};
use windows::Win32::Storage::FileSystem::SECURITY_IDENTIFICATION;
use windows::Win32::System::Pipes::{GetNamedPipeServerProcessId, WaitNamedPipeW};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::core::{HSTRING, PWSTR};

pub struct ClientConfig {
    pub pipe_name: String,
    pub token: String,
    /// When set, the pipe must be served by this very executable (no pipe squatting).
    pub expected_server: Option<PathBuf>,
    pub connect_timeout: Duration,
    pub answer_timeout: Duration,
}

/// How long the relay and the `ilot` command wait for the pipe to accept them.
pub const CONNECT_TIMEOUT: Duration = Duration::from_millis(300);

/// îlot's own executable, next to its relays.
pub fn sibling_ilot() -> io::Result<PathBuf> {
    Ok(std::env::current_exe()?.with_file_name("ilot.exe"))
}

/// The current user's îlot: its pipe, this start's token, and the executable that must
/// serve the pipe (îlot next to the caller; any other program is refused).
pub fn local_config(answer_timeout: Duration) -> io::Result<ClientConfig> {
    let token_file = token_path().ok_or_else(|| io::Error::other("LOCALAPPDATA missing"))?;
    Ok(ClientConfig {
        pipe_name: pipe_name(&current_user_sid()?),
        token: std::fs::read_to_string(token_file)?.trim().to_owned(),
        expected_server: Some(sibling_ilot()?),
        connect_timeout: CONNECT_TIMEOUT,
        answer_timeout,
    })
}

/// Sends one event and waits for îlot's reply until `answer_timeout`.
/// `Ok(None)` means îlot did not answer in time.
pub fn send(config: &ClientConfig, request: &Request) -> io::Result<Option<Reply>> {
    let line = encode_line(request)?;
    let mut pipe = open_pipe(&config.pipe_name, config.connect_timeout)?;
    if let Some(expected) = &config.expected_server {
        check_server(&pipe, expected)?;
    }
    pipe.write_all(&line)?;
    read_reply(pipe, config.answer_timeout)
}

pub fn request(token: &str, source: &str, event: &str, payload: serde_json::Value, wait: bool) -> Request {
    Request {
        version: PROTOCOL_VERSION,
        token: token.to_owned(),
        source: source.to_owned(),
        event: event.to_owned(),
        payload,
        wait,
        env: None,
    }
}

/// Opens the pipe at the identification level: whoever serves it may learn who the relay
/// is, never act as the user (std adds `SECURITY_SQOS_PRESENT` itself:
/// https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html#tymethod.security_qos_flags).
fn open_pipe(name: &str, timeout: Duration) -> io::Result<File> {
    let deadline = Instant::now() + timeout;
    loop {
        match OpenOptions::new().read(true).write(true).security_qos_flags(SECURITY_IDENTIFICATION.0).open(name) {
            Ok(file) => return Ok(file),
            Err(error) if error.raw_os_error() == Some(ERROR_PIPE_BUSY.0 as i32) => {
                let left = deadline.saturating_duration_since(Instant::now());
                if left.is_zero() {
                    return Err(error);
                }
                // SAFETY: plain wait on a pipe name; failure only means we retry or give up.
                let _ = unsafe { WaitNamedPipeW(&HSTRING::from(name), left.as_millis() as u32) };
            }
            Err(error) => return Err(error),
        }
    }
}

/// Refuses a pipe served by any other program than the expected îlot executable.
fn check_server(pipe: &File, expected: &Path) -> io::Result<()> {
    let actual = server_image(pipe)?;
    if same_path(&actual, expected) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("pipe served by {}", actual.display()),
        ))
    }
}

fn server_image(pipe: &File) -> io::Result<PathBuf> {
    // SAFETY: the pipe handle is valid while `pipe` lives; the process handle is closed
    // before returning and the name buffer is large enough for any Windows path.
    unsafe {
        let mut pid = 0u32;
        GetNamedPipeServerProcessId(HANDLE(pipe.as_raw_handle()), &mut pid)?;
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)?;
        let mut buffer = [0u16; 32 * 1024];
        let mut len = buffer.len() as u32;
        let result =
            QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut len);
        let _ = CloseHandle(process);
        result?;
        Ok(PathBuf::from(String::from_utf16_lossy(&buffer[..len as usize])))
    }
}

/// Windows paths compare without regard to case.
pub fn same_path(a: &Path, b: &Path) -> bool {
    a.as_os_str().to_string_lossy().to_lowercase() == b.as_os_str().to_string_lossy().to_lowercase()
}

fn read_reply(pipe: File, timeout: Duration) -> io::Result<Option<Reply>> {
    let (sender, receiver) = mpsc::channel();
    // A blocking read cannot be cancelled: it runs on its own thread and is abandoned
    // on timeout (the relay process exits right after).
    std::thread::spawn(move || {
        let mut reader = BufReader::new(pipe);
        let _ = sender.send(read_line_limited(&mut reader, MAX_MESSAGE_BYTES));
    });
    match receiver.recv_timeout(timeout) {
        Ok(Ok(Some(line))) => serde_json::from_slice(&line).map(Some).map_err(io::Error::other),
        Ok(Ok(None)) => Ok(None),
        Ok(Err(error)) => Err(error),
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Security::{SECURITY_IMPERSONATION_LEVEL, SecurityIdentification, SecurityImpersonation};

    #[test]
    fn missing_pipe_is_an_error_not_an_answer() {
        let config = ClientConfig {
            pipe_name: r"\\.\pipe\ilot-test-missing-pipe".into(),
            token: "t".into(),
            expected_server: None,
            connect_timeout: Duration::from_millis(100),
            answer_timeout: Duration::from_millis(100),
        };
        let request = request("t", "claude-code", "PreToolUse", serde_json::json!({}), false);
        assert!(send(&config, &request).is_err());
    }

    #[test]
    fn the_expected_server_sits_next_to_the_caller() {
        let ilot = sibling_ilot().unwrap();
        assert_eq!(ilot.file_name().unwrap(), "ilot.exe");
        assert_eq!(ilot.parent(), std::env::current_exe().unwrap().parent());
    }

    /// Serves one connection on a fresh pipe, opened by `open`, and returns the level at
    /// which the server could act as that client.
    fn granted_level(test: &str, open: impl FnOnce(&str) -> File) -> SECURITY_IMPERSONATION_LEVEL {
        use std::io::Read;
        use std::os::windows::io::FromRawHandle;
        use windows::Win32::Security::{GetTokenInformation, RevertToSelf, TOKEN_QUERY, TokenImpersonationLevel};
        use windows::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
        use windows::Win32::System::Pipes::{CreateNamedPipeW, ImpersonateNamedPipeClient, PIPE_TYPE_BYTE, PIPE_WAIT};
        use windows::Win32::System::Threading::{GetCurrentThread, OpenThreadToken};

        let name = format!(r"\\.\pipe\ilot-test-qos-{test}-{}", std::process::id());
        // SAFETY: a plain pipe with the default security; its handle is owned by `server`.
        let pipe = unsafe {
            CreateNamedPipeW(&HSTRING::from(name.as_str()), PIPE_ACCESS_DUPLEX, PIPE_TYPE_BYTE | PIPE_WAIT, 1, 512, 512, 0, None)
        };
        assert!(!pipe.is_invalid());
        // SAFETY: the handle was just created and is closed only by this file.
        let mut server = unsafe { File::from_raw_handle(pipe.0) };
        // A new instance accepts its client at once; the byte waits in the pipe's buffer.
        let mut client = open(&name);
        client.write_all(b"x").unwrap();
        server.read_exact(&mut [0u8; 1]).unwrap();
        // SAFETY: this thread acts as the client only between the two calls; the token
        // handle is closed before returning and the level is a plain enum.
        unsafe {
            ImpersonateNamedPipeClient(pipe).unwrap();
            let mut token = HANDLE::default();
            let opened = OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, true, &mut token);
            RevertToSelf().unwrap();
            opened.unwrap();
            let mut level = SECURITY_IMPERSONATION_LEVEL::default();
            let mut len = 0u32;
            let read = GetTokenInformation(
                token,
                TokenImpersonationLevel,
                Some((&raw mut level).cast()),
                size_of::<SECURITY_IMPERSONATION_LEVEL>() as u32,
                &mut len,
            );
            let _ = CloseHandle(token);
            read.unwrap();
            level
        }
    }

    #[test]
    fn the_server_can_identify_the_relay_but_never_act_as_it() {
        let relay = granted_level("relay", |name| open_pipe(name, Duration::from_secs(1)).unwrap());
        assert_eq!(relay, SecurityIdentification);
        // The same check on a plain open: the level the relay used to give away.
        let plain = granted_level("plain", |name| OpenOptions::new().read(true).write(true).open(name).unwrap());
        assert_eq!(plain, SecurityImpersonation);
    }

    #[test]
    fn paths_compare_case_insensitively() {
        assert!(same_path(Path::new(r"C:\Apps\ilot.exe"), Path::new(r"c:\apps\ILOT.EXE")));
        assert!(!same_path(Path::new(r"C:\Apps\ilot.exe"), Path::new(r"C:\Temp\ilot.exe")));
    }
}
