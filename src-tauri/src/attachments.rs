//! Files dropped on the island (prompt 12, lot 3). Dropping only remembers a path: the
//! file is not opened, not even looked at. The interface may then turn one of the paths
//! just dropped into an attachment (an id and a name, still without opening anything);
//! its content is read only when the user sends a question about it, here in the core,
//! with a size limit, as text or as an image.
//!
//! A path the user did not drop on the island is refused: the interface cannot make îlot
//! read any other file.

use base64::Engine;
use serde::Serialize;
use std::collections::VecDeque;
use std::io::Read;
use std::path::{Path, PathBuf};
use crate::locale::{Text, tr};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Paths of the latest drops, kept for a while.
const MAX_DROPPED: usize = 16;
const DROP_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_ATTACHED: usize = 16;
/// A text file sent to a model: about 50 000 words at most.
pub const MAX_TEXT_BYTES: u64 = 200 * 1024;
/// An image: Anthropic takes up to 5 MB per image, sent in base64.
pub const MAX_IMAGE_BYTES: u64 = 3_750 * 1024;

pub const NOT_DROPPED: Text<'static> = Text::AttachmentNotDropped;
pub const UNKNOWN: Text<'static> = Text::AttachmentUnknown;

/// What the interface knows of an attachment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub name: String,
    /// Guessed from the name only: `text`, `image`, or `other`.
    pub kind: &'static str,
}

/// An attachment's content, once read for a question.
#[derive(Clone, Debug, PartialEq)]
pub enum Loaded {
    Text(String),
    Image { mime: &'static str, base64: String },
}

#[derive(Default)]
struct Inner {
    dropped: VecDeque<(PathBuf, Instant)>,
    attached: VecDeque<(String, PathBuf)>,
    serial: u64,
}

/// The paths dropped on the island and the attachments made from them.
#[derive(Default)]
pub struct Attachments {
    inner: Mutex<Inner>,
}

const IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];
const TEXT_EXTENSIONS: [&str; 34] = [
    "txt", "md", "markdown", "csv", "tsv", "json", "jsonl", "yaml", "yml", "toml", "xml", "html", "htm", "css", "js",
    "ts", "tsx", "jsx", "svelte", "py", "rs", "go", "java", "c", "h", "cpp", "cs", "sh", "ps1", "sql", "log", "ini",
    "srt", "tex",
];

pub fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string())
}

/// The kind a name suggests; the content decides when the file is read.
pub fn kind_of(path: &Path) -> &'static str {
    let extension = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        "image"
    } else if TEXT_EXTENSIONS.contains(&extension.as_str()) {
        "text"
    } else {
        "other"
    }
}

impl Attachments {
    /// A drop on the island: its paths are remembered, nothing is opened.
    pub fn remember(&self, paths: &[PathBuf]) {
        let mut inner = self.inner.lock().unwrap();
        let now = Instant::now();
        for path in paths {
            inner.dropped.push_back((path.clone(), now));
        }
        while inner.dropped.len() > MAX_DROPPED {
            inner.dropped.pop_front();
        }
    }

    /// Makes an attachment of a path dropped lately. Still nothing is opened.
    pub fn attach(&self, path: &str, now: Instant) -> Result<Attachment, String> {
        let mut inner = self.inner.lock().unwrap();
        inner.dropped.retain(|(_, at)| now.duration_since(*at) < DROP_TTL);
        let wanted = Path::new(path);
        let path = inner.dropped.iter().rev().find(|(p, _)| p == wanted).map(|(p, _)| p.clone()).ok_or(NOT_DROPPED)?;
        inner.serial += 1;
        let id = format!("f{}", inner.serial);
        inner.attached.push_back((id.clone(), path.clone()));
        while inner.attached.len() > MAX_ATTACHED {
            inner.attached.pop_front();
        }
        Ok(Attachment { id, name: file_name(&path), kind: kind_of(&path) })
    }

    pub fn path_of(&self, id: &str) -> Option<PathBuf> {
        self.inner.lock().unwrap().attached.iter().find(|(i, _)| i == id).map(|(_, p)| p.clone())
    }

    /// Reads an attachment for a question the user sends now.
    pub fn load(&self, id: &str) -> Result<(String, Loaded), String> {
        let path = self.path_of(id).ok_or(UNKNOWN)?;
        let loaded = read_for_chat(&path)?;
        Ok((file_name(&path), loaded))
    }
}

fn image_mime(head: &[u8]) -> Option<&'static str> {
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if head.starts_with(b"\xff\xd8\xff") {
        Some("image/jpeg")
    } else if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

fn kilobytes(bytes: u64) -> u64 {
    bytes / 1024
}

/// The content of a file, as the chat sends it: an image (PNG, JPEG, GIF, WebP) by its
/// first bytes, otherwise UTF-8 text without NUL. Anything else is refused, and said so.
pub fn read_for_chat(path: &Path) -> Result<Loaded, String> {
    let mut file = std::fs::File::open(path).map_err(|_| tr(Text::FileCannotOpen { name: &file_name(path) }))?;
    let mut head = [0u8; 16];
    let mut seen = 0;
    while seen < head.len() {
        match file.read(&mut head[seen..]) {
            Ok(0) => break,
            Ok(n) => seen += n,
            Err(_) => return Err(tr(Text::FileCannotRead { name: &file_name(path) })),
        }
    }
    let head = &head[..seen];
    let image = image_mime(head);
    let limit = if image.is_some() { MAX_IMAGE_BYTES } else { MAX_TEXT_BYTES };
    let mut bytes = head.to_vec();
    file.take(limit + 1 - seen as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| tr(Text::FileCannotRead { name: &file_name(path) }))?;
    if bytes.len() as u64 > limit {
        return Err(tr(Text::FileTooBig { name: &file_name(path), kilobytes: kilobytes(limit) }));
    }
    if let Some(mime) = image {
        return Ok(Loaded::Image { mime, base64: base64::engine::general_purpose::STANDARD.encode(&bytes) });
    }
    match String::from_utf8(bytes) {
        Ok(text) if !text.contains('\0') => Ok(Loaded::Text(text)),
        _ => Err(tr(Text::FileNotTextOrImage { name: &file_name(path) })),
    }
}

