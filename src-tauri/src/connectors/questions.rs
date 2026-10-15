//! Questions an agent asks the user (Claude Code's `AskUserQuestion`), and the user's
//! answers given from îlot. The tool's input is read as documented
//! (https://code.claude.com/docs/en/hooks#askuserquestion: one to four questions, each with
//! its text, a short `header`, two to four `options` of `{label, description}` and an
//! optional `multiSelect`). A question îlot cannot show exactly as asked (too many, too
//! long, two with the same text…) is left to the terminal.
//!
//! An answer is checked against the question it answers: only the labels offered, one of
//! them for a simple choice, and a short line of text for « Autre réponse ». Nothing is
//! ever chosen for the user.

use super::claude_code::is_deceptive;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

pub const MAX_QUESTIONS: usize = 4;
const OPTIONS: std::ops::RangeInclusive<usize> = 2..=4;
const MAX_QUESTION_CHARS: usize = 1_000;
const MAX_HEADER_CHARS: usize = 40;
const MAX_LABEL_CHARS: usize = 200;
const MAX_DESCRIPTION_CHARS: usize = 600;
/// Longest « Autre réponse » the user may type.
pub const MAX_OTHER_CHARS: usize = 500;
/// Multi-select answers join labels with commas
/// (https://code.claude.com/docs/en/hooks#askuserquestion; the Agent SDK joins them
/// with ", ": https://code.claude.com/docs/en/agent-sdk/user-input).
const JOIN: &str = ", ";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub question: String,
    pub header: String,
    pub options: Vec<Choice>,
    pub multi_select: bool,
    /// The user may type an answer of their own (Claude's « Other »).
    pub free_text: bool,
}

/// The user's answer to one question: labels picked among its options, and the text of
/// « Autre réponse » if any.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct QuestionAnswer {
    pub picked: Vec<String>,
    pub other: Option<String>,
}

/// A text field: `Some(Some(text))` when it holds a usable text, `Some(None)` when an
/// optional one is absent or blank, `None` when it cannot be shown as is.
fn text(value: &Value, key: &str, max: usize, required: bool) -> Option<Option<String>> {
    match value.get(key) {
        None | Some(Value::Null) if !required => Some(None),
        Some(Value::String(s)) if !s.trim().is_empty() && s.chars().count() <= max => Some(Some(s.clone())),
        Some(Value::String(s)) if !required && s.trim().is_empty() => Some(None),
        _ => None,
    }
}

