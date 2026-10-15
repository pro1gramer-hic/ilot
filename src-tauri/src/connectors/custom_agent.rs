//! `ilot_agent`: a key any hook payload may carry to give its session a badge of its own,
//! its dot in the pill and its tile in the hub (docs/AGENTS.md):
//!
//! ```json
//! { "ilot_agent": { "id": "qwen", "name": "Qwen Code", "color": "teal" } }
//! ```
//!
//! or simply `{ "ilot_agent": "Qwen Code" }`. Every value is checked and bounded: an id of
//! a few plain characters (made from the name when missing), a short visible name that
//! never takes the name of an AI îlot follows by its own hooks, and a color from îlot's
//! list (a `#rrggbb` goes to the nearest one, so that every color reads in every theme).
//! Nothing else of it is ever used.

use super::activity::shorten;
use super::claude_code::is_deceptive;
use crate::locale::{Text, tr};
use serde::Serialize;
use serde_json::Value;

pub const KEY: &str = "ilot_agent";
const ID_CHARS: usize = 24;
pub const NAME_CHARS: usize = 24;

/// The badge colors (`--agent-*` in src/lib/tokens.css, checked in the seven themes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentColor {
    Sky,
    Teal,
    Lime,
    Gold,
    Coral,
    Pink,
    Violet,
    Slate,
}

/// Each color with its name and its value in the tokens.
const COLORS: [(AgentColor, &str, [u8; 3]); 8] = [
    (AgentColor::Sky, "sky", [0x6c, 0xb8, 0xff]),
    (AgentColor::Teal, "teal", [0x3c, 0xcf, 0xb4]),
    (AgentColor::Lime, "lime", [0xa8, 0xd6, 0x5a]),
    (AgentColor::Gold, "gold", [0xe3, 0xc2, 0x6b]),
    (AgentColor::Coral, "coral", [0xff, 0x8a, 0x6b]),
    (AgentColor::Pink, "pink", [0xf0, 0x7a, 0xb8]),
    (AgentColor::Violet, "violet", [0xb4, 0x93, 0xff]),
    (AgentColor::Slate, "slate", [0x9a, 0xa6, 0xb8]),
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CustomAgent {
    pub id: String,
    pub name: String,
    pub color: AgentColor,
}

/// The AIs îlot follows through hooks of their own: a badge never borrows their name.
pub const RESERVED_NAMES: [&str; 11] =
    ["claude", "claude code", "gemini", "gemini cli", "codex", "cursor", "copilot", "opencode", "amp", "bob", "kiro"];

/// `name (autre)` when the name is one of an AI îlot follows by its own hooks.
pub fn unreserved(name: String) -> String {
    let words: Vec<String> = name.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_owned).collect();
    let key = words.join(" ");
    if RESERVED_NAMES.iter().any(|reserved| key == *reserved || key.starts_with(&format!("{reserved} "))) {
        tr(Text::OtherAgent { name: &name })
    } else {
        name
    }
}

/// The first visible line of a text the user chose, without the characters that could
/// disguise it, cut to `max` characters.
pub fn clean(text: &str, max: usize) -> Option<String> {
    let line = text.lines().find(|l| !l.trim().is_empty()).unwrap_or_default();
    let visible: String = line.chars().filter(|c| !is_deceptive(*c)).collect();
    let visible = visible.trim();
    (!visible.is_empty()).then(|| shorten(visible, max))
}

/// An id made of lowercase letters, digits and dashes.
fn slug(text: &str) -> Option<String> {
    let mut id = String::new();
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            id.push(c);
        } else if !id.is_empty() && !id.ends_with('-') {
            id.push('-');
        }
    }
    let id: String = id.trim_end_matches('-').chars().take(ID_CHARS).collect();
    let id = id.trim_end_matches('-').to_owned();
    (!id.is_empty()).then_some(id)
}

fn hex(text: &str) -> Option<[u8; 3]> {
    let digits = text.strip_prefix('#')?;
    let full: String = match digits.len() {
        3 => digits.chars().flat_map(|c| [c, c]).collect(),
        6 => digits.to_owned(),
        _ => return None,
    };
    let value = u32::from_str_radix(&full, 16).ok()?;
    Some([(value >> 16) as u8, (value >> 8) as u8, value as u8])
}

/// A color by its name, or the nearest one to a `#rrggbb`; `None` for anything else.
fn color(text: &str) -> Option<AgentColor> {
    let text = text.trim().to_lowercase();
    if let Some((color, _, _)) = COLORS.iter().find(|(_, name, _)| *name == text) {
        return Some(*color);
    }
    let [r, g, b] = hex(&text)?;
    let distance = |[x, y, z]: [u8; 3]| (i32::from(x) - i32::from(r)).pow(2) + (i32::from(y) - i32::from(g)).pow(2) + (i32::from(z) - i32::from(b)).pow(2);
    COLORS.iter().min_by_key(|(_, _, rgb)| distance(*rgb)).map(|(color, _, _)| *color)
}

