//! One message, two notifications: WhatsApp's desktop app and its web tab both tell
//! Windows about the same message, a few seconds apart. îlot shows it once. Only a
//! fingerprint (a hash) and an arrival time are kept, in memory, for a few seconds: never
//! the text.

use std::collections::VecDeque;
use std::hash::{DefaultHasher, Hash, Hasher};

/// Copies of one message arrive within this delay of each other.
pub const DELAY_MS: u64 = 10_000;
/// Messages remembered at most: a burst of a few seconds.
const MAX_RECENT: usize = 64;

fn hash(value: &impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// Fingerprint of a message: its rule, sender and text, whatever the case and spacing.
pub fn fingerprint(rule: &str, lines: &[String]) -> u64 {
    let words: Vec<Vec<String>> =
        lines.iter().map(|line| line.split_whitespace().map(str::to_lowercase).collect()).collect();
    hash(&(rule, words))
}

#[derive(Clone, Copy)]
struct Seen {
    print: u64,
    /// Hash of the app name Windows gave (« WhatsApp », « Mozilla Firefox »).
    source: u64,
    at: u64,
}

/// Messages shown during the last seconds.
#[derive(Default)]
pub struct Recent {
    seen: VecDeque<Seen>,
}

impl Recent {
    /// True when another app sent the same message less than `DELAY_MS` ago: the island
    /// already shows it. Otherwise the message is remembered. Two identical messages from
    /// the same app (« ok », « ok ») are both shown.
    pub fn is_copy(&mut self, print: u64, source: &str, at: u64) -> bool {
        let source = hash(&source);
        self.seen.retain(|seen| seen.at.abs_diff(at) <= DELAY_MS);
        if self.seen.iter().any(|seen| seen.print == print && seen.source != source) {
            return true;
        }
        self.seen.push_back(Seen { print, source, at });
        while self.seen.len() > MAX_RECENT {
            self.seen.pop_front();
        }
        false
    }

    pub fn clear(&mut self) {
        self.seen.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(list: &[&str]) -> Vec<String> {
        list.iter().map(|line| (*line).to_owned()).collect()
    }

    #[test]
    fn the_app_and_the_tab_show_one_message_once() {
        let mut recent = Recent::default();
        let message = fingerprint("whatsapp", &lines(&["Maman", "Tu rentres à quelle heure ?"]));
        assert!(!recent.is_copy(message, "WhatsApp", 1_000));
        assert!(recent.is_copy(message, "Mozilla Firefox", 3_500));
        // Either side may come first.
        let other = fingerprint("whatsapp", &lines(&["Léa", "On se voit demain ?"]));
        assert!(!recent.is_copy(other, "Mozilla Firefox", 4_000));
        assert!(recent.is_copy(other, "WhatsApp", 4_200));
    }

    #[test]
    fn the_same_app_repeating_itself_is_shown_twice() {
        let mut recent = Recent::default();
        let ok = fingerprint("whatsapp", &lines(&["Maman", "ok"]));
        assert!(!recent.is_copy(ok, "WhatsApp", 1_000));
        assert!(!recent.is_copy(ok, "WhatsApp", 2_000));
    }

    #[test]
    fn copies_too_far_apart_are_two_messages() {
        let mut recent = Recent::default();
        let message = fingerprint("whatsapp", &lines(&["Maman", "Tu as pensé au pain ?"]));
        assert!(!recent.is_copy(message, "WhatsApp", 1_000));
        assert!(!recent.is_copy(message, "Mozilla Firefox", 1_000 + DELAY_MS + 1));
    }

    #[test]
    fn another_sender_text_or_rule_is_another_message() {
        let mut recent = Recent::default();
        assert!(!recent.is_copy(fingerprint("whatsapp", &lines(&["Maman", "ok"])), "WhatsApp", 0));
        assert!(!recent.is_copy(fingerprint("whatsapp", &lines(&["Papa", "ok"])), "Mozilla Firefox", 10));
        assert!(!recent.is_copy(fingerprint("whatsapp", &lines(&["Maman", "ok !"])), "Mozilla Firefox", 20));
        assert!(!recent.is_copy(fingerprint("web", &lines(&["Maman", "ok"])), "Mozilla Firefox", 30));
    }

    #[test]
    fn case_and_spacing_do_not_matter() {
        let app = fingerprint("whatsapp", &lines(&["Maman", "Tu rentres ?"]));
        assert_eq!(app, fingerprint("whatsapp", &lines(&["maman ", "tu  rentres ?"])));
        assert_ne!(app, fingerprint("whatsapp", &lines(&["Maman Tu", "rentres ?"])));
    }

    #[test]
    fn memory_is_bounded_and_can_be_cleared() {
        let mut recent = Recent::default();
        for i in 0..(MAX_RECENT as u64 + 10) {
            recent.is_copy(i, "WhatsApp", 1_000);
        }
        assert_eq!(recent.seen.len(), MAX_RECENT);
        assert!(!recent.is_copy(0, "Mozilla Firefox", 1_000), "the oldest was forgotten");
        assert!(recent.is_copy(MAX_RECENT as u64 + 9, "Mozilla Firefox", 1_000));
        recent.clear();
        assert!(recent.seen.is_empty());
    }
}
