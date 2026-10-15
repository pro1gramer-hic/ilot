//! The command line of `ilot`, read without any shell and without guessing: options are
//! `--name value` or `--name=value`, and everything after `--` (or after the first word
//! that is not an option) is the command `run` starts, kept exactly as given.

use crate::texts::{Text, tr};
use std::ffi::OsString;

/// What `ilot emit --kind` accepts: the generic connector's event names.
pub const KINDS: [&str; 5] = ["working", "turn_done", "error", "session_start", "session_end"];

/// Who speaks: the name the user gives the tool, and the project (by default, the
/// working folder's name).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Who {
    pub source: String,
    pub project: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Emit {
    pub who: Who,
    pub kind: String,
    pub title: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    Emit(Emit),
    /// Starts the command, tells îlot before and after, exits with its code.
    Run { who: Who, command: Vec<OsString> },
    /// For the WSL script, which runs the command in Linux itself: the "before" half.
    RunBegin { who: Who, command: Vec<OsString> },
    /// For the WSL script: the "after" half, with the command's exit code.
    RunEnd { who: Who, code: i32 },
}

struct Parsed {
    options: Vec<(&'static str, String)>,
    rest: Vec<OsString>,
    help: bool,
}

impl Parsed {
    fn get(&self, name: &str) -> Option<String> {
        self.options.iter().find(|(n, _)| *n == name).map(|(_, v)| v.clone())
    }

