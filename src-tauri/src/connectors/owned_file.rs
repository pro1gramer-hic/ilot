//! A file îlot owns inside an agent's folder: Copilot CLI and Kiro read every hooks file of
//! their `hooks/` folder, OpenCode and Amp every plugin of their `plugins/` folder, so îlot
//! writes one file of its own there (`ilot.json`, `ilot.js`, `ilot.ts`) instead of editing
//! the user's. Connecting writes it atomically; disconnecting deletes it. A file of that
//! name that îlot did not write is never overwritten nor deleted: there is nothing of the
//! user's to back up, since nothing of the user's is ever touched.

use super::{Status, Target};
use crate::atomic::write_atomic;
use crate::locale::{Text, tr};
use std::io;

/// What a file at îlot's place is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Owner {
    /// Written by îlot, as it writes it now.
    Current,
    /// Written by an older îlot: to repair.
    Older,
    /// Someone else's.
    Other,
}

pub struct OwnedFile {
    /// The file, from the home folder (`.copilot/hooks/ilot.json`).
    pub path: &'static str,
    /// The folder that tells the agent is installed (`.copilot`).
    pub agent_dir: &'static str,
    /// Reads a file found at `path`.
    pub owner: fn(&str) -> Owner,
}

impl OwnedFile {
    pub fn status(&self, target: &Target) -> Status {
        match std::fs::read_to_string(target.home_file(self.path)) {
            Ok(text) => match (self.owner)(&text) {
                Owner::Current => Status::Connected,
                Owner::Older => Status::NeedsRepair,
                Owner::Other => Status::Installed,
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if target.home_file(self.agent_dir).is_dir() { Status::Installed } else { Status::NotInstalled }
            }
            Err(_) => Status::NeedsRepair,
        }
    }

    /// Writes îlot's file, unless a file of the user's is in the way.
    pub fn write(&self, target: &Target, content: &str) -> io::Result<()> {
        let path = target.home_file(self.path);
        if let Ok(text) = std::fs::read_to_string(&path)
            && (self.owner)(&text) == Owner::Other
        {
            return Err(io::Error::other(tr(Text::NotWrittenByIlot { path: &format!("~/{}", self.path) })));
        }
        write_atomic(&path, content.as_bytes())
    }

    /// Deletes îlot's file; a file of the user's, or none, is left as it is.
    pub fn remove(&self, target: &Target) -> io::Result<()> {
        let path = target.home_file(self.path);
        match std::fs::read_to_string(&path) {
            Ok(text) if (self.owner)(&text) != Owner::Other => std::fs::remove_file(path),
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

/// First line of the plugins îlot writes (JavaScript and TypeScript).
pub const PLUGIN_MARKER: &str = "// Installed by îlot";

/// A plugin's owner, from its first line and the revision it names.
pub fn plugin_owner(text: &str, revision: &str) -> Owner {
    if !text.starts_with(PLUGIN_MARKER) {
        Owner::Other
    } else if text.lines().any(|line| line == revision) {
        Owner::Current
    } else {
        Owner::Older
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const REVISION: &str = "// ilot-plugin-revision: 2";

    fn owner(text: &str) -> Owner {
        plugin_owner(text, REVISION)
    }

    const FILE: OwnedFile = OwnedFile { path: ".agent/plugins/ilot.js", agent_dir: ".agent", owner };

    fn home(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ilot-owned-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_plugin_is_written_then_removed() {
        let dir = home("cycle");
        let target = Target::windows(dir.clone());
        assert_eq!(FILE.status(&target), Status::NotInstalled);
        std::fs::create_dir_all(dir.join(".agent")).unwrap();
        assert_eq!(FILE.status(&target), Status::Installed);
        FILE.write(&target, &format!("{PLUGIN_MARKER}: test\n{REVISION}\n")).unwrap();
        assert_eq!(FILE.status(&target), Status::Connected);
        FILE.write(&target, &format!("{PLUGIN_MARKER}: test\n// ilot-plugin-revision: 1\n")).unwrap();
        assert_eq!(FILE.status(&target), Status::NeedsRepair, "an older plugin");
        FILE.remove(&target).unwrap();
        assert!(!dir.join(".agent/plugins/ilot.js").exists());
        assert_eq!(FILE.status(&target), Status::Installed);
        FILE.remove(&target).unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_file_of_the_user_s_is_never_touched() {
        let dir = home("theirs");
        let target = Target::windows(dir.clone());
        let theirs = dir.join(".agent/plugins/ilot.js");
        std::fs::create_dir_all(theirs.parent().unwrap()).unwrap();
        std::fs::write(&theirs, "export const Mine = async () => ({});\n").unwrap();
        assert_eq!(FILE.status(&target), Status::Installed);
        assert!(FILE.write(&target, &format!("{PLUGIN_MARKER}\n{REVISION}\n")).is_err());
        FILE.remove(&target).unwrap();
        assert_eq!(std::fs::read_to_string(&theirs).unwrap(), "export const Mine = async () => ({});\n");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_file_of_the_user_s_is_refused_in_the_language_of_the_moment() {
        crate::locale::speak(crate::locale::Lang::En);
        let dir = home("theirs-en");
        let target = Target::windows(dir.clone());
        let theirs = dir.join(".agent/plugins/ilot.js");
        std::fs::create_dir_all(theirs.parent().unwrap()).unwrap();
        std::fs::write(&theirs, "export const Mine = 1;\n").unwrap();
        let error = FILE.write(&target, &format!("{PLUGIN_MARKER}\n{REVISION}\n")).unwrap_err();
        assert_eq!(error.to_string(), "~/.agent/plugins/ilot.js already exists and was not written by îlot: rename it first.");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
