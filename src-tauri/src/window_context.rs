//! Mochi dropped on a window (prompt 12, lot 3): îlot offers that window as context for
//! the chat. In three steps, each one a click of the user:
//! 1. the drop: only the window's title and program are looked at, to check the block
//!    list (password managers, banks, sign-in pages, system prompts). A blocked window is
//!    never read; the island says so.
//! 2. « Lire la fenêtre »: its text is read with UI Automation, the accessibility
//!    interface of Windows, and shown in the island as a preview. Nothing leaves the PC.
//! 3. « Joindre au chat »: the preview joins the next question the user sends.
//!
//! The read itself: on a thread without windows, in the multithreaded apartment, one
//! lookup at a time (the WhatsApp tabs share the turn, `notifications/uia.rs`), with short
//! UI Automation timeouts, a deadline, an element budget and a text ceiling. A window
//! holding any password field is not read at all, and no element marked `IsPassword` is
//! ever touched. No screenshot. A program run as administrator cannot be read by îlot
//! (UIPI): the island says that too.
//! https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-threading
//! https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-cachingforclients
//! https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-automation-element-propids
//! https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-supporteditcontroltype
//! https://learn.microsoft.com/en-us/windows/win32/api/uiautomationclient/nf-uiautomationclient-iuiautomationtextrange-gettext
//! https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-securityoverview

use crate::locale::{Text, tr};
use crate::window::WINDOW_LABEL;
use serde::Serialize;
use std::collections::VecDeque;
use std::ffi::c_void;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
use windows::Win32::Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::{
    CUIAutomation8, IUIAutomation, IUIAutomation2, IUIAutomationElement, IUIAutomationTextPattern, TreeScope_Children,
    TreeScope_Subtree, UIA_CONTROLTYPE_ID, UIA_ControlTypePropertyId, UIA_DataItemControlTypeId, UIA_DocumentControlTypeId,
    UIA_EditControlTypeId, UIA_HeaderItemControlTypeId, UIA_HyperlinkControlTypeId, UIA_IsPasswordPropertyId,
    UIA_ListItemControlTypeId, UIA_NamePropertyId, UIA_TextControlTypeId, UIA_TextPatternId, UIA_TreeItemControlTypeId,
};
use windows::Win32::UI::WindowsAndMessaging::{GetWindowTextW, GetWindowThreadProcessId};
use windows::core::Interface;

pub const EVENT_PROPOSED: &str = "context:proposed";
/// Text kept from a window, at most.
pub const MAX_CHARS: usize = 8000;
const MAX_ELEMENTS: usize = 3000;
const READ_TIME: Duration = Duration::from_millis(2500);
const UIA_TIMEOUT_MS: u32 = 1000;
/// Windows offered at once (older proposals are forgotten).
const MAX_PROPOSALS: usize = 4;

pub const UNKNOWN: Text<'static> = Text::ContextUnknown;
pub const NOT_CONFIRMED: Text<'static> = Text::ContextNotConfirmed;
pub const NOTHING: Text<'static> = Text::ContextNothing;
pub const GONE: Text<'static> = Text::ContextGone;

/// Why a window is not read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block {
    PasswordManager,
    Bank,
    SignIn,
    System,
    PasswordField,
    Admin,
}

impl Block {
    pub fn sentence(self) -> String {
        tr(match self {
            Self::PasswordManager => Text::BlockPasswordManager,
            Self::Bank => Text::BlockBank,
            Self::SignIn => Text::BlockSignIn,
            Self::System => Text::BlockSystem,
            Self::PasswordField => Text::BlockPasswordField,
            Self::Admin => Text::BlockAdmin,
        })
    }
}

/// Programs never read, by file name.
const BLOCKED_PROGRAMS: [(&str, Block); 23] = [
    ("keepass.exe", Block::PasswordManager),
    ("keepassxc.exe", Block::PasswordManager),
    ("1password.exe", Block::PasswordManager),
    ("bitwarden.exe", Block::PasswordManager),
    ("dashlane.exe", Block::PasswordManager),
    ("nordpass.exe", Block::PasswordManager),
    ("enpass.exe", Block::PasswordManager),
    ("roboform.exe", Block::PasswordManager),
    ("keeperpasswordmanager.exe", Block::PasswordManager),
    ("lastpass.exe", Block::PasswordManager),
    ("proton pass.exe", Block::PasswordManager),
    ("protonpass.exe", Block::PasswordManager),
    ("pwsafe.exe", Block::PasswordManager),
    ("authy desktop.exe", Block::PasswordManager),
    ("credentialuibroker.exe", Block::System),
    ("consent.exe", Block::System),
    ("logonui.exe", Block::System),
    ("lockapp.exe", Block::System),
    ("sechealthui.exe", Block::System),
    ("credwiz.exe", Block::System),
    ("certmgr.exe", Block::System),
    ("ledger live.exe", Block::Bank),
    ("exodus.exe", Block::Bank),
];

