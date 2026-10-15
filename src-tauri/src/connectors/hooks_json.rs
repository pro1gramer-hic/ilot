//! Editing the `hooks` block of an agent's JSON settings. Claude Code
//! (`~/.claude/settings.json`), Gemini CLI (`~/.gemini/settings.json`), Codex
//! (`~/.codex/hooks.json`) and IBM Bob (`~/.bob/settings/settings.json`) share its shape:
//! event → groups → handlers; Cursor (`~/.cursor/hooks.json`) lists the handlers right
//! under the event (`HandlerFormat::flat`). Only the handler differs (`HandlerFormat`).
//! îlot's handlers are the ones that start its relay with plain arguments
//! (`is_ilot_handler`); nothing else is ever touched.
//!
//! The file is written back as indented JSON (its indentation kept). A symbolic link
//! stays a link (its target is written), a file inside WSL keeps its permissions, and a
//! write the agent makes at the same time is never lost (`edit`).

use super::Status;
use crate::atomic::replace_atomic;
use crate::locale::{Text, tr};
use crate::wsl::{self, sh_quote};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::io;
use std::path::{Path, PathBuf};

pub const BACKUP_SUFFIX: &str = ".avant-ilot";
/// The relay under Windows, next to îlot wherever it is installed.
const RELAY_EXE: &str = "ilot-hook.exe";
/// Read, change, write: tries before giving up when the agent keeps writing the file.
const ATTEMPTS: usize = 3;

/// One handler îlot adds for one event.
pub struct HookSpec<'a> {
    pub event: &'a str,
    pub timeout_s: u32,
    /// Fire and forget: the agent does not wait for the relay.
    pub background: bool,
    /// The group's `matcher` (a tool name): the handler only runs for it.
    pub matcher: Option<&'a str>,
    /// The event the relay is told, when it is not `event` (one hook event split in two
    /// handlers, such as Claude's `PreToolUse` and its `AskUserQuestion` question).
    pub relay_event: Option<&'a str>,
}

impl<'a> HookSpec<'a> {
    pub const fn new(event: &'a str, timeout_s: u32, background: bool) -> Self {
        Self { event, timeout_s, background, matcher: None, relay_event: None }
    }

    fn relay_event(&self) -> &'a str {
        match self.relay_event {
            Some(event) => event,
            None => self.event,
        }
    }
}

/// How an agent starts a hook given as one command line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellKind {
    /// `bash -c` (Gemini CLI) or `$SHELL -lc` (Codex), inside WSL.
    Posix,
    /// `powershell -Command`: Gemini CLI on Windows.
    PowerShell,
    /// `cmd.exe /C`: Codex on Windows.
    Cmd,
}

impl ShellKind {
    /// The relay and its arguments as one command line for this shell. Every piece comes
    /// from îlot (the relay's path, the source and event names), never from an agent.
    ///
    /// One limit stays: `Posix` quotes for `sh`, and Codex runs the line with `$SHELL -lc`.
    /// A login shell that is not POSIX (fish, nu) reads `'\''` and `\` its own way, so
    /// with a home folder holding `'` or `\` the hook fails and Codex asks in its terminal.
    pub fn command_line(self, program: &str, args: &[&str]) -> io::Result<String> {
        if let Some(arg) = args.iter().find(|a| !is_plain_arg(a)) {
            return Err(io::Error::other(tr(Text::UnsafeArgument { arg })));
        }
        let program = match self {
            Self::Posix => sh_quote(program),
            Self::PowerShell => format!("& '{}'", powershell_escape(program)),
            // Inside double quotes cmd only reads `"`, `%`, and `!` when delayed expansion
            // is on in the registry (`DelayedExpansion`): a path holding one is refused.
            Self::Cmd if program.contains(['"', '%', '!']) => {
                return Err(io::Error::other(tr(Text::PathNotForCmd { path: program })));
            }
            Self::Cmd => format!("\"{program}\""),
        };
        Ok(std::iter::once(program).chain(args.iter().map(|a| (*a).to_owned())).collect::<Vec<_>>().join(" "))
    }
}

/// The only arguments îlot puts on a command line: source and event names.
fn is_plain_arg(arg: &str) -> bool {
    !arg.is_empty() && arg.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn is_powershell_quote(c: char) -> bool {
    matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{201b}')
}

/// PowerShell ends a single-quoted string at `'` and at the typographic single quotes;
/// doubling any of them keeps it as a plain character. `&` then reads the path as a
/// wildcard pattern (`C:\a[1]\x.exe` starts `C:\a1\x.exe` when that one exists): a
/// backtick before `[`, `]` and `` ` `` keeps them as they are.
fn powershell_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if is_powershell_quote(c) {
            out.push(c);
        } else if matches!(c, '[' | ']' | '`') {
            out.push('`');
        }
        out.push(c);
    }
    out
}

/// The program a handler starts and what follows it. A command line îlot wrote starts
/// with the program quoted for its shell (`ShellKind::command_line`); any other `command`
/// is a program on its own (the exec form of Claude Code).
fn split_program(command: &str) -> Option<(String, &str)> {
    if let Some(quoted) = command.strip_prefix("& '") {
        unquote_powershell(quoted)
    } else if let Some(quoted) = command.strip_prefix('\'') {
        unquote_posix(quoted)
    } else if let Some(quoted) = command.strip_prefix('"') {
        quoted.split_once('"').map(|(program, rest)| (program.to_owned(), rest))
    } else {
        Some((command.to_owned(), ""))
    }
}

