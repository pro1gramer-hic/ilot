//! What `ilot` says, in îlot's language: the one chosen in its settings
//! (`%APPDATA%\ilot\settings.json`), Windows' own with « auto » (ilot_bridge::lang). Each
//! language says every text (one `match` each, which the compiler wants complete).

use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lang {
    Fr,
    En,
}

const CODES: [&str; 2] = ["fr", "en"];

/// The language îlot's settings ask for (none when they cannot be read).
fn chosen() -> String {
    std::env::var_os("APPDATA")
        .and_then(|dir| std::fs::read(PathBuf::from(dir).join("ilot").join("settings.json")).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|settings| settings.get("language").and_then(serde_json::Value::as_str).map(str::to_owned))
        .unwrap_or_default()
}

/// The tests speak French; the tool speaks îlot's language.
fn lang() -> Lang {
    if cfg!(test) {
        return Lang::Fr;
    }
    static LANG: OnceLock<Lang> = OnceLock::new();
    *LANG.get_or_init(|| {
        let chosen = chosen();
        let system = if CODES.contains(&chosen.as_str()) { Vec::new() } else { ilot_bridge::lang::system_languages() };
        if ilot_bridge::lang::resolve(&chosen, &system, &CODES, "en") == "fr" { Lang::Fr } else { Lang::En }
    })
}

