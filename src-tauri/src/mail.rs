//! E-mail from the island (prompt 12, lot 3). The draft is written in the island; on
//! « Envoyer », and only then, it is handed to the mail app, which opens it ready to go:
//! the user sees exactly what leaves and sends it from there. îlot never sends mail by
//! itself and holds no mail account.
//!
//! In this order:
//! 1. Simple MAPI (`MAPISendMailW`, mapi32.dll, Windows 8 and later) with its compose
//!    window (`MAPI_DIALOG_MODELESS`, which includes `MAPI_DIALOG`): the mail app
//!    registered for MAPI (Outlook classic, Thunderbird…) opens the message with its
//!    attachment. Never without the dialog, which could send at once.
//!    https://learn.microsoft.com/en-us/windows/win32/api/mapi/nc-mapi-mapisendmailw
//! 2. Outlook classic by its command line, when MAPI fails and Outlook is installed:
//!    `outlook.exe /c ipm.note /m <to>?subject=…&body=… /a <file>`, arguments passed apart,
//!    no shell. https://support.microsoft.com/en-us/office/command-line-switches-for-microsoft-office-products-079164cd-4ef5-4178-b235-441737deb3a6
//! 3. A `mailto:` link for the default mail app (new Outlook, a webmail). It cannot carry
//!    a file (RFC 6068): the file's folder opens with the file selected, to drag it in.
//!    https://www.rfc-editor.org/rfc/rfc6068.html
//!
//! New Outlook does not support MAPI (support.microsoft.com, « new Outlook users
//! unexpectedly see the Welcome to Outlook 2016 setup wizard »): with it, step 3 runs.

use crate::attachments::{Attachments, file_name};
use crate::locale::{Text, tr};
use serde::{Deserialize, Serialize};
use std::ffi::{OsString, c_void};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{State, WebviewWindow};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW};
use windows::Win32::System::Mapi::{
    LPMAPISENDMAILW, MAPI_DIALOG, MAPI_E_USER_ABORT, MAPI_LOGON_UI, MAPI_TO, MapiFileDescW, MapiMessageW, MapiRecipDescW,
    SUCCESS_SUCCESS,
};
use windows::Win32::System::Registry::{HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
use windows::Win32::UI::Shell::{ILCreateFromPathW, ILFree, SHOpenFolderAndSelectItems, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{PCWSTR, PWSTR, s, w};

/// `MAPI_DIALOG_MODELESS`: 0x4 together with `MAPI_DIALOG`; a client that does not know
/// 0x4 still sees `MAPI_DIALOG` and shows its (modal) compose window.
const MAPI_DIALOG_MODELESS: u32 = 0x4 | MAPI_DIALOG;
/// A modal compose window keeps the call open: past this wait, it is on screen.
const MAPI_WAIT: Duration = Duration::from_secs(4);
const MAX_RECIPIENTS: usize = 10;
const MAX_ADDRESS: usize = 254;
const MAX_SUBJECT: usize = 255;
const MAX_BODY: usize = 100_000;
/// Longest `mailto:` link handed to Windows; the text is cut past it (and copied whole).
const MAX_LINK: usize = 2000;

/// The draft written in the island.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub to: String,
    pub subject: String,
    pub body: String,
    /// An attachment made from a file dropped on the island.
    #[serde(default)]
    pub attachment: Option<String>,
}

/// The draft, checked.
#[derive(Debug, PartialEq, Eq)]
pub struct Mail {
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,
}

/// How the draft left the island.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Handed {
    /// `mapi`, `outlook` or `mailto`.
    pub way: &'static str,
    pub client: String,
    /// What the user still has to do, if anything.
    pub note: Option<String>,
}

/// The mail app the island names before « Envoyer ».
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailApp {
    pub name: String,
    pub mapi: bool,
}

/// `marie@exemple.fr`: one `@`, a dot after it, no space, quote, angle bracket or control
/// character, and nothing that could read as a command-line switch.
pub fn valid_address(address: &str) -> bool {
    let Some((local, domain)) = address.split_once('@') else { return false };
    address.len() <= MAX_ADDRESS
        && !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !address.starts_with(['-', '/'])
        && !domain.contains('@')
        && address.chars().all(|c| (c.is_ascii_graphic() && !"\"'<>()[]\\,;:%?&=#`|^{} ".contains(c)) || (!c.is_ascii() && !c.is_control() && !c.is_whitespace()))
}

