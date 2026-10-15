//! `ilot`: tells îlot what any tool is doing. In WSL it is `~/.local/bin/ilot`, installed
//! by the "Autre" connector; under Windows it is `ilot-cli.exe`, next to îlot.
//!
//! - `ilot emit` sends one event.
//! - `ilot run -- <commande>` starts the command itself, without a shell and with its
//!   arguments as given, tells îlot when it starts and how it ends, and exits with the
//!   command's exit code whatever îlot does.
//!
//! Like the relay, it talks only to the current user's îlot, through its pipe, and only
//! if the pipe is served by the `ilot.exe` next to it.

mod args;
mod texts;

use args::{Command, Who};
use ilot_bridge::client::{self, ClientConfig};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::time::Duration;
use texts::{Text, tr};
use windows::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT, SetConsoleCtrlHandler};
use windows::core::BOOL;

/// Every event of this command goes to the generic connector.
const SOURCE: &str = "generic";
const RECEIPT_TIMEOUT: Duration = Duration::from_secs(2);

fn main() {
    let code = match args::parse(std::env::args_os().skip(1)) {
        Ok(Command::Help) => {
            println!("{}", tr(Text::Usage));
            0
        }
        Ok(command) => {
            if matches!(command, Command::Run { .. }) {
                outlive_ctrl_c();
            }
            let config = client::local_config(RECEIPT_TIMEOUT).ok();
            let cwd = std::env::current_dir().unwrap_or_default();
            execute(command, config.as_ref(), &cwd)
        }
        Err(message) => {
            eprintln!("{}\n{}", tr(Text::Failed { message: &message }), tr(Text::Usage));
            2
        }
    };
    // The full code, not a byte: Windows exit codes are 32-bit.
    std::process::exit(code);
}

fn execute(command: Command, config: Option<&ClientConfig>, cwd: &Path) -> i32 {
    match command {
        Command::Help => 0,
        Command::Emit(emit) => {
            let payload = payload(&emit.who, cwd, emit.title.as_deref(), emit.detail.as_deref());
            if send(config, &emit.kind, payload) {
                0
            } else {
                eprintln!("{}", tr(Text::EventNotSent));
                1
            }
        }
        Command::Run { who, command } => run(config, &who, &command, cwd),
        Command::RunBegin { who, command } => {
            begin(config, &who, &command, cwd);
            0
        }
        Command::RunEnd { who, code } => {
            end(config, &who, code, cwd);
            0
        }
    }
}

/// Runs the command and gives its exit code back, îlot or not.
fn run(config: Option<&ClientConfig>, who: &Who, command: &[OsString], cwd: &Path) -> i32 {
    begin(config, who, command, cwd);
    let code = match std::process::Command::new(&command[0]).args(&command[1..]).status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            eprintln!("{}", tr(Text::CannotStart { program: &command[0].to_string_lossy(), error: &error.to_string() }));
            // The usual shell codes: not found, or found but not runnable.
            if error.kind() == io::ErrorKind::NotFound { 127 } else { 126 }
        }
    };
    end(config, who, code, cwd);
    code
}

/// The session starts and works on the command.
fn begin(config: Option<&ClientConfig>, who: &Who, command: &[OsString], cwd: &Path) {
    let line = command_line(command);
    let heard = send(config, "session_start", payload(who, cwd, None, None))
        && send(config, "working", payload(who, cwd, Some(&line), None));
    if !heard {
        eprintln!("{}", tr(Text::CommandRunsAnyway));
    }
}

/// The turn ends: done for exit code 0, failed otherwise.
fn end(config: Option<&ClientConfig>, who: &Who, code: i32, cwd: &Path) {
    let (kind, detail) = if code == 0 { ("turn_done", None) } else { ("error", Some(tr(Text::ExitCode { code }))) };
    send(config, kind, payload(who, cwd, None, detail.as_deref()));
}

