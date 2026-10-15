//! The named pipe relays talk to: `\\.\pipe\ilot-<SID>`, reachable by the current user
//! only (protected DACL, remote clients refused, first instance or nothing), one JSON
//! line per request, 256 KiB at most, and the per-start token from `pipe.token`
//! (docs/06-SECURITE.md). No network port is ever opened.

use crate::atomic::write_atomic;
use ilot_bridge::identity::{SecurityDescriptor, owner_only_sddl};
use ilot_bridge::protocol::{MAX_MESSAGE_BYTES, PROTOCOL_VERSION, Reply, Request, encode_line, same_secret, wait_for};
use std::ffi::c_void;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{NamedPipeServer, PipeMode, ServerOptions};
use tokio::sync::{Semaphore, oneshot};
use windows::Win32::Security::SECURITY_ATTRIBUTES;

/// Relays served at once. Beyond, a connection is closed unanswered and its agent asks in
/// its own terminal: a flood can exhaust neither the pipe instances nor îlot's memory.
const MAX_CLIENTS: usize = 64;
/// Permission requests waiting at once, same answer beyond: fake requests cannot bury
/// the island, and events still get through.
const MAX_WAITING: usize = 32;
/// Wait before trying again to create a pipe instance.
const RETRY: Duration = Duration::from_millis(250);

/// What the server does with a valid request.
pub enum Outcome {
    /// Answer right away (every event gets at least a receipt).
    Reply(Reply),
    /// A permission: wait for the user's answer.
    Wait { id: String, answer: oneshot::Receiver<Reply> },
}

pub trait Handler: Send + Sync + 'static {
    fn handle(&self, request: Request) -> Outcome;
    /// The waiting relay is gone or the time is up: the request disappears.
    fn cancel(&self, id: &str);
}

pub struct ServerConfig {
    pub pipe_name: String,
    pub token: String,
    /// Where relays read the token, written only once the pipe is îlot's.
    pub token_file: PathBuf,
    pub sid: String,
    pub permission_timeout: Duration,
}

fn create(config: &ServerConfig, descriptor: &SecurityDescriptor, first: bool) -> io::Result<NamedPipeServer> {
    let mut options = ServerOptions::new();
    options.first_pipe_instance(first).reject_remote_clients(true).pipe_mode(PipeMode::Byte);
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.as_ptr(),
        bInheritHandle: false.into(),
    };
    // SAFETY: the attributes and the descriptor they point to outlive the call.
    unsafe { options.create_with_security_attributes_raw(&config.pipe_name, (&raw mut attributes).cast::<c_void>()) }
}

/// Pipe creation can fail for a moment (system resources): îlot waits and tries again
/// rather than stopping its bridge for the rest of the session.
async fn retry<T>(mut attempt: impl FnMut() -> io::Result<T>) -> T {
    loop {
        match attempt() {
            Ok(value) => return value,
            Err(_) => tokio::time::sleep(RETRY).await,
        }
    }
}

/// Creates the first pipe instance, then writes the token: an îlot that cannot own the
/// pipe (another one serves it) fails before touching the token in use. Then serves forever.
pub fn start(config: ServerConfig, handler: Arc<dyn Handler>) -> io::Result<impl Future<Output = ()>> {
    let descriptor = SecurityDescriptor::from_sddl(&owner_only_sddl(&config.sid))?;
    let first = create(&config, &descriptor, true)?;
    write_atomic(&config.token_file, config.token.as_bytes())?;
    let clients = Arc::new(Semaphore::new(MAX_CLIENTS));
    let waiting = Arc::new(Semaphore::new(MAX_WAITING));
    Ok(async move {
        let mut server = first;
        loop {
            let connected = server.connect().await.is_ok();
            // The next instance exists before this one is served or dropped: no client
            // ever finds the pipe missing while îlot runs.
            let next = retry(|| create(&config, &descriptor, false)).await;
            let pipe = std::mem::replace(&mut server, next);
            // A failed connection, or one over the limit, is dropped unanswered.
            if !connected {
                continue;
            }
            let Ok(slot) = Arc::clone(&clients).try_acquire_owned() else {
                continue;
            };
            let handler = Arc::clone(&handler);
            let waiting = Arc::clone(&waiting);
            let token = config.token.clone();
            let timeout = config.permission_timeout;
            tokio::spawn(async move {
                let _ = serve_client(pipe, &token, timeout, handler.as_ref(), &waiting).await;
                drop(slot);
            });
        }
    })
}

