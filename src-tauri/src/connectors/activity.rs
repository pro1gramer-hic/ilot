//! What an agent's tool does, in a few words ("édite api.py"), and the lines it changed.
//! The tool names of the agents îlot follows never mean two different things, so one
//! table serves all of them.

use super::text;
use crate::events::{Action, Activity};
use serde_json::Value;

const TARGET_CHARS: usize = 48;

/// First non-empty line, trimmed and cut to a readable length.
pub fn first_line(text: &str) -> String {
    shorten(text.lines().find(|l| !l.trim().is_empty()).unwrap_or_default().trim(), 120)
}

pub fn shorten(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_owned(),
    }
}

fn file_name(path: &str) -> String {
    path.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or(path).to_owned()
}

/// First file an `apply_patch` patch touches (Codex): `*** Update File: src/api.py`.
fn patch_file(patch: &str) -> Option<String> {
    patch.lines().find_map(|line| {
        ["*** Update File: ", "*** Add File: ", "*** Delete File: "]
            .iter()
            .find_map(|prefix| line.strip_prefix(prefix))
            .map(|path| file_name(path.trim()))
    })
}

/// What the tool does, with a short target. Besides Claude Code, Codex and Gemini CLI, the
/// table knows the tools of the agents îlot follows for their activity: Cursor (`Shell`),
/// OpenCode (lower case: `bash`, `edit`, `write`, `filePath`), Amp (`edit_file`,
/// `create_file`, `cmd`) and Kiro (`execute_bash`, `fs_read`, `fs_write`).
pub fn tool_activity(tool: &str, input: &Value) -> Activity {
    let file = || {
        ["file_path", "filePath", "notebook_path", "absolute_path", "dir_path", "path"]
            .iter()
            .find_map(|key| text(input, key))
            .map(file_name)
    };
    let short = |keys: &[&str]| keys.iter().find_map(|key| text(input, key)).map(|t| shorten(t.lines().next().unwrap_or(t), TARGET_CHARS));
    let (action, target) = match tool {
        "Bash" | "run_shell_command" | "Shell" | "bash" | "execute_bash" | "execute_pwsh" => (Action::Run, short(&["command", "cmd"])),
        "Read" | "read_file" | "read_many_files" | "read" | "fs_read" => (Action::Read, file()),
        "Edit" | "MultiEdit" | "NotebookEdit" | "replace" | "edit" | "edit_file" | "fs_write" => (Action::Edit, file()),
        "apply_patch" => (Action::Edit, text(input, "command").and_then(patch_file)),
        "Write" | "write_file" | "write" | "create_file" => (Action::Write, file()),
        "list_directory" | "list" => (Action::Browse, file()),
        "Grep" | "Glob" | "glob" | "grep_search" | "grep" => (Action::Search, short(&["pattern"])),
        "WebFetch" | "webfetch" => (Action::Fetch, short(&["url"])),
        "web_fetch" => (Action::Fetch, short(&["prompt"])),
        "WebSearch" | "google_web_search" => (Action::Search, short(&["query"])),
        "Task" | "Agent" | "spawn_agent" | "invoke_agent" | "task" => (Action::Delegate, short(&["description", "agent_name"])),
        "TodoWrite" | "update_plan" | "write_todos" | "todowrite" => (Action::Plan, None),
        "AskUserQuestion" | "request_user_input" | "question" => (Action::Ask, None),
        other => {
            let name = other.strip_prefix("mcp__").or_else(|| other.strip_prefix("mcp_")).unwrap_or(other);
            (Action::Use, Some(shorten(name, TARGET_CHARS)))
        }
    };
    Activity::new(action, target)
}

/// Lines added and removed by a successful edit: the agent's own patch when it gives one,
/// otherwise the edit's old and new text.
pub fn changed_lines(tool: &str, input: &Value, response: Option<&Value>) -> (u32, u32) {
    if let Some(counts) = response.and_then(structured_patch_counts) {
        return counts;
    }
    match tool {
        "Edit" | "replace" | "edit" | "edit_file" | "fs_write" if old_new(input).is_some() => {
            old_new(input).map_or((0, 0), |(old, new)| diff_counts(old, new))
        }
        "MultiEdit" => edits_counts(input.get("edits").unwrap_or(&Value::Null)),
        "Write" | "write_file" | "write" | "create_file" | "fs_write" => {
            (["content", "file_text"].iter().find_map(|key| text(input, key)).map_or(0, |c| c.lines().count() as u32), 0)
        }
        "apply_patch" => text(input, "command").map_or((0, 0), |patch| prefixed_counts(patch.lines())),
        _ => (0, 0),
    }
}