/// Checks a draft: addresses separated by commas or semicolons (none: the mail app asks),
/// a one-line subject, a bounded body with Windows line ends.
pub fn check(draft: &Draft) -> Result<Mail, String> {
    let to: Vec<String> = draft.to.split([',', ';']).map(str::trim).filter(|a| !a.is_empty()).map(str::to_owned).collect();
    if let Some(bad) = to.iter().find(|a| !valid_address(a)) {
        return Err(tr(Text::MailBadAddress { address: &bad.chars().take(60).collect::<String>() }));
    }
    if to.len() > MAX_RECIPIENTS {
        return Err(tr(Text::MailTooManyRecipients { max: MAX_RECIPIENTS }));
    }
    let subject: String = draft.subject.chars().map(|c| if c.is_control() { ' ' } else { c }).collect::<String>().trim().to_owned();
    if subject.chars().count() > MAX_SUBJECT {
        return Err(tr(Text::MailSubjectTooLong { max: MAX_SUBJECT }));
    }
    if draft.body.chars().count() > MAX_BODY {
        return Err(Text::MailTooLong.into());
    }
    let body = draft.body.replace("\r\n", "\n").replace('\r', "\n").replace('\n', "\r\n");
    if body.contains('\0') {
        return Err(Text::MailInvalid.into());
    }
    Ok(Mail { to, subject, body })
}

/// Percent-encodes everything but the unreserved characters (RFC 3986), as `mailto:`
/// wants for `subject` and `body` (RFC 6068, section 2).
fn encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.as_bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(byte) {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// `to?subject=…&body=…`, shared by `mailto:` and Outlook's `/m`. The body is cut so that
/// the whole stays under `limit` characters; the second value says whether it was.
fn address_and_fields(mail: &Mail, limit: usize) -> (String, bool) {
    let to = mail.to.iter().map(|a| encode(a).replace("%40", "@")).collect::<Vec<_>>().join(",");
    let mut fields = Vec::new();
    if !mail.subject.is_empty() {
        fields.push(format!("subject={}", encode(&mail.subject)));
    }
    let head = if fields.is_empty() { to.clone() } else { format!("{to}?{}", fields.join("&")) };
    if mail.body.is_empty() {
        return (head, false);
    }
    let joiner = if fields.is_empty() { "?" } else { "&" };
    let mut body = String::new();
    let mut cut = false;
    for c in mail.body.chars() {
        let piece = encode(&c.to_string());
        if head.len() + joiner.len() + 5 + body.len() + piece.len() > limit {
            cut = true;
            break;
        }
        body.push_str(&piece);
    }
    (format!("{head}{joiner}body={body}"), cut)
}

/// The `mailto:` link of a draft, and whether its text had to be cut.
pub fn mailto(mail: &Mail) -> (String, bool) {
    let (rest, cut) = address_and_fields(mail, MAX_LINK - "mailto:".len());
    (format!("mailto:{rest}"), cut)
}

/// Outlook classic's arguments, each one apart (never a command line to parse).
pub fn outlook_args(mail: &Mail, file: Option<&Path>) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec!["/c".into(), "ipm.note".into()];
    if !mail.to.is_empty() {
        args.push("/m".into());
        args.push(address_and_fields(mail, 8000).0.into());
    }
    if let Some(file) = file {
        args.push("/a".into());
        args.push(file.as_os_str().to_owned());
    }
    args
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

/// The strings of a Simple MAPI message, in UTF-16, kept alive for the call.
#[derive(Debug, PartialEq, Eq)]
pub struct MapiParts {
    pub subject: Vec<u16>,
    pub body: Vec<u16>,
    /// Display name and `SMTP:` address of each recipient.
    pub recipients: Vec<(Vec<u16>, Vec<u16>)>,
    /// Full path and file name of the attachment.
    pub file: Option<(Vec<u16>, Vec<u16>)>,
}

/// A recipient given by name and address is not resolved: the address is used as is,
/// written `SMTP:address` (MAPISendMailW, « Handling recipient information »).
pub fn mapi_parts(mail: &Mail, file: Option<&Path>) -> MapiParts {
    MapiParts {
        subject: wide(&mail.subject),
        body: wide(&mail.body),
        recipients: mail.to.iter().map(|a| (wide(a), wide(&format!("SMTP:{a}")))).collect(),
        file: file.map(|path| (wide(&path.to_string_lossy()), wide(&file_name(path)))),
    }
}

enum MapiOutcome {
    /// The compose window opened (or the call is still waiting on it).
    Opened,
    /// The user closed it without sending.
    Cancelled,
    /// No MAPI, or it failed: the next way is tried.
    Failed,
}

/// Hands the message to the MAPI mail app, on a thread of its own (a modal compose
/// window keeps the call open until it closes).
fn send_mapi(mut parts: MapiParts) -> MapiOutcome {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        // SAFETY: mapi32.dll is loaded from System32 only; the function pointer is the
        // documented MAPISendMailW; every pointer of the message points into `parts`, alive
        // until the call returns.
        let code = unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let Ok(module) = LoadLibraryExW(w!("mapi32.dll"), None, LOAD_LIBRARY_SEARCH_SYSTEM32) else {
                let _ = sender.send(None);
                return;
            };
            let Some(function) = GetProcAddress(module, s!("MAPISendMailW")) else {
                let _ = sender.send(None);
                return;
            };
            let send: LPMAPISENDMAILW = std::mem::transmute(function);
            let Some(send) = send else {
                let _ = sender.send(None);
                return;
            };
            let mut recipients: Vec<MapiRecipDescW> = parts
                .recipients
                .iter_mut()
                .map(|(name, address)| MapiRecipDescW {
                    ulRecipClass: MAPI_TO,
                    lpszName: PWSTR(name.as_mut_ptr()),
                    lpszAddress: PWSTR(address.as_mut_ptr()),
                    ..Default::default()
                })
                .collect();
            let mut files: Vec<MapiFileDescW> = parts
                .file
                .iter_mut()
                .map(|(path, name)| MapiFileDescW {
                    nPosition: u32::MAX,
                    lpszPathName: PWSTR(path.as_mut_ptr()),
                    lpszFileName: PWSTR(name.as_mut_ptr()),
                    ..Default::default()
                })
                .collect();
            let message = MapiMessageW {
                lpszSubject: PWSTR(parts.subject.as_mut_ptr()),
                lpszNoteText: PWSTR(parts.body.as_mut_ptr()),
                nRecipCount: recipients.len() as u32,
                lpRecips: if recipients.is_empty() { std::ptr::null_mut() } else { recipients.as_mut_ptr() },
                nFileCount: files.len() as u32,
                lpFiles: if files.is_empty() { std::ptr::null_mut() } else { files.as_mut_ptr() },
                ..Default::default()
            };
            send(0, 0, &message, MAPI_LOGON_UI | MAPI_DIALOG_MODELESS, 0)
        };
        let _ = sender.send(Some(code));
    });
    match receiver.recv_timeout(MAPI_WAIT) {
        Err(_) => MapiOutcome::Opened,
        Ok(Some(SUCCESS_SUCCESS)) => MapiOutcome::Opened,
        Ok(Some(MAPI_E_USER_ABORT)) => MapiOutcome::Cancelled,
        Ok(_) => MapiOutcome::Failed,
    }
}