/// Reads one line (newline excluded), refused as soon as it passes 256 KiB, whether or
/// not it ends in the chunk just read.
async fn read_line(reader: &mut (impl AsyncReadExt + Unpin)) -> io::Result<Vec<u8>> {
    let mut line = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = reader.read(&mut chunk).await?;
        if n == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        let end = chunk[..n].iter().position(|&b| b == b'\n');
        let part = &chunk[..end.unwrap_or(n)];
        if line.len() + part.len() > MAX_MESSAGE_BYTES {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "message too large"));
        }
        line.extend_from_slice(part);
        if end.is_some() {
            return Ok(line);
        }
    }
}

/// Reads one request, checks it, and answers. A bad request gets no answer at all, nor
/// does a permission over the waiting limit.
async fn serve_client(
    pipe: NamedPipeServer,
    token: &str,
    timeout: Duration,
    handler: &dyn Handler,
    waiting: &Semaphore,
) -> io::Result<()> {
    let (mut reader, mut writer) = tokio::io::split(pipe);
    let line = tokio::time::timeout(Duration::from_secs(5), read_line(&mut reader))
        .await
        .map_err(|_| io::Error::from(io::ErrorKind::TimedOut))??;
    let request: Request = serde_json::from_slice(&line).map_err(io::Error::other)?;
    if request.version != PROTOCOL_VERSION || !same_secret(&request.token, token) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "bad token or version"));
    }
    let _place = if request.wait {
        Some(waiting.try_acquire().map_err(|_| io::Error::other("too many waiting requests"))?)
    } else {
        None
    };
    // The relay's own wait for this event (a question waits less than a permission).
    let timeout = wait_for(&request.source, &request.event).map_or(timeout, |wait| wait.min(timeout));
    let reply = match handler.handle(request) {
        Outcome::Reply(reply) => reply,
        Outcome::Wait { id, answer } => {
            let mut probe = [0u8; 1];
            tokio::select! {
                answer = answer => answer.unwrap_or_default(),
                _ = reader.read(&mut probe) => {
                    handler.cancel(&id);
                    return Ok(());
                }
                () = tokio::time::sleep(timeout) => {
                    handler.cancel(&id);
                    Reply::default()
                }
            }
        }
    };
    writer.write_all(&encode_line(&reply)?).await?;
    writer.flush().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use ilot_bridge::client::{self, ClientConfig};
    use ilot_bridge::identity::current_user_sid;
    use serde_json::json;
    use std::fs::File;
    use std::io::Write;
    use std::sync::Mutex;
    use std::time::Instant;

    /// Records requests; answers permissions through channels the test controls.
    #[derive(Default)]
    struct Fake {
        seen: Mutex<Vec<Request>>,
        cancelled: Mutex<Vec<String>>,
        waiting: Mutex<Vec<oneshot::Sender<Reply>>>,
    }

    impl Handler for Fake {
        fn handle(&self, request: Request) -> Outcome {
            let wait = request.wait;
            self.seen.lock().unwrap().push(request);
            if wait {
                let (sender, answer) = oneshot::channel();
                self.waiting.lock().unwrap().push(sender);
                Outcome::Wait { id: "p1".into(), answer }
            } else {
                Outcome::Reply(Reply::default())
            }
        }

        fn cancel(&self, id: &str) {
            self.cancelled.lock().unwrap().push(id.to_owned());
        }
    }

    fn names(test: &str) -> (String, String) {
        let sid = current_user_sid().unwrap();
        (format!(r"\\.\pipe\ilot-test-{test}-{}", std::process::id()), sid)
    }

    /// A token file of the test's own, never îlot's real one.
    fn token_file(test: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ilot-test-{test}-{}", std::process::id())).join("pipe.token")
    }

    fn client_config(pipe: &str, token: &str, answer_ms: u64) -> ClientConfig {
        ClientConfig {
            pipe_name: pipe.into(),
            token: token.into(),
            expected_server: Some(std::env::current_exe().unwrap()),
            connect_timeout: Duration::from_millis(500),
            answer_timeout: Duration::from_millis(answer_ms),
        }
    }

    fn serve(test: &str, timeout: Duration) -> (String, Arc<Fake>, tokio::runtime::Runtime) {
        let (pipe, sid) = names(test);
        let fake = Arc::new(Fake::default());
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        let token_file = token_file(test);
        let config = ServerConfig {
            pipe_name: pipe.clone(),
            token: "secret".into(),
            token_file: token_file.clone(),
            sid,
            permission_timeout: timeout,
        };
        let handler: Arc<dyn Handler> = fake.clone();
        let _guard = runtime.enter();
        let server = start(config, handler).unwrap();
        runtime.spawn(server);
        assert_eq!(std::fs::read_to_string(&token_file).unwrap(), "secret");
        std::fs::remove_dir_all(token_file.parent().unwrap()).unwrap();
        (pipe, fake, runtime)
    }

    fn request(token: &str, wait: bool) -> Request {
        client::request(token, "claude-code", if wait { "PermissionRequest" } else { "PreToolUse" }, json!({ "session_id": "s" }), wait)
    }

    /// Opens the pipe like a bare client, waiting for a free instance.
    fn connect(pipe: &str) -> File {
        let start = Instant::now();
        loop {
            match std::fs::OpenOptions::new().read(true).write(true).open(pipe) {
                Ok(file) => return file,
                Err(error) => assert!(start.elapsed() < Duration::from_secs(5), "{error}"),
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn wait_until(mut done: impl FnMut() -> bool) {
        let start = Instant::now();
        while !done() {
            assert!(start.elapsed() < Duration::from_secs(5), "timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn read(bytes: &[u8]) -> io::Result<Vec<u8>> {
        tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(read_line(&mut &bytes[..]))
    }

    #[test]
    fn events_are_received_and_acknowledged() {
        let (pipe, fake, _rt) = serve("event", Duration::from_secs(5));
        let reply = client::send(&client_config(&pipe, "secret", 2000), &request("secret", false)).unwrap();
        assert_eq!(reply, Some(Reply::default()));
        assert_eq!(fake.seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn a_wrong_token_gets_no_answer_and_is_not_handled() {
        let (pipe, fake, _rt) = serve("token", Duration::from_secs(5));
        let reply = client::send(&client_config(&pipe, "nope", 1000), &request("nope", false));
        assert!(matches!(reply, Ok(None) | Err(_)));
        assert!(fake.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_permission_waits_for_the_user_answer() {
        let (pipe, fake, _rt) = serve("allow", Duration::from_secs(5));
        let config = client_config(&pipe, "secret", 3000);
        let answered = std::thread::spawn(move || client::send(&config, &request("secret", true)).unwrap());
        wait_until(|| !fake.waiting.lock().unwrap().is_empty());
        let sender = fake.waiting.lock().unwrap().pop().unwrap();
        sender.send(Reply { output: Some("{\"decision\":\"allow\"}".into()) }).unwrap();
        assert_eq!(answered.join().unwrap().unwrap().output.as_deref(), Some("{\"decision\":\"allow\"}"));
    }

    #[test]
    fn no_answer_in_time_means_nothing_printed() {
        let (pipe, fake, _rt) = serve("timeout", Duration::from_millis(200));
        let reply = client::send(&client_config(&pipe, "secret", 3000), &request("secret", true)).unwrap();
        assert_eq!(reply, Some(Reply::default()));
        assert_eq!(fake.cancelled.lock().unwrap().as_slice(), ["p1"]);
    }

    #[test]
    fn a_relay_that_gives_up_cancels_the_request() {
        let (pipe, fake, _rt) = serve("gone", Duration::from_secs(5));
        {
            // A relay killed by the agent: it sent its request, then its end closed.
            let mut file = connect(&pipe);
            file.write_all(&encode_line(&request("secret", true)).unwrap()).unwrap();
            wait_until(|| !fake.waiting.lock().unwrap().is_empty());
        }
        wait_until(|| !fake.cancelled.lock().unwrap().is_empty());
        assert_eq!(fake.cancelled.lock().unwrap().as_slice(), ["p1"]);
    }

    #[test]
    fn oversized_and_malformed_messages_are_dropped() {
        let (pipe, fake, _rt) = serve("garbage", Duration::from_secs(5));
        for payload in [vec![b'x'; MAX_MESSAGE_BYTES + 10], b"{not json}\n".to_vec()] {
            let _ = connect(&pipe).write_all(&payload);
        }
        std::thread::sleep(Duration::from_millis(200));
        assert!(fake.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_line_over_the_limit_is_refused_even_when_it_ends_in_the_last_chunk() {
        let mut exact = vec![b'a'; MAX_MESSAGE_BYTES];
        exact.push(b'\n');
        assert_eq!(read(&exact).unwrap().len(), MAX_MESSAGE_BYTES);
        // 256 KiB is a whole number of chunks: the extra bytes come with the newline.
        let mut over = vec![b'a'; MAX_MESSAGE_BYTES + 100];
        over.push(b'\n');
        assert_eq!(read(&over).unwrap_err().kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn a_second_server_takes_neither_the_pipe_nor_the_token() {
        let (pipe, _fake, rt) = serve("twice", Duration::from_secs(5));
        // The token of the îlot already running.
        let token_file = token_file("twice");
        write_atomic(&token_file, b"in use").unwrap();
        let config = ServerConfig {
            pipe_name: pipe,
            token: "x".into(),
            token_file: token_file.clone(),
            sid: current_user_sid().unwrap(),
            permission_timeout: Duration::from_secs(1),
        };
        let _guard = rt.enter();
        assert!(start(config, Arc::new(Fake::default())).is_err());
        assert_eq!(std::fs::read_to_string(&token_file).unwrap(), "in use");
        std::fs::remove_dir_all(token_file.parent().unwrap()).unwrap();
    }

    #[test]
    fn without_its_token_the_pipe_is_not_kept() {
        let (pipe, sid) = names("no-token");
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        let _guard = runtime.enter();
        // The token's folder is a file: the token cannot be written.
        let blocker = token_file("no-token").with_extension("blocker");
        write_atomic(&blocker, b"").unwrap();
        let config = |token_file: PathBuf| ServerConfig {
            pipe_name: pipe.clone(),
            token: "secret".into(),
            token_file,
            sid: sid.clone(),
            permission_timeout: Duration::from_secs(1),
        };
        assert!(start(config(blocker.join("pipe.token")), Arc::new(Fake::default())).is_err());
        // The pipe was released: the next start owns it.
        let token_file = token_file("no-token");
        assert!(start(config(token_file.clone()), Arc::new(Fake::default())).is_ok());
        std::fs::remove_dir_all(token_file.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_failed_creation_is_tried_again_after_a_wait() {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
        let mut attempts = 0;
        let start = Instant::now();
        let created = runtime.block_on(retry(|| {
            attempts += 1;
            if attempts < 3 { Err(io::Error::other("no instance")) } else { Ok(attempts) }
        }));
        assert_eq!(created, 3);
        assert!(start.elapsed() >= RETRY * 2);
    }

    #[test]
    fn permissions_over_the_limit_get_no_answer_and_events_still_pass() {
        let (pipe, fake, _rt) = serve("crowd", Duration::from_secs(30));
        let held: Vec<File> = (0..MAX_WAITING)
            .map(|_| {
                let mut file = connect(&pipe);
                file.write_all(&encode_line(&request("secret", true)).unwrap()).unwrap();
                file
            })
            .collect();
        wait_until(|| fake.waiting.lock().unwrap().len() == MAX_WAITING);
        let extra = client::send(&client_config(&pipe, "secret", 1000), &request("secret", true));
        assert!(matches!(extra, Ok(None) | Err(_)));
        assert_eq!(fake.seen.lock().unwrap().len(), MAX_WAITING);
        let event = client::send(&client_config(&pipe, "secret", 2000), &request("secret", false)).unwrap();
        assert_eq!(event, Some(Reply::default()));
        drop(held);
    }

    #[test]
    fn connections_over_the_limit_are_dropped_until_one_ends() {
        let (pipe, fake, _rt) = serve("flood", Duration::from_secs(5));
        // Clients that connect and say nothing hold their place (up to 5 s).
        let idle: Vec<File> = (0..MAX_CLIENTS).map(|_| connect(&pipe)).collect();
        let config = client_config(&pipe, "secret", 1000);
        assert!(matches!(client::send(&config, &request("secret", false)), Ok(None) | Err(_)));
        assert!(fake.seen.lock().unwrap().is_empty());
        drop(idle);
        wait_until(|| client::send(&config, &request("secret", false)).is_ok_and(|reply| reply.is_some()));
        assert_eq!(fake.seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn the_relay_refuses_a_pipe_served_by_another_program() {
        let (pipe, fake, _rt) = serve("squat", Duration::from_secs(5));
        let mut config = client_config(&pipe, "secret", 1000);
        config.expected_server = Some(std::path::PathBuf::from(r"C:\Program Files\ilot\ilot.exe"));
        assert!(client::send(&config, &request("secret", false)).is_err());
        assert!(fake.seen.lock().unwrap().is_empty());
    }
}
