//! Claude Code: hooks in `~/.claude/settings.json`, full activity and permissions.
//! Formats from https://code.claude.com/docs/en/hooks, checked against real captures
//! in `tests/fixtures/claude-code/` (docs/JOURNAL.md, phase 2).

use super::hook_events::{self, cut};
use super::hooks_json::{self, Call, HandlerFormat, HookSpec};
use super::questions::{self, QuestionAnswer};
use super::{Answer, Caps, Connector, PermissionView, Status, Target, text, tidy_relay};
use crate::events::{AgentEvent, Source};
use crate::locale::{Text, tr};
use crate::risk::Risk;
use ilot_bridge::protocol::QUESTION_EVENT;
use serde_json::{Value, json};
use std::io;
use std::path::Path;

const SETTINGS: &str = ".claude/settings.json";
/// Fire-and-forget events run in the background: Claude never waits for îlot (the
/// `.exe` start through WSL interop costs 60 to 80 ms). Only the permission waits, 120 s
/// at most, and the questions, 60 s at most (the relay gives up at 55 s), after which
/// Claude asks in its terminal. The question's hook is a `PreToolUse` of its own, kept
/// apart by its `matcher`: an `async` hook cannot answer
/// (https://code.claude.com/docs/en/hooks#allow-with-updatedinput). A connection made
/// before it existed reads « à réparer » until the user plugs Claude again.
const SPECS: [HookSpec<'static>; 13] = [
    HookSpec::new("SessionStart", 10, true),
    HookSpec::new("SessionEnd", 10, true),
    HookSpec::new("UserPromptSubmit", 10, true),
    HookSpec::new("PreToolUse", 10, true),
    HookSpec::new("PostToolUse", 10, true),
    HookSpec::new("PostToolUseFailure", 10, true),
    HookSpec::new("Notification", 10, true),
    HookSpec::new("Stop", 10, true),
    HookSpec::new("StopFailure", 10, true),
    HookSpec::new("SubagentStart", 10, true),
    HookSpec::new("SubagentStop", 10, true),
    HookSpec::new("PermissionRequest", 120, false),
    HookSpec { matcher: Some("AskUserQuestion"), relay_event: Some(QUESTION_EVENT), ..HookSpec::new("PreToolUse", 60, false) },
];
/// Exec form: the relay is started without a shell, its arguments passed as they are.
const FORMAT: HandlerFormat =
    HandlerFormat { call: Call::Exec, timeout_in_ms: false, background_key: Some("async"), name: None, flat: false, version: None };

pub struct ClaudeCode;

impl Connector for ClaudeCode {
    fn id(&self) -> &'static str {
        "claude-code"
    }

    fn display_name(&self) -> String {
        "Claude".into()
    }

    fn capabilities(&self) -> Caps {
        Caps { activity: true, permissions: true }
    }

    fn status(&self, target: &Target) -> Status {
        hooks_json::status(&target.home_file(SETTINGS), &SPECS)
    }

    fn connect(&self, target: &Target, relay_exe: &Path) -> io::Result<()> {
        let relay = target.install_relay(relay_exe)?;
        hooks_json::plug(&target.home_file(SETTINGS), &relay, self.id(), &SPECS, &FORMAT)
    }

    fn disconnect(&self, target: &Target) -> io::Result<()> {
        hooks_json::unplug(&target.home_file(SETTINGS))?;
        tidy_relay(target)
    }

    fn parse(&self, event: &str, payload: &Value, at: u64) -> Option<AgentEvent> {
        hook_events::parse(Source::ClaudeCode, event, payload, at)
    }

    fn permission(&self, event: &str, payload: &Value) -> Option<PermissionView> {
        if event == QUESTION_EVENT {
            return question_view(payload);
        }
        if event != "PermissionRequest" {
            return None;
        }
        let view = hook_events::permission_view(&self.display_name(), payload)?;
        let can_always = text(payload, "tool_name") == Some("Bash")
            && !view.truncated
            && view.extras.is_empty()
            && exact_rule_possible(&view.detail);
        Some(PermissionView { can_always, ..view })
    }

    fn answer(&self, event: &str, payload: &Value, answer: Answer) -> Option<String> {
        if event != "PermissionRequest" {
            return None;
        }
        match answer {
            Answer::Always => {}
            // « Tout suspendre » also stops Claude's turn (`interrupt`, documented for a
            // refusal), so that it does not look for another way while îlot is suspended.
            Answer::Suspended => {
                return Some(hook_events::output(json!({
                    "behavior": "deny",
                    "message": hook_events::SUSPENDED_MESSAGE,
                    "interrupt": true
                })));
            }
            _ => return hook_events::decision(answer),
        }
        // Only for a request where the button was offered (checked again here, whatever the
        // interface sent); the rule lives in Claude's memory for this session, never in a file.
        if !self.permission(event, payload)?.can_always {
            return None;
        }
        let command = text(payload.get("tool_input")?, "command")?;
        Some(hook_events::output(json!({
            "behavior": "allow",
            "updatedPermissions": [{
                "type": "addRules",
                "rules": [{ "toolName": "Bash", "ruleContent": command }],
                "behavior": "allow",
                "destination": "session"
            }]
        })))
    }

    /// The documented answer to `AskUserQuestion`: `allow` with `updatedInput` holding the
    /// questions as received and the user's `answers`
    /// (https://code.claude.com/docs/en/hooks#allow-with-updatedinput). Only for a whole
    /// payload, and only when every question has an answer of the user's.
    fn answer_questions(&self, event: &str, payload: &Value, answers: &[QuestionAnswer]) -> Option<String> {
        if event != QUESTION_EVENT || cut(payload) {
            return None;
        }
        let input = payload.get("tool_input")?;
        let answers = questions::answers(&questions::read(input, true)?, answers)?;
        Some(
            json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": "allow",
                    "updatedInput": { "questions": input.get("questions")?, "answers": answers }
                }
            })
            .to_string(),
        )
    }
}

