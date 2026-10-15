//! The latest change of a session, as the agent's own hook describes it: îlot never reads
//! the agent's files, but an edit's payload says what it replaces (`old_string` and
//! `new_string` of Claude's `Edit`, Gemini's `replace`, Cursor's `afterFileEdit`), what it
//! writes (`content`) or the patch it applies (Codex's `apply_patch`). Kept in memory
//! only, one per session, and bounded: a few hundred short lines at most.

use super::activity::shorten;
use super::text;
use serde::Serialize;
use serde_json::Value;

/// Lines kept for one change.
pub const MAX_LINES: usize = 160;
/// A longer line is cut.
pub const MAX_LINE_CHARS: usize = 240;
/// Unchanged lines kept around a change.
const CONTEXT: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LineKind {
    /// Removed.
    Del,
    /// Added.
    Add,
    /// Unchanged, around a change.
    Same,
    /// Lines left out between two changes.
    Gap,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DiffLine {
    pub kind: LineKind,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiff {
    /// The file's name (`api.py`).
    pub file: String,
    pub lines: Vec<DiffLine>,
    pub added: u32,
    pub removed: u32,
    /// Lines were left out (too many, or the payload was cut by the relay).
    pub truncated: bool,
}

/// The keys an edit's old and new text, its file and its content go by, depending on
/// the agent (Claude, Gemini, Cursor: snake case; OpenCode: camel case; Amp, Kiro:
/// `old_str`).
const OLD: [&str; 3] = ["old_string", "oldString", "old_str"];
const NEW: [&str; 3] = ["new_string", "newString", "new_str"];
const FILE: [&str; 5] = ["file_path", "filePath", "path", "absolute_path", "notebook_path"];
const CONTENT: [&str; 2] = ["content", "file_text"];

fn first<'a>(input: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| text(input, key))
}

fn file_name(path: &str) -> String {
    path.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or(path).to_owned()
}

/// Builds a diff line by line, within the limits.
struct Builder {
    lines: Vec<DiffLine>,
    added: u32,
    removed: u32,
    truncated: bool,
}

impl Builder {
    fn new() -> Self {
        Self { lines: Vec::new(), added: 0, removed: 0, truncated: false }
    }

    fn push(&mut self, kind: LineKind, line: &str) {
        match kind {
            LineKind::Add => self.added += 1,
            LineKind::Del => self.removed += 1,
            _ => {}
        }
        if self.lines.len() >= MAX_LINES {
            self.truncated = true;
            return;
        }
        let text = shorten(line.trim_end_matches('\r'), MAX_LINE_CHARS);
        self.truncated |= text.chars().count() > MAX_LINE_CHARS;
        self.lines.push(DiffLine { kind, text });
    }

    /// One replacement: the unchanged lines at both ends are dropped, but a few are kept
    /// around the change.
    fn replace(&mut self, old: &str, new: &str) {
        if !self.lines.is_empty() {
            self.push(LineKind::Gap, "");
        }
        let a: Vec<&str> = old.lines().collect();
        let b: Vec<&str> = new.lines().collect();
        let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
        let suffix = a[prefix..].iter().rev().zip(b[prefix..].iter().rev()).take_while(|(x, y)| x == y).count();
        for line in &a[prefix.saturating_sub(CONTEXT)..prefix] {
            self.push(LineKind::Same, line);
        }
        for line in &a[prefix..a.len() - suffix] {
            self.push(LineKind::Del, line);
        }
        for line in &b[prefix..b.len() - suffix] {
            self.push(LineKind::Add, line);
        }
        let after = a.len() - suffix;
        for line in &a[after..(after + CONTEXT).min(a.len())] {
            self.push(LineKind::Same, line);
        }
    }

    fn finish(self, path: &str, cut: bool) -> Option<FileDiff> {
        (!self.lines.is_empty()).then(|| FileDiff {
            file: file_name(path),
            lines: self.lines,
            added: self.added,
            removed: self.removed,
            truncated: self.truncated || cut,
        })
    }
}

/// The first file of a Codex patch and its lines (`*** Update File: src/api.py`, then
/// `@@`, `-`, `+` and ` ` lines, until the next file).
fn patch(text: &str, cut: bool) -> Option<FileDiff> {
    let mut path = None;
    let mut builder = Builder::new();
    for line in text.lines() {
        if let Some(header) = ["*** Update File: ", "*** Add File: ", "*** Delete File: "].iter().find_map(|p| line.strip_prefix(p)) {
            if path.is_some() {
                builder.truncated = true;
                break;
            }
            path = Some(header.trim().to_owned());
        } else if path.is_none() || line.starts_with("*** ") {
            continue;
        } else if line.starts_with("@@") {
            if !builder.lines.is_empty() {
                builder.push(LineKind::Gap, "");
            }
        } else if let Some(rest) = line.strip_prefix('+') {
            builder.push(LineKind::Add, rest);
        } else if let Some(rest) = line.strip_prefix('-') {
            builder.push(LineKind::Del, rest);
        } else if let Some(rest) = line.strip_prefix(' ') {
            builder.push(LineKind::Same, rest);
        }
    }
    builder.finish(&path?, cut)
}

/// What a successful edit changed, from its tool's input; `None` for any other tool.
/// `cut`: the relay shortened the payload, so the change is not whole.
pub fn from_tool(tool: &str, input: &Value, cut: bool) -> Option<FileDiff> {
    let path = || first(input, &FILE);
    let mut builder = Builder::new();
    match tool {
        "Edit" | "replace" | "edit" | "edit_file" | "str_replace" | "Write" | "write_file" | "write" | "create_file" | "fs_write"
            if first(input, &OLD).is_some() =>
        {
            builder.replace(first(input, &OLD)?, first(input, &NEW).unwrap_or(""));
        }
        "Write" | "write_file" | "write" | "create_file" | "fs_write" => {
            for line in first(input, &CONTENT)?.lines() {
                builder.push(LineKind::Add, line);
            }
        }
        "MultiEdit" => edits(&mut builder, input.get("edits")?)?,
        "apply_patch" => return patch(text(input, "command")?, cut),
        _ => return None,
    }
    builder.finish(path()?, cut)
}

/// Cursor's `afterFileEdit`: a file and the `old_string`/`new_string` pairs applied to it
/// (https://cursor.com/docs/hooks, "afterFileEdit").
pub fn from_edits(path: &str, list: &Value, cut: bool) -> Option<FileDiff> {
    let mut builder = Builder::new();
    edits(&mut builder, list)?;
    builder.finish(path, cut)
}

fn edits(builder: &mut Builder, list: &Value) -> Option<()> {
    for edit in list.as_array()? {
        builder.replace(first(edit, &OLD).unwrap_or(""), first(edit, &NEW).unwrap_or(""));
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn kinds(diff: &FileDiff) -> Vec<(LineKind, &str)> {
        diff.lines.iter().map(|l| (l.kind, l.text.as_str())).collect()
    }

    #[test]
    fn an_edit_shows_what_it_replaces_with_a_little_context() {
        // Claude Code's Edit input (https://code.claude.com/docs/en/hooks, PreToolUse "Edit").
        let input = json!({ "file_path": "/home/user/demo/api.py", "old_string": "a\nb\nc\nold\nd\ne\nf", "new_string": "a\nb\nc\nnew\nnewer\nd\ne\nf" });
        let diff = from_tool("Edit", &input, false).unwrap();
        assert_eq!(diff.file, "api.py");
        assert_eq!(
            kinds(&diff),
            [(LineKind::Same, "b"), (LineKind::Same, "c"), (LineKind::Del, "old"), (LineKind::Add, "new"), (LineKind::Add, "newer"), (LineKind::Same, "d"), (LineKind::Same, "e")]
        );
        assert_eq!((diff.added, diff.removed, diff.truncated), (2, 1, false));
    }

    #[test]
    fn writes_and_other_agents_keys_read_the_same() {
        let write = from_tool("Write", &json!({ "file_path": "C:\\p\\notes.md", "content": "# Titre\ntexte" }), false).unwrap();
        assert_eq!((write.file.as_str(), write.added, write.removed), ("notes.md", 2, 0));
        // OpenCode's edit tool, Amp's edit_file, Kiro's fs_write.
        let opencode = from_tool("edit", &json!({ "filePath": "/p/a.ts", "oldString": "x", "newString": "y" }), false).unwrap();
        assert_eq!(kinds(&opencode), [(LineKind::Del, "x"), (LineKind::Add, "y")]);
        let amp = from_tool("edit_file", &json!({ "path": "/p/b.rs", "old_str": "1", "new_str": "2" }), false).unwrap();
        assert_eq!(amp.file, "b.rs");
        let kiro = from_tool("fs_write", &json!({ "command": "create", "path": "/p/c.txt", "file_text": "hi" }), false).unwrap();
        assert_eq!(kinds(&kiro), [(LineKind::Add, "hi")]);
        assert!(from_tool("Bash", &json!({ "command": "ls" }), false).is_none());
        assert!(from_tool("Edit", &json!({ "old_string": "a", "new_string": "b" }), false).is_none(), "no file, no diff");
    }

    #[test]
    fn several_edits_are_separated_by_a_gap() {
        let cursor = json!([{ "old_string": "a", "new_string": "A" }, { "old_string": "z", "new_string": "Z" }]);
        let diff = from_edits("/p/main.go", &cursor, false).unwrap();
        assert_eq!(kinds(&diff), [(LineKind::Del, "a"), (LineKind::Add, "A"), (LineKind::Gap, ""), (LineKind::Del, "z"), (LineKind::Add, "Z")]);
        let multi = from_tool("MultiEdit", &json!({ "file_path": "/p/x.ts", "edits": cursor }), false).unwrap();
        assert_eq!(multi.lines.len(), 5);
    }

    #[test]
    fn a_codex_patch_shows_its_first_file() {
        let command = "*** Begin Patch\n*** Update File: src/api.py\n@@ def f():\n-    return 1\n+    return 2\n     pass\n*** Add File: new.py\n+print('x')\n*** End Patch\n";
        let diff = from_tool("apply_patch", &json!({ "command": command }), false).unwrap();
        assert_eq!(diff.file, "api.py");
        assert_eq!(kinds(&diff), [(LineKind::Del, "    return 1"), (LineKind::Add, "    return 2"), (LineKind::Same, "    pass")]);
        assert!(diff.truncated, "a second file was left out");
    }

    #[test]
    fn diffs_stay_small() {
        let big = (0..1000).map(|i| format!("line {i} {}", "x".repeat(400))).collect::<Vec<_>>().join("\n");
        let diff = from_tool("Write", &json!({ "file_path": "/p/big.txt", "content": big }), false).unwrap();
        assert_eq!(diff.lines.len(), MAX_LINES);
        assert!(diff.lines.iter().all(|l| l.text.chars().count() <= MAX_LINE_CHARS + 1));
        assert_eq!(diff.added, 1000, "the count stays true");
        assert!(diff.truncated);
        let cut = from_tool("Edit", &json!({ "file_path": "/p/a", "old_string": "a", "new_string": "b" }), true).unwrap();
        assert!(cut.truncated, "a payload cut by the relay is not whole");
    }
}