/// An edit's old and new text, whatever the agent names them (`old_string`, OpenCode's
/// `oldString`, Amp's and Kiro's `old_str`).
fn old_new(input: &Value) -> Option<(&str, &str)> {
    let old = ["old_string", "oldString", "old_str"].iter().find_map(|key| text(input, key));
    let new = ["new_string", "newString", "new_str"].iter().find_map(|key| text(input, key));
    (old.is_some() || new.is_some()).then(|| (old.unwrap_or(""), new.unwrap_or("")))
}

/// Lines changed by a list of `{old_string, new_string}` edits (Claude's `MultiEdit`,
/// Cursor's `afterFileEdit`).
pub fn edits_counts(edits: &Value) -> (u32, u32) {
    edits
        .as_array()
        .into_iter()
        .flatten()
        .map(|e| old_new(e).unwrap_or(("", "")))
        .map(|(old, new)| diff_counts(old, new))
        .fold((0, 0), |(a, r), (x, y)| (a + x, r + y))
}

/// Counts `+` and `-` lines of lines taken from a patch, file headers excluded.
fn prefixed_counts<'a>(lines: impl Iterator<Item = &'a str>) -> (u32, u32) {
    let mut counts = (0, 0);
    for line in lines {
        if line.starts_with('+') && !line.starts_with("+++") {
            counts.0 += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            counts.1 += 1;
        }
    }
    counts
}

/// Claude's `structuredPatch`: hunks of prefixed lines.
fn structured_patch_counts(response: &Value) -> Option<(u32, u32)> {
    let hunks = response.get("structuredPatch")?.as_array()?;
    let lines = hunks.iter().filter_map(|h| h.get("lines")?.as_array()).flatten().filter_map(Value::as_str);
    Some(prefixed_counts(lines))
}

