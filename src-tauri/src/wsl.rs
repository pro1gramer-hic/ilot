//! WSL distributions seen from Windows: listing, home folders, file paths and modes, and
//! the scripts installed in `~/.local/bin` (the relay for the agents' hooks, the `ilot`
//! command for any other tool). Only `wsl.exe` from System32 is started, with fixed
//! arguments, a distribution name taken from `wsl.exe -l -q` and paths built by îlot
//! (never from events).

use crate::atomic::write_atomic;
use crate::locale::{Text, tr};
use std::io;
use std::os::windows::process::CommandExt;
use std::path::{Component, Path, PathBuf, Prefix};
use std::process::Command;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{FOLDERID_System, KF_FLAG_DEFAULT, SHGetKnownFolderPath};

/// No console window flashes when the GUI starts `wsl.exe`.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
pub const RELAY_SCRIPT: &str = ".local/bin/ilot-hook";
pub const CLI_SCRIPT: &str = ".local/bin/ilot";
/// First comment of every script îlot writes: a file without it is the user's own.
pub const SCRIPT_MARKER: &str = "# Installed by îlot";

/// Full path of a program of Windows itself (`System32\…`). Started by its name alone,
/// a program is first looked for in îlot's own folder, which the user can write to.
pub fn system_program(relative: &str) -> io::Result<PathBuf> {
    // SAFETY: the string Windows allocates is copied, then freed once with CoTaskMemFree,
    // as SHGetKnownFolderPath requires.
    let folder = unsafe {
        let path = SHGetKnownFolderPath(&FOLDERID_System, KF_FLAG_DEFAULT, None).map_err(io::Error::other)?;
        let folder = path.to_string();
        CoTaskMemFree(Some(path.0.cast_const().cast()));
        folder.map_err(io::Error::other)?
    };
    Ok(PathBuf::from(folder).join(relative))
}

fn wsl(args: &[&str]) -> io::Result<Vec<u8>> {
    let output = Command::new(system_program("wsl.exe")?).args(args).creation_flags(CREATE_NO_WINDOW).output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(io::Error::other(format!("wsl.exe {} failed", args.join(" "))))
    }
}

/// `wsl.exe -l -q` writes UTF-16LE, one distribution per line.
pub fn parse_distro_list(bytes: &[u8]) -> Vec<String> {
    let (pairs, _) = bytes.as_chunks::<2>();
    let units: Vec<u16> = pairs.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
    String::from_utf16_lossy(&units)
        .lines()
        .map(|line| line.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}' || c == '\0').to_owned())
        .filter(|name| !name.is_empty() && is_valid_distro_name(name))
        .collect()
}

