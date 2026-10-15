//! Framing of streamed answers: Server-Sent Events (Anthropic, OpenAI, LM Studio, Gemini)
//! and one JSON object per line (Ollama). Bytes come in pieces of any size; only whole
//! lines are decoded, so a character cut between two pieces is never broken.
//! https://html.spec.whatwg.org/multipage/server-sent-events.html#event-stream-interpretation
//! https://docs.ollama.com/api/streaming

use crate::locale::Text;

/// Longest line kept while waiting for its end: a stream that never ends a line stops.
const MAX_LINE_BYTES: usize = 4 * 1024 * 1024;

pub const MALFORMED: Text<'static> = Text::ChatMalformed;

/// Complete lines out of a byte stream, without their `\n` or `\r\n`.
#[derive(Default)]
pub struct Lines {
    pending: Vec<u8>,
}

impl Lines {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, Text<'static>> {
        self.pending.extend_from_slice(bytes);
        let mut lines = Vec::new();
        while let Some(end) = self.pending.iter().position(|&b| b == b'\n') {
            let mut line: Vec<u8> = self.pending.drain(..=end).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            lines.push(String::from_utf8_lossy(&line).into_owned());
        }
        if self.pending.len() > MAX_LINE_BYTES {
            return Err(MALFORMED);
        }
        Ok(lines)
    }

    /// The last line, when the stream ends without a line end.
    pub fn finish(&mut self) -> Option<String> {
        let rest = std::mem::take(&mut self.pending);
        let text = String::from_utf8_lossy(&rest).trim_end_matches('\r').to_owned();
        (!text.is_empty()).then_some(text)
    }
}

/// One server-sent event: its `event:` name, if any, and its `data:` lines joined by `\n`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub name: Option<String>,
    pub data: String,
}

/// Events out of a byte stream: an empty line sends the event gathered so far.
#[derive(Default)]
pub struct Sse {
    lines: Lines,
    name: Option<String>,
    data: Vec<String>,
}

impl Sse {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Event>, Text<'static>> {
        let mut events = Vec::new();
        for line in self.lines.push(bytes)? {
            self.line(&line, &mut events);
        }
        Ok(events)
    }

    /// What is left when the stream closes (a last event without its empty line).
    pub fn finish(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        if let Some(line) = self.lines.finish() {
            self.line(&line, &mut events);
        }
        self.line("", &mut events);
        events
    }

    fn line(&mut self, line: &str, events: &mut Vec<Event>) {
        if line.is_empty() {
            if !self.data.is_empty() {
                events.push(Event { name: self.name.take(), data: self.data.join("\n") });
                self.data.clear();
            }
            self.name = None;
            return;
        }
        if line.starts_with(':') {
            return;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "data" => self.data.push(value.to_owned()),
            "event" => self.name = Some(value.to_owned()),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_survive_any_cut_even_inside_a_character() {
        let text = "{\"t\":\"é\"}\r\n{\"t\":\"ok\"}\n";
        let bytes = text.as_bytes();
        for cut in 1..bytes.len() {
            let mut lines = Lines::default();
            let mut all = lines.push(&bytes[..cut]).unwrap();
            all.extend(lines.push(&bytes[cut..]).unwrap());
            assert_eq!(all, ["{\"t\":\"é\"}", "{\"t\":\"ok\"}"], "cut at {cut}");
            assert_eq!(lines.finish(), None);
        }
        let mut lines = Lines::default();
        assert!(lines.push(b"{\"done\":true}").unwrap().is_empty());
        assert_eq!(lines.finish().as_deref(), Some("{\"done\":true}"));
    }

    #[test]
    fn a_line_that_never_ends_stops_the_stream() {
        let mut lines = Lines::default();
        assert_eq!(lines.push(&vec![b'x'; MAX_LINE_BYTES + 1]), Err(MALFORMED));
    }

    /// The streaming example of https://platform.claude.com/docs/en/build-with-claude/streaming
    #[test]
    fn server_sent_events_with_names_comments_and_split_pieces() {
        let stream = "event: message_start\ndata: {\"type\": \"message_start\"}\n\n: keep-alive\n\nevent: content_block_delta\ndata: {\"type\": \"content_block_delta\", \"index\": 0, \"delta\": {\"type\": \"text_delta\", \"text\": \"Hello\"}}\n\nevent: ping\ndata: {\"type\": \"ping\"}\r\n\r\ndata: a\ndata: b\n\n";
        let mut sse = Sse::default();
        let mut events = Vec::new();
        for piece in stream.as_bytes().chunks(7) {
            events.extend(sse.push(piece).unwrap());
        }
        events.extend(sse.finish());
        let names: Vec<Option<&str>> = events.iter().map(|e| e.name.as_deref()).collect();
        assert_eq!(names, [Some("message_start"), Some("content_block_delta"), Some("ping"), None]);
        assert!(events[1].data.contains("\"text\": \"Hello\""));
        assert_eq!(events[3].data, "a\nb");
    }

    #[test]
    fn a_last_event_without_its_empty_line_still_counts() {
        let mut sse = Sse::default();
        assert!(sse.push(b"data: {\"candidates\":[]}").unwrap().is_empty());
        assert_eq!(sse.finish(), [Event { name: None, data: "{\"candidates\":[]}".into() }]);
        assert!(Sse::default().finish().is_empty());
    }

    #[test]
    fn a_broken_stream_is_said_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let mut lines = Lines::default();
        let error = lines.push(&vec![b'x'; MAX_LINE_BYTES + 1]).unwrap_err();
        assert_eq!(String::from(error), "Unreadable answer from the provider");
    }
}