/// Words and phrases of a title that keep a window unread (accents and case removed,
/// matched as whole words).
const BLOCKED_WORDS: [(&str, Block); 79] = [
    ("1password", Block::PasswordManager),
    ("bitwarden", Block::PasswordManager),
    ("keepass", Block::PasswordManager),
    ("keepassxc", Block::PasswordManager),
    ("lastpass", Block::PasswordManager),
    ("dashlane", Block::PasswordManager),
    ("nordpass", Block::PasswordManager),
    ("enpass", Block::PasswordManager),
    ("roboform", Block::PasswordManager),
    ("keeper", Block::PasswordManager),
    ("proton pass", Block::PasswordManager),
    ("gestionnaire d identification", Block::PasswordManager),
    ("credential manager", Block::PasswordManager),
    ("mot de passe", Block::SignIn),
    ("mots de passe", Block::SignIn),
    ("password", Block::SignIn),
    ("passwords", Block::SignIn),
    ("passwort", Block::SignIn),
    ("contrasena", Block::SignIn),
    ("connexion", Block::SignIn),
    ("se connecter", Block::SignIn),
    ("identifiez vous", Block::SignIn),
    ("authentification", Block::SignIn),
    ("login", Block::SignIn),
    ("log in", Block::SignIn),
    ("sign in", Block::SignIn),
    ("signin", Block::SignIn),
    ("sign up", Block::SignIn),
    ("inscription", Block::SignIn),
    ("2fa", Block::SignIn),
    ("mfa", Block::SignIn),
    ("otp", Block::SignIn),
    ("authenticator", Block::SignIn),
    ("code de verification", Block::SignIn),
    ("verification code", Block::SignIn),
    ("franceconnect", Block::SignIn),
    ("france connect", Block::SignIn),
    ("banque", Block::Bank),
    ("bank", Block::Bank),
    ("banking", Block::Bank),
    ("credit agricole", Block::Bank),
    ("bnp", Block::Bank),
    ("societe generale", Block::Bank),
    ("lcl", Block::Bank),
    ("caisse d epargne", Block::Bank),
    ("credit mutuel", Block::Bank),
    ("cic", Block::Bank),
    ("boursorama", Block::Bank),
    ("boursobank", Block::Bank),
    ("fortuneo", Block::Bank),
    ("monabanq", Block::Bank),
    ("ing", Block::Bank),
    ("hsbc", Block::Bank),
    ("revolut", Block::Bank),
    ("n26", Block::Bank),
    ("qonto", Block::Bank),
    ("nickel", Block::Bank),
    ("lydia", Block::Bank),
    ("sumeria", Block::Bank),
    ("paypal", Block::Bank),
    ("wise", Block::Bank),
    ("binance", Block::Bank),
    ("coinbase", Block::Bank),
    ("kraken", Block::Bank),
    ("metamask", Block::Bank),
    ("trade republic", Block::Bank),
    ("degiro", Block::Bank),
    ("carte bancaire", Block::Bank),
    ("credit card", Block::Bank),
    ("paiement", Block::Bank),
    ("payment", Block::Bank),
    ("checkout", Block::Bank),
    ("iban", Block::Bank),
    ("rib", Block::Bank),
    ("virement", Block::Bank),
    ("impots", Block::Bank),
    ("ameli", Block::Bank),
    ("urssaf", Block::Bank),
    ("coffre fort", Block::PasswordManager),
];

fn plain(c: char) -> char {
    match c {
        'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => 'a',
        'ç' => 'c',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' | 'ì' => 'i',
        'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ÿ' => 'y',
        'ñ' => 'n',
        c => c,
    }
}

/// Lowercase words without accents, one space around each: « Se connecter – BNP »
/// becomes « se connecter bnp » with spaces at both ends.
pub fn words(text: &str) -> String {
    let mut out = String::from(" ");
    for c in text.chars().flat_map(char::to_lowercase).map(plain) {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.ends_with(' ') {
            out.push(' ');
        }
    }
    if !out.ends_with(' ') {
        out.push(' ');
    }
    out
}