/// The default value of a registry key, as text.
fn registry_text(root: HKEY, key: PCWSTR) -> Option<String> {
    let mut buffer = [0u16; 1024];
    let mut size = (buffer.len() * 2) as u32;
    // SAFETY: the buffer and its size in bytes go together; RegGetValueW writes within it.
    let read = unsafe {
        RegGetValueW(root, key, PCWSTR::null(), RRF_RT_REG_SZ, None, Some(buffer.as_mut_ptr().cast::<c_void>()), Some(&mut size))
    };
    if read.is_err() {
        return None;
    }
    let length = (size as usize / 2).saturating_sub(1);
    let text = String::from_utf16_lossy(&buffer[..length.min(buffer.len())]).trim().to_owned();
    (!text.is_empty()).then_some(text)
}

/// The mail app registered for Simple MAPI (`Software\Clients\Mail`, the user's choice
/// first). https://learn.microsoft.com/en-us/previous-versions/windows/desktop/windowsmapi/mapi32-dll-stub-registry-settings
fn mapi_client() -> Option<String> {
    let key = w!("Software\\Clients\\Mail");
    registry_text(HKEY_CURRENT_USER, key).or_else(|| registry_text(HKEY_LOCAL_MACHINE, key))
}

/// Where Outlook classic is installed (`App Paths\OUTLOOK.EXE`).
fn outlook() -> Option<PathBuf> {
    let key = w!("Software\\Microsoft\\Windows\\CurrentVersion\\App Paths\\OUTLOOK.EXE");
    let path = registry_text(HKEY_LOCAL_MACHINE, key).or_else(|| registry_text(HKEY_CURRENT_USER, key))?;
    let path = PathBuf::from(path.trim_matches('"'));
    path.is_file().then_some(path)
}

