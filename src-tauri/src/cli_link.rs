//! "Install command-line tool" (D-126): the `~/.local/bin/statglass` link to the CLI
//! inside this app's bundle, its status for Settings, and its repair at launch when the
//! app has moved. The link points into the bundle, so updates keep it current.

use std::ffi::OsStr;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::error::CommandError;

/// The CLI's file name inside `Contents/MacOS` (the app's own executable is `statglass`).
const CLI_NAME: &str = "statglass-cli";
/// The link's name on PATH.
const LINK_NAME: &str = "statglass";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum CliLinkState {
    /// No link, or this build has no bundled CLI to link to (a development build).
    Missing,
    /// The link points at this app's CLI.
    Current,
    /// A link to another copy of StatGlass's CLI (`<X>.app/Contents/MacOS/statglass-cli`),
    /// installed or not; installing re-points it.
    Stale,
    /// A link to anything else, presumably the user's own; replaced only on request.
    Foreign,
    /// Something other than a link is at the name, or the link's directory is not
    /// private to this user. Never replaced; `reason` says which.
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
pub struct CliLinkStatus {
    /// `~/.local/bin/statglass`.
    pub link_path: String,
    /// The bundled CLI the link should point at; `null` without a bundle.
    pub target: Option<String>,
    pub state: CliLinkState,
    /// Where the existing link points, as stored in it; `null` when there is no link.
    pub points_to: Option<String>,
    /// Why the state is `blocked`, as a sentence for Settings; `null` otherwise.
    pub reason: Option<String>,
    /// Whether the link's directory is on this app's `PATH`. An app opened from Finder
    /// sees launchd's short `PATH`, so false can mean "not in a GUI app's PATH" while a
    /// login shell has it.
    pub on_path: bool,
}

/// This app's bundled CLI, when the running executable is `<X>.app/Contents/MacOS/<exe>`
/// and the CLI is beside it.
fn bundled_cli(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let bundle = macos.parent()?.parent()?;
    let in_bundle = macos.file_name()? == "MacOS" && bundle.extension().is_some_and(|e| e == "app");
    let cli = macos.join(CLI_NAME);
    (in_bundle && cli.is_file()).then_some(cli)
}

/// Whether `path` names a StatGlass CLI: `<X>.app/Contents/MacOS/statglass-cli`.
fn is_bundled_cli_path(path: &Path) -> bool {
    path.file_name().is_some_and(|n| n == CLI_NAME)
        && path
            .parent()
            .filter(|m| m.file_name().is_some_and(|n| n == "MacOS"))
            .and_then(Path::parent)
            .filter(|c| c.file_name().is_some_and(|n| n == "Contents"))
            .and_then(Path::parent)
            .is_some_and(|b| b.extension().is_some_and(|e| e == "app"))
}

fn current_cli() -> Option<PathBuf> {
    let exe = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .ok()?;
    bundled_cli(&exe)
}

fn link_dir(app: &AppHandle) -> Result<PathBuf, CommandError> {
    let home = app.path().home_dir().map_err(|e| CommandError::Internal {
        message: format!("no home directory: {e}"),
    })?;
    Ok(home.join(".local/bin"))
}

/// Why the link may not go in `dir`, if it may not: the directory it resolves to
/// (through any symlinks) must be owned by this user and writable by no one else, or
/// someone else could swap the link for their own program.
fn unsafe_dir(dir: &Path) -> Option<String> {
    let real = match std::fs::canonicalize(dir) {
        Ok(real) => real,
        Err(e) => return Some(format!("{} can't be resolved: {e}.", dir.display())),
    };
    let meta = match std::fs::symlink_metadata(&real) {
        Ok(meta) => meta,
        Err(e) => return Some(format!("{} can't be read: {e}.", real.display())),
    };
    if !meta.is_dir() || meta.uid() != rustix::process::geteuid().as_raw() {
        return Some(format!(
            "{} is not a directory owned by you.",
            real.display()
        ));
    }
    (meta.mode() & 0o022 != 0).then(|| {
        format!(
            "{} is writable by other users. Remove their write access to install.",
            real.display()
        )
    })
}

/// The link's state in `dir` against `target`.
fn status(dir: &Path, target: Option<&Path>, path_env: Option<&OsStr>) -> CliLinkStatus {
    let link = dir.join(LINK_NAME);
    let points_to = std::fs::read_link(&link).ok();
    let dir_problem = std::fs::symlink_metadata(dir)
        .ok()
        .and_then(|_| unsafe_dir(dir));
    let (state, reason) = match (dir_problem, std::fs::symlink_metadata(&link), target) {
        (Some(reason), _, _) => (CliLinkState::Blocked, Some(reason)),
        (None, Err(_), _) => (CliLinkState::Missing, None),
        (None, Ok(meta), _) if !meta.file_type().is_symlink() => (
            CliLinkState::Blocked,
            Some(format!(
                "{} is not a link. Move it to install.",
                link.display()
            )),
        ),
        (None, Ok(_), None) => (CliLinkState::Missing, None),
        (None, Ok(_), Some(target)) => match &points_to {
            Some(p) if p == target => (CliLinkState::Current, None),
            Some(p) if is_bundled_cli_path(p) => (CliLinkState::Stale, None),
            _ => (CliLinkState::Foreign, None),
        },
    };
    CliLinkStatus {
        link_path: link.display().to_string(),
        target: target.map(|t| t.display().to_string()),
        state,
        points_to: points_to.map(|p| p.display().to_string()),
        reason,
        on_path: path_env.is_some_and(|p| std::env::split_paths(p).any(|d| d == dir)),
    }
}

/// Keeps temporary link names distinct within this process (Settings and launch repair).
static TMP_SEQ: AtomicU32 = AtomicU32::new(0);

/// Creates or re-points the link in `dir` (created when missing) at `target`, through a
/// temporary link moved into place, so the name is never absent. Refuses a directory
/// that isn't private to this user, and never replaces anything but a symlink.
fn install(dir: &Path, target: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    if let Some(reason) = unsafe_dir(dir) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, reason));
    }
    let link = dir.join(LINK_NAME);
    let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!(".{LINK_NAME}.{}.{seq}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    std::os::unix::fs::symlink(target, &tmp)?;
    swap_in(&tmp, &link)
}