/// Why a window must not be read, from its program (full path) and its title alone.
pub fn blocked(program: &str, title: &str) -> Option<Block> {
    let file = program.rsplit(['\\', '/']).next().unwrap_or(program).to_lowercase();
    if let Some((_, block)) = BLOCKED_PROGRAMS.iter().find(|(name, _)| *name == file) {
        return Some(*block);
    }
    let title = words(title);
    BLOCKED_WORDS.iter().find(|(phrase, _)| title.contains(&format!(" {phrase} "))).map(|(_, block)| *block)
}

/// The program's name as shown: its file name without `.exe`.
pub fn app_name(program: &str) -> String {
    let file = program.rsplit(['\\', '/']).next().unwrap_or(program);
    file.strip_suffix(".exe").or_else(|| file.strip_suffix(".EXE")).unwrap_or(file).to_owned()
}

/// What the island hears when Mochi lands on a window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub id: String,
    /// Empty for a blocked window: not even its title goes to the island.
    pub title: String,
    pub app: String,
    pub blocked: Option<String>,
}

/// What « Lire la fenêtre » shows before anything is joined.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub id: String,
    pub title: String,
    pub app: String,
    pub text: String,
    pub truncated: bool,
}

struct Entry {
    id: String,
    hwnd: isize,
    pid: u32,
    title: String,
    app: String,
    blocked: Option<Block>,
    text: Option<String>,
    confirmed: bool,
}

#[derive(Default)]
struct Inner {
    serial: u64,
    list: VecDeque<Entry>,
}

/// The windows offered as context, and what the user did with each.
#[derive(Default)]
pub struct Contexts {
    inner: Mutex<Inner>,
}

/// What `read_window` needs to know of a proposal.
struct Target {
    hwnd: isize,
    pid: u32,
    blocked: Option<Block>,
}

impl Contexts {
    pub fn propose(&self, hwnd: isize, pid: u32, title: &str, program: &str) -> Proposal {
        let block = blocked(program, title);
        let mut inner = self.inner.lock().unwrap();
        inner.serial += 1;
        let id = format!("w{}", inner.serial);
        let app = app_name(program);
        inner.list.push_back(Entry {
            id: id.clone(),
            hwnd,
            pid,
            title: title.to_owned(),
            app: app.clone(),
            blocked: block,
            text: None,
            confirmed: false,
        });
        while inner.list.len() > MAX_PROPOSALS {
            inner.list.pop_front();
        }
        Proposal { id, title: if block.is_some() { String::new() } else { title.to_owned() }, app, blocked: block.map(Block::sentence) }
    }

    fn target(&self, id: &str) -> Result<Target, String> {
        let inner = self.inner.lock().unwrap();
        let entry = inner.list.iter().find(|e| e.id == id).ok_or(UNKNOWN)?;
        Ok(Target { hwnd: entry.hwnd, pid: entry.pid, blocked: entry.blocked })
    }

    /// Keeps what was read (or the reason it was not) and returns the preview.
    fn store(&self, id: &str, read: Result<(String, bool), Block>) -> Result<Preview, String> {
        let mut inner = self.inner.lock().unwrap();
        let entry = inner.list.iter_mut().find(|e| e.id == id).ok_or(UNKNOWN)?;
        match read {
            Ok((text, truncated)) => {
                entry.text = Some(text.clone());
                entry.confirmed = false;
                Ok(Preview { id: id.to_owned(), title: entry.title.clone(), app: entry.app.clone(), text, truncated })
            }
            Err(block) => {
                entry.blocked = Some(block);
                entry.text = None;
                Err(block.sentence())
            }
        }
    }

    /// « Joindre au chat »: only a text the user has seen in the preview.
    pub fn confirm(&self, id: &str) -> Result<(), String> {
        let mut inner = self.inner.lock().unwrap();
        let entry = inner.list.iter_mut().find(|e| e.id == id).ok_or(UNKNOWN)?;
        if entry.blocked.is_some() || entry.text.is_none() {
            return Err(NOT_CONFIRMED.into());
        }
        entry.confirmed = true;
        Ok(())
    }

    pub fn forget(&self, id: &str) {
        self.inner.lock().unwrap().list.retain(|e| e.id != id);
    }