/// After the opening quote of `sh_quote`: `'\''` is a quote inside the path.
fn unquote_posix(mut text: &str) -> Option<(String, &str)> {
    let mut program = String::new();
    loop {
        let (part, rest) = text.split_once('\'')?;
        program.push_str(part);
        match rest.strip_prefix(r"\''") {
            Some(next) => {
                program.push('\'');
                text = next;
            }
            None => return Some((program, rest)),
        }
    }
}

/// After the opening quote of `powershell_escape`: a doubled quote is one quote, a
/// backtick keeps the next character.
fn unquote_powershell(text: &str) -> Option<(String, &str)> {
    let mut program = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '`' {
            program.push(chars.next()?.1);
        } else if is_powershell_quote(c) && chars.next_if(|&(_, next)| is_powershell_quote(next)).is_none() {
            return Some((program, &text[i + c.len_utf8()..]));
        } else {
            program.push(c);
        }
    }
    None
}

/// îlot's relay: the WSL script `~/.local/bin/ilot-hook`, or `ilot-hook.exe` under
/// Windows wherever îlot is (a moved install or a dev build still comes off).
fn is_relay(program: &str) -> bool {
    let script = program.starts_with('/') && program.ends_with(&format!("/{}", wsl::RELAY_SCRIPT));
    let exe = Path::new(program).is_absolute()
        && program.rsplit(['\\', '/']).next().is_some_and(|name| name.eq_ignore_ascii_case(RELAY_EXE));
    script || exe
}

/// How the agent calls the relay.
#[derive(Clone, Copy, Debug)]
pub enum Call {
    /// Claude Code: `command` is the program, `args` its arguments, no shell.
    Exec,
    /// Gemini CLI and Codex: `command` is a command line for their shell.
    Shell(ShellKind),
}

/// The shape of îlot's handler in one agent's config.
#[derive(Clone, Copy, Debug)]
pub struct HandlerFormat {
    pub call: Call,
    /// Gemini counts `timeout` in milliseconds, Claude and Codex in seconds.
    pub timeout_in_ms: bool,
    /// Key that runs a handler in the background (`async`), for agents that have one.
    pub background_key: Option<&'static str>,
    /// `name` written on each handler (Gemini lists and disables hooks by name).
    pub name: Option<&'static str>,
    /// Handlers go right under the event, without groups nor `type` (Cursor).
    pub flat: bool,
    /// `version` the file must hold at its root (Cursor: `1`), added when missing.
    pub version: Option<u32>,
}

/// îlot's handlers start its relay, followed only by the plain arguments îlot writes. A
/// command that merely mentions `ilot-hook` is the user's own.
fn is_ilot_handler(handler: &Value) -> bool {
    let Some((program, rest)) = handler.get("command").and_then(Value::as_str).and_then(split_program) else {
        return false;
    };
    let plain = rest.is_empty() || rest.strip_prefix(' ').is_some_and(|args| args.split(' ').all(is_plain_arg));
    plain && is_relay(&program)
}

/// The arguments of a command line that starts îlot's relay with plain arguments only
/// (`'/home/u/.local/bin/ilot-hook' kiro Stop` → `["kiro", "Stop"]`); `None` for any
/// other command.
pub fn relay_args(command: &str) -> Option<Vec<String>> {
    let (program, rest) = split_program(command)?;
    let args: Vec<String> = rest.split(' ').filter(|arg| !arg.is_empty()).map(str::to_owned).collect();
    let plain = rest.is_empty() || (rest.starts_with(' ') && rest[1..].split(' ').all(is_plain_arg));
    (plain && is_relay(&program)).then_some(args)
}

/// True for îlot's relay given as a program on its own (exec form).
pub fn is_relay_program(program: &str) -> bool {
    is_relay(program)
}

/// The event îlot's handler tells the relay: its second argument.
fn relay_event_of(handler: &Value) -> Option<String> {
    if let Some(args) = handler.get("args").and_then(Value::as_array) {
        return args.get(1).and_then(Value::as_str).map(str::to_owned);
    }
    let (_, rest) = split_program(handler.get("command")?.as_str()?)?;
    rest.split(' ').filter(|arg| !arg.is_empty()).nth(1).map(str::to_owned)
}

/// The handlers listed under one event: those of its groups (`{"hooks": […]}`), and those
/// listed right under it (Cursor).
fn handlers(groups: &Value) -> impl Iterator<Item = &Value> {
    groups.as_array().into_iter().flatten().flat_map(|entry| match entry.get("hooks").and_then(Value::as_array) {
        Some(handlers) => handlers.iter().collect::<Vec<_>>(),
        None => vec![entry],
    })
}

/// True when one of the event's entries holds îlot's handler for `spec`: in a group with
/// the same matcher, or right under the event.
fn holds_spec(groups: &Value, spec: &HookSpec<'_>) -> bool {
    let ours = |handler: &Value| is_ilot_handler(handler) && relay_event_of(handler).as_deref() == Some(spec.relay_event());
    groups.as_array().into_iter().flatten().any(|entry| match entry.get("hooks").and_then(Value::as_array) {
        Some(handlers) => entry.get("matcher").and_then(Value::as_str) == spec.matcher && handlers.iter().any(ours),
        None => ours(entry),
    })
}

fn holds_ilot_handlers(settings: &Value) -> bool {
    settings
        .get("hooks")
        .and_then(Value::as_object)
        .is_some_and(|hooks| hooks.values().flat_map(handlers).any(is_ilot_handler))
}