/// Distribution names are letters, digits, dots, dashes and underscores.
pub fn is_valid_distro_name(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

pub fn distros() -> Vec<String> {
    wsl(&["-l", "-q"]).map(|out| parse_distro_list(&out)).unwrap_or_default()
}

/// Linux home folder of the default user of a distribution.
pub fn linux_home(distro: &str) -> Option<String> {
    let out = wsl(&["-d", distro, "--exec", "printenv", "HOME"]).ok()?;
    let home = String::from_utf8(out).ok()?.trim().to_owned();
    home.starts_with('/').then_some(home)
}

/// Windows path of a file inside a distribution: `\\wsl.localhost\Ubuntu\home\…`.
pub fn unc_path(distro: &str, linux_path: &str) -> PathBuf {
    let mut path = PathBuf::from(format!(r"\\wsl.localhost\{distro}\"));
    for part in linux_path.split('/').filter(|p| !p.is_empty()) {
        path.push(part);
    }
    path
}

/// The distribution and Linux path of a file seen from Windows: the reverse of
/// `unc_path`. `None` for a file outside WSL.
pub fn split_unc(path: &Path) -> Option<(String, String)> {
    let mut parts = path.components();
    let Some(Component::Prefix(prefix)) = parts.next() else { return None };
    let Prefix::UNC(server, share) = prefix.kind() else { return None };
    let distro = share.to_str().filter(|d| server.eq_ignore_ascii_case("wsl.localhost") && is_valid_distro_name(d))?;
    let mut linux = String::new();
    for part in parts {
        match part {
            Component::RootDir => {}
            Component::Normal(name) => {
                linux.push('/');
                linux.push_str(name.to_str()?);
            }
            _ => return None,
        }
    }
    (!linux.is_empty()).then(|| (distro.to_owned(), linux))
}

fn split_or_fail(path: &Path) -> io::Result<(String, String)> {
    split_unc(path).ok_or_else(|| io::Error::other(tr(Text::NotAWslFile { path: &path.display().to_string() })))
}

/// The file a symbolic link inside a distribution points to, followed by Linux: Windows
/// cannot follow it through `\\wsl.localhost`. A link to a missing file gives the file
/// it would be.
pub fn real_path(path: &Path) -> io::Result<PathBuf> {
    let (distro, linux) = split_or_fail(path)?;
    let out = wsl(&["-d", &distro, "--exec", "realpath", "-m", &linux])?;
    let real = String::from_utf8(out).map_err(io::Error::other)?;
    match real.strip_suffix('\n') {
        Some(real) if real.starts_with('/') => Ok(unc_path(&distro, real)),
        _ => Err(io::Error::other(tr(Text::UnreadableLink { path: &path.display().to_string() }))),
    }
}

/// `stat -c %a` gives the permissions in octal: `600`, `644`, `4755`.
pub fn parse_mode(out: &[u8]) -> Option<String> {
    let mode = std::str::from_utf8(out).ok()?.trim_end();
    (matches!(mode.len(), 3 | 4) && mode.bytes().all(|b| matches!(b, b'0'..=b'7'))).then(|| mode.to_owned())
}

/// Permissions of a file inside a distribution. Written through `\\wsl.localhost`, a
/// file gets the default ones (`644`): îlot copies them back on what it writes.
pub fn file_mode(path: &Path) -> io::Result<String> {
    let (distro, linux) = split_or_fail(path)?;
    let out = wsl(&["-d", &distro, "--exec", "stat", "-c", "%a", &linux])?;
    parse_mode(&out).ok_or_else(|| io::Error::other(tr(Text::UnreadableMode { path: &path.display().to_string() })))
}

/// `mode` comes from `file_mode`, so it is only octal digits.
pub fn set_mode(path: &Path, mode: &str) -> io::Result<()> {
    let (distro, linux) = split_or_fail(path)?;
    wsl(&["-d", &distro, "--exec", "chmod", mode, &linux]).map(drop)
}

/// Linux path of a Windows file as the distribution sees it (`/mnt/c/…`).
pub fn to_linux_path(distro: &str, windows_path: &Path) -> io::Result<String> {
    let path = windows_path.to_str().ok_or_else(|| io::Error::other("non UTF-8 path"))?;
    let out = wsl(&["-d", distro, "--exec", "wslpath", "-u", path])?;
    Ok(String::from_utf8(out).map_err(io::Error::other)?.trim().to_owned())
}

/// Quotes a value for `sh`: everything between single quotes, quotes themselves escaped.
pub fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Variables the relay reads to tell where the agent runs (`RelayEnv`). A Windows program
/// started from WSL only gets the variables named in `WSLENV`
/// (https://learn.microsoft.com/en-us/windows/wsl/filesystems#share-environment-variables-between-windows-and-wsl-with-wslenv).
pub const RELAY_VARIABLES: &str = "WT_SESSION:TERM_PROGRAM:CURSOR_VERSION";

/// The script the agents' hooks call inside WSL: it hands its arguments and stdin to the
/// Windows relay through WSL interop, with the few variables the relay reads (fixed
/// names; their values are checked by the relay and never run).
pub fn relay_script(relay_linux_path: &str) -> String {
    format!(
        "#!/bin/sh\n{SCRIPT_MARKER}: forwards agent hook events to îlot on Windows.\n# Removed when every AI is disconnected in îlot's settings.\nexport WSLENV=\"${{WSLENV:+$WSLENV:}}{RELAY_VARIABLES}\"\nexec {} \"$@\"\n",
        sh_quote(relay_linux_path)
    )
}

/// The `ilot` command inside WSL. `ilot emit` goes straight to `ilot-cli.exe`. `ilot run`
/// runs the command here, in Linux, as given (`"$@"`, no `eval`), and only tells îlot
/// before and after; its exit code is the command's. The command runs even when
/// `ilot-cli.exe` cannot start (WSL interop off, îlot removed from Windows); a usage
/// error (no `--source`, no command) is left to `ilot-cli.exe` to explain. Ctrl+C stops
/// the command, and the script still reports how it ended.
pub fn cli_script(cli_linux_path: &str) -> String {
    format!(
        r#"#!/bin/sh
{marker}: "ilot emit" and "ilot run" tell îlot on Windows what a tool does.
# Removed when "Autre" is disconnected in îlot's settings.
cli={cli}
[ "$1" = run ] || exec "$cli" "$@"
shift
name=
project=
while [ $# -gt 0 ]; do
  case $1 in
    --) shift; break ;;
    --source=*) name=${{1#--source=}}; shift ;;
    --project=*) project=${{1#--project=}}; shift ;;
    --source|--project)
      [ $# -ge 2 ] || exec "$cli" run "$1"
      if [ "$1" = --source ]; then name=$2; else project=$2; fi
      shift 2 ;;
    -*) exec "$cli" run "$1" ;;
    *) break ;;
  esac
done
[ -n "$name" ] && [ $# -gt 0 ] || exec "$cli" run --source "$name" -- "$@"
[ -n "$project" ] || project=${{PWD##*/}}
"$cli" run-begin --source "$name" --project "$project" -- "$@" || true
trap : INT
"$@"
code=$?
"$cli" run-end --source "$name" --project "$project" --code "$code"
exit "$code"
"#,
        marker = SCRIPT_MARKER,
        cli = sh_quote(cli_linux_path)
    )
}

/// Whether the file at a script's place is îlot's own (it holds `SCRIPT_MARKER`); `None`
/// when there is no file. Anything else (the user's own script, a link Windows cannot
/// read) is never overwritten nor removed.
fn script_is_ours(path: &Path) -> io::Result<Option<bool>> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
        Ok(_) => Ok(Some(std::fs::read_to_string(path).is_ok_and(|script| script.contains(SCRIPT_MARKER)))),
    }
}

/// Writes a script into the distribution and makes it executable. Returns its Linux path.
fn install_script(distro: &str, home: &str, relative: &str, content: &str) -> io::Result<String> {
    let script_path = format!("{home}/{relative}");
    let file = unc_path(distro, &script_path);
    if script_is_ours(&file)? == Some(false) {
        return Err(io::Error::other(tr(Text::NotWrittenByIlot { path: &format!("~/{relative}") })));
    }
    write_atomic(&file, content.as_bytes())?;
    wsl(&["-d", distro, "--exec", "chmod", "755", &script_path])?;
    Ok(script_path)
}

/// Installs the relay script; returns its Linux path, the command the agents' hooks call.
pub fn install_relay_script(distro: &str, home: &str, relay_exe: &Path) -> io::Result<String> {
    install_script(distro, home, RELAY_SCRIPT, &relay_script(&to_linux_path(distro, relay_exe)?))
}

/// Installs `~/.local/bin/ilot`, which calls `ilot-cli.exe`.
pub fn install_cli_script(distro: &str, home: &str, cli_exe: &Path) -> io::Result<String> {
    install_script(distro, home, CLI_SCRIPT, &cli_script(&to_linux_path(distro, cli_exe)?))
}

/// Removes one of îlot's scripts; a script already gone, or one îlot did not write, is
/// left as it is.
pub fn remove_script(distro: &str, home: &str, relative: &str) -> io::Result<()> {
    let file = unc_path(distro, &format!("{home}/{relative}"));
    if script_is_ours(&file)? != Some(true) {
        return Ok(());
    }
    match std::fs::remove_file(file) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    #[test]
    fn parses_the_utf16_distribution_list() {
        let list = utf16("Ubuntu\r\ndocker-desktop\r\n\r\nDebian-12\r\n");
        assert_eq!(parse_distro_list(&list), vec!["Ubuntu", "docker-desktop", "Debian-12"]);
    }

    #[test]
    fn rejects_names_that_could_be_options_or_paths() {
        assert!(is_valid_distro_name("Ubuntu-24.04"));
        assert!(!is_valid_distro_name("Ubuntu; rm -rf ~"));
        assert!(!is_valid_distro_name(r"..\..\Windows"));
        assert_eq!(parse_distro_list(&utf16("bad name\r\nUbuntu\r\n")), vec!["Ubuntu"]);
    }

    #[test]
    fn builds_unc_paths() {
        assert_eq!(
            unc_path("Ubuntu", "/home/hic/.claude/settings.json"),
            PathBuf::from(r"\\wsl.localhost\Ubuntu\home\hic\.claude\settings.json")
        );
    }

    #[test]
    fn unc_paths_split_back_into_distribution_and_linux_path() {
        let linux = "/home/hic/.claude/settings.json";
        assert_eq!(split_unc(&unc_path("Ubuntu-24.04", linux)), Some(("Ubuntu-24.04".to_owned(), linux.to_owned())));
        assert_eq!(split_unc(Path::new(r"\\WSL.LOCALHOST\Debian\home\o'neil\a b")), Some(("Debian".into(), "/home/o'neil/a b".into())));
        assert_eq!(split_unc(Path::new(r"C:\Users\HP 240\.claude\settings.json")), None);
        assert_eq!(split_unc(Path::new(r"\\server\Ubuntu\home\hic")), None);
        assert_eq!(split_unc(Path::new(r"\\wsl.localhost\bad name\home")), None);
        assert_eq!(split_unc(Path::new(r"\\wsl.localhost\Ubuntu\home\..\etc\passwd")), None);
    }

    #[test]
    fn modes_are_only_octal_digits() {
        assert_eq!(parse_mode(b"600\n"), Some("600".into()));
        assert_eq!(parse_mode(b"4755\n"), Some("4755".into()));
        for bad in [&b""[..], b"\n", b"99\n", b"-R 777\n", b"644 \n600\n", b"stat: cannot stat\n", b"77777\n"] {
            assert_eq!(parse_mode(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn programs_of_windows_come_from_system32() {
        let wsl = system_program("wsl.exe").unwrap();
        assert!(wsl.is_absolute() && wsl.exists(), "{}", wsl.display());
        assert!(wsl.parent().unwrap().file_name().unwrap().eq_ignore_ascii_case("System32"));
        assert!(system_program(r"WindowsPowerShell\v1.0\powershell.exe").unwrap().exists());
    }

    #[test]
    fn only_scripts_with_the_marker_are_ilot_s() {
        let dir = std::env::temp_dir().join(format!("ilot-wsl-scripts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let script = dir.join("ilot-hook");
        assert_eq!(script_is_ours(&script).unwrap(), None);
        std::fs::write(&script, "#!/bin/sh\nexec ~/bin/my-ilot-hook \"$@\"\n").unwrap();
        assert_eq!(script_is_ours(&script).unwrap(), Some(false));
        std::fs::write(&script, relay_script("/mnt/c/ilot/ilot-hook.exe")).unwrap();
        assert_eq!(script_is_ours(&script).unwrap(), Some(true));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn quotes_paths_with_spaces_and_quotes() {
        assert_eq!(sh_quote("/mnt/c/Users/HP 240/ilot-hook.exe"), "'/mnt/c/Users/HP 240/ilot-hook.exe'");
        assert_eq!(sh_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn the_script_execs_the_windows_relay() {
        let script = relay_script("/mnt/c/Users/HP 240/dev/ilot/target/debug/ilot-hook.exe");
        assert!(script.starts_with("#!/bin/sh\n"));
        assert!(script.lines().nth(1).unwrap().starts_with(SCRIPT_MARKER));
        assert!(script.ends_with("exec '/mnt/c/Users/HP 240/dev/ilot/target/debug/ilot-hook.exe' \"$@\"\n"));
        // The relay hears where the agent runs; the user's own WSLENV entries stay.
        assert!(script.contains("\nexport WSLENV=\"${WSLENV:+$WSLENV:}WT_SESSION:TERM_PROGRAM:CURSOR_VERSION\"\n"));
    }

    #[test]
    fn the_ilot_script_runs_the_command_itself_and_reports_its_end() {
        let script = cli_script("/mnt/c/Users/HP 240/ilot/ilot-cli.exe");
        assert!(script.starts_with("#!/bin/sh\n"));
        assert!(script.contains("cli='/mnt/c/Users/HP 240/ilot/ilot-cli.exe'\n"));
        assert!(script.lines().nth(1).unwrap().starts_with(SCRIPT_MARKER));
        assert!(script.contains("exec \"$cli\" \"$@\""));
        assert!(script.contains("\n\"$@\"\ncode=$?\n"));
        assert!(script.contains("project=${PWD##*/}"));
        assert!(!script.contains("eval"), "the command is never re-read by a shell");
        assert!(script.contains(" -- \"$@\" || true\n"), "the command runs even when ilot-cli.exe cannot start");
        assert!(!script.contains("|| exit"));
    }

    /// Real WSL, on a throwaway home under `ILOT_WSL_TEST_DIR` (or `/tmp`): the `ilot`
    /// script runs the command without `ilot-cli.exe`, and a `~/.local/bin/ilot-hook` of
    /// the user's own is never overwritten nor removed.
    #[test]
    #[ignore = "needs WSL; run with --ignored"]
    fn scripts_inside_wsl_on_a_throwaway_home() {
        let distro = distros().into_iter().next().expect("a WSL distribution");
        let base = std::env::var("ILOT_WSL_TEST_DIR").unwrap_or_else(|_| "/tmp".into());
        let home = format!("{base}/ilot-test-scripts-{}", std::process::id());
        let run = |args: &[&str]| {
            let mut all = vec!["-d", distro.as_str(), "--cd", "/tmp", "--exec"];
            all.extend_from_slice(args);
            Command::new(system_program("wsl.exe").unwrap()).args(&all).output().unwrap()
        };
        let cli = install_script(&distro, &home, CLI_SCRIPT, &cli_script("/nonexistent/ilot-cli.exe")).unwrap();
        let out = run(&[cli.as_str(), "run", "--source", "test", "--", "sh", "-c", "echo ran; exit 3"]);
        assert_eq!(String::from_utf8_lossy(&out.stdout), "ran\n");
        assert_eq!(out.status.code(), Some(3));
        assert_eq!(run(&[cli.as_str(), "run", "--source", "test"]).status.code(), Some(127), "a usage error does not run anything");

        let own = "#!/bin/sh\necho mine\n";
        let relay = unc_path(&distro, &format!("{home}/{RELAY_SCRIPT}"));
        std::fs::write(&relay, own).unwrap();
        assert!(install_script(&distro, &home, RELAY_SCRIPT, &relay_script("/x/ilot-hook.exe")).is_err());
        remove_script(&distro, &home, RELAY_SCRIPT).unwrap();
        assert_eq!(std::fs::read_to_string(&relay).unwrap(), own);
        std::fs::remove_file(&relay).unwrap();
        install_script(&distro, &home, RELAY_SCRIPT, &relay_script("/x/ilot-hook.exe")).unwrap();
        remove_script(&distro, &home, RELAY_SCRIPT).unwrap();
        assert!(!relay.exists());
        std::fs::remove_dir_all(unc_path(&distro, &home)).unwrap();
    }

    #[test]
    #[ignore = "needs WSL; run with --ignored"]
    fn lists_real_distributions_and_homes() {
        let distros = distros();
        println!("{distros:?}");
        for distro in distros {
            println!("{distro}: {:?}", linux_home(&distro));
        }
    }
}
