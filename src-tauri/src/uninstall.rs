//! `ilot.exe --uninstall-cleanup [--remove-data]`, run by the uninstaller before it removes
//! the files: unplugs every AI (configs given back as they were, relay scripts removed),
//! turns off the start with Windows, removes the identity package, and with
//! `--remove-data` deletes îlot's own settings and the keys it kept in the Windows
//! Credential Manager (`ilot:*`). Only files îlot wrote are touched; the
//! backups `<file>.avant-ilot` stay with the user.

use crate::connectors::{self, Status};
use std::io;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// Windows PowerShell, by its full path in System32.
const POWERSHELL: &str = r"WindowsPowerShell\v1.0\powershell.exe";
/// Fixed command (no data from outside): the identity package of packaging/identity.
const REMOVE_IDENTITY: &str = "Get-AppxPackage -Name io.github.hic-69.ilot | Remove-AppxPackage";

/// Returns the number of steps that failed (0 = everything cleaned).
pub fn run(remove_data: bool) -> i32 {
    let mut failures = 0;
    for target in connectors::all_targets() {
        for connector in connectors::registry() {
            if matches!(connector.status(&target), Status::Connected | Status::NeedsRepair)
                && connector.disconnect(&target).is_err()
            {
                failures += 1;
            }
        }
        if connectors::tidy_relay(&target).is_err() {
            failures += 1;
        }
    }
    if let Ok(exe) = std::env::current_exe()
        && crate::autostart::set(false, &exe).is_err()
    {
        failures += 1;
    }
    let removed = crate::wsl::system_program(POWERSHELL).and_then(|powershell| {
        Command::new(powershell)
            .args(["-NoProfile", "-NonInteractive", "-Command", REMOVE_IDENTITY])
            .creation_flags(CREATE_NO_WINDOW)
            .status()
    });
    if !removed.is_ok_and(|s| s.success()) {
        failures += 1;
    }
    if remove_data {
        let files = [crate::settings::settings_path(), crate::stats::stats_path(), ilot_bridge::identity::token_path()];
        for file in files.into_iter().flatten() {
            if remove_file_and_empty_folder(&file).is_err() {
                failures += 1;
            }
        }
        // The keys saved in the Credential Manager go with the rest of îlot's data.
        failures += crate::vault::remove_all() as i32;
    }
    failures
}

/// Deletes a file îlot owns, then its folder if nothing else is left in it.
pub fn remove_file_and_empty_folder(file: &Path) -> io::Result<()> {
    match std::fs::remove_file(file) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    if let Some(dir) = file.parent()
        && std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_none())
    {
        std::fs::remove_dir(dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_the_file_and_the_folder_left_empty() {
        let dir = std::env::temp_dir().join(format!("ilot-uninstall-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        std::fs::write(&file, "{}").unwrap();
        remove_file_and_empty_folder(&file).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn keeps_a_folder_that_holds_something_else() {
        let dir = std::env::temp_dir().join(format!("ilot-uninstall-keep-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("settings.json"), "{}").unwrap();
        std::fs::write(dir.join("notes.txt"), "mine").unwrap();
        remove_file_and_empty_folder(&dir.join("settings.json")).unwrap();
        assert!(dir.join("notes.txt").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn powershell_is_the_one_of_system32() {
        let powershell = crate::wsl::system_program(POWERSHELL).unwrap();
        assert!(powershell.is_absolute() && powershell.exists(), "{}", powershell.display());
        assert!(powershell.to_string_lossy().to_lowercase().ends_with(r"\system32\windowspowershell\v1.0\powershell.exe"));
    }

    #[test]
    fn a_missing_file_is_fine() {
        let dir = std::env::temp_dir().join(format!("ilot-uninstall-missing-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("other")).unwrap();
        assert!(remove_file_and_empty_folder(&dir.join("nothing.json")).is_ok());
        assert!(dir.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