fn question(value: &Value, free_text: bool) -> Option<Question> {
    let options = value.get("options")?.as_array()?;
    if !OPTIONS.contains(&options.len()) {
        return None;
    }
    let options = options
        .iter()
        .map(|option| {
            Some(Choice {
                label: text(option, "label", MAX_LABEL_CHARS, true)??,
                description: text(option, "description", MAX_DESCRIPTION_CHARS, false)?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    if !options.iter().enumerate().all(|(i, a)| options[..i].iter().all(|b| b.label != a.label)) {
        return None;
    }
    let multi_select = match value.get("multiSelect") {
        None | Some(Value::Null) => false,
        Some(flag) => flag.as_bool()?,
    };
    Some(Question {
        question: text(value, "question", MAX_QUESTION_CHARS, true)??,
        header: text(value, "header", MAX_HEADER_CHARS, false)?.unwrap_or_default(),
        options,
        multi_select,
        free_text,
    })
}

/// The questions of an `AskUserQuestion` input, or `None` when îlot cannot show them
/// exactly as asked.
pub fn read(input: &Value, free_text: bool) -> Option<Vec<Question>> {
    let list = input.get("questions")?.as_array()?;
    if list.is_empty() || list.len() > MAX_QUESTIONS {
        return None;
    }
    let questions = list.iter().map(|q| question(q, free_text)).collect::<Option<Vec<_>>>()?;
    let distinct = questions.iter().enumerate().all(|(i, a)| questions[..i].iter().all(|b| b.question != a.question));
    distinct.then_some(questions)
}

/// The text the user typed, as one visible line, or `None` when it cannot be sent as is.
fn other_text(other: &str) -> Option<String> {
    let line = other.trim();
    let clean = !line.is_empty() && line.chars().count() <= MAX_OTHER_CHARS && !line.chars().any(is_deceptive);
    clean.then(|| line.to_owned())
}

/// The value of one answer: the label chosen, or the labels (and typed text) joined. `None`
/// when the answer does not fit its question.
fn answer_value(question: &Question, answer: &QuestionAnswer) -> Option<String> {
    let offered = |label: &String| question.options.iter().any(|o| &o.label == label);
    let unique = answer.picked.iter().enumerate().all(|(i, a)| !answer.picked[..i].contains(a));
    if !unique || !answer.picked.iter().all(offered) {
        return None;
    }
    let other = match &answer.other {
        Some(text) if question.free_text => Some(other_text(text)?),
        Some(_) => return None,
        None => None,
    };
    // Labels in the order they were offered, then the typed text.
    let mut parts: Vec<String> =
        question.options.iter().filter(|o| answer.picked.contains(&o.label)).map(|o| o.label.clone()).collect();
    parts.extend(other);
    match (question.multi_select, parts.len()) {
        (_, 0) => None,
        (false, 1) => parts.pop(),
        (false, _) => None,
        (true, _) => Some(parts.join(JOIN)),
    }
}

/// `answers` as Claude reads it: each question's text → its answer. `None` unless every
/// question has a valid answer.
pub fn answers(questions: &[Question], given: &[QuestionAnswer]) -> Option<Map<String, Value>> {
    if questions.len() != given.len() {
        return None;
    }
    questions.iter().zip(given).map(|(q, a)| Some((q.question.clone(), json!(answer_value(q, a)?)))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The documented example (https://code.claude.com/docs/en/hooks#askuserquestion),
    /// with a second, multiple-choice question.
    fn input() -> Value {
        json!({ "questions": [
            { "question": "Which framework?", "header": "Framework", "options": [
                { "label": "React", "description": "Component library" },
                { "label": "Vue", "description": "Progressive framework" }
            ], "multiSelect": false },
            { "question": "Which tests should run?", "header": "Tests", "options": [
                { "label": "Unit" }, { "label": "E2E" }, { "label": "Lint" }
            ], "multiSelect": true }
        ] })
    }

    fn pick(labels: &[&str]) -> QuestionAnswer {
        QuestionAnswer { picked: labels.iter().map(|l| (*l).to_owned()).collect(), other: None }
    }

    #[test]
    fn documented_questions_read_as_asked() {
        let questions = read(&input(), true).unwrap();
        assert_eq!(questions.len(), 2);
        assert_eq!(questions[0].header, "Framework");
        assert_eq!(questions[0].options[1], Choice { label: "Vue".into(), description: Some("Progressive framework".into()) });
        assert!(!questions[0].multi_select && questions[1].multi_select);
        assert_eq!(questions[1].options[0].description, None);
    }

    #[test]
    fn questions_that_cannot_be_shown_exactly_go_to_the_terminal() {
        let mut too_many = input();
        too_many["questions"] = json!(vec![input()["questions"][0].clone(); 5]);
        assert!(read(&too_many, true).is_none(), "five questions");
        let mut same = input();
        same["questions"][1]["question"] = json!("Which framework?");
        assert!(read(&same, true).is_none(), "answers are keyed by the question's text");
        let mut one_option = input();
        one_option["questions"][0]["options"] = json!([{ "label": "React" }]);
        assert!(read(&one_option, true).is_none());
        let mut twin_labels = input();
        twin_labels["questions"][0]["options"][1]["label"] = json!("React");
        assert!(read(&twin_labels, true).is_none());
        let mut long = input();
        long["questions"][0]["options"][0]["label"] = json!("x".repeat(201));
        assert!(read(&long, true).is_none());
        assert!(read(&json!({ "questions": [] }), true).is_none());
        assert!(read(&json!({}), true).is_none());
    }

    #[test]
    fn answers_map_each_question_to_its_labels() {
        let questions = read(&input(), true).unwrap();
        let given = [pick(&["Vue"]), pick(&["Lint", "Unit"])];
        let map = answers(&questions, &given).unwrap();
        assert_eq!(map["Which framework?"], "Vue");
        assert_eq!(map["Which tests should run?"], "Unit, Lint", "in the order offered");
    }

    #[test]
    fn a_typed_answer_is_one_clean_line() {
        let questions = read(&input(), true).unwrap();
        let other = |text: &str| QuestionAnswer { picked: vec![], other: Some(text.into()) };
        let map = answers(&questions, &[other("  Svelte  "), QuestionAnswer { picked: vec!["E2E".into()], other: Some("Perf".into()) }]).unwrap();
        assert_eq!(map["Which framework?"], "Svelte");
        assert_eq!(map["Which tests should run?"], "E2E, Perf");
        assert!(answers(&questions, &[other("a\nb"), pick(&["Unit"])]).is_none(), "no hidden line");
        assert!(answers(&questions, &[other("\u{202e}x"), pick(&["Unit"])]).is_none());
        assert!(answers(&questions, &[other(&"y".repeat(501)), pick(&["Unit"])]).is_none());
        assert!(answers(&questions, &[other("   "), pick(&["Unit"])]).is_none());
        let closed = read(&input(), false).unwrap();
        assert!(answers(&closed, &[other("Svelte"), pick(&["Unit"])]).is_none(), "no free text offered");
    }

    #[test]
    fn nothing_is_answered_that_was_not_offered_or_not_chosen() {
        let questions = read(&input(), true).unwrap();
        assert!(answers(&questions, &[pick(&["Angular"]), pick(&["Unit"])]).is_none());
        assert!(answers(&questions, &[pick(&["React", "Vue"]), pick(&["Unit"])]).is_none(), "one choice only");
        assert!(answers(&questions, &[pick(&[]), pick(&["Unit"])]).is_none(), "no answer");
        assert!(answers(&questions, &[pick(&["React"]), pick(&["Unit", "Unit"])]).is_none());
        assert!(answers(&questions, &[pick(&["React"])]).is_none(), "every question answered");
    }
}