/// A color that stays the same for the same id when none was given.
fn color_of(id: &str) -> AgentColor {
    let sum = id.bytes().fold(0usize, |acc, b| acc.wrapping_mul(31).wrapping_add(usize::from(b)));
    COLORS[sum % COLORS.len()].0
}

/// The badge a payload asks for, checked; `None` when there is none or it is unusable.
pub fn read(payload: &Value) -> Option<CustomAgent> {
    let (name, id, wanted) = match payload.get(KEY)? {
        Value::String(name) => (name.as_str(), None, None),
        Value::Object(fields) => (
            fields.get("name").and_then(Value::as_str)?,
            fields.get("id").and_then(Value::as_str),
            fields.get("color").and_then(Value::as_str),
        ),
        _ => return None,
    };
    let name = unreserved(clean(name, NAME_CHARS)?);
    let id = match id {
        Some(id) => slug(id).filter(|slugged| slugged == &id.to_lowercase())?,
        None => slug(&name)?,
    };
    let color = wanted.and_then(color).unwrap_or_else(|| color_of(&id));
    Some(CustomAgent { id, name, color })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_documented_example_gives_its_badge() {
        let payload = json!({ "session_id": "s", "ilot_agent": { "id": "qwen", "name": "Qwen Code", "color": "teal" } });
        assert_eq!(read(&payload), Some(CustomAgent { id: "qwen".into(), name: "Qwen Code".into(), color: AgentColor::Teal }));
        assert_eq!(serde_json::to_value(read(&payload).unwrap()).unwrap(), json!({ "id": "qwen", "name": "Qwen Code", "color": "teal" }));
    }

    #[test]
    fn a_name_alone_is_enough() {
        let agent = read(&json!({ "ilot_agent": "Mon outil" })).unwrap();
        assert_eq!((agent.id.as_str(), agent.name.as_str()), ("mon-outil", "Mon outil"));
        assert_eq!(agent.color, read(&json!({ "ilot_agent": "Mon outil" })).unwrap().color, "the same color every time");
        assert!(read(&json!({ "session_id": "s" })).is_none());
        assert!(read(&json!({ "ilot_agent": 42 })).is_none());
        assert!(read(&json!({ "ilot_agent": { "id": "x" } })).is_none(), "a name is needed");
    }

    #[test]
    fn hex_colors_go_to_the_nearest_of_the_list() {
        let with = |c: &str| read(&json!({ "ilot_agent": { "name": "A", "color": c } })).unwrap().color;
        assert_eq!(with("#3ccfb4"), AgentColor::Teal);
        assert_eq!(with("#00ffcc"), AgentColor::Teal);
        assert_eq!(with("#f0f"), AgentColor::Pink);
        assert_eq!(with("VIOLET"), AgentColor::Violet);
        // Anything else: the color of the id.
        let fallback = read(&json!({ "ilot_agent": { "name": "A" } })).unwrap().color;
        assert_eq!(with("red; background: url(x)"), fallback);
        assert_eq!(with("#12345"), fallback);
    }

    #[test]
    fn values_are_bounded_and_cannot_disguise_themselves() {
        let agent = read(&json!({ "ilot_agent": { "name": "Outil\u{202e}\u{200b} très long qui dépasse largement la limite\nrm -rf" } })).unwrap();
        assert_eq!(agent.name, format!("{}…", "Outil très long qui dépa"));
        assert!(agent.id.len() <= ID_CHARS);
        assert!(read(&json!({ "ilot_agent": { "name": "\u{200b}" } })).is_none());
        // An id must already be plain: nothing is made of a strange one.
        assert!(read(&json!({ "ilot_agent": { "id": "../x", "name": "A" } })).is_none());
        assert!(read(&json!({ "ilot_agent": { "id": "Qwen", "name": "A" } })).is_some(), "case does not matter");
        assert!(read(&json!({ "ilot_agent": { "id": "a".repeat(40), "name": "A" } })).is_none());
    }

    #[test]
    fn the_followed_ais_keep_their_names() {
        for (name, shown) in [("Claude", "Claude (autre)"), ("Cursor", "Cursor (autre)"), ("kiro cli", "kiro cli (autre)"), ("Qwen", "Qwen")] {
            assert_eq!(read(&json!({ "ilot_agent": name })).unwrap().name, shown);
        }
    }

    #[test]
    fn a_borrowed_name_is_marked_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(unreserved("Claude".into()), "Claude (other)");
        assert_eq!(unreserved("Qwen".into()), "Qwen");
    }
}