/// Every text of `ilot`.
#[derive(Clone, Copy, Debug)]
pub enum Text<'a> {
    Usage,
    /// An error, before the usage.
    Failed { message: &'a str },
    SourceRequired,
    KindRequired,
    NeedsValue { option: &'a str },
    GivenTwice { option: &'a str },
    CommandMissing,
    CodeNeedsNumber,
    UnreadableArgument { arg: &'a str },
    UnexpectedArgument { arg: &'a str },
    UnknownOption { option: &'a str },
    UnknownKind { kind: &'a str, expected: &'a str },
    UnknownCommand { command: &'a str },
    EventNotSent,
    CommandRunsAnyway,
    CannotStart { program: &'a str, error: &'a str },
    /// The detail of a failed command, shown in îlot.
    ExitCode { code: i32 },
}

const NOT_HEARD_FR: &str = "ilot : îlot ne répond pas (est-il lancé, dans la même session Windows ?)";
const NOT_HEARD_EN: &str = "ilot: îlot isn't answering (is it running, in the same Windows session?)";

fn french(text: Text<'_>) -> String {
    match text {
        Text::Usage => "Utilisation :
  ilot emit --source <nom> --kind <working|turn_done|error|session_start|session_end> [--title \"…\"] [--project <nom>] [--detail \"…\"]
  ilot run --source <nom> [--project <nom>] -- <commande> [arguments…]
Sous Windows, la commande s'appelle ilot-cli.exe."
            .into(),
        Text::Failed { message } => format!("ilot : {message}"),
        Text::SourceRequired => "--source est obligatoire".into(),
        Text::KindRequired => "--kind est obligatoire".into(),
        Text::NeedsValue { option } => format!("--{option} attend une valeur"),
        Text::GivenTwice { option } => format!("--{option} est donné deux fois"),
        Text::CommandMissing => "il manque la commande à lancer, après --".into(),
        Text::CodeNeedsNumber => "--code attend un nombre".into(),
        Text::UnreadableArgument { arg } => format!("argument illisible : {arg}"),
        Text::UnexpectedArgument { arg } => format!("argument inattendu : {arg}"),
        Text::UnknownOption { option } => format!("option inconnue : {option}"),
        Text::UnknownKind { kind, expected } => format!("--kind inconnu : {kind} (attendu : {expected})"),
        Text::UnknownCommand { command } => format!("commande inconnue : {command} (emit ou run)"),
        Text::EventNotSent => format!("{NOT_HEARD_FR} : événement non transmis."),
        Text::CommandRunsAnyway => format!("{NOT_HEARD_FR} : la commande tourne quand même."),
        Text::CannotStart { program, error } => format!("ilot : impossible de lancer {program} : {error}"),
        Text::ExitCode { code } => format!("code de sortie {code}"),
    }
}

fn english(text: Text<'_>) -> String {
    match text {
        Text::Usage => "Usage:
  ilot emit --source <name> --kind <working|turn_done|error|session_start|session_end> [--title \"…\"] [--project <name>] [--detail \"…\"]
  ilot run --source <name> [--project <name>] -- <command> [arguments…]
On Windows, the command is called ilot-cli.exe."
            .into(),
        Text::Failed { message } => format!("ilot: {message}"),
        Text::SourceRequired => "--source is required".into(),
        Text::KindRequired => "--kind is required".into(),
        Text::NeedsValue { option } => format!("--{option} needs a value"),
        Text::GivenTwice { option } => format!("--{option} is given twice"),
        Text::CommandMissing => "the command to run is missing, after --".into(),
        Text::CodeNeedsNumber => "--code needs a number".into(),
        Text::UnreadableArgument { arg } => format!("unreadable argument: {arg}"),
        Text::UnexpectedArgument { arg } => format!("unexpected argument: {arg}"),
        Text::UnknownOption { option } => format!("unknown option: {option}"),
        Text::UnknownKind { kind, expected } => format!("unknown --kind: {kind} (expected: {expected})"),
        Text::UnknownCommand { command } => format!("unknown command: {command} (emit or run)"),
        Text::EventNotSent => format!("{NOT_HEARD_EN}: event not sent."),
        Text::CommandRunsAnyway => format!("{NOT_HEARD_EN}: the command runs anyway."),
        Text::CannotStart { program, error } => format!("ilot: couldn't start {program}: {error}"),
        Text::ExitCode { code } => format!("exit code {code}"),
    }
}

fn text_in(lang: Lang, text: Text<'_>) -> String {
    match lang {
        Lang::Fr => french(text),
        Lang::En => english(text),
    }
}

/// `text` in îlot's language.
pub fn tr(text: Text<'_>) -> String {
    text_in(lang(), text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Text<'static>; 17] = [
        Text::Usage,
        Text::Failed { message: "<value>" },
        Text::SourceRequired,
        Text::KindRequired,
        Text::NeedsValue { option: "<value>" },
        Text::GivenTwice { option: "<value>" },
        Text::CommandMissing,
        Text::CodeNeedsNumber,
        Text::UnreadableArgument { arg: "<value>" },
        Text::UnexpectedArgument { arg: "<value>" },
        Text::UnknownOption { option: "<value>" },
        Text::UnknownKind { kind: "<value>", expected: "<value>" },
        Text::UnknownCommand { command: "<value>" },
        Text::EventNotSent,
        Text::CommandRunsAnyway,
        Text::CannotStart { program: "<value>", error: "<value>" },
        Text::ExitCode { code: 7 },
    ];

    #[test]
    fn every_text_exists_in_both_languages() {
        for text in ALL {
            let (fr, en) = (text_in(Lang::Fr, text), text_in(Lang::En, text));
            assert!(!fr.is_empty() && !en.is_empty(), "{text:?}");
            assert_ne!(fr, en, "{text:?}");
            assert_eq!(fr.contains("<value>"), en.contains("<value>"), "{text:?}");
            assert_eq!(fr.contains('7'), en.contains('7'), "{text:?}");
            assert!(!en.contains(" :") && !en.contains(" ?"), "{text:?}: {en:?}");
        }
    }

    #[test]
    fn the_tests_speak_french_and_english_reads_naturally() {
        assert_eq!(tr(Text::ExitCode { code: 3 }), "code de sortie 3");
        assert_eq!(text_in(Lang::En, Text::ExitCode { code: 3 }), "exit code 3");
        assert_eq!(text_in(Lang::En, Text::NeedsValue { option: "source" }), "--source needs a value");
        assert!(text_in(Lang::En, Text::Usage).starts_with("Usage:\n  ilot emit --source <name>"));
    }
}
