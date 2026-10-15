//! Who may talk to îlot: the current Windows user only. The pipe name carries the
//! user's SID and the pipe's access list grants that SID alone (docs/06-SECURITE.md).

use std::ffi::c_void;
use std::io;
use std::path::PathBuf;
use windows::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, LocalFree};
use windows::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows::Win32::Security::{
    GetTokenInformation, PSECURITY_DESCRIPTOR, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows::core::{HSTRING, PWSTR};

/// String form of the current user's SID, for example `S-1-5-21-…-1001`.
pub fn current_user_sid() -> io::Result<String> {
    // SAFETY: the token handle is closed below; the buffer is sized by the first call
    // and the SID string is freed with LocalFree as the API requires.
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)?;
        let mut needed = 0u32;
        let _ = GetTokenInformation(token, TokenUser, None, 0, &mut needed);
        let mut buffer = vec![0u8; needed as usize];
        let read = GetTokenInformation(
            token,
            TokenUser,
            Some(buffer.as_mut_ptr().cast::<c_void>()),
            needed,
            &mut needed,
        );
        let _ = CloseHandle(token);
        read?;
        let user = &*buffer.as_ptr().cast::<TOKEN_USER>();
        let mut text = PWSTR::null();
        ConvertSidToStringSidW(user.User.Sid, &mut text)?;
        let sid = text.to_string().map_err(io::Error::other);
        let _ = LocalFree(Some(HLOCAL(text.0.cast())));
        sid
    }
}

pub fn pipe_name(sid: &str) -> String {
    format!(r"\\.\pipe\ilot-{sid}")
}

/// `%LOCALAPPDATA%\ilot\pipe.token`: the per-start secret relays must present.
pub fn token_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("ilot").join("pipe.token"))
}

/// SDDL that grants full access to one SID and to nobody else (protected DACL).
pub fn owner_only_sddl(sid: &str) -> String {
    format!("D:P(A;;GA;;;{sid})")
}

/// A security descriptor built from SDDL, freed on drop.
pub struct SecurityDescriptor(PSECURITY_DESCRIPTOR);

impl SecurityDescriptor {
    pub fn from_sddl(sddl: &str) -> io::Result<Self> {
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        // SAFETY: the API allocates the descriptor, released in Drop with LocalFree.
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                &HSTRING::from(sddl),
                SDDL_REVISION_1,
                &mut descriptor,
                None,
            )?;
        }
        Ok(Self(descriptor))
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self.0.0
    }
}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        // SAFETY: the pointer came from ConvertStringSecurityDescriptorToSecurityDescriptorW.
        unsafe {
            let _ = LocalFree(Some(HLOCAL(self.0.0)));
        }
    }
}

// SAFETY: the descriptor is immutable once built and only read by the system.
unsafe impl Send for SecurityDescriptor {}
unsafe impl Sync for SecurityDescriptor {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_user_has_a_sid() {
        let sid = current_user_sid().unwrap();
        assert!(sid.starts_with("S-1-5-"), "{sid}");
    }

    #[test]
    fn pipe_name_contains_the_sid() {
        assert_eq!(pipe_name("S-1-5-21-1-2-3-1001"), r"\\.\pipe\ilot-S-1-5-21-1-2-3-1001");
    }

    #[test]
    fn sddl_grants_only_the_given_sid() {
        assert_eq!(owner_only_sddl("S-1-5-21-9"), "D:P(A;;GA;;;S-1-5-21-9)");
        assert!(SecurityDescriptor::from_sddl(&owner_only_sddl(&current_user_sid().unwrap())).is_ok());
    }

    #[test]
    fn invalid_sddl_is_an_error() {
        assert!(SecurityDescriptor::from_sddl("D:P(A;;GA;;;not-a-sid)").is_err());
    }
}