    /// Title, program and text of a context the user joined; anything else is refused.
    pub fn confirmed(&self, id: &str) -> Result<(String, String, String), String> {
        let inner = self.inner.lock().unwrap();
        let entry = inner.list.iter().find(|e| e.id == id).ok_or(UNKNOWN)?;
        match (&entry.text, entry.confirmed, entry.blocked) {
            (Some(text), true, None) => Ok((entry.title.clone(), entry.app.clone(), text.clone())),
            _ => Err(NOT_CONFIRMED.into()),
        }
    }
}

/// True when the program runs elevated (or cannot even be asked: a protected process).
fn elevated(process: HANDLE) -> bool {
    // SAFETY: plain queries on a process handle the caller owns; the token is closed here.
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token).is_err() {
            return true;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = 0u32;
        let read = GetTokenInformation(
            token,
            TokenElevation,
            Some((&raw mut elevation).cast::<c_void>()),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        );
        let _ = CloseHandle(token);
        read.is_err() || elevation.TokenIsElevated != 0
    }
}

/// UIPI keeps a program of a lower level from reading an elevated one.
fn out_of_reach(pid: u32) -> bool {
    // SAFETY: the pseudo handle of this process needs no closing; the other one is closed.
    unsafe {
        if elevated(GetCurrentProcess()) {
            return false;
        }
        match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(process) => {
                let high = elevated(process);
                let _ = CloseHandle(process);
                high
            }
            Err(_) => true,
        }
    }
}

const NAMED: [UIA_CONTROLTYPE_ID; 6] = [
    UIA_TextControlTypeId,
    UIA_HyperlinkControlTypeId,
    UIA_ListItemControlTypeId,
    UIA_DataItemControlTypeId,
    UIA_TreeItemControlTypeId,
    UIA_HeaderItemControlTypeId,
];

/// Text gathered within the ceiling, without the same line twice in a row.
struct Gathered {
    text: String,
    chars: usize,
    full: bool,
}

impl Gathered {
    fn add(&mut self, piece: &str) {
        let piece = piece.trim();
        if piece.is_empty() || self.full || self.text.lines().last().is_some_and(|last| last == piece) {
            return;
        }
        let room = MAX_CHARS.saturating_sub(self.chars);
        let taken: String = piece.chars().take(room).collect();
        self.chars += taken.chars().count() + 1;
        if !self.text.is_empty() {
            self.text.push('\n');
        }
        self.text.push_str(&taken);
        if taken.chars().count() < piece.chars().count() || self.chars >= MAX_CHARS {
            self.full = true;
        }
    }
}

enum Unread {
    Blocked(Block),
    Failed(Text<'static>),
}

/// Reads the text of a window with UI Automation. Off the interface thread only.
fn read_window(hwnd: isize, deadline: Instant) -> Result<(String, bool), Unread> {
    let _turn = crate::notifications::uia_turn();
    let failed = |_| Unread::Failed(GONE);
    // SAFETY: COM is joined in the multithreaded apartment (a no-op when this thread
    // already is); then plain calls on objects UI Automation returns. A password field is
    // looked for by its property only: its value is never asked for.
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let client: IUIAutomation = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).map_err(failed)?;
        if let Ok(timeouts) = client.cast::<IUIAutomation2>() {
            let _ = timeouts.SetConnectionTimeout(UIA_TIMEOUT_MS);
            let _ = timeouts.SetTransactionTimeout(UIA_TIMEOUT_MS);
        }
        let root = client.ElementFromHandle(HWND(hwnd as *mut c_void)).map_err(failed)?;
        let password = client.CreatePropertyCondition(UIA_IsPasswordPropertyId, &VARIANT::from(true)).map_err(failed)?;
        match root.FindFirst(TreeScope_Subtree, &password) {
            Ok(_) => return Err(Unread::Blocked(Block::PasswordField)),
            // A null element: no password field anywhere in the window.
            Err(e) if e.code().is_ok() => {}
            // Unsure: not read.
            Err(_) => return Err(Unread::Failed(GONE)),
        }
        let every = client.ControlViewCondition().map_err(failed)?;
        let cache = client.CreateCacheRequest().map_err(failed)?;
        for property in [UIA_ControlTypePropertyId, UIA_NamePropertyId, UIA_IsPasswordPropertyId] {
            cache.AddProperty(property).map_err(failed)?;
        }
        let mut gathered = Gathered { text: String::new(), chars: 0, full: false };
        let mut queue: VecDeque<IUIAutomationElement> = VecDeque::from([root]);
        let mut seen = 0usize;
        while let Some(element) = queue.pop_front() {
            if gathered.full || seen >= MAX_ELEMENTS || Instant::now() >= deadline {
                break;
            }
            let Ok(children) = element.FindAllBuildCache(TreeScope_Children, &every, &cache) else { continue };
            for index in 0..children.Length().unwrap_or(0) {
                let Ok(child) = children.GetElement(index) else { continue };
                seen += 1;
                if child.CachedIsPassword().is_ok_and(|p| p.as_bool()) {
                    return Err(Unread::Blocked(Block::PasswordField));
                }
                let Ok(kind) = child.CachedControlType() else { continue };
                if kind == UIA_DocumentControlTypeId || kind == UIA_EditControlTypeId {
                    let range = child
                        .GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
                        .and_then(|pattern| pattern.DocumentRange());
                    if let Ok(range) = range {
                        let room = MAX_CHARS.saturating_sub(gathered.chars) as i32 + 1;
                        if let Ok(text) = range.GetText(room) {
                            gathered.add(&text.to_string());
                        }
                        continue;
                    }
                } else if NAMED.contains(&kind)
                    && let Ok(name) = child.CachedName()
                {
                    gathered.add(&name.to_string());
                }
                queue.push_back(child);
            }
        }
        if gathered.text.trim().is_empty() {
            return Err(Unread::Failed(NOTHING));
        }
        Ok((gathered.text, gathered.full))
    }
}

