//! The vault on Windows: generic credentials of the Credential Manager, written with
//! `CredWriteW` and read with `CredReadW` for the user's logon session.
//!
//! Persistence `CRED_PERSIST_LOCAL_MACHINE`: the key stays on this PC and never travels
//! with a roaming profile (`CRED_PERSIST_ENTERPRISE` would). The crates `keyring` and
//! `windows-native-keyring-store` would wrap the same four calls, with `Enterprise` by
//! default; calling them directly adds no dependency and keeps every byte in view.
//! https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw
//! https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credwritew
//! https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credreadw
//! https://learn.microsoft.com/en-us/windows/win32/api/wincred/nf-wincred-credenumeratew

use super::{MAX_SECRET_BYTES, Secret, Store, wipe};
use crate::locale::{Text, tr};
use windows::Win32::Foundation::ERROR_NOT_FOUND;
use windows::Win32::Security::Credentials::{
    CRED_FLAGS, CRED_PERSIST, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredEnumerateW,
    CredFree, CredReadW, CredWriteW,
};
use windows::core::{HRESULT, PCWSTR, PWSTR};

/// Shown in « Gestionnaire d'identification » next to each key (with a comment in the
/// language of the moment).
const USER: &str = "îlot";

pub struct CredentialManager {
    persist: CRED_PERSIST,
}

impl CredentialManager {
    pub fn local() -> Self {
        Self { persist: CRED_PERSIST_LOCAL_MACHINE }
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain([0]).collect()
}

fn not_found(error: &windows::core::Error) -> bool {
    error.code() == HRESULT::from_win32(ERROR_NOT_FOUND.0)
}

/// The Windows error code only: never a message built from what was written.
fn failure(error: &windows::core::Error, say: impl FnOnce(&str) -> String) -> String {
    say(&format!("0x{:08X}", error.code().0))
}

impl Store for CredentialManager {
    fn write(&self, target: &str, secret: &[u8]) -> Result<(), String> {
        if secret.len() > MAX_SECRET_BYTES {
            return Err(super::TOO_LONG.into());
        }
        let mut target = wide(target);
        let mut user = wide(USER);
        let mut comment = wide(&tr(Text::CredentialComment));
        let mut blob = secret.to_vec();
        let credential = CREDENTIALW {
            Flags: CRED_FLAGS(0),
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(target.as_mut_ptr()),
            Comment: PWSTR(comment.as_mut_ptr()),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: self.persist,
            UserName: PWSTR(user.as_mut_ptr()),
            ..Default::default()
        };
        // SAFETY: every pointer of `credential` points into a buffer alive until the call
        // returns; CredWriteW copies what it keeps.
        let written = unsafe { CredWriteW(&credential, 0) };
        wipe(&mut blob);
        written.map_err(|e| failure(&e, |code| tr(Text::CredentialSaveFailed { code })))
    }

    fn read(&self, target: &str) -> Result<Option<Secret>, String> {
        let target = wide(target);
        let mut found: *mut CREDENTIALW = std::ptr::null_mut();
        // SAFETY: a null-terminated name and an out-pointer that CredReadW fills with a
        // single block, freed below with CredFree.
        match unsafe { CredReadW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None, &mut found) } {
            Err(e) if not_found(&e) => Ok(None),
            Err(e) => Err(failure(&e, |code| tr(Text::CredentialReadFailed { code }))),
            Ok(()) => {
                // SAFETY: on success `found` points to a CREDENTIALW whose blob holds
                // `CredentialBlobSize` bytes; both live until CredFree.
                let bytes = unsafe {
                    let credential = &*found;
                    let size = credential.CredentialBlobSize as usize;
                    let copy = if credential.CredentialBlob.is_null() || size == 0 {
                        Vec::new()
                    } else {
                        let blob = std::slice::from_raw_parts_mut(credential.CredentialBlob, size);
                        let copy = blob.to_vec();
                        wipe(blob);
                        copy
                    };
                    CredFree(found.cast());
                    copy
                };
                Ok(Some(Secret::new(bytes)))
            }
        }
    }

    fn delete(&self, target: &str) -> Result<bool, String> {
        let target = wide(target);
        // SAFETY: a null-terminated name; nothing is returned to free.
        match unsafe { CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None) } {
            Ok(()) => Ok(true),
            Err(e) if not_found(&e) => Ok(false),
            Err(e) => Err(failure(&e, |code| tr(Text::CredentialRemoveFailed { code }))),
        }
    }

    fn list(&self, prefix: &str) -> Result<Vec<String>, String> {
        let filter = wide(&format!("{prefix}*"));
        let mut count = 0u32;
        let mut list: *mut *mut CREDENTIALW = std::ptr::null_mut();
        // SAFETY: a null-terminated filter and out-pointers filled with one block freed
        // with CredFree; only the target names are read from it.
        match unsafe { CredEnumerateW(PCWSTR(filter.as_ptr()), None, &mut count, &mut list) } {
            Err(e) if not_found(&e) => Ok(Vec::new()),
            Err(e) => Err(failure(&e, |code| tr(Text::CredentialListFailed { code }))),
            Ok(()) => {
                // SAFETY: `list` holds `count` pointers to credentials, valid until CredFree.
                let names = unsafe {
                    let names = std::slice::from_raw_parts(list, count as usize)
                        .iter()
                        .filter_map(|credential| (**credential).TargetName.to_string().ok())
                        .collect();
                    CredFree(list.cast());
                    names
                };
                Ok(names)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::Security::Credentials::CRED_PERSIST_SESSION;

    /// Session persistence: the test's credentials vanish at sign-out even if a run is cut
    /// short; each one is deleted at the end anyway.
    fn session() -> CredentialManager {
        CredentialManager { persist: CRED_PERSIST_SESSION }
    }

    fn name(test: &str) -> String {
        format!("ilot-test-{}:{test}", std::process::id())
    }

    #[test]
    fn writes_reads_replaces_and_deletes_a_generic_credential() {
        let store = session();
        let target = name("roundtrip");
        assert!(store.read(&target).unwrap().is_none());
        store.write(&target, "sk-test-één".as_bytes()).unwrap();
        assert_eq!(store.read(&target).unwrap().unwrap().bytes(), "sk-test-één".as_bytes());
        store.write(&target, b"sk-test-2").unwrap();
        assert_eq!(store.read(&target).unwrap().unwrap().bytes(), b"sk-test-2");
        assert_eq!(store.list(&name("")).unwrap(), std::slice::from_ref(&target));
        assert!(store.delete(&target).unwrap());
        assert!(!store.delete(&target).unwrap());
        assert!(store.read(&target).unwrap().is_none());
        assert!(store.list(&name("")).unwrap().is_empty());
    }

    #[test]
    fn the_largest_secret_windows_takes_fits_and_a_larger_one_is_refused_first() {
        let store = session();
        let target = name("largest");
        store.write(&target, &[b'k'; MAX_SECRET_BYTES]).unwrap();
        assert_eq!(store.read(&target).unwrap().unwrap().bytes().len(), MAX_SECRET_BYTES);
        assert_eq!(store.write(&target, &[b'k'; MAX_SECRET_BYTES + 1]).unwrap_err(), super::super::TOO_LONG);
        store.delete(&target).unwrap();
    }
}