/// Lines added and removed when `old` becomes `new`, ignoring identical lines at both ends.
pub fn diff_counts(old: &str, new: &str) -> (u32, u32) {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let suffix = a[prefix..].iter().rev().zip(b[prefix..].iter().rev()).take_while(|(x, y)| x == y).count();
    ((b.len() - prefix - suffix) as u32, (a.len() - prefix - suffix) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn activity(tool: &str, input: Value) -> (String, Option<String>) {
        let a = tool_activity(tool, &input);
        (a.verb, a.target)
    }

    #[test]
    fn the_three_agents_tools_read_the_same() {
        let lance = ("lance".to_owned(), Some("ls".to_owned()));
        assert_eq!(activity("Bash", json!({ "command": "ls" })), lance);
        assert_eq!(activity("run_shell_command", json!({ "command": "ls\necho" })), lance);
        let edit = ("édite".to_owned(), Some("api.py".to_owned()));
        assert_eq!(activity("Edit", json!({ "file_path": "/p/api.py" })), edit);
        assert_eq!(activity("replace", json!({ "file_path": "/p/api.py" })), edit);
        assert_eq!(activity("apply_patch", json!({ "command": "*** Begin Patch\n*** Update File: src/api.py\n@@\n-a\n+b\n*** End Patch\n" })), edit);
        assert_eq!(activity("read_file", json!({ "file_path": "C:\\p\\notes.txt" })).1.as_deref(), Some("notes.txt"));
        assert_eq!(activity("update_plan", json!({})), ("planifie".to_owned(), None));
    }

    #[test]
    fn the_tools_of_the_followed_agents_read_the_same() {
        let lance = |t: &str| ("lance".to_owned(), Some(t.to_owned()));
        assert_eq!(activity("Shell", json!({ "command": "ls" })), lance("ls"), "Cursor");
        assert_eq!(activity("bash", json!({ "command": "npm test" })), lance("npm test"), "OpenCode");
        assert_eq!(activity("Bash", json!({ "cmd": "git status" })), lance("git status"), "Amp");
        assert_eq!(activity("execute_pwsh", json!({ "command": "dir" })), lance("dir"), "Kiro on Windows");
        assert_eq!(activity("edit", json!({ "filePath": "/p/a.ts" })), ("édite".to_owned(), Some("a.ts".to_owned())));
        assert_eq!(activity("create_file", json!({ "path": "/p/b.md" })), ("écrit".to_owned(), Some("b.md".to_owned())));
        assert_eq!(activity("fs_read", json!({ "path": "/p/c.rs" })).0, "lit");
        assert_eq!(activity("AskUserQuestion", json!({ "questions": [] })), ("demande".to_owned(), None));
        assert_eq!(activity("invoke_agent", json!({ "agent_name": "codebase_investigator" })).1.as_deref(), Some("codebase_investigator"));
        assert_eq!(changed_lines("edit", &json!({ "oldString": "a", "newString": "b\nc" }), None), (2, 1));
        assert_eq!(changed_lines("edit_file", &json!({ "old_str": "a", "new_str": "" }), None), (0, 1));
        assert_eq!(changed_lines("create_file", &json!({ "content": "a\nb" }), None), (2, 0));
        assert_eq!(changed_lines("fs_write", &json!({ "command": "create", "file_text": "x" }), None), (1, 0));
        assert_eq!(edits_counts(&json!([{ "old_string": "a", "new_string": "b" }, { "new_string": "c" }])), (2, 1));
    }

    #[test]
    fn unknown_and_mcp_tools_are_named() {
        assert_eq!(activity("mcp__github__create_issue", json!({})).1.as_deref(), Some("github__create_issue"));
        assert_eq!(activity("mcp_fs_read", json!({})).1.as_deref(), Some("fs_read"));
        assert_eq!(activity("tracker_create_task", json!({})).0, "utilise");
    }

    #[test]
    fn long_targets_are_cut() {
        let (_, target) = activity("Bash", json!({ "command": "x".repeat(200) }));
        assert_eq!(target.unwrap().chars().count(), TARGET_CHARS + 1);
        assert_eq!(first_line("\n\n  bonjour  \nsuite"), "bonjour");
    }

    #[test]
    fn edits_count_changed_lines() {
        assert_eq!(diff_counts("a\nb\nc", "a\nB\nB2\nc"), (2, 1));
        assert_eq!(diff_counts("", "x\ny"), (2, 0));
        assert_eq!(diff_counts("same", "same"), (0, 0));
        // Shaped like the documented Claude Edit tool response.
        let response = json!({ "structuredPatch": [{ "lines": [" ctx", "-old", "+new", "+new2"] }] });
        assert_eq!(changed_lines("Edit", &json!({ "old_string": "a", "new_string": "b" }), Some(&response)), (2, 1));
        assert_eq!(changed_lines("Edit", &json!({ "old_string": "a\nb", "new_string": "a\nc\nd" }), None), (2, 1));
        assert_eq!(changed_lines("replace", &json!({ "old_string": "a", "new_string": "b" }), None), (1, 1));
        assert_eq!(changed_lines("write_file", &json!({ "content": "a\nb\nc" }), None), (3, 0));
    }

    #[test]
    fn codex_patches_count_their_own_lines() {
        let patch = "*** Begin Patch\n*** Update File: api.py\n@@ def f():\n-    return 1\n+    return 2\n+    # done\n*** Add File: new.py\n+print('x')\n*** End Patch\n";
        assert_eq!(changed_lines("apply_patch", &json!({ "command": patch }), Some(&json!("Success"))), (3, 1));
    }

    #[test]
    fn the_verbs_follow_the_language_and_the_action_does_not() {
        crate::locale::speak(crate::locale::Lang::En);
        let edit = tool_activity("Edit", &json!({ "file_path": "/p/api.py" }));
        assert_eq!((edit.action, edit.verb.as_str(), edit.title()), (Action::Edit, "edits", "edits api.py".to_owned()));
        assert_eq!(activity("Bash", json!({ "command": "ls" })), ("runs".to_owned(), Some("ls".to_owned())));
        assert_eq!(activity("list_directory", json!({ "dir_path": "/p/src" })).0, "lists");
        assert_eq!(activity("WebFetch", json!({ "url": "https://example.com" })).0, "browses");
        assert_eq!(tool_activity("tracker_create_task", &json!({})).action, Action::Use);
        let json = serde_json::to_value(tool_activity("Write", &json!({ "file_path": "b.md" }))).unwrap();
        assert_eq!((json["action"].as_str(), json["verb"].as_str()), (Some("write"), Some("writes")));
    }
}