/// Opens the folder of a file with the file selected, ready to be dragged into a mail.
fn show_in_folder(path: &Path) -> bool {
    let wide = wide(&path.to_string_lossy());
    // SAFETY: a null-terminated path; the item list is freed once used.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let item = ILCreateFromPathW(PCWSTR(wide.as_ptr()));
        if item.is_null() {
            return false;
        }
        let shown = SHOpenFolderAndSelectItems(item, None, 0).is_ok();
        ILFree(Some(item));
        shown
    }
}

fn open_link(link: &str) -> bool {
    let wide = wide(link);
    // SAFETY: a null-terminated `mailto:` link; ShellExecuteW opens it with the app the
    // user chose for that scheme. Above 32 means it worked.
    let result = unsafe { ShellExecuteW(None, w!("open"), PCWSTR(wide.as_ptr()), None, None, SW_SHOWNORMAL) };
    result.0 as isize > 32
}

/// The name the island shows before « Envoyer » (« Microsoft Outlook va s'ouvrir »).
#[tauri::command]
pub fn mail_app() -> MailApp {
    match mapi_client() {
        Some(name) => MailApp { name, mapi: true },
        None => MailApp { name: tr(Text::MailYourApp), mapi: false },
    }
}

/// « Envoyer »: hands the draft to the mail app, which opens it ready to send.
#[tauri::command]
pub async fn mail_send(draft: Draft, window: WebviewWindow, attachments: State<'_, Attachments>) -> Result<Handed, String> {
    let mail = check(&draft)?;
    let file = match &draft.attachment {
        Some(id) => Some(attachments.path_of(id).ok_or(crate::attachments::UNKNOWN)?),
        None => None,
    };
    // A window handle is not `Send`: kept as a number across the wait.
    let owner = window.hwnd().ok().map(|hwnd| hwnd.0 as isize);
    let handed = tauri::async_runtime::spawn_blocking(move || hand_over(&mail, file.as_deref()))
        .await
        .map_err(|_| tr(Text::MailCannotOpen))??;
    if let (Some(owner), Some(copy)) = (owner, &handed.1) {
        let _ = crate::clipboard::write(HWND(owner as *mut c_void), copy);
    }
    Ok(handed.0)
}