/// Lets the interface attach a file the user just dropped on the island.
#[tauri::command]
pub fn file_attach(path: String, attachments: tauri::State<'_, Attachments>) -> Result<Attachment, String> {
    attachments.attach(&path, Instant::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;

    /// A file of its own folder (tests run side by side), which `forget` removes whole.
    fn temp(name: &str, content: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ilot-attach-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    /// Removes a file made by `temp` and its folder: the tests leave nothing in Temp.
    fn forget(path: PathBuf) {
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn dropping_and_attaching_never_open_the_file() {
        let attachments = Attachments::default();
        // A path to nothing at all: remembering and attaching it cannot have read it.
        let ghost = PathBuf::from(r"C:\nulle-part\rapport secret.pdf");
        attachments.remember(std::slice::from_ref(&ghost));
        let attached = attachments.attach(&ghost.to_string_lossy(), Instant::now()).unwrap();
        assert_eq!((attached.name.as_str(), attached.kind), ("rapport secret.pdf", "other"));
        assert!(attachments.load(&attached.id).is_err(), "only now is it opened");

        // A file locked by someone else: attached fine, read only when asked.
        let path = temp("verrou.txt", b"contenu");
        let lock = std::fs::OpenOptions::new().read(true).share_mode(0).open(&path).unwrap();
        attachments.remember(std::slice::from_ref(&path));
        let attached = attachments.attach(&path.to_string_lossy(), Instant::now()).unwrap();
        assert!(attachments.load(&attached.id).is_err());
        drop(lock);
        assert_eq!(attachments.load(&attached.id).unwrap(), ("verrou.txt".into(), Loaded::Text("contenu".into())));
        forget(path);
    }

    #[test]
    fn only_a_path_dropped_lately_can_be_attached() {
        let attachments = Attachments::default();
        assert_eq!(attachments.attach(r"C:\Users\moi\.ssh\id_ed25519", Instant::now()).unwrap_err(), NOT_DROPPED);
        let path = PathBuf::from(r"C:\docs\note.md");
        attachments.remember(std::slice::from_ref(&path));
        let later = Instant::now() + DROP_TTL + Duration::from_secs(1);
        assert_eq!(attachments.attach(r"C:\docs\note.md", later).unwrap_err(), NOT_DROPPED);
        attachments.remember(std::slice::from_ref(&path));
        let first = attachments.attach(r"C:\docs\note.md", Instant::now()).unwrap();
        assert_eq!(first.kind, "text");
        assert_eq!(attachments.path_of(&first.id), Some(path));
        assert_eq!(attachments.load("f999").unwrap_err(), UNKNOWN);
    }

    #[test]
    fn content_decides_between_text_image_and_refusal() {
        let png = temp("pixel.png", b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR");
        assert!(matches!(read_for_chat(&png).unwrap(), Loaded::Image { mime: "image/png", .. }));
        let disguised = temp("photo.txt", b"\xff\xd8\xff\xe0\0\x10JFIF");
        assert!(matches!(read_for_chat(&disguised).unwrap(), Loaded::Image { mime: "image/jpeg", .. }));
        let webp = temp("a.webp", b"RIFF\x10\0\0\0WEBPVP8 ");
        assert!(matches!(read_for_chat(&webp).unwrap(), Loaded::Image { mime: "image/webp", .. }));
        let text = temp("notes.md", "Réunion à 14 h".as_bytes());
        assert_eq!(read_for_chat(&text).unwrap(), Loaded::Text("Réunion à 14 h".into()));
        let binary = temp("archive.zip", b"PK\x03\x04\0\0\x08\0");
        assert!(read_for_chat(&binary).unwrap_err().contains("ne sait lire que du texte et des images"));
        let big = temp("gros.log", &vec![b'a'; MAX_TEXT_BYTES as usize + 1]);
        assert_eq!(read_for_chat(&big).unwrap_err(), "gros.log est trop gros pour le chat (200 Ko au plus).");
        let edge = temp("juste.log", &vec![b'a'; MAX_TEXT_BYTES as usize]);
        assert!(read_for_chat(&edge).is_ok());
        for path in [png, disguised, webp, text, binary, big, edge] {
            forget(path);
        }
    }

    #[test]
    fn refusals_are_said_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let attachments = Attachments::default();
        assert_eq!(attachments.attach(r"C:\x\never-dropped.md", Instant::now()).unwrap_err(), "This file wasn't dropped on the island");
        assert_eq!(attachments.load("f999").unwrap_err(), "Attachment not found: drop the file again");
        let binary = temp("binary-en.bin", &[0, 1, 2, 3]);
        assert_eq!(
            read_for_chat(&binary).unwrap_err(),
            "Mochi can only read text and images (PNG, JPEG, GIF, WebP): binary-en.bin is neither."
        );
        forget(binary);
    }
}
