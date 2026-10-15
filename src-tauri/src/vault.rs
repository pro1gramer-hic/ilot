//! The vault: the secrets the user gives îlot (the chat's API keys, the services' tokens)
//! live in the operating system's own store, never in a file, a setting or a log.
//!
//! One layer for every feature: `store`, `has`, `remove`, and `read` for the core only.
//! The interface can save a key, ask whether one is saved, or erase it (`secret_*`
//! commands); no command ever hands a secret back. Names are `chat:<provider>` or
//! `service:<id>`; anything else is refused.
//!
//! Each platform plugs its own store behind `Store`. Windows: the Credential Manager
//! (`credential_manager.rs`). A macOS Keychain or a Linux Secret Service module would
//! implement the same trait and be picked the same way.

#[cfg(windows)]
mod credential_manager;

use crate::locale::Text;

/// The only names the interface may use.
pub const PREFIXES: [&str; 2] = ["chat:", "service:"];
/// Every secret îlot writes is filed under this prefix (Microsoft suggests prefixing the
/// target name with the publisher's), so that uninstalling finds them all.
pub const TARGET_PREFIX: &str = "ilot:";
/// Longest name after its prefix.
const MAX_NAME_CHARS: usize = 64;
/// `CRED_MAX_CREDENTIAL_BLOB_SIZE` (5 × 512 bytes): a longer secret is refused before
/// anything is written.
/// https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw
pub const MAX_SECRET_BYTES: usize = 2560;

pub const BAD_NAME: Text<'static> = Text::SecretBadName;
pub const EMPTY: Text<'static> = Text::KeyEmpty;
pub const TOO_LONG: Text<'static> = Text::KeyTooLong;
pub const CONTROL: Text<'static> = Text::KeyControl;

/// A platform's secret store. Errors are sentences in the language of the moment that never
/// quote the secret.
pub trait Store {
    fn write(&self, target: &str, secret: &[u8]) -> Result<(), String>;
    fn read(&self, target: &str) -> Result<Option<Secret>, String>;
    /// True when something was there.
    fn delete(&self, target: &str) -> Result<bool, String>;
    /// Every target name that starts with `prefix`.
    fn list(&self, prefix: &str) -> Result<Vec<String>, String>;
}

/// Bytes of a secret, zeroed when dropped. No `Debug`, no `Display`.
pub struct Secret(Vec<u8>);

impl Secret {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        wipe(&mut self.0);
    }
}