/// The three ways, in order. The second value is a text to copy for the user.
fn hand_over(mail: &Mail, file: Option<&Path>) -> Result<(Handed, Option<String>), String> {
    let both = || format!("{}\r\n\r\n{}", mail.subject, mail.body);
    if let Some(client) = mapi_client() {
        match send_mapi(mapi_parts(mail, file)) {
            MapiOutcome::Opened => return Ok((Handed { way: "mapi", client, note: None }, None)),
            MapiOutcome::Cancelled => return Err(tr(Text::MailClosedUnsent { client: &client })),
            MapiOutcome::Failed => {}
        }
    }
    if let Some(outlook) = outlook() {
        let started = std::process::Command::new(&outlook).args(outlook_args(mail, file)).spawn().is_ok();
        if started {
            let note = mail.to.is_empty().then(|| tr(Text::MailCopiedForOutlook));
            let copy = mail.to.is_empty().then(both);
            return Ok((Handed { way: "outlook", client: "Outlook".into(), note }, copy));
        }
    }
    let (link, cut) = mailto(mail);
    if !open_link(&link) {
        return Err(Text::MailNoApp.into());
    }
    let mut notes = Vec::new();
    if let Some(file) = file
        && show_in_folder(file)
    {
        notes.push(tr(Text::MailDragFile { name: &file_name(file) }));
    }
    if cut {
        notes.push(tr(Text::MailTextCopied));
    }
    let note = (!notes.is_empty()).then(|| notes.join(" "));
    Ok((Handed { way: "mailto", client: tr(Text::MailYourMail), note }, cut.then(|| mail.body.clone())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(to: &str, subject: &str, body: &str) -> Draft {
        Draft { to: to.into(), subject: subject.into(), body: body.into(), attachment: None }
    }

    #[test]
    fn addresses_are_plain_and_never_switches() {
        for good in ["marie@exemple.fr", "l.ahouansou+ilot@mail.co.uk", "élodie@société.fr"] {
            assert!(valid_address(good), "{good}");
        }
        for bad in ["", "marie", "@exemple.fr", "marie@exemple", "marie@.fr", "a b@c.fr", "\"x\"@c.fr", "/a@c.fr", "-x@c.fr", "x@c.fr?bcc=y@z.fr", "a@b@c.fr", "x@c.fr&body=1"] {
            assert!(!valid_address(bad), "{bad}");
        }
    }

    #[test]
    fn a_draft_is_checked_before_anything_opens() {
        let mail = check(&draft(" marie@exemple.fr ; paul@exemple.fr,", "Compte\nrendu", "Bonjour\nVoici")).unwrap();
        assert_eq!(mail.to, ["marie@exemple.fr", "paul@exemple.fr"]);
        assert_eq!(mail.subject, "Compte rendu");
        assert_eq!(mail.body, "Bonjour\r\nVoici");
        assert!(check(&draft("", "", "")).unwrap().to.is_empty(), "the mail app asks for the address");
        assert!(check(&draft("marie@exemple.fr, oops", "", "")).unwrap_err().starts_with("Adresse invalide : oops"));
        let many = (0..11).map(|i| format!("p{i}@x.fr")).collect::<Vec<_>>().join(",");
        assert!(check(&draft(&many, "", "")).is_err());
        assert!(check(&draft("", &"o".repeat(256), "")).is_err());
    }

    /// RFC 6068, section 6.1: `mailto:addr?subject=…&body=…`, percent-encoded UTF-8.
    #[test]
    fn the_mailto_link_carries_address_subject_and_body() {
        let mail = check(&draft("marie@exemple.fr", "Rapport & chiffres", "Ligne 1\nÉté")).unwrap();
        let (link, cut) = mailto(&mail);
        assert_eq!(link, "mailto:marie@exemple.fr?subject=Rapport%20%26%20chiffres&body=Ligne%201%0D%0A%C3%89t%C3%A9");
        assert!(!cut);
        let empty = check(&draft("", "", "")).unwrap();
        assert_eq!(mailto(&empty).0, "mailto:");
        let long = check(&draft("marie@exemple.fr", "x", &"é".repeat(5000))).unwrap();
        let (link, cut) = mailto(&long);
        assert!(cut);
        assert!(link.len() <= MAX_LINK);
        assert!(!link.ends_with('%'));
    }

    #[test]
    fn outlook_gets_each_argument_apart() {
        let mail = check(&draft("marie@exemple.fr", "Bonjour", "Texte")).unwrap();
        let file = Path::new(r"C:\Users\moi\Documents\rapport final.pdf");
        let args = outlook_args(&mail, Some(file));
        let args: Vec<&str> = args.iter().map(|a| a.to_str().unwrap()).collect();
        assert_eq!(args, ["/c", "ipm.note", "/m", "marie@exemple.fr?subject=Bonjour&body=Texte", "/a", r"C:\Users\moi\Documents\rapport final.pdf"]);
        let nobody = check(&draft("", "Bonjour", "")).unwrap();
        assert_eq!(outlook_args(&nobody, None), ["/c", "ipm.note"]);
    }

    #[test]
    fn simple_mapi_gets_wide_strings_and_smtp_addresses() {
        let mail = check(&draft("marie@exemple.fr", "Objet", "Texte")).unwrap();
        let parts = mapi_parts(&mail, Some(Path::new(r"C:\docs\note.md")));
        assert_eq!(parts.subject, wide("Objet"));
        assert_eq!(parts.recipients, [(wide("marie@exemple.fr"), wide("SMTP:marie@exemple.fr"))]);
        assert_eq!(parts.file, Some((wide(r"C:\docs\note.md"), wide("note.md"))));
        assert_eq!(*parts.body.last().unwrap(), 0);
    }

    /// Nothing is sent by building, checking or describing a draft: the only way out is
    /// `hand_over`, called by `mail_send`, the command of the « Envoyer » button.
    #[test]
    fn no_mail_without_the_send_button() {
        let source = include_str!("mail.rs");
        let calls = source.matches(concat!("hand_", "over(")).count();
        assert_eq!(calls, 2, "its definition and the call in mail_send only");
        let sends = source.matches(concat!("send_", "mapi(")).count();
        assert_eq!(sends, 2, "its definition and the call in hand_over only");
    }

    #[test]
    fn a_draft_is_refused_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        assert_eq!(check(&draft("marie@exemple.fr, oops", "", "")).unwrap_err(), "Invalid address: oops");
        let many = (0..11).map(|i| format!("p{i}@x.fr")).collect::<Vec<_>>().join(",");
        assert_eq!(check(&draft(&many, "", "")).unwrap_err(), "10 recipients at most");
        assert_eq!(check(&draft("", &"o".repeat(256), "")).unwrap_err(), "Subject too long: 255 characters at most");
    }
}
