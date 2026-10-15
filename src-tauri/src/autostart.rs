//! Start with Windows (option, off by default): one value under the current user's `Run`
//! key, pointing at îlot's executable. Nothing else, and removed when turned off.

use std::io;
use std::path::Path;
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, REG_SZ, RegDeleteKeyValueW, RegSetKeyValueW};
use windows::core::HSTRING;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "ilot";

/// The command Windows runs at sign-in: the quoted path of the executable.
pub fn run_command(exe: &Path) -> String {
    format!("\"{}\"", exe.display())
}

pub fn set(enabled: bool, exe: &Path) -> io::Result<()> {
    let key = HSTRING::from(RUN_KEY);
    let name = HSTRING::from(VALUE_NAME);
    // SAFETY: the strings outlive the calls; the data pointer covers the UTF-16 value
    // and its terminating zero, as REG_SZ requires.
    unsafe {
        if enabled {
            let value: Vec<u16> = run_command(exe).encode_utf16().chain(std::iter::once(0)).collect();
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                &key,
                &name,
                REG_SZ.0,
                Some(value.as_ptr().cast()),
                (value.len() * 2) as u32,
            )
            .ok()?;
        } else {
            let result = RegDeleteKeyValueW(HKEY_CURRENT_USER, &key, &name);
            if result.is_err() && result != ERROR_FILE_NOT_FOUND {
                result.ok()?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_is_quoted_for_spaces() {
        assert_eq!(
            run_command(Path::new(r"C:\Users\HP 240\AppData\Local\îlot\ilot.exe")),
            r#""C:\Users\HP 240\AppData\Local\îlot\ilot.exe""#
        );
    }
}