fn handler(relay: &str, source: &str, spec: &HookSpec<'_>, format: &HandlerFormat) -> io::Result<Value> {
    let mut handler = Map::new();
    // Cursor's handlers have no `type` (https://cursor.com/docs/hooks, "Configuration").
    if !format.flat {
        handler.insert("type".into(), json!("command"));
    }
    if let Some(name) = format.name {
        handler.insert("name".into(), json!(name));
    }
    match format.call {
        Call::Exec => {
            handler.insert("command".into(), json!(relay));
            handler.insert("args".into(), json!([source, spec.relay_event()]));
        }
        Call::Shell(shell) => {
            handler.insert("command".into(), json!(shell.command_line(relay, &[source, spec.relay_event()])?));
        }
    }
    let timeout = if format.timeout_in_ms { spec.timeout_s * 1000 } else { spec.timeout_s };
    handler.insert("timeout".into(), json!(timeout));
    if spec.background
        && let Some(key) = format.background_key
    {
        handler.insert(key.into(), json!(true));
    }
    Ok(Value::Object(handler))
}

/// Adds îlot's handlers (after removing older ones, so connecting twice repairs).
/// `relay` is the relay as the agent sees it; its arguments are the source and the event.
pub fn add_hooks(settings: &mut Value, relay: &str, source: &str, specs: &[HookSpec<'_>], format: &HandlerFormat) -> io::Result<()> {
    let handlers = specs.iter().map(|spec| handler(relay, source, spec, format)).collect::<io::Result<Vec<_>>>()?;
    remove_hooks(settings);
    if !settings.is_object() {
        *settings = json!({});
    }
    let root = settings.as_object_mut().expect("object");
    if let Some(version) = format.version {
        root.entry("version").or_insert_with(|| json!(version));
    }
    let hooks = root.entry("hooks").or_insert_with(|| json!({}));
    if !hooks.is_object() {
        *hooks = json!({});
    }
    let hooks = hooks.as_object_mut().expect("object");
    for (spec, handler) in specs.iter().zip(handlers) {
        let groups = hooks.entry(spec.event).or_insert_with(|| json!([]));
        if let Some(groups) = groups.as_array_mut() {
            groups.push(match (format.flat, spec.matcher) {
                (true, _) => handler,
                (false, Some(matcher)) => json!({ "matcher": matcher, "hooks": [handler] }),
                (false, None) => json!({ "hooks": [handler] }),
            });
        }
    }
    Ok(())
}

/// Removes îlot's handlers only, then the groups, events and `hooks` block left empty.
/// Returns true when something was removed.
pub fn remove_hooks(settings: &mut Value) -> bool {
    let Some(root) = settings.as_object_mut() else { return false };
    let Some(hooks) = root.get_mut("hooks").and_then(Value::as_object_mut) else { return false };
    let mut removed = false;
    for groups in hooks.values_mut() {
        let Some(groups) = groups.as_array_mut() else { continue };
        for group in groups.iter_mut() {
            if let Some(handlers) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                let before = handlers.len();
                handlers.retain(|h| !is_ilot_handler(h));
                removed |= handlers.len() != before;
            }
        }
        // Handlers listed right under the event (Cursor).
        let before = groups.len();
        groups.retain(|entry| entry.get("hooks").is_some() || !is_ilot_handler(entry));
        removed |= groups.len() != before;
        groups.retain(|g| g.get("hooks").and_then(Value::as_array).is_none_or(|h| !h.is_empty()));
    }
    if removed {
        hooks.retain(|_, groups| groups.as_array().is_none_or(|g| !g.is_empty()));
        if hooks.is_empty() {
            root.remove("hooks");
        }
    }
    removed
}

/// How many of the given handlers are in place (same event, matcher and relay event): a
/// connection made before a handler was added counts as partial, to be repaired.
pub fn connected_events(settings: &Value, specs: &[HookSpec<'_>]) -> usize {
    let Some(hooks) = settings.get("hooks") else { return 0 };
    specs.iter().filter(|spec| hooks.get(spec.event).is_some_and(|groups| holds_spec(groups, spec))).count()
}

/// Indentation of a JSON text, to write the file back the way it was written.
pub fn detect_indent(text: &str) -> String {
    text.lines()
        .skip(1)
        .find(|line| !line.trim().is_empty())
        .map(|line| line.chars().take_while(|c| *c == ' ' || *c == '\t').collect::<String>())
        .filter(|indent| !indent.is_empty())
        .unwrap_or_else(|| "  ".to_owned())
}

pub fn to_pretty(value: &Value, indent: &str) -> io::Result<String> {
    let mut out = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(indent.as_bytes());
    let mut serializer = serde_json::Serializer::with_formatter(&mut out, formatter);
    value.serialize(&mut serializer).map_err(io::Error::other)?;
    out.push(b'\n');
    String::from_utf8(out).map_err(io::Error::other)
}

pub fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(BACKUP_SUFFIX);
    path.with_file_name(name)
}

/// The file to read and write. A symbolic link (dotfiles) is followed, so the link stays
/// a link: by Linux inside WSL (Windows cannot follow it through `\\wsl.localhost`), by
/// Windows otherwise.
fn real_file(path: &Path) -> io::Result<PathBuf> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            if wsl::split_unc(path).is_some() { wsl::real_path(path) } else { std::fs::canonicalize(path) }
        }
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(path.to_owned()),
    }
}