/// Mochi was let go at (x, y), in the buddy window's coordinates: the window under him, if
/// it is not îlot's, is offered to the island. Only its title and program are looked at.
#[tauri::command]
pub fn buddy_drop_window(x: f64, y: f64, app: AppHandle) -> bool {
    let Some(space) = app.state::<crate::buddy::Buddy>().space() else { return false };
    if !(x.is_finite() && y.is_finite()) {
        return false;
    }
    let (px, py) = (x * space, y * space);
    let windows = crate::buddy::visible_windows(Some(std::process::id()));
    let Some(&(hwnd, _)) = windows.iter().find(|(_, rect)| rect.contains(px, py)) else { return false };
    let handle = HWND(hwnd as *mut c_void);
    let mut title = [0u16; 512];
    let mut pid = 0u32;
    // SAFETY: read-only queries on a handle EnumWindows just gave.
    let length = unsafe {
        GetWindowThreadProcessId(handle, Some(&mut pid));
        GetWindowTextW(handle, &mut title).max(0) as usize
    };
    let title = String::from_utf16_lossy(&title[..length]);
    let program = crate::notifications::program_path(handle).unwrap_or_default();
    let proposal = app.state::<Contexts>().propose(hwnd, pid, &title, &program);
    let _ = app.emit_to(WINDOW_LABEL, EVENT_PROPOSED, proposal);
    true
}

/// « Lire la fenêtre »: reads it now and returns the preview (nothing is sent).
#[tauri::command]
pub async fn context_read(id: String, app: AppHandle) -> Result<Preview, String> {
    let target = app.state::<Contexts>().target(&id)?;
    if let Some(block) = target.blocked {
        return Err(block.sentence());
    }
    let read = tauri::async_runtime::spawn_blocking(move || {
        if out_of_reach(target.pid) {
            return Err(Unread::Blocked(Block::Admin));
        }
        read_window(target.hwnd, Instant::now() + READ_TIME)
    })
    .await
    .map_err(|_| GONE)?;
    match read {
        Ok(text) => app.state::<Contexts>().store(&id, Ok(text)),
        Err(Unread::Blocked(block)) => app.state::<Contexts>().store(&id, Err(block)),
        Err(Unread::Failed(message)) => Err(message.into()),
    }
}

/// « Joindre au chat »: the previewed text goes with the next question.
#[tauri::command]
pub fn context_confirm(id: String, contexts: State<'_, Contexts>) -> Result<(), String> {
    contexts.confirm(&id)
}