    fn who(&self) -> Result<Who, String> {
        let source = self.get("source").filter(|s| !s.trim().is_empty()).ok_or_else(|| tr(Text::SourceRequired))?;
        Ok(Who { source, project: self.get("project").filter(|p| !p.trim().is_empty()) })
    }
}

fn read_options(args: impl IntoIterator<Item = OsString>, allowed: &[&'static str], takes_command: bool) -> Result<Parsed, String> {
    let mut args = args.into_iter();
    let mut parsed = Parsed { options: Vec::new(), rest: Vec::new(), help: false };
    while let Some(arg) = args.next() {
        let Some(text) = arg.to_str() else {
            return Err(tr(Text::UnreadableArgument { arg: &arg.to_string_lossy() }));
        };
        if text == "--" || !text.starts_with('-') {
            if !takes_command {
                return Err(tr(Text::UnexpectedArgument { arg: text }));
            }
            if text != "--" {
                parsed.rest.push(arg);
            }
            parsed.rest.extend(args);
            break;
        }
        if text == "-h" || text == "--help" {
            parsed.help = true;
            continue;
        }
        let (name, inline) = match text.trim_start_matches("--").split_once('=') {
            Some((name, value)) => (name, Some(value.to_owned())),
            None => (text.trim_start_matches("--"), None),
        };
        let Some(&name) = allowed.iter().find(|a| **a == name) else {
            return Err(tr(Text::UnknownOption { option: text }));
        };
        let value = match inline {
            Some(value) => value,
            None => args
                .next()
                .and_then(|v| v.into_string().ok())
                .ok_or_else(|| tr(Text::NeedsValue { option: name }))?,
        };
        if parsed.get(name).is_some() {
            return Err(tr(Text::GivenTwice { option: name }));
        }
        parsed.options.push((name, value));
    }
    Ok(parsed)
}

/// Reads `ilot`'s arguments (without the program name).
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Command, String> {
    let mut args = args.into_iter();
    let Some(sub) = args.next() else { return Ok(Command::Help) };
    match sub.to_str().unwrap_or_default() {
        "-h" | "--help" | "help" => Ok(Command::Help),
        "emit" => {
            let parsed = read_options(args, &["source", "kind", "title", "project", "detail"], false)?;
            if parsed.help {
                return Ok(Command::Help);
            }
            let kind = parsed.get("kind").ok_or_else(|| tr(Text::KindRequired))?;
            if !KINDS.contains(&kind.as_str()) {
                return Err(tr(Text::UnknownKind { kind: &kind, expected: &KINDS.join(", ") }));
            }
            Ok(Command::Emit(Emit { who: parsed.who()?, kind, title: parsed.get("title"), detail: parsed.get("detail") }))
        }
        sub @ ("run" | "run-begin") => {
            let parsed = read_options(args, &["source", "project"], true)?;
            if parsed.help {
                return Ok(Command::Help);
            }
            let who = parsed.who()?;
            if parsed.rest.is_empty() {
                return Err(tr(Text::CommandMissing));
            }
            Ok(if sub == "run" { Command::Run { who, command: parsed.rest } } else { Command::RunBegin { who, command: parsed.rest } })
        }
        "run-end" => {
            let parsed = read_options(args, &["source", "project", "code"], false)?;
            let code = parsed.get("code").and_then(|c| c.parse().ok()).ok_or_else(|| tr(Text::CodeNeedsNumber))?;
            Ok(Command::RunEnd { who: parsed.who()?, code })
        }
        other => Err(tr(Text::UnknownCommand { command: other })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    fn who(source: &str, project: Option<&str>) -> Who {
        Who { source: source.into(), project: project.map(str::to_owned) }
    }

    #[test]
    fn emit_reads_every_option_in_both_spellings() {
        let parsed = parse(args(&["emit", "--source", "ci", "--kind=working", "--title", "Analyse des logs", "--project=epilot", "--detail", "lit app.log"]));
        assert_eq!(
            parsed.unwrap(),
            Command::Emit(Emit {
                who: who("ci", Some("epilot")),
                kind: "working".into(),
                title: Some("Analyse des logs".into()),
                detail: Some("lit app.log".into()),
            })
        );
    }

    #[test]
    fn run_keeps_the_command_exactly_as_given() {
        let command = ["sh", "-c", "echo \"$1\" | wc -c; rm *", "--source", "--", "a b"];
        let mut all = args(&["run", "--source", "tests", "--"]);
        all.extend(args(&command));
        assert_eq!(parse(all).unwrap(), Command::Run { who: who("tests", None), command: args(&command) });
        // Without `--`, the first word that is not an option starts the command.
        assert_eq!(
            parse(args(&["run", "--source=tests", "cargo", "test", "--release"])).unwrap(),
            Command::Run { who: who("tests", None), command: args(&["cargo", "test", "--release"]) }
        );
    }

    #[test]
    fn the_wsl_script_halves_parse() {
        assert_eq!(
            parse(args(&["run-begin", "--source", "ci", "--project", "site", "--", "make"])).unwrap(),
            Command::RunBegin { who: who("ci", Some("site")), command: args(&["make"]) }
        );
        assert_eq!(
            parse(args(&["run-end", "--source", "ci", "--project", "site", "--code", "130"])).unwrap(),
            Command::RunEnd { who: who("ci", Some("site")), code: 130 }
        );
    }

    #[test]
    fn mistakes_are_explained() {
        let error = |list: &[&str]| parse(args(list)).unwrap_err();
        assert_eq!(error(&["emit", "--kind", "working"]), "--source est obligatoire");
        assert!(error(&["emit", "--source", "ci", "--kind", "done"]).starts_with("--kind inconnu : done"));
        assert_eq!(error(&["emit", "--source", "ci"]), "--kind est obligatoire");
        assert_eq!(error(&["emit", "--source"]), "--source attend une valeur");
        assert_eq!(error(&["emit", "--source", "a", "--source", "b", "--kind", "error"]), "--source est donné deux fois");
        assert_eq!(error(&["emit", "--color", "red"]), "option inconnue : --color");
        assert_eq!(error(&["emit", "--source", "ci", "--kind", "working", "extra"]), "argument inattendu : extra");
        assert_eq!(error(&["run", "--source", "ci"]), "il manque la commande à lancer, après --");
        assert_eq!(error(&["run", "--source", " ", "--", "ls"]), "--source est obligatoire");
        assert_eq!(error(&["run-end", "--source", "ci", "--code", "x"]), "--code attend un nombre");
        assert_eq!(error(&["launch"]), "commande inconnue : launch (emit ou run)");
    }

    #[test]
    fn help_is_there_when_asked_or_lost() {
        assert_eq!(parse(args(&[])).unwrap(), Command::Help);
        assert_eq!(parse(args(&["--help"])).unwrap(), Command::Help);
        assert_eq!(parse(args(&["run", "-h"])).unwrap(), Command::Help);
    }
}