/// Moves the link at `tmp` to `link`. An existing name is exchanged with it atomically
/// (`renameatx_np` with `RENAME_SWAP`) and what came out is checked afterwards: anything
/// but a symlink is swapped back, so a file created after any earlier check is never
/// overwritten. Removes `tmp`, except when it holds the user's file after a failed
/// swap back.
fn swap_in(tmp: &Path, link: &Path) -> io::Result<()> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    let cleanup = |e: io::Error| {
        let _ = std::fs::remove_file(tmp);
        e
    };
    match renameat_with(CWD, tmp, CWD, link, RenameFlags::NOREPLACE) {
        Ok(()) => return Ok(()),
        Err(rustix::io::Errno::EXIST) => {}
        Err(e) => return Err(cleanup(e.into())),
    }
    renameat_with(CWD, tmp, CWD, link, RenameFlags::EXCHANGE).map_err(|e| cleanup(e.into()))?;
    // `tmp` now holds what was at `link`.
    let was_link = std::fs::symlink_metadata(tmp)
        .ok()
        .is_none_or(|m| m.file_type().is_symlink());
    if was_link {
        let _ = std::fs::remove_file(tmp);
        return Ok(());
    }
    match renameat_with(CWD, tmp, CWD, link, RenameFlags::EXCHANGE) {
        Ok(()) => Err(cleanup(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} exists and is not a link", link.display()),
        ))),
        Err(e) => Err(io::Error::other(format!(
            "{} is not a link, and moving it back failed ({e}); it is at {}",
            link.display(),
            tmp.display()
        ))),
    }
}

/// Whether the link in `dir` is one this app made for a copy of itself that is gone: a
/// link to `<X>.app/Contents/MacOS/statglass-cli`, other than `target`, whose target no
/// longer exists. A link to another copy that is still installed is left alone, so with
/// two copies the last one launched doesn't take the link over.
fn needs_repair(dir: &Path, target: &Path) -> bool {
    let Ok(points_to) = std::fs::read_link(dir.join(LINK_NAME)) else {
        return false;
    };
    is_bundled_cli_path(&points_to) && points_to != target && !dir.join(&points_to).exists()
}