/// One session per name and project: successive `emit` and `run` of a tool meet there.
fn payload(who: &Who, cwd: &Path, title: Option<&str>, detail: Option<&str>) -> Value {
    let project = who
        .project
        .clone()
        .unwrap_or_else(|| cwd.file_name().map_or_else(|| "?".to_owned(), |n| n.to_string_lossy().into_owned()));
    json!({
        "name": who.source,
        "session_id": format!("{}@{project}", who.source),
        "cwd": cwd.to_string_lossy(),
        "project": project,
        "title": title,
        "detail": detail,
    })
}

/// The command as the island shows it.
fn command_line(command: &[OsString]) -> String {
    command
        .iter()
        .map(|arg| {
            let arg = arg.to_string_lossy();
            if arg.contains(char::is_whitespace) { format!("\"{arg}\"") } else { arg.into_owned() }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn send(config: Option<&ClientConfig>, kind: &str, payload: Value) -> bool {
    let Some(config) = config else { return false };
    let request = client::request(&config.token, SOURCE, kind, payload, false);
    matches!(client::send(config, &request), Ok(Some(_)))
}

/// Ctrl+C reaches the command and `ilot run` alike: `ilot run` lets the command decide,
/// then reports how it ended. (The handler is not inherited: the command keeps the
/// usual Ctrl+C.)
fn outlive_ctrl_c() {
    unsafe extern "system" fn handled(kind: u32) -> BOOL {
        BOOL::from(kind == CTRL_C_EVENT || kind == CTRL_BREAK_EVENT)
    }
    // SAFETY: the handler only compares its argument and lives as long as the process.
    let _ = unsafe { SetConsoleCtrlHandler(Some(handled), true) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use ilot_bridge::protocol::{Reply, Request, encode_line};
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::ServerOptions;

    type Log = Arc<Mutex<Vec<Request>>>;

    /// A fake îlot on a test pipe: keeps every request and acknowledges it, like îlot.
    fn fake_ilot(test: &str) -> (ClientConfig, Log, tokio::runtime::Runtime) {
        let pipe = format!(r"\\.\pipe\ilot-cli-test-{test}-{}", std::process::id());
        let log = Log::default();
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        let mut server = {
            let _guard = runtime.enter();
            ServerOptions::new().first_pipe_instance(true).create(&pipe).unwrap()
        };
        let (name, seen) = (pipe.clone(), Arc::clone(&log));
        runtime.spawn(async move {
            loop {
                if server.connect().await.is_err() {
                    continue;
                }
                let next = ServerOptions::new().create(&name).unwrap();
                let mut reader = BufReader::new(std::mem::replace(&mut server, next));
                let mut line = Vec::new();
                if reader.read_until(b'\n', &mut line).await.is_ok() {
                    if let Ok(request) = serde_json::from_slice::<Request>(&line) {
                        seen.lock().unwrap().push(request);
                    }
                    let _ = reader.get_mut().write_all(&encode_line(&Reply::default()).unwrap()).await;
                }
            }
        });
        let config = ClientConfig {
            pipe_name: pipe,
            token: "t".into(),
            expected_server: Some(std::env::current_exe().unwrap()),
            connect_timeout: Duration::from_millis(500),
            answer_timeout: RECEIPT_TIMEOUT,
        };
        (config, log, runtime)
    }

    fn events(log: &Log) -> Vec<(String, Value)> {
        log.lock().unwrap().iter().map(|r| (r.event.clone(), r.payload.clone())).collect()
    }

    fn who() -> Who {
        Who { source: "tests".into(), project: None }
    }

    fn cmd(script: &str) -> Vec<OsString> {
        ["cmd.exe", "/D", "/C", script].iter().map(OsString::from).collect()
    }

    #[test]
    fn emit_sends_one_generic_event() {
        let (config, log, _rt) = fake_ilot("emit");
        let command = args::parse(["emit", "--source", "ci", "--kind", "working", "--title", "Analyse des logs"].map(OsString::from)).unwrap();
        assert_eq!(execute(command, Some(&config), Path::new(r"C:\dev\epilot")), 0);
        let request = &log.lock().unwrap()[0];
        assert_eq!((request.source.as_str(), request.event.as_str(), request.wait), ("generic", "working", false));
        assert_eq!(request.token, "t");
        assert_eq!(
            request.payload,
            json!({ "name": "ci", "session_id": "ci@epilot", "cwd": r"C:\dev\epilot", "project": "epilot", "title": "Analyse des logs", "detail": null })
        );
    }

    #[test]
    fn run_reports_the_start_and_a_failure_with_the_same_exit_code() {
        let (config, log, _rt) = fake_ilot("fail");
        assert_eq!(run(Some(&config), &who(), &cmd("exit 3"), Path::new(r"C:\dev\site")), 3);
        let events = events(&log);
        let kinds: Vec<_> = events.iter().map(|(e, _)| e.as_str()).collect();
        assert_eq!(kinds, ["session_start", "working", "error"]);
        assert_eq!(events[1].1["title"], "cmd.exe /D /C \"exit 3\"");
        assert_eq!(events[2].1["detail"], "code de sortie 3");
        assert!(events.iter().all(|(_, p)| p["session_id"] == "tests@site"));
    }

    #[test]
    fn run_reports_success() {
        let (config, log, _rt) = fake_ilot("ok");
        let who = Who { source: "tests".into(), project: Some("epilot".into()) };
        assert_eq!(run(Some(&config), &who, &cmd("exit 0"), Path::new(r"C:\dev\site")), 0);
        let events = events(&log);
        assert_eq!(events.last().unwrap().0, "turn_done");
        assert_eq!(events[0].1["session_id"], "tests@epilot");
    }

    #[test]
    fn a_missing_program_fails_like_a_shell_would() {
        let (config, log, _rt) = fake_ilot("missing");
        let command = vec![OsString::from("ilot-no-such-program-here")];
        assert_eq!(run(Some(&config), &who(), &command, Path::new(r"C:\dev\site")), 127);
        assert_eq!(events(&log).last().unwrap().0, "error");
    }

    #[test]
    fn without_ilot_run_still_runs_and_emit_says_so() {
        let missing = ClientConfig {
            pipe_name: r"\\.\pipe\ilot-cli-test-nobody-here".into(),
            token: "t".into(),
            expected_server: None,
            connect_timeout: Duration::from_millis(100),
            answer_timeout: Duration::from_millis(100),
        };
        for config in [None, Some(&missing)] {
            assert_eq!(run(config, &who(), &cmd("exit 4"), Path::new(r"C:\x")), 4);
            let emit = args::parse(["emit", "--source", "ci", "--kind", "error"].map(OsString::from)).unwrap();
            assert_eq!(execute(emit, config, Path::new(r"C:\x")), 1);
        }
    }

    #[test]
    fn the_wsl_halves_send_the_same_events_as_run() {
        let (config, log, _rt) = fake_ilot("halves");
        let who = Who { source: "tests".into(), project: Some("site".into()) };
        let begin = Command::RunBegin { who: who.clone(), command: vec!["make".into(), "all".into()] };
        assert_eq!(execute(begin, Some(&config), Path::new(r"\\wsl.localhost\Ubuntu\home\user\site")), 0);
        assert_eq!(execute(Command::RunEnd { who, code: 130 }, Some(&config), Path::new("/")), 0);
        let events = events(&log);
        let kinds: Vec<_> = events.iter().map(|(e, _)| e.as_str()).collect();
        assert_eq!(kinds, ["session_start", "working", "error"]);
        assert_eq!(events[1].1["title"], "make all");
        assert_eq!(events[2].1["detail"], "code de sortie 130");
    }

    #[test]
    fn a_pipe_served_by_another_program_hears_nothing() {
        let (mut config, log, _rt) = fake_ilot("squat");
        config.expected_server = Some(r"C:\Program Files\ilot\ilot.exe".into());
        assert!(!send(Some(&config), "working", json!({})));
        assert!(log.lock().unwrap().is_empty());
    }
}