/// What the island shows for `AskUserQuestion`: the questions, ready to answer, or nothing
/// when they cannot be shown exactly as asked (Claude then asks in its terminal).
fn question_view(payload: &Value) -> Option<PermissionView> {
    let asked = questions::read(payload.get("tool_input")?, true)?;
    let title = tr(Text::ClaudeAsks { count: asked.len() });
    Some(PermissionView {
        title,
        detail: asked[0].question.clone(),
        content: None,
        extras: Vec::new(),
        reason: None,
        truncated: cut(payload),
        can_always: false,
        risk: Risk::Low,
        warnings: Vec::new(),
        questions: asked,
    })
}

/// A Claude rule is a pattern: `*` would widen it, and hidden characters could make the
/// rule differ from what the user read. Claude also strips wrappers (`timeout`, `nice`,
/// bare `xargs`…) and some variables before matching a rule
/// (https://code.claude.com/docs/en/permissions), so a rule only stays close to the
/// command read when that command is one plain program call. A command that runs code
/// of the project (`./build.sh`, `npm test`, `make`, `python app.py`…) would also stay
/// allowed after the agent changed that code. In all these cases "Toujours" is not
/// offered.
pub fn exact_rule_possible(command: &str) -> bool {
    let program = command.split_whitespace().next().unwrap_or("");
    !command.is_empty()
        && !command.contains(['*', '&', '|', ';', '<', '>', '$', '`', '(', ')', '\\'])
        && !command.chars().any(is_deceptive)
        && !looks_spoofed(command)
        && !program.contains(['/', '='])
        && !RUNS_OTHER_CODE.contains(&program)
}

/// Programs that run code chosen elsewhere (a project file, a script, another command)
/// or that Claude strips before matching a rule.
const RUNS_OTHER_CODE: &[&str] = &[
    ".", "bash", "builtin", "bun", "bundle", "bunx", "cargo", "cmake", "command", "deno", "dotnet", "env",
    "eval", "exec", "fish", "go", "gradle", "java", "just", "ksh", "make", "mvn", "nice", "nohup", "node",
    "noglob", "npm", "npx", "perl", "php", "pip", "pip3", "pnpm", "poetry", "pwsh", "pytest", "python",
    "python3", "rake", "ruby", "sh", "source", "stdbuf", "sudo", "time", "timeout", "tox", "uv", "uvx",
    "xargs", "yarn", "zsh",
];