/// « Non merci », or the chip removed: the text is dropped.
#[tauri::command]
pub fn context_forget(id: String, contexts: State<'_, Contexts>) {
    contexts.forget(&id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_managers_banks_and_sign_in_pages_are_blocked() {
        let cases = [
            (r"C:\Program Files\KeePassXC\KeePassXC.exe", "Base.kdbx - KeePassXC", Block::PasswordManager),
            (r"C:\Users\moi\AppData\Local\1Password\app\8\1Password.exe", "1Password", Block::PasswordManager),
            (r"C:\Program Files\Mozilla Firefox\firefox.exe", "Bitwarden Web vault — Mozilla Firefox", Block::PasswordManager),
            (r"C:\Windows\System32\consent.exe", "Contrôle de compte d'utilisateur", Block::System),
            (r"C:\Program Files\Google\Chrome\Application\chrome.exe", "Mon espace client - Crédit Agricole - Google Chrome", Block::Bank),
            (r"C:\Program Files\Mozilla Firefox\firefox.exe", "BoursoBank — Mozilla Firefox", Block::Bank),
            (r"C:\Program Files\Mozilla Firefox\firefox.exe", "Société Générale : comptes", Block::Bank),
            (r"C:\Program Files\Mozilla Firefox\firefox.exe", "ING - Mes comptes", Block::Bank),
            (r"C:\x\msedge.exe", "PayPal : paiement", Block::Bank),
            (r"C:\x\msedge.exe", "Se connecter à votre compte Microsoft", Block::SignIn),
            (r"C:\x\msedge.exe", "Sign in - Google Accounts", Block::SignIn),
            (r"C:\x\chrome.exe", "Réinitialiser le mot de passe", Block::SignIn),
            (r"C:\x\chrome.exe", "Code de vérification — GitHub", Block::SignIn),
            (r"C:\Windows\explorer.exe", "Gestionnaire d'identification", Block::PasswordManager),
        ];
        for (program, title, block) in cases {
            assert_eq!(blocked(program, title), Some(block), "{title}");
        }
    }

    #[test]
    fn ordinary_windows_are_allowed_and_words_count_whole() {
        let cases = [
            (r"C:\Windows\notepad.exe", "Liste de courses.txt - Bloc-notes"),
            (r"C:\x\Code.exe", "pricing.ts — ilot — Visual Studio Code"),
            (r"C:\x\firefox.exe", "Loading the morning news — Mozilla Firefox"),
            (r"C:\x\firefox.exe", "Otherwise, a nice recipe"),
            (r"C:\x\WINWORD.EXE", "Compte rendu de réunion.docx - Word"),
            (r"C:\x\firefox.exe", "Bankable ideas: a podcast"),
        ];
        for (program, title) in cases {
            assert_eq!(blocked(program, title), None, "{title}");
        }
        assert_eq!(words("  Se connecter – BNP Paribas!"), " se connecter bnp paribas ");
        assert_eq!(app_name(r"C:\Program Files\Mozilla Firefox\firefox.exe"), "firefox");
    }

    #[test]
    fn a_blocked_window_is_never_read_nor_titled_in_the_island() {
        let contexts = Contexts::default();
        let proposal = contexts.propose(7, 1, "Mes comptes — BoursoBank", r"C:\x\firefox.exe");
        assert_eq!(proposal.title, "", "the title stays in the core");
        assert_eq!(proposal.blocked, Some(Block::Bank.sentence()));
        assert_eq!(contexts.target(&proposal.id).unwrap().blocked, Some(Block::Bank));
        assert_eq!(contexts.confirm(&proposal.id).unwrap_err(), NOT_CONFIRMED);
        assert_eq!(contexts.confirmed(&proposal.id).unwrap_err(), NOT_CONFIRMED);
    }

    #[test]
    fn a_window_s_text_goes_to_the_chat_only_once_previewed_and_joined() {
        let contexts = Contexts::default();
        let proposal = contexts.propose(9, 2, "Rapport.docx - Word", r"C:\x\WINWORD.EXE");
        assert_eq!(proposal.blocked, None);
        assert_eq!(contexts.confirmed(&proposal.id).unwrap_err(), NOT_CONFIRMED, "not read yet");
        assert_eq!(contexts.confirm(&proposal.id).unwrap_err(), NOT_CONFIRMED, "nothing to join yet");
        let preview = contexts.store(&proposal.id, Ok(("Chiffres du trimestre".into(), false))).unwrap();
        assert_eq!((preview.title.as_str(), preview.app.as_str()), ("Rapport.docx - Word", "WINWORD"));
        assert_eq!(contexts.confirmed(&proposal.id).unwrap_err(), NOT_CONFIRMED, "previewed, not joined");
        contexts.confirm(&proposal.id).unwrap();
        assert_eq!(contexts.confirmed(&proposal.id).unwrap().2, "Chiffres du trimestre");
        contexts.forget(&proposal.id);
        assert_eq!(contexts.confirmed(&proposal.id).unwrap_err(), UNKNOWN);
    }

    #[test]
    fn a_password_field_found_while_reading_blocks_the_window() {
        let contexts = Contexts::default();
        let proposal = contexts.propose(9, 2, "Portail", r"C:\x\app.exe");
        assert_eq!(contexts.store(&proposal.id, Err(Block::PasswordField)).unwrap_err(), Block::PasswordField.sentence());
        assert_eq!(contexts.confirm(&proposal.id).unwrap_err(), NOT_CONFIRMED);
    }

    #[test]
    fn only_the_latest_windows_stay_offered() {
        let contexts = Contexts::default();
        let ids: Vec<String> = (0..6).map(|i| contexts.propose(i, 1, "Bloc-notes", "notepad.exe").id).collect();
        assert!(contexts.target(&ids[0]).is_err());
        assert!(contexts.target(&ids[5]).is_ok());
    }

    #[test]
    fn gathered_text_stops_at_the_ceiling_and_skips_repeats() {
        let mut gathered = Gathered { text: String::new(), chars: 0, full: false };
        gathered.add("Bonjour");
        gathered.add("Bonjour");
        gathered.add("  ");
        gathered.add("Mochi");
        assert_eq!(gathered.text, "Bonjour\nMochi");
        gathered.add(&"x".repeat(MAX_CHARS));
        assert!(gathered.full);
        assert!(gathered.text.chars().count() <= MAX_CHARS + 2);
    }

    /// The real capabilities, compiled by build.rs, decide who may call what: the island
    /// reaches the chat, the vault, files, e-mail and windows; Mochi's window only offers
    /// the window it is dropped on; the mini-player none of it.
    mod access {
        use tauri::ipc::{CallbackFn, InvokeBody};
        use tauri::test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder};
        use tauri::webview::InvokeRequest;
        use tauri::{App, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

        const LOCAL: &str = "http://tauri.localhost";
        const ISLAND_ONLY: [&str; 15] = [
            "secret_store",
            "secret_has",
            "secret_remove",
            "chat_send",
            "chat_cancel",
            "chat_models",
            "file_attach",
            "mail_app",
            "mail_send",
            "context_read",
            "context_confirm",
            "context_forget",
            "dictation_available",
            "dictation_start",
            "dictation_stop",
        ];

        fn app() -> App<MockRuntime> {
            mock_builder().build(tauri::generate_context!()).unwrap()
        }

        fn window(app: &App<MockRuntime>, label: &str) -> WebviewWindow<MockRuntime> {
            WebviewWindowBuilder::new(app, label, WebviewUrl::default()).build().unwrap()
        }

        fn allowed(window: &WebviewWindow<MockRuntime>, cmd: &str) -> bool {
            let request = InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                url: LOCAL.parse().unwrap(),
                body: InvokeBody::default(),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_owned(),
            };
            get_ipc_response(window, request).is_err_and(|e| !e.to_string().contains("not allowed"))
        }

        #[test]
        fn the_island_reaches_the_chat_and_mochi_s_window_only_offers_a_window() {
            let app = app();
            let main = window(&app, crate::window::WINDOW_LABEL);
            let buddy = window(&app, crate::buddy::BUDDY_LABEL);
            let mini = window(&app, crate::players::MINI_LABEL);
            for cmd in ISLAND_ONLY {
                assert!(allowed(&main, cmd), "island: {cmd}");
                assert!(!allowed(&buddy, cmd), "buddy: {cmd}");
                assert!(!allowed(&mini, cmd), "mini-player: {cmd}");
            }
            assert!(allowed(&buddy, "buddy_drop_window"));
            assert!(!allowed(&main, "buddy_drop_window"));
            assert!(!allowed(&mini, "buddy_drop_window"));
        }
    }

    /// Windows of this test, on their own thread with a message loop: UI Automation reads
    /// them from the test thread as it reads any other program's window.
    mod win32 {
        use super::super::*;
        use std::sync::mpsc;
        use std::thread::JoinHandle;
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::System::Threading::GetCurrentThreadId;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, DispatchMessageW, ES_PASSWORD, GetMessageW, MSG, PostThreadMessageW,
            SW_SHOWNOACTIVATE, ShowWindow, TranslateMessage, WINDOW_STYLE, WM_QUIT, WS_CHILD, WS_EX_NOACTIVATE,
            WS_POPUP, WS_VISIBLE,
        };
        use windows::core::{PCWSTR, w};

        struct TestWindow {
            window: isize,
            thread: u32,
            pump: Option<JoinHandle<()>>,
        }

        impl TestWindow {
            /// A titled popup holding two lines of static text, and a password box if asked.
            fn open(with_password: bool) -> Self {
                let (sender, receiver) = mpsc::channel();
                let pump = std::thread::spawn(move || {
                    // SAFETY: creates and shows windows owned by this thread, then runs
                    // their message loop until the test posts WM_QUIT.
                    unsafe {
                        let window = CreateWindowExW(
                            WS_EX_NOACTIVATE,
                            w!("STATIC"),
                            w!("îlot test contexte"),
                            WINDOW_STYLE(WS_POPUP.0),
                            -24_000,
                            -22_000,
                            400,
                            200,
                            None,
                            None,
                            None,
                            None,
                        )
                        .unwrap();
                        let line = |text: PCWSTR, y: i32, extra: u32, class: PCWSTR| {
                            CreateWindowExW(
                                Default::default(),
                                class,
                                text,
                                WS_CHILD | WS_VISIBLE | WINDOW_STYLE(extra),
                                0,
                                y,
                                380,
                                30,
                                Some(window),
                                None,
                                None,
                                None,
                            )
                            .unwrap();
                        };
                        line(w!("Liste des courses"), 0, 0, w!("STATIC"));
                        line(w!("Pain, beurre, confiture"), 40, 0, w!("STATIC"));
                        if with_password {
                            line(w!("hunter2"), 80, ES_PASSWORD as u32, w!("EDIT"));
                        }
                        let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
                        sender.send((window.0 as isize, GetCurrentThreadId())).unwrap();
                        let mut message = MSG::default();
                        while GetMessageW(&mut message, None, 0, 0).as_bool() {
                            let _ = TranslateMessage(&message);
                            DispatchMessageW(&message);
                        }
                        let _ = DestroyWindow(window);
                    }
                });
                let (window, thread) = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
                Self { window, thread, pump: Some(pump) }
            }
        }

        impl Drop for TestWindow {
            fn drop(&mut self) {
                // SAFETY: asks the pump thread to leave its loop and destroy its window.
                unsafe {
                    let _ = PostThreadMessageW(self.thread, WM_QUIT, WPARAM(0), LPARAM(0));
                }
                if let Some(pump) = self.pump.take() {
                    let _ = pump.join();
                }
            }
        }

        fn soon() -> Instant {
            Instant::now() + Duration::from_secs(3)
        }

        #[test]
        fn the_text_of_a_window_is_read() {
            let window = TestWindow::open(false);
            let (text, truncated) = read_window(window.window, soon()).ok().expect("readable");
            assert!(text.contains("Liste des courses"), "{text}");
            assert!(text.contains("Pain, beurre, confiture"), "{text}");
            assert!(!truncated);
            assert!(!out_of_reach(std::process::id()), "the test runs at the user's level");
        }

        #[test]
        fn a_window_with_a_password_box_is_not_read_at_all() {
            let window = TestWindow::open(true);
            match read_window(window.window, soon()) {
                Err(Unread::Blocked(Block::PasswordField)) => {}
                Ok((text, _)) => panic!("read: {text}"),
                Err(_) => panic!("another refusal"),
            }
        }

        #[test]
        fn a_closed_window_cannot_be_read() {
            assert!(matches!(read_window(0x7fff_fff0, soon()), Err(Unread::Failed(_))));
        }
    }

    #[test]
    fn blocked_windows_are_explained_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let contexts = Contexts::default();
        let proposal = contexts.propose(7, 1, "Mes comptes — BoursoBank", r"C:\x\firefox.exe");
        assert_eq!(proposal.blocked.as_deref(), Some("Bank or payment: îlot doesn't read this window."));
        assert_eq!(contexts.confirm(&proposal.id).unwrap_err(), "This window's text wasn't attached to the chat");
        assert_eq!(contexts.confirm("w999").unwrap_err(), "This window is no longer offered: drop Mochi on it again");
    }
}