/// At launch: re-points a link this app created when the app has moved. Nothing in a
/// development build without a bundle.
pub fn repair(app: &AppHandle) {
    let (Some(target), Ok(dir)) = (current_cli(), link_dir(app)) else {
        return;
    };
    if needs_repair(&dir, &target) {
        match install(&dir, &target) {
            Ok(()) => tracing::info!("re-pointed {} at {}", LINK_NAME, target.display()),
            Err(e) => tracing::warn!("re-pointing the command-line tool link: {e}"),
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn cli_link_status(app: AppHandle) -> Result<CliLinkStatus, CommandError> {
    let dir = link_dir(&app)?;
    Ok(status(
        &dir,
        current_cli().as_deref(),
        std::env::var_os("PATH").as_deref(),
    ))
}

/// Creates or re-points the link, replacing a `stale` or `foreign` symlink (Settings
/// confirms a `foreign` one first). Anything else at the name, or a link directory that
/// isn't private, is left alone and reported as `blocked`; `invalid_argument` in a build
/// without a bundled CLI.
#[tauri::command]
#[specta::specta]
pub fn cli_install_link(app: AppHandle) -> Result<CliLinkStatus, CommandError> {
    let dir = link_dir(&app)?;
    let target = current_cli().ok_or_else(|| CommandError::InvalidArgument {
        message: "This build has no bundled command-line tool".into(),
    })?;
    let path_env = std::env::var_os("PATH");
    let before = status(&dir, Some(&target), path_env.as_deref());
    if before.state == CliLinkState::Blocked {
        return Ok(before);
    }
    let installed = install(&dir, &target);
    let after = status(&dir, Some(&target), path_env.as_deref());
    match installed {
        Ok(()) => Ok(after),
        // Refused: something appeared at the name, or the directory isn't private.
        Err(_) if after.state == CliLinkState::Blocked => Ok(after),
        Err(e) => Err(CommandError::Internal {
            message: format!("Could not link the command-line tool: {e}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::{PermissionsExt, symlink};

    use super::*;

    fn bundle(root: &Path, name: &str) -> PathBuf {
        let macos = root.join(name).join("Contents/MacOS");
        std::fs::create_dir_all(&macos).unwrap();
        std::fs::write(macos.join("statglass"), "").unwrap();
        std::fs::write(macos.join(CLI_NAME), "").unwrap();
        macos.join(CLI_NAME)
    }

    #[test]
    fn finds_the_cli_beside_a_bundled_executable_only() {
        let tmp = tempfile::tempdir().unwrap();
        let cli = bundle(tmp.path(), "StatGlass.app");
        let exe = cli.with_file_name("statglass");
        assert_eq!(bundled_cli(&exe), Some(cli));
        let loose = tmp.path().join("target/debug");
        std::fs::create_dir_all(&loose).unwrap();
        assert_eq!(bundled_cli(&loose.join("statglass")), None);
    }

    #[test]
    fn installs_repoints_and_tells_stale_from_foreign_links() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("home/.local/bin");
        let old = bundle(tmp.path(), "Old/StatGlass.app");
        let new = bundle(tmp.path(), "StatGlass.app");
        let path_env = std::env::join_paths([dir.clone()]).unwrap();

        let s = status(&dir, Some(&new), Some(&path_env));
        assert_eq!((s.state, s.on_path), (CliLinkState::Missing, true));
        install(&dir, &old).unwrap();
        let s = status(&dir, Some(&new), None);
        assert_eq!(s.state, CliLinkState::Stale);
        assert_eq!(s.points_to, Some(old.display().to_string()));
        install(&dir, &new).unwrap();
        let s = status(&dir, Some(&new), None);
        assert_eq!((s.state, s.reason), (CliLinkState::Current, None));
        // Without a bundle there is nothing to compare against.
        assert_eq!(status(&dir, None, None).state, CliLinkState::Missing);

        std::fs::remove_file(dir.join(LINK_NAME)).unwrap();
        symlink("/usr/local/bin/other", dir.join(LINK_NAME)).unwrap();
        let s = status(&dir, Some(&new), None);
        assert_eq!(s.state, CliLinkState::Foreign);
        assert_eq!(s.points_to.as_deref(), Some("/usr/local/bin/other"));
        assert!(
            !needs_repair(&dir, &new),
            "a link we did not make is left alone"
        );
        install(&dir, &new).unwrap();
        assert_eq!(status(&dir, Some(&new), None).state, CliLinkState::Current);
        let leftovers = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(leftovers, 1, "no temporary link is left behind");
    }

    #[test]
    fn never_replaces_a_file_or_directory_at_the_name() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("bin");
        let new = bundle(tmp.path(), "StatGlass.app");
        std::fs::create_dir_all(&dir).unwrap();
        let link = dir.join(LINK_NAME);

        std::fs::write(&link, "#!/bin/sh\n").unwrap();
        let s = status(&dir, Some(&new), None);
        assert_eq!(s.state, CliLinkState::Blocked);
        assert!(
            s.reason
                .unwrap()
                .ends_with("statglass is not a link. Move it to install.")
        );
        let err = install(&dir, &new).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "#!/bin/sh\n");

        std::fs::remove_file(&link).unwrap();
        std::fs::create_dir(&link).unwrap();
        std::fs::write(link.join("keep"), "x").unwrap();
        assert_eq!(status(&dir, Some(&new), None).state, CliLinkState::Blocked);
        assert!(install(&dir, &new).is_err());
        assert_eq!(std::fs::read_to_string(link.join("keep")).unwrap(), "x");
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            1,
            "the temporary link is removed"
        );
    }

    /// The swap itself is the guard, not a check before it: a regular file at the name
    /// (as if created after `install` looked) is swapped back, untouched.
    #[test]
    fn the_swap_puts_back_a_file_that_is_not_a_link() {
        let tmp = tempfile::tempdir().unwrap();
        let link = tmp.path().join(LINK_NAME);
        let staged = tmp.path().join("staged");
        std::fs::write(&link, "user's").unwrap();
        symlink("/somewhere", &staged).unwrap();
        let err = swap_in(&staged, &link).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read_to_string(&link).unwrap(), "user's");
        assert!(std::fs::symlink_metadata(&staged).is_err());
    }

    #[test]
    fn repairs_only_a_link_whose_copy_is_gone() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("bin");
        let other = bundle(tmp.path(), "Other/StatGlass.app");
        let new = bundle(tmp.path(), "StatGlass.app");
        install(&dir, &other).unwrap();
        assert!(
            !needs_repair(&dir, &new),
            "another copy that is still installed keeps the link"
        );
        std::fs::remove_dir_all(tmp.path().join("Other")).unwrap();
        assert!(
            needs_repair(&dir, &new),
            "a link to a removed copy is repaired"
        );
        install(&dir, &new).unwrap();
        assert!(!needs_repair(&dir, &new));
    }

    #[test]
    fn refuses_a_link_directory_others_can_write() {
        let tmp = tempfile::tempdir().unwrap();
        let new = bundle(tmp.path(), "StatGlass.app");
        let shared = tmp.path().join("shared");
        std::fs::create_dir(&shared).unwrap();
        std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o777)).unwrap();

        let s = status(&shared, Some(&new), None);
        assert_eq!(s.state, CliLinkState::Blocked);
        assert!(s.reason.unwrap().contains("is writable by other users"));
        let err = install(&shared, &new).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        assert!(std::fs::symlink_metadata(shared.join(LINK_NAME)).is_err());

        // Reached through a symlink, the directory it resolves to is the one checked.
        let via = tmp.path().join("home-bin");
        symlink(&shared, &via).unwrap();
        assert_eq!(status(&via, Some(&new), None).state, CliLinkState::Blocked);
        assert!(install(&via, &new).is_err());

        std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o755)).unwrap();
        install(&via, &new).unwrap();
        assert_eq!(status(&via, Some(&new), None).state, CliLinkState::Current);
    }
}