/// A file's bytes; `None` when it is missing.
fn read_bytes(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// A settings file as read: its bytes (`None` when missing), its value, its indentation.
struct Snapshot {
    bytes: Option<Vec<u8>>,
    settings: Value,
    indent: String,
}

/// Reads a settings file; a missing file reads as an empty object. A file that is not
/// valid JSON is an error: îlot never overwrites what it cannot understand.
fn read_settings(path: &Path) -> io::Result<Snapshot> {
    let bytes = read_bytes(path)?;
    let invalid = |e: &dyn std::fmt::Display| io::Error::new(io::ErrorKind::InvalidData, format!("{}: {e}", path.display()));
    let text = match &bytes {
        Some(bytes) => std::str::from_utf8(bytes).map_err(|e| invalid(&e))?,
        None => "",
    };
    let (settings, indent) = if text.trim().is_empty() {
        (json!({}), "  ".to_owned())
    } else {
        (serde_json::from_str(text).map_err(|e| invalid(&e))?, detect_indent(text))
    };
    Ok(Snapshot { bytes, settings, indent })
}

/// Writes a file atomically. Inside WSL it first gets `mode`, the permissions of the
/// file it stands for (written through `\\wsl.localhost`, it would get `644`).
fn write(path: &Path, bytes: &[u8], mode: Option<&str>, unchanged: impl FnOnce() -> io::Result<bool>) -> io::Result<bool> {
    replace_atomic(path, bytes, |tmp| mode.map_or(Ok(()), |mode| wsl::set_mode(tmp, mode)), unchanged)
}

/// Reads the settings, lets `change` edit them (`false`: nothing to write), and writes
/// them back only if the file is still as it was read: when the agent wrote it
/// meanwhile, îlot starts again from its new content. Before îlot's handlers go into a
/// file that has none, the file is saved next to the path given (`<file>.avant-ilot`,
/// kept after the disconnection); a file that only existed for îlot (`{}` left, no
/// backup; `leftover` lists the root keys îlot itself adds, such as Cursor's `version`)
/// is deleted.
fn edit(path: &Path, leftover: &[&str], mut change: impl FnMut(&mut Value) -> io::Result<bool>) -> io::Result<()> {
    let real = real_file(path)?;
    let backup = backup_path(path);
    for _ in 0..ATTEMPTS {
        let Snapshot { bytes, mut settings, indent } = read_settings(&real)?;
        let without_ilot = !holds_ilot_handlers(&settings);
        if !change(&mut settings)? {
            return Ok(());
        }
        let unchanged = || -> io::Result<bool> { Ok(read_bytes(&real)? == bytes) };
        let only_ilot = settings.as_object().is_some_and(|root| root.keys().all(|key| leftover.contains(&key.as_str())));
        let done = if only_ilot && !backup.exists() {
            let same = unchanged()?;
            if same {
                remove_file(&real)?;
            }
            same
        } else {
            let mode = match &bytes {
                Some(_) if wsl::split_unc(&real).is_some() => Some(wsl::file_mode(&real)?),
                _ => None,
            };
            if let Some(original) = &bytes
                && without_ilot
            {
                write(&backup, original, mode.as_deref(), || Ok(true))?;
            }
            write(&real, to_pretty(&settings, &indent)?.as_bytes(), mode.as_deref(), unchanged)?
        };
        if done {
            return Ok(());
        }
    }
    Err(io::Error::other(tr(Text::FileChanging { path: &path.display().to_string() })))
}

fn remove_file(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Where an agent stands: its folder missing means not installed; all of îlot's events
/// present means connected; some of them, or a file îlot cannot read, needs a repair.
pub fn status(path: &Path, specs: &[HookSpec<'_>]) -> Status {
    if !path.parent().is_some_and(Path::exists) {
        return Status::NotInstalled;
    }
    match real_file(path).and_then(|real| read_settings(&real)) {
        Ok(snapshot) => match connected_events(&snapshot.settings, specs) {
            0 => Status::Installed,
            n if n == specs.len() => Status::Connected,
            _ => Status::NeedsRepair,
        },
        Err(_) => Status::NeedsRepair,
    }
}

/// Plugs îlot's handlers into a settings file.
pub fn plug(path: &Path, relay: &str, source: &str, specs: &[HookSpec<'_>], format: &HandlerFormat) -> io::Result<()> {
    edit(path, &[], |settings| add_hooks(settings, relay, source, specs, format).map(|()| true))
}

/// Takes îlot's handlers out of a settings file, leaving the rest as it was.
pub fn unplug(path: &Path) -> io::Result<()> {
    unplug_with_root(path, &[])
}

/// Same, for a file whose root also holds keys îlot adds (`HandlerFormat::version`): a
/// file îlot created leaves with them.
pub fn unplug_with_root(path: &Path, root_keys: &[&str]) -> io::Result<()> {
    edit(path, root_keys, |settings| Ok(remove_hooks(settings)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPECS: [HookSpec<'static>; 2] = [HookSpec::new("PreToolUse", 10, true), HookSpec::new("PermissionRequest", 120, false)];
    const EXEC: HandlerFormat =
        HandlerFormat { call: Call::Exec, timeout_in_ms: false, background_key: Some("async"), name: None, flat: false, version: None };
    const PLAIN: HandlerFormat =
        HandlerFormat { call: Call::Exec, timeout_in_ms: false, background_key: None, name: None, flat: false, version: None };
    /// The relay script as a WSL hook calls it.
    const RELAY: &str = "/home/user/.local/bin/ilot-hook";

    /// A real-world shaped user config with its own hooks and settings.
    fn user_settings() -> Value {
        json!({
            "permissions": { "allow": ["Bash(npm run:*)"] },
            "hooks": {
                "PostToolUse": [
                    { "matcher": "Write|Edit", "hooks": [{ "type": "command", "command": "prettier --write" }] }
                ]
            },
            "model": "opus"
        })
    }

    #[test]
    fn adds_one_handler_per_event_with_exec_args() {
        let mut settings = user_settings();
        add_hooks(&mut settings, RELAY, "claude-code", &SPECS, &EXEC).unwrap();
        let handler = &settings["hooks"]["PreToolUse"][0]["hooks"][0];
        assert_eq!(handler["command"], RELAY);
        assert_eq!(handler["args"], json!(["claude-code", "PreToolUse"]));
        assert_eq!(handler["async"], true);
        assert_eq!(settings["hooks"]["PermissionRequest"][0]["hooks"][0]["timeout"], 120);
        assert!(settings["hooks"]["PermissionRequest"][0]["hooks"][0].get("async").is_none());
        assert_eq!(connected_events(&settings, &SPECS), 2);
    }

    #[test]
    fn connect_then_disconnect_gives_back_the_same_settings() {
        let mut settings = user_settings();
        add_hooks(&mut settings, RELAY, "claude-code", &SPECS, &EXEC).unwrap();
        assert!(remove_hooks(&mut settings));
        assert_eq!(settings, user_settings());
        assert_eq!(serde_json::to_string(&settings).unwrap(), serde_json::to_string(&user_settings()).unwrap());
    }

    #[test]
    fn connecting_twice_does_not_duplicate() {
        let mut settings = json!({});
        add_hooks(&mut settings, RELAY, "claude-code", &SPECS, &PLAIN).unwrap();
        add_hooks(&mut settings, RELAY, "claude-code", &SPECS, &PLAIN).unwrap();
        assert_eq!(settings["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn removal_keeps_handlers_added_by_the_user_meanwhile() {
        let mut settings = json!({});
        add_hooks(&mut settings, RELAY, "claude-code", &SPECS, &PLAIN).unwrap();
        settings["hooks"]["PreToolUse"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "matcher": "Bash", "hooks": [{ "type": "command", "command": "audit.sh" }] }));
        remove_hooks(&mut settings);
        assert_eq!(settings, json!({ "hooks": { "PreToolUse": [{ "matcher": "Bash", "hooks": [{ "type": "command", "command": "audit.sh" }] }] } }));
    }

    #[test]
    fn nothing_to_remove_leaves_settings_alone() {
        let mut settings = user_settings();
        assert!(!remove_hooks(&mut settings));
        assert_eq!(settings, user_settings());
        assert_eq!(connected_events(&settings, &SPECS), 0);
    }

    #[test]
    fn only_handlers_that_start_the_relay_are_ilot_s() {
        let ours = [
            // Claude Code (exec form, since phase 2), in WSL and under Windows.
            json!({ "type": "command", "command": "/home/o'neil/.local/bin/ilot-hook", "args": ["claude-code", "Stop"], "async": true }),
            json!({ "type": "command", "command": r"C:\Users\HP 240\AppData\Local\îlot\ilot-hook.exe", "args": ["claude-code", "Stop"] }),
            // Gemini CLI and Codex (one command line), from a dev build too.
            json!({ "type": "command", "name": "ilot-hook", "command": r"'/home/o'\''neil/.local/bin/ilot-hook' gemini-cli BeforeTool", "timeout": 5000 }),
            json!({ "type": "command", "command": r"& 'C:\dev\ilot`[2`]\target\debug\ILOT-HOOK.EXE' gemini-cli AfterAgent" }),
            json!({ "type": "command", "command": r#""C:\Users\HP 240\ilot\ilot-hook.exe" codex PermissionRequest"#, "timeout": 120 }),
        ];
        let theirs = [
            json!({ "type": "command", "command": "my-ilot-hook-logger.sh" }),
            json!({ "type": "command", "command": "echo ilot-hook" }),
            json!({ "type": "command", "command": "/opt/ilot-hook/notify.sh Stop" }),
            json!({ "type": "command", "command": "/home/user/bin/ilot-hook", "args": ["claude-code", "Stop"] }),
            json!({ "type": "command", "command": "/home/user/.local/bin/ilot-hook claude-code Stop" }),
            json!({ "type": "command", "command": "'/home/user/.local/bin/ilot-hook' codex Stop; notify-send done" }),
            json!({ "type": "command", "command": "'/home/user/.local/bin/ilot-hook' codex Stop | tee -a hooks.log" }),
            json!({ "type": "command", "command": "'/home/user/.local/bin/ilot-hook" }),
            json!({ "type": "command", "command": "ilot-hook.exe codex Stop" }),
            json!({ "type": "command", "command": r"C:\tools\not-ilot-hook.exe" }),
        ];
        for handler in &ours {
            assert!(is_ilot_handler(handler), "{handler}");
        }
        for handler in &theirs {
            assert!(!is_ilot_handler(handler), "{handler}");
        }
        let all: Vec<Value> = ours.iter().chain(&theirs).cloned().collect();
        let mut settings = json!({ "hooks": { "Stop": [{ "hooks": all }] } });
        assert!(holds_ilot_handlers(&settings));
        assert!(remove_hooks(&mut settings));
        assert_eq!(settings["hooks"]["Stop"][0]["hooks"], json!(theirs));
        assert!(!holds_ilot_handlers(&settings));
    }

    #[test]
    fn shell_handlers_hold_one_command_line() {
        let gemini = HandlerFormat {
            call: Call::Shell(ShellKind::Posix),
            timeout_in_ms: true,
            background_key: None,
            name: Some("ilot-hook"),
            flat: false,
            version: None,
        };
        let mut settings = json!({});
        add_hooks(&mut settings, RELAY, "gemini-cli", &SPECS, &gemini).unwrap();
        assert_eq!(
            settings["hooks"]["PreToolUse"][0]["hooks"][0],
            json!({ "type": "command", "name": "ilot-hook", "command": "'/home/user/.local/bin/ilot-hook' gemini-cli PreToolUse", "timeout": 10000 })
        );
        assert_eq!(connected_events(&settings, &SPECS), 2);
        assert!(remove_hooks(&mut settings));
        assert_eq!(settings, json!({}));
    }

    #[test]
    fn command_lines_are_quoted_for_each_shell() {
        let windows = r"C:\Users\HP 240\ilot\ilot-hook.exe";
        assert_eq!(ShellKind::Posix.command_line("/home/o'neil/.local/bin/ilot-hook", &["codex", "Stop"]).unwrap(), r"'/home/o'\''neil/.local/bin/ilot-hook' codex Stop");
        assert_eq!(ShellKind::PowerShell.command_line(windows, &["gemini-cli", "AfterAgent"]).unwrap(), r"& 'C:\Users\HP 240\ilot\ilot-hook.exe' gemini-cli AfterAgent");
        assert_eq!(ShellKind::PowerShell.command_line(r"C:\Users\O'Neil\ilot-hook.exe", &[]).unwrap(), r"& 'C:\Users\O''Neil\ilot-hook.exe'");
        assert_eq!(ShellKind::PowerShell.command_line("C:\\D\u{2019}Arc\\ilot-hook.exe", &[]).unwrap(), "& 'C:\\D\u{2019}\u{2019}Arc\\ilot-hook.exe'");
        // Checked with Windows PowerShell 5.1: unescaped, `a[1]` starts `a1\ilot-hook.exe`.
        assert_eq!(ShellKind::PowerShell.command_line(r"C:\dev\a[1]\ilot-hook.exe", &[]).unwrap(), r"& 'C:\dev\a`[1`]\ilot-hook.exe'");
        assert_eq!(ShellKind::PowerShell.command_line(r"C:\dev\a`b\ilot-hook.exe", &[]).unwrap(), r"& 'C:\dev\a``b\ilot-hook.exe'");
        assert_eq!(ShellKind::Cmd.command_line(windows, &["codex", "PermissionRequest"]).unwrap(), r#""C:\Users\HP 240\ilot\ilot-hook.exe" codex PermissionRequest"#);
        assert!(ShellKind::Cmd.command_line(r"C:\100%\ilot-hook.exe", &[]).is_err());
        assert!(ShellKind::Cmd.command_line(r"C:\Users\Wow!\ilot-hook.exe", &[]).is_err());
        assert!(ShellKind::Cmd.command_line(r#"C:\a"b\ilot-hook.exe"#, &[]).is_err());
        assert!(ShellKind::Posix.command_line(RELAY, &["codex; rm -rf ~"]).is_err());
        assert!(ShellKind::Posix.command_line(RELAY, &[""]).is_err());
    }

    #[test]
    fn command_lines_read_back_into_the_relay() {
        let windows = [
            r"C:\Users\HP 240\AppData\Local\îlot\ilot-hook.exe",
            r"C:\Users\O'Neil\ilot-hook.exe",
            "C:\\D\u{2019}Arc\\ilot-hook.exe",
            r"C:\dev\a[1]\b`c\ilot (copie)\ilot-hook.exe",
        ];
        for path in windows {
            for shell in [ShellKind::PowerShell, ShellKind::Cmd] {
                let line = shell.command_line(path, &["codex", "Stop"]).unwrap();
                assert_eq!(split_program(&line), Some((path.to_owned(), " codex Stop")), "{line}");
                assert!(is_ilot_handler(&json!({ "command": line })), "{line}");
            }
        }
        for path in [RELAY, "/home/o'neil/.local/bin/ilot-hook", "/home/a b/.local/bin/ilot-hook"] {
            let line = ShellKind::Posix.command_line(path, &["gemini-cli", "BeforeTool"]).unwrap();
            assert_eq!(split_program(&line), Some((path.to_owned(), " gemini-cli BeforeTool")), "{line}");
            assert!(is_ilot_handler(&json!({ "command": line })), "{line}");
        }
    }

    /// Claude's question: a second `PreToolUse` handler, in a group of its own with
    /// `matcher: "AskUserQuestion"`, that tells the relay `AskUserQuestion`.
    const QUESTION: HookSpec<'static> =
        HookSpec { matcher: Some("AskUserQuestion"), relay_event: Some("AskUserQuestion"), ..HookSpec::new("PreToolUse", 60, false) };

    #[test]
    fn a_matcher_gets_its_own_group_and_relay_event() {
        let specs = [HookSpec::new("PreToolUse", 10, true), QUESTION];
        let mut settings = user_settings();
        add_hooks(&mut settings, RELAY, "claude-code", &specs, &EXEC).unwrap();
        let groups = settings["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(groups.len(), 2);
        assert!(groups[0].get("matcher").is_none());
        assert_eq!(groups[1]["matcher"], "AskUserQuestion");
        assert_eq!(groups[1]["hooks"][0], json!({ "type": "command", "command": RELAY, "args": ["claude-code", "AskUserQuestion"], "timeout": 60 }));
        assert_eq!(connected_events(&settings, &specs), 2);
        assert!(remove_hooks(&mut settings));
        assert_eq!(settings, user_settings());
    }

    #[test]
    fn a_connection_made_before_a_new_handler_is_to_repair() {
        // Plugged with the observation handler only, as before the question existed.
        let mut settings = user_settings();
        add_hooks(&mut settings, RELAY, "claude-code", &[HookSpec::new("PreToolUse", 10, true)], &EXEC).unwrap();
        let now = [HookSpec::new("PreToolUse", 10, true), QUESTION];
        assert_eq!(connected_events(&settings, &now), 1, "the old handler is not the question's");
        // A user's group with the same matcher but no îlot handler does not count either.
        settings["hooks"]["PreToolUse"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "matcher": "AskUserQuestion", "hooks": [{ "type": "command", "command": "my-script.sh" }] }));
        assert_eq!(connected_events(&settings, &now), 1);
    }

    #[test]
    fn flat_handlers_sit_right_under_the_event() {
        let cursor = HandlerFormat {
            call: Call::Shell(ShellKind::Posix),
            timeout_in_ms: false,
            background_key: None,
            name: None,
            flat: true,
            version: Some(1),
        };
        let specs = [HookSpec::new("stop", 10, false), HookSpec::new("afterFileEdit", 10, false)];
        let mut settings = json!({ "version": 1, "hooks": { "stop": [{ "command": "./audit.sh" }] } });
        add_hooks(&mut settings, RELAY, "cursor", &specs, &cursor).unwrap();
        assert_eq!(settings["hooks"]["stop"][1], json!({ "command": "'/home/user/.local/bin/ilot-hook' cursor stop", "timeout": 10 }));
        assert_eq!(connected_events(&settings, &specs), 2);
        assert!(remove_hooks(&mut settings));
        assert_eq!(settings, json!({ "version": 1, "hooks": { "stop": [{ "command": "./audit.sh" }] } }));

        let mut empty = json!({});
        add_hooks(&mut empty, RELAY, "cursor", &specs, &cursor).unwrap();
        assert_eq!(empty["version"], 1, "the schema version Cursor requires");
    }

    #[test]
    fn a_flat_file_created_for_ilot_leaves_with_it() {
        let path = temp("flat");
        let cursor = HandlerFormat { call: Call::Exec, timeout_in_ms: false, background_key: None, name: None, flat: true, version: Some(1) };
        plug(&path, RELAY, "cursor", &[HookSpec::new("stop", 10, false)], &cursor).unwrap();
        assert_eq!(status(&path, &[HookSpec::new("stop", 10, false)]), Status::Connected);
        unplug_with_root(&path, &["version"]).unwrap();
        assert!(!path.exists());
        // The user's own file keeps its `version`.
        std::fs::write(&path, "{\n  \"version\": 1\n}\n").unwrap();
        plug(&path, RELAY, "cursor", &[HookSpec::new("stop", 10, false)], &cursor).unwrap();
        unplug_with_root(&path, &["version"]).unwrap();
        assert_eq!(read(&path), "{\n  \"version\": 1\n}\n");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_missing_agent_folder_means_not_installed() {
        let path = std::env::temp_dir().join("ilot-no-agent-here").join("settings.json");
        assert_eq!(status(&path, &SPECS), Status::NotInstalled);
    }

    #[test]
    fn indentation_is_detected() {
        assert_eq!(detect_indent("{\n    \"a\": 1\n}"), "    ");
        assert_eq!(detect_indent("{\n\t\"a\": 1\n}"), "\t");
        assert_eq!(detect_indent("{}"), "  ");
    }

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ilot-hooks-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("settings.json")
    }

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn file_round_trip_restores_the_original_bytes() {
        let path = temp("bytes");
        let original = "{\n    \"permissions\": {\n        \"allow\": [\n            \"Read\"\n        ]\n    },\n    \"theme\": \"dark\"\n}\n";
        std::fs::write(&path, original).unwrap();
        plug(&path, RELAY, "claude-code", &SPECS, &EXEC).unwrap();
        assert_eq!(status(&path, &SPECS), Status::Connected);
        assert_eq!(read(&backup_path(&path)), original);
        unplug(&path).unwrap();
        assert_eq!(read(&path), original);
        assert_eq!(read(&backup_path(&path)), original, "the backup stays after the disconnection");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_file_created_for_ilot_disappears_on_removal() {
        let path = temp("created");
        plug(&path, RELAY, "claude-code", &SPECS, &PLAIN).unwrap();
        assert!(path.exists() && !backup_path(&path).exists());
        unplug(&path).unwrap();
        assert!(!path.exists());
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn invalid_json_is_never_overwritten() {
        let path = temp("invalid");
        std::fs::write(&path, "{ \"a\": ").unwrap();
        assert!(plug(&path, RELAY, "claude-code", &SPECS, &PLAIN).is_err());
        assert_eq!(read(&path), "{ \"a\": ");
        assert_eq!(status(&path, &SPECS), Status::NeedsRepair);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn the_backup_is_taken_again_at_each_new_connection() {
        let path = temp("backup");
        std::fs::write(&path, "{\n  \"model\": \"opus\"\n}\n").unwrap();
        plug(&path, RELAY, "claude-code", &SPECS, &EXEC).unwrap();
        unplug(&path).unwrap();
        let edited = "{\n  \"model\": \"sonnet\"\n}\n";
        std::fs::write(&path, edited).unwrap();
        plug(&path, RELAY, "claude-code", &SPECS, &EXEC).unwrap();
        assert_eq!(read(&backup_path(&path)), edited, "a new connection saves the file as it is now");
        plug(&path, RELAY, "claude-code", &SPECS, &EXEC).unwrap();
        assert_eq!(read(&backup_path(&path)), edited, "a repair keeps the file as it was without îlot");
        unplug(&path).unwrap();
        assert_eq!(read(&path), edited);
        assert_eq!(read(&backup_path(&path)), edited);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_write_by_the_agent_meanwhile_is_kept() {
        let path = temp("race");
        std::fs::write(&path, "{\n  \"model\": \"opus\"\n}\n").unwrap();
        let theirs = "{\n  \"model\": \"opus\",\n  \"theme\": \"dark\"\n}\n";
        let mut reads = 0;
        edit(&path, &[], |settings| {
            reads += 1;
            if reads == 1 {
                // The agent writes its file between îlot's read and îlot's write.
                std::fs::write(&path, theirs)?;
            }
            add_hooks(settings, RELAY, "claude-code", &SPECS, &PLAIN).map(|()| true)
        })
        .unwrap();
        assert_eq!(reads, 2);
        let written: Value = serde_json::from_str(&read(&path)).unwrap();
        assert_eq!(written["theme"], "dark");
        assert_eq!(connected_events(&written, &SPECS), 2);
        assert_eq!(read(&backup_path(&path)), theirs);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_file_that_keeps_changing_is_left_to_the_agent() {
        let path = temp("busy");
        std::fs::write(&path, "{}\n").unwrap();
        let mut reads = 0;
        let edited = edit(&path, &[], |settings| {
            reads += 1;
            std::fs::write(&path, format!("{{ \"n\": {reads} }}\n"))?;
            add_hooks(settings, RELAY, "claude-code", &SPECS, &PLAIN).map(|()| true)
        });
        assert!(edited.is_err());
        assert_eq!(reads, ATTEMPTS);
        assert_eq!(read(&path), format!("{{ \"n\": {ATTEMPTS} }}\n"));
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// Under Windows, a link to the settings is followed too (creating one needs the
    /// developer mode or an administrator: without either, nothing to check).
    #[test]
    fn a_linked_settings_file_stays_a_link() {
        let path = temp("link");
        let target = path.with_file_name("dotfiles-settings.json");
        let original = "{\n  \"model\": \"opus\"\n}\n";
        std::fs::write(&target, original).unwrap();
        if let Err(e) = std::os::windows::fs::symlink_file(&target, &path) {
            eprintln!("no link possible here: {e}");
            std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
            return;
        }
        plug(&path, RELAY, "claude-code", &SPECS, &EXEC).unwrap();
        assert!(std::fs::symlink_metadata(&path).unwrap().file_type().is_symlink());
        assert_eq!(status(&path, &SPECS), Status::Connected);
        assert_eq!(read(&backup_path(&path)), original);
        unplug(&path).unwrap();
        assert!(std::fs::symlink_metadata(&path).unwrap().file_type().is_symlink());
        assert_eq!(read(&target), original);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    /// Real WSL, on a throwaway home under `ILOT_WSL_TEST_DIR` (or `/tmp`): a private
    /// settings file (`600`) stays private, backup included, and a linked one (dotfiles)
    /// stays a link.
    #[test]
    #[ignore = "needs WSL; run with --ignored"]
    fn modes_and_links_inside_wsl_on_a_throwaway_home() {
        let distro = wsl::distros().into_iter().next().expect("a WSL distribution");
        let base = std::env::var("ILOT_WSL_TEST_DIR").unwrap_or_else(|_| "/tmp".into());
        let home = format!("{base}/ilot-test-modes-{}", std::process::id());
        let linux = |args: &[&str]| {
            let mut all = vec!["-d", distro.as_str(), "--exec"];
            all.extend_from_slice(args);
            let out = std::process::Command::new(wsl::system_program("wsl.exe").unwrap()).args(&all).output().unwrap();
            (out.status.success(), String::from_utf8_lossy(&out.stdout).trim_end().to_owned())
        };
        let original = "{\n  \"env\": {\n    \"API_KEY\": \"secret\"\n  }\n}\n";
        let dotfile = format!("{home}/dotfiles/claude.json");
        let link = format!("{home}/.claude/settings.json");
        let private = format!("{home}/.gemini/settings.json");
        assert!(linux(&["mkdir", "-p", &format!("{home}/dotfiles"), &format!("{home}/.claude"), &format!("{home}/.gemini")]).0);
        for file in [&dotfile, &private] {
            std::fs::write(wsl::unc_path(&distro, file), original).unwrap();
            assert!(linux(&["chmod", "600", file]).0);
        }
        assert!(linux(&["ln", "-s", "../dotfiles/claude.json", &link]).0);
        for settings in [&link, &private] {
            let path = wsl::unc_path(&distro, settings);
            plug(&path, &format!("{home}/.local/bin/ilot-hook"), "claude-code", &SPECS, &EXEC).unwrap();
            assert_eq!(status(&path, &SPECS), Status::Connected, "{settings}");
            assert!(linux(&["cat", settings]).1.contains("/.local/bin/ilot-hook"), "{settings}");
            assert_eq!(linux(&["stat", "-L", "-c", "%a", settings]).1, "600", "{settings}");
            let backup = format!("{settings}{BACKUP_SUFFIX}");
            assert_eq!(read(&wsl::unc_path(&distro, &backup)), original, "{backup}");
            assert_eq!(linux(&["stat", "-c", "%a", &backup]).1, "600", "{backup}");
            unplug(&path).unwrap();
            assert_eq!(linux(&["cat", settings]).1, original.trim_end(), "{settings}");
            assert_eq!(linux(&["stat", "-L", "-c", "%a", settings]).1, "600", "{settings}");
        }
        assert!(linux(&["test", "-L", &link]).0, "the link stays a link");
        assert_eq!(linux(&["readlink", &link]).1, "../dotfiles/claude.json");
        assert!(linux(&["rm", "-r", &home]).0);
    }

    #[test]
    fn a_refused_argument_is_said_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let error = ShellKind::Posix.command_line("/usr/bin/ilot-hook", &["a b"]).unwrap_err();
        assert_eq!(error.to_string(), "unsafe argument: a b");
    }
}