/// Overwrites a buffer that held a secret (volatile writes: the compiler keeps them).
pub fn wipe(bytes: &mut [u8]) {
    for byte in bytes.iter_mut() {
        // SAFETY: a valid, aligned, exclusive reference to one byte of the buffer.
        unsafe { std::ptr::write_volatile(byte, 0) };
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}

/// Zeroes a string that held a secret, then lets it go.
pub fn wipe_string(mut text: String) {
    // SAFETY: zero bytes are valid UTF-8, so the string stays a string until it is freed.
    wipe(unsafe { text.as_bytes_mut() });
}

/// `chat:anthropic`, `service:github`: a known prefix, then 1 to 64 lowercase letters,
/// digits, `.`, `_` or `-`.
pub fn valid_name(name: &str) -> bool {
    PREFIXES.iter().any(|prefix| {
        name.strip_prefix(prefix).is_some_and(|rest| {
            !rest.is_empty()
                && rest.chars().count() <= MAX_NAME_CHARS
                && rest.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
        })
    })
}

fn target(name: &str) -> Result<String, String> {
    if valid_name(name) { Ok(format!("{TARGET_PREFIX}{name}")) } else { Err(BAD_NAME.into()) }
}

/// A key as pasted: spaces and line ends around it go; it must be short and printable.
fn checked(secret: &str) -> Result<&str, String> {
    let secret = secret.trim();
    if secret.is_empty() {
        Err(EMPTY.into())
    } else if secret.len() > MAX_SECRET_BYTES {
        Err(TOO_LONG.into())
    } else if secret.chars().any(char::is_control) {
        Err(CONTROL.into())
    } else {
        Ok(secret)
    }
}

pub fn store_in(store: &dyn Store, name: &str, secret: &str) -> Result<(), String> {
    let target = target(name)?;
    store.write(&target, checked(secret)?.as_bytes())
}

pub fn has_in(store: &dyn Store, name: &str) -> Result<bool, String> {
    Ok(store.read(&target(name)?)?.is_some())
}

pub fn remove_in(store: &dyn Store, name: &str) -> Result<(), String> {
    store.delete(&target(name)?).map(|_| ())
}

pub fn read_in(store: &dyn Store, name: &str) -> Result<Option<String>, String> {
    let Some(secret) = store.read(&target(name)?)? else { return Ok(None) };
    std::str::from_utf8(secret.bytes()).map(|text| Some(text.to_owned())).map_err(|_| Text::KeyUnreadable.into())
}

/// Every secret of îlot, removed (uninstall with « supprimer mes données »). Returns how
/// many could not be removed.
pub fn remove_all_in(store: &dyn Store) -> usize {
    match store.list(TARGET_PREFIX) {
        Ok(targets) => targets.iter().filter(|t| store.delete(t).is_err()).count(),
        Err(_) => 1,
    }
}

#[cfg(windows)]
fn platform() -> credential_manager::CredentialManager {
    credential_manager::CredentialManager::local()
}

/// Saves (or replaces) a secret.
pub fn store(name: &str, secret: &str) -> Result<(), String> {
    store_in(&platform(), name, secret)
}

/// True when a secret is saved under this name (a refused name is never saved).
pub fn has(name: &str) -> bool {
    has_in(&platform(), name).unwrap_or(false)
}

/// Erases a secret; nothing to erase is fine.
pub fn remove(name: &str) -> Result<(), String> {
    remove_in(&platform(), name)
}

/// The secret itself, for the core's own requests. Never sent to a window.
pub fn read(name: &str) -> Result<Option<String>, String> {
    read_in(&platform(), name)
}

pub fn remove_all() -> usize {
    remove_all_in(&platform())
}

/// Saves a key typed in Réglages. The string is zeroed once written.
#[tauri::command]
pub async fn secret_store(name: String, secret: String) -> Result<(), String> {
    let result = store(&name, &secret);
    wipe_string(secret);
    result
}

/// « Clé enregistrée » or not: the only thing the interface learns about a secret.
#[tauri::command]
pub async fn secret_has(name: String) -> Result<bool, String> {
    if valid_name(&name) { Ok(has(&name)) } else { Err(BAD_NAME.into()) }
}

#[tauri::command]
pub async fn secret_remove(name: String) -> Result<(), String> {
    remove(&name)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    /// A store in memory, for the rules above.
    #[derive(Default)]
    pub struct Memory(pub Mutex<BTreeMap<String, Vec<u8>>>);

    impl Store for Memory {
        fn write(&self, target: &str, secret: &[u8]) -> Result<(), String> {
            self.0.lock().unwrap().insert(target.into(), secret.to_vec());
            Ok(())
        }
        fn read(&self, target: &str) -> Result<Option<Secret>, String> {
            Ok(self.0.lock().unwrap().get(target).map(|s| Secret::new(s.clone())))
        }
        fn delete(&self, target: &str) -> Result<bool, String> {
            Ok(self.0.lock().unwrap().remove(target).is_some())
        }
        fn list(&self, prefix: &str) -> Result<Vec<String>, String> {
            Ok(self.0.lock().unwrap().keys().filter(|k| k.starts_with(prefix)).cloned().collect())
        }
    }

    const KEY: &str = "sk-ant-api03-TEST-0123456789abcdefghijklmnopqrstuvwxyz";

    #[test]
    fn only_chat_and_service_names_are_accepted() {
        for name in ["chat:anthropic", "chat:gemini", "chat:openai", "service:github", "service:n8n-prod", "service:a.b_c"] {
            assert!(valid_name(name), "{name}");
        }
        for name in ["", "chat:", "anthropic", "ilot:chat:openai", "chat:Anthropic", "chat:a b", "chat:../x", "service:é", "secret:x", "chat:x\n"] {
            assert!(!valid_name(name), "{name:?}");
        }
        assert!(!valid_name(&format!("chat:{}", "a".repeat(65))));
        let store = Memory::default();
        assert_eq!(store_in(&store, "settings:theme", KEY).unwrap_err(), BAD_NAME);
        assert_eq!(has_in(&store, "../../etc").unwrap_err(), BAD_NAME);
        assert_eq!(remove_in(&store, "chat").unwrap_err(), BAD_NAME);
        assert_eq!(read_in(&store, "x").unwrap_err(), BAD_NAME);
        assert!(store.0.lock().unwrap().is_empty());
    }

    #[test]
    fn stores_reads_replaces_and_removes_under_the_ilot_prefix() {
        let store = Memory::default();
        assert!(!has_in(&store, "chat:openai").unwrap());
        store_in(&store, "chat:openai", &format!("  {KEY}\n")).unwrap();
        assert!(has_in(&store, "chat:openai").unwrap());
        assert_eq!(store.0.lock().unwrap().keys().collect::<Vec<_>>(), ["ilot:chat:openai"]);
        assert_eq!(read_in(&store, "chat:openai").unwrap().as_deref(), Some(KEY), "trimmed");
        store_in(&store, "chat:openai", "sk-other").unwrap();
        assert_eq!(read_in(&store, "chat:openai").unwrap().as_deref(), Some("sk-other"));
        remove_in(&store, "chat:openai").unwrap();
        remove_in(&store, "chat:openai").unwrap();
        assert_eq!(read_in(&store, "chat:openai").unwrap(), None);
    }

    #[test]
    fn empty_long_and_invisible_secrets_are_refused_without_quoting_them() {
        let store = Memory::default();
        assert_eq!(store_in(&store, "chat:gemini", " \n").unwrap_err(), EMPTY);
        let long = "k".repeat(MAX_SECRET_BYTES + 1);
        assert_eq!(store_in(&store, "chat:gemini", &long).unwrap_err(), TOO_LONG);
        let hidden = format!("{KEY}\u{7}");
        let error = store_in(&store, "chat:gemini", &hidden).unwrap_err();
        assert_eq!(error, CONTROL);
        assert!(!error.contains(KEY));
        assert!(store.0.lock().unwrap().is_empty());
        store_in(&store, "chat:gemini", &"k".repeat(MAX_SECRET_BYTES)).unwrap();
    }

    #[test]
    fn uninstalling_removes_every_secret_of_ilot_and_only_them() {
        let store = Memory::default();
        store_in(&store, "chat:anthropic", KEY).unwrap();
        store_in(&store, "service:github", KEY).unwrap();
        store.write("someone-else:token", b"x").unwrap();
        assert_eq!(remove_all_in(&store), 0);
        assert_eq!(store.0.lock().unwrap().keys().collect::<Vec<_>>(), ["someone-else:token"]);
    }

    #[test]
    fn a_secret_is_zeroed_once_dropped() {
        let mut bytes = KEY.as_bytes().to_vec();
        wipe(&mut bytes);
        assert!(bytes.iter().all(|&b| b == 0));
        let secret = Secret::new(KEY.as_bytes().to_vec());
        assert_eq!(secret.bytes(), KEY.as_bytes());
    }

    /// Nothing that leaves the vault for the interface or a log carries the key: errors
    /// are fixed sentences, and the settings (serialized as saved) never hold it.
    #[test]
    fn the_key_never_shows_in_errors_or_settings() {
        let store = Memory::default();
        let errors = [
            store_in(&store, "bad", KEY).unwrap_err(),
            store_in(&store, "chat:anthropic", &format!("{KEY}\u{0}")).unwrap_err(),
            store_in(&store, "chat:anthropic", &format!("{KEY}{}", "x".repeat(MAX_SECRET_BYTES))).unwrap_err(),
        ];
        for error in errors {
            assert!(!error.contains("sk-ant"), "{error}");
        }
        store_in(&store, "chat:anthropic", KEY).unwrap();
        let settings = crate::settings::Settings::default();
        let json = serde_json::to_string(&settings).unwrap();
        assert!(!json.contains(KEY) && !json.contains("sk-"));
    }

    #[test]
    fn refusals_are_said_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let store = Memory::default();
        assert_eq!(store_in(&store, "settings:theme", KEY).unwrap_err(), "Secret name refused");
        assert_eq!(store_in(&store, "chat:gemini", " \n").unwrap_err(), "The key is empty");
        assert_eq!(store_in(&store, "chat:gemini", &"k".repeat(MAX_SECRET_BYTES + 1)).unwrap_err(), "Key too long: 2,560 bytes at most");
    }
}