/// Same ranges as `src/lib/reveal.ts`: control, bidi, invisible, filler and look-alike
/// space characters.
pub fn is_deceptive(c: char) -> bool {
    let code = c as u32;
    code < 0x20
        || (0x7f..=0x9f).contains(&code)
        || matches!(code, 0xa0 | 0xad | 0x034f | 0x061c | 0x115f | 0x1160 | 0x17b4 | 0x17b5 | 0x180f | 0x2800 | 0x3000 | 0x3164 | 0xfeff | 0xffa0)
        || (0x180b..=0x180e).contains(&code)
        || (0x2000..=0x200f).contains(&code)
        || (0x2028..=0x202f).contains(&code)
        || (0x205f..=0x206f).contains(&code)
        || (0xfe00..=0xfe0f).contains(&code)
        || (0xfff9..=0xfffb).contains(&code)
        || (0x1d173..=0x1d17a).contains(&code)
        || (0xe0000..=0xe007f).contains(&code)
        || (0xe0100..=0xe01ef).contains(&code)
}

/// Same rule as `looksSpoofed` in `src/lib/reveal.ts`: a word that mixes Latin letters
/// with Greek or Cyrillic ones (`gіthub.com` with a Cyrillic `і`), or combining marks
/// stacked on one letter.
pub fn looks_spoofed(text: &str) -> bool {
    let combining = |c: char| matches!(c as u32, 0x0300..=0x036f | 0x1ab0..=0x1aff | 0x1dc0..=0x1dff | 0x20d0..=0x20ff | 0xfe20..=0xfe2f);
    let chars: Vec<char> = text.chars().collect();
    if chars.windows(2).any(|pair| combining(pair[0]) && combining(pair[1])) {
        return true;
    }
    text.split(|c: char| c.is_whitespace() || c.is_ascii_punctuation()).any(|word| {
        let latin = word.chars().any(|c| c.is_ascii_alphabetic());
        let other = word.chars().any(|c| matches!(c as u32, 0x0370..=0x052f));
        latin && other
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connectors::hook_events::DENY_MESSAGE;
    use crate::events::EventKind;
    use ilot_bridge::protocol::TRUNCATED_FLAG;

    fn fixture(event: &str) -> Value {
        let path = format!("{}/../tests/fixtures/claude-code/{event}.json", env!("CARGO_MANIFEST_DIR"));
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    fn parse(event: &str) -> AgentEvent {
        ClaudeCode.parse(event, &fixture(event), 1).unwrap()
    }

    #[test]
    fn every_real_capture_parses() {
        for event in [
            "SessionStart",
            "SessionEnd",
            "UserPromptSubmit",
            "PreToolUse",
            "PostToolUse",
            "PostToolUseFailure",
            "PermissionRequest",
            "Stop",
            "StopFailure",
            "SubagentStart",
            "SubagentStop",
        ] {
            let parsed = parse(event);
            assert_eq!(parsed.source, Source::ClaudeCode, "{event}");
            assert_eq!(parsed.project, "demo", "{event}");
            assert!(!parsed.session_id.is_empty(), "{event}");
        }
    }

    #[test]
    fn events_map_to_the_common_kinds() {
        assert_eq!(parse("SessionStart").kind, EventKind::SessionStart);
        assert_eq!(parse("UserPromptSubmit").kind, EventKind::Working);
        assert_eq!(parse("PreToolUse").kind, EventKind::Tool);
        assert_eq!(parse("PermissionRequest").kind, EventKind::Permission);
        assert_eq!(parse("Stop").kind, EventKind::TurnDone);
        assert_eq!(parse("StopFailure").kind, EventKind::Error);
        assert_eq!(parse("SessionEnd").kind, EventKind::SessionEnd);
    }

    #[test]
    fn tool_activity_is_short_french() {
        let event = parse("PostToolUse");
        assert_eq!(event.title, "lance ls");
        let failure = parse("PostToolUseFailure");
        assert!(failure.detail.unwrap().starts_with("Exit code"));
    }

    #[test]
    fn unknown_events_and_payloads_without_session_are_ignored() {
        assert!(ClaudeCode.parse("PreCompact", &fixture("Stop"), 1).is_none());
        assert!(ClaudeCode.parse("Stop", &json!({ "cwd": "/x" }), 1).is_none());
    }

    /// The real capture, with a command that a session rule can match as read.
    fn plain_request() -> Value {
        let mut payload = fixture("PermissionRequest");
        payload["tool_input"]["command"] = json!("git status --short");
        payload
    }

    #[test]
    fn real_permission_request_shows_the_exact_command() {
        let view = ClaudeCode.permission("PermissionRequest", &fixture("PermissionRequest")).unwrap();
        assert_eq!(view.title, "Claude veut exécuter une commande");
        assert_eq!(view.detail, "python3 -c \"print('allowed-by-ilot')\"");
        assert_eq!(view.reason.as_deref(), Some("Print allowed-by-ilot via python3"));
        assert!(!view.can_always, "python3 runs code chosen elsewhere");
        assert!(!view.truncated);
        assert!(ClaudeCode.permission("PermissionRequest", &plain_request()).unwrap().can_always);
    }

    #[test]
    fn toujours_is_only_offered_for_one_plain_program_call() {
        for command in ["git status --short", "ls -la /tmp", "cargo-audit --version", "gh pr view 12"] {
            assert!(exact_rule_possible(command), "{command}");
        }
        for command in [
            "git push && rm -rf ~",
            "echo --force origin main | xargs git push",
            "timeout 5 git push",
            "./build.sh",
            "npm test",
            "make install",
            "python3 app.py",
            "bash -c ls",
            "FOO=1 git push",
            "/usr/bin/git push",
            "cat $(ls)",
            "git log > out.txt",
            "git push; reboot",
            "curl https://gіthub.com/x",
        ] {
            assert!(!exact_rule_possible(command), "{command}");
        }
    }

    #[test]
    fn toujours_is_refused_for_wildcards_hidden_characters_and_cut_payloads() {
        assert!(!exact_rule_possible("rm *.log"));
        assert!(!exact_rule_possible("ls\u{202e}gpj.exe"));
        assert!(!exact_rule_possible("echo a\nrm -rf ~"));
        let mut payload = fixture("PermissionRequest");
        payload[TRUNCATED_FLAG] = json!(true);
        assert!(!ClaudeCode.permission("PermissionRequest", &payload).unwrap().can_always);
        payload["tool_input"]["command"] = json!("rm *.log");
        assert_eq!(ClaudeCode.answer("PermissionRequest", &payload, Answer::Always), None);
    }

    #[test]
    fn answers_use_the_documented_format() {
        let payload = fixture("PermissionRequest");
        let allow: Value = serde_json::from_str(&ClaudeCode.answer("PermissionRequest", &payload, Answer::Allow).unwrap()).unwrap();
        assert_eq!(allow, json!({ "hookSpecificOutput": { "hookEventName": "PermissionRequest", "decision": { "behavior": "allow" } } }));
        let deny: Value = serde_json::from_str(&ClaudeCode.answer("PermissionRequest", &payload, Answer::Deny).unwrap()).unwrap();
        assert_eq!(deny["hookSpecificOutput"]["decision"]["behavior"], "deny");
        assert_eq!(deny["hookSpecificOutput"]["decision"]["message"], DENY_MESSAGE);
        assert_eq!(ClaudeCode.answer("PermissionRequest", &payload, Answer::Always), None, "python3: no rule");
        let always: Value = serde_json::from_str(&ClaudeCode.answer("PermissionRequest", &plain_request(), Answer::Always).unwrap()).unwrap();
        let update = &always["hookSpecificOutput"]["decision"]["updatedPermissions"][0];
        assert_eq!(update["destination"], "session");
        assert_eq!(update["rules"][0], json!({ "toolName": "Bash", "ruleContent": "git status --short" }));
        assert_eq!(ClaudeCode.answer("PreToolUse", &payload, Answer::Allow), None);
    }

    #[test]
    fn tout_suspendre_refuses_and_stops_the_turn() {
        let output = ClaudeCode.answer("PermissionRequest", &plain_request(), Answer::Suspended).unwrap();
        let decision = &serde_json::from_str::<Value>(&output).unwrap()["hookSpecificOutput"]["decision"];
        assert_eq!(*decision, json!({ "behavior": "deny", "message": "Refusé : îlot a tout suspendu.", "interrupt": true }));
    }

    #[test]
    fn other_tools_get_a_readable_request_without_toujours() {
        let payload = json!({ "tool_name": "Write", "tool_input": { "file_path": "/home/user/demo/notes.txt", "content": "x" } });
        let view = ClaudeCode.permission("PermissionRequest", &payload).unwrap();
        assert_eq!(view.title, "Claude veut modifier un fichier");
        assert_eq!(view.detail, "/home/user/demo/notes.txt");
        assert!(!view.can_always);
    }

    #[test]
    fn hidden_characters_match_the_interface_rules() {
        for c in ['\u{1b}', '\u{a0}', '\u{200b}', '\u{202e}', '\u{2066}', '\u{feff}', '\u{e0041}', '\n', '\t'] {
            assert!(is_deceptive(c), "{c:?}");
        }
        for c in ['\u{3164}', '\u{2800}', '\u{034f}', '\u{180b}', '\u{1d173}', '\u{115f}', '\u{ffa0}'] {
            assert!(is_deceptive(c), "{c:?}");
        }
        for c in ['a', 'é', '«', '−', ' '] {
            assert!(!is_deceptive(c), "{c:?}");
        }
    }

    #[test]
    fn mixed_alphabets_and_stacked_marks_look_spoofed() {
        assert!(looks_spoofed("curl https://gіthub.com"), "Cyrillic і in a Latin word");
        assert!(looks_spoofed("rm fi\u{301}\u{301}le"));
        assert!(!looks_spoofed("echo café"));
        assert!(!looks_spoofed("echo привет world"), "two words, each in one alphabet");
    }

    #[test]
    fn connect_then_disconnect_restores_a_copy_of_a_real_config() {
        let dir = std::env::temp_dir().join(format!("ilot-claude-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".claude")).unwrap();
        let original = "{\n  \"permissions\": {\n    \"allow\": [\n      \"Bash(npm run:*)\"\n    ]\n  },\n  \"hooks\": {\n    \"Stop\": [\n      {\n        \"hooks\": [\n          {\n            \"type\": \"command\",\n            \"command\": \"notify-send done\"\n          }\n        ]\n      }\n    ]\n  }\n}\n";
        let settings = dir.join(".claude").join("settings.json");
        std::fs::write(&settings, original).unwrap();
        let target = Target::windows(dir.clone());
        let relay = dir.join("ilot-hook.exe");
        assert_eq!(ClaudeCode.status(&target), Status::Installed);
        ClaudeCode.connect(&target, &relay).unwrap();
        assert_eq!(ClaudeCode.status(&target), Status::Connected);
        ClaudeCode.disconnect(&target).unwrap();
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), original);
        assert_eq!(ClaudeCode.status(&target), Status::Installed);
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Real WSL file operations (UNC writes, relay script, chmod) on a throwaway home in
    /// `/tmp`, never on the user's real configuration.
    #[test]
    #[ignore = "needs WSL; run with --ignored"]
    fn connect_and_disconnect_inside_wsl_on_a_throwaway_home() {
        let distro = crate::wsl::distros().into_iter().next().expect("a WSL distribution");
        let home = format!("/tmp/ilot-test-home-{}", std::process::id());
        let settings = crate::wsl::unc_path(&distro, &format!("{home}/.claude/settings.json"));
        std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
        let original = "{\n  \"model\": \"opus\"\n}\n";
        std::fs::write(&settings, original).unwrap();
        let target = Target::wsl(&distro, &home);
        let relay = std::env::current_exe().unwrap().with_file_name("ilot-hook.exe");
        ClaudeCode.connect(&target, &relay).unwrap();
        assert_eq!(ClaudeCode.status(&target), Status::Connected);
        let script = crate::wsl::unc_path(&distro, &format!("{home}/.local/bin/ilot-hook"));
        assert!(std::fs::read_to_string(&script).unwrap().contains("ilot-hook.exe"));
        ClaudeCode.disconnect(&target).unwrap();
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), original);
        assert!(!script.exists(), "the relay script leaves with the last connector");
        std::fs::remove_dir_all(crate::wsl::unc_path(&distro, &home)).unwrap();
    }

    /// The documented `AskUserQuestion` example (https://code.claude.com/docs/en/hooks#askuserquestion)
    /// with a second, multiple-choice question.
    fn question(picked: &[&[&str]]) -> Vec<QuestionAnswer> {
        picked.iter().map(|labels| QuestionAnswer { picked: labels.iter().map(|l| (*l).to_owned()).collect(), other: None }).collect()
    }

    #[test]
    fn a_question_shows_in_the_island_and_waits() {
        let payload = fixture("AskUserQuestion");
        let event = ClaudeCode.parse("AskUserQuestion", &payload, 1).unwrap();
        assert_eq!((event.kind, event.detail.as_deref()), (EventKind::Permission, Some("Which framework?")));
        let view = ClaudeCode.permission("AskUserQuestion", &payload).unwrap();
        assert_eq!(view.title, "Claude te pose 2 questions");
        assert_eq!(view.questions.len(), 2);
        assert!(view.questions[1].multi_select && view.questions[0].free_text);
        assert!(!view.can_always && view.extras.is_empty());
        // The asynchronous observation of the same call is only an activity.
        assert_eq!(ClaudeCode.parse("PreToolUse", &payload, 1).unwrap().title, "demande");
    }

    #[test]
    fn answers_use_the_documented_updated_input() {
        let payload = fixture("AskUserQuestion");
        let output = ClaudeCode.answer_questions("AskUserQuestion", &payload, &question(&[&["Vue"], &["Lint", "Tests"]])).unwrap();
        let output: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(output["hookSpecificOutput"]["hookEventName"], "PreToolUse");
        assert_eq!(output["hookSpecificOutput"]["permissionDecision"], "allow");
        let updated = &output["hookSpecificOutput"]["updatedInput"];
        assert_eq!(updated["questions"], payload["tool_input"]["questions"], "the questions as received");
        assert_eq!(updated["answers"], json!({ "Which framework?": "Vue", "Which checks should run before each commit?": "Tests, Lint" }));
    }

    #[test]
    fn a_question_is_never_answered_without_the_user_s_answer() {
        let payload = fixture("AskUserQuestion");
        assert_eq!(ClaudeCode.answer_questions("AskUserQuestion", &payload, &question(&[&["Vue"]])), None, "one question left");
        assert_eq!(ClaudeCode.answer_questions("AskUserQuestion", &payload, &question(&[&["Svelte"], &["Lint"]])), None);
        assert_eq!(ClaudeCode.answer_questions("PermissionRequest", &payload, &question(&[&["Vue"], &["Lint"]])), None);
        // Allow, Always or Suspended are not answers to a question: the terminal asks.
        for answer in [Answer::Allow, Answer::Always, Answer::Terminal, Answer::Suspended] {
            assert_eq!(ClaudeCode.answer("AskUserQuestion", &payload, answer), None, "{answer:?}");
        }
        let mut cut_payload = payload.clone();
        cut_payload[ilot_bridge::protocol::TRUNCATED_FLAG] = json!(true);
        assert!(ClaudeCode.permission("AskUserQuestion", &cut_payload).unwrap().truncated);
        assert_eq!(ClaudeCode.answer_questions("AskUserQuestion", &cut_payload, &question(&[&["Vue"], &["Lint"]])), None);
        let mut odd = payload;
        odd["tool_input"]["questions"][0]["options"] = json!([{ "label": "Seul" }]);
        assert!(ClaudeCode.permission("AskUserQuestion", &odd).is_none(), "shown as asked, or left to the terminal");
    }

    #[test]
    fn the_question_hook_is_synchronous_and_an_older_connection_needs_repair() {
        let dir = std::env::temp_dir().join(format!("ilot-claude-question-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".claude")).unwrap();
        let target = Target::windows(dir.clone());
        ClaudeCode.connect(&target, &dir.join("ilot-hook.exe")).unwrap();
        let settings = dir.join(".claude").join("settings.json");
        let written: Value = serde_json::from_str(&std::fs::read_to_string(&settings).unwrap()).unwrap();
        let question = &written["hooks"]["PreToolUse"][1];
        assert_eq!(question["matcher"], "AskUserQuestion");
        assert_eq!(question["hooks"][0]["args"], json!(["claude-code", "AskUserQuestion"]));
        assert_eq!(question["hooks"][0]["timeout"], 60);
        assert!(question["hooks"][0].get("async").is_none(), "an async hook cannot answer");
        // As connected before the question existed: the island says « à réparer ».
        let mut older = written.clone();
        older["hooks"]["PreToolUse"].as_array_mut().unwrap().pop();
        std::fs::write(&settings, serde_json::to_string_pretty(&older).unwrap()).unwrap();
        assert_eq!(ClaudeCode.status(&target), Status::NeedsRepair);
        ClaudeCode.connect(&target, &dir.join("ilot-hook.exe")).unwrap();
        assert_eq!(ClaudeCode.status(&target), Status::Connected);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn no_claude_folder_means_not_installed() {
        let target = Target::windows(std::env::temp_dir().join("ilot-no-claude-here"));
        assert_eq!(ClaudeCode.status(&target), Status::NotInstalled);
    }

    #[test]
    fn questions_are_announced_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let view = ClaudeCode.permission("AskUserQuestion", &fixture("AskUserQuestion")).unwrap();
        assert_eq!(view.title, "Claude has 2 questions for you");
        assert_eq!(parse("SessionStart").title, "starts");
    }
}
