//! Atomic file writes: write to a temporary file in the same folder, then rename it
//! over the target, so a crash never leaves a half-written file behind.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    replace_atomic(path, bytes, |_| Ok(()), || Ok(true)).map(drop)
}

/// `write_atomic` for a file someone else may write too. `prepare` gets the temporary
/// file while it is still empty (to give it the target's permissions before any byte
/// lands in it); `unchanged` is asked just before the rename, and `false` gives up: the
/// target stays as it is and the result is `Ok(false)`.
pub fn replace_atomic(
    path: &Path,
    bytes: &[u8],
    prepare: impl FnOnce(&Path) -> io::Result<()>,
    unchanged: impl FnOnce() -> io::Result<bool>,
) -> io::Result<bool> {
    let tmp = temp_path(path)?;
    let replaced = write_then_rename(&tmp, path, bytes, prepare, unchanged);
    if !matches!(replaced, Ok(true)) {
        let _ = fs::remove_file(&tmp);
    }
    replaced
}

fn temp_path(path: &Path) -> io::Result<PathBuf> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no parent"))?;
    fs::create_dir_all(dir)?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let mut tmp_name = name.to_os_string();
    tmp_name.push(".ilot-tmp");
    Ok(dir.join(tmp_name))
}

fn write_then_rename(
    tmp: &Path,
    path: &Path,
    bytes: &[u8],
    prepare: impl FnOnce(&Path) -> io::Result<()>,
    unchanged: impl FnOnce() -> io::Result<bool>,
) -> io::Result<bool> {
    {
        let mut file = fs::File::create(tmp)?;
        prepare(tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    if !unchanged()? {
        return Ok(false);
    }
    fs::rename(tmp, path).map(|()| true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ilot-atomic-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn names(dir: &Path) -> Vec<std::ffi::OsString> {
        fs::read_dir(dir).unwrap().map(|e| e.unwrap().file_name()).collect()
    }

    #[test]
    fn creates_missing_folders_and_file() {
        let dir = temp_dir("create");
        let path = dir.join("a").join("settings.json");
        write_atomic(&path, b"{}").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"{}");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn replaces_existing_file_and_leaves_no_temp_file() {
        let dir = temp_dir("replace");
        let path = dir.join("settings.json");
        write_atomic(&path, b"old").unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(names(&dir), vec![std::ffi::OsString::from("settings.json")]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_temporary_file_is_prepared_while_still_empty() {
        let dir = temp_dir("prepare");
        let path = dir.join("settings.json");
        let mut seen = None;
        let replaced = replace_atomic(&path, b"secret", |tmp| {
            seen = Some((tmp.to_owned(), fs::read(tmp)?));
            Ok(())
        }, || Ok(true));
        assert!(replaced.unwrap());
        assert_eq!(seen, Some((dir.join("settings.json.ilot-tmp"), Vec::new())));
        assert_eq!(fs::read(&path).unwrap(), b"secret");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_target_changed_meanwhile_is_left_alone() {
        let dir = temp_dir("changed");
        let path = dir.join("settings.json");
        write_atomic(&path, b"theirs").unwrap();
        assert!(!replace_atomic(&path, b"ours", |_| Ok(()), || Ok(false)).unwrap());
        assert_eq!(fs::read(&path).unwrap(), b"theirs");
        assert_eq!(names(&dir), vec![std::ffi::OsString::from("settings.json")]);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_failed_preparation_writes_nothing() {
        let dir = temp_dir("failed");
        let path = dir.join("settings.json");
        write_atomic(&path, b"old").unwrap();
        let failed = replace_atomic(&path, b"new", |_| Err(io::Error::other("chmod failed")), || Ok(true));
        assert!(failed.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"old");
        assert_eq!(names(&dir), vec![std::ffi::OsString::from("settings.json")]);
        fs::remove_dir_all(dir).unwrap();
    }
}
