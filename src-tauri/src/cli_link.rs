//! "Install command-line tool" (D-126): the `~/.local/bin/statglass` link to the CLI
//! inside this app's bundle, its status for Settings, and its repair at launch when the
//! app has moved. The link points into the bundle, so updates keep it current.

use std::io;
use std::path::{Path, PathBuf};

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
    /// A link that points somewhere else; installing re-points it.
    Stale,
    /// Something other than a link is there; it is never replaced.
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, specta::Type)]
pub struct CliLinkStatus {
    /// `~/.local/bin/statglass`.
    pub link_path: String,
    /// The bundled CLI the link should point at; `null` without a bundle.
    pub target: Option<String>,
    pub state: CliLinkState,
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

/// The link's state in `dir` against `target`.
fn status(dir: &Path, target: Option<&Path>, path_env: Option<&std::ffi::OsStr>) -> CliLinkStatus {
    let link = dir.join(LINK_NAME);
    let state = match (std::fs::symlink_metadata(&link), target) {
        (Err(_), _) => CliLinkState::Missing,
        (Ok(meta), _) if !meta.file_type().is_symlink() => CliLinkState::Blocked,
        (Ok(_), None) => CliLinkState::Missing,
        (Ok(_), Some(target)) => match std::fs::read_link(&link) {
            Ok(points_to) if points_to == target => CliLinkState::Current,
            _ => CliLinkState::Stale,
        },
    };
    CliLinkStatus {
        link_path: link.display().to_string(),
        target: target.map(|t| t.display().to_string()),
        state,
        on_path: path_env.is_some_and(|p| std::env::split_paths(p).any(|d| d == dir)),
    }
}

/// Creates or re-points the link in `dir` (created when missing) at `target`, through a
/// temporary link renamed over it, so the name is never absent. Never replaces a file.
fn install(dir: &Path, target: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let link = dir.join(LINK_NAME);
    if let Ok(meta) = std::fs::symlink_metadata(&link)
        && !meta.file_type().is_symlink()
    {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} exists and is not a link", link.display()),
        ));
    }
    let tmp = dir.join(format!(".{LINK_NAME}.{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    std::os::unix::fs::symlink(target, &tmp)?;
    std::fs::rename(&tmp, &link).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Whether the link in `dir` is one this app made for another copy of itself: a link to
/// `<X>.app/Contents/MacOS/statglass-cli` other than `target`.
fn needs_repair(dir: &Path, target: &Path) -> bool {
    let Ok(points_to) = std::fs::read_link(dir.join(LINK_NAME)) else {
        return false;
    };
    let ours = points_to.file_name().is_some_and(|n| n == CLI_NAME)
        && points_to
            .parent()
            .filter(|m| m.file_name().is_some_and(|n| n == "MacOS"))
            .and_then(Path::parent)
            .filter(|c| c.file_name().is_some_and(|n| n == "Contents"))
            .and_then(Path::parent)
            .is_some_and(|b| b.extension().is_some_and(|e| e == "app"));
    ours && points_to != target
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

/// Creates or re-points the link. A file that is not a link is left alone and reported
/// as `blocked`; `invalid_argument` in a build without a bundled CLI.
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
    install(&dir, &target).map_err(|e| CommandError::Internal {
        message: format!("Could not link the command-line tool: {e}"),
    })?;
    Ok(status(&dir, Some(&target), path_env.as_deref()))
}

#[cfg(test)]
mod tests {
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
    fn installs_repoints_and_never_replaces_a_file() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("home/.local/bin");
        let old = bundle(tmp.path(), "Old/StatGlass.app");
        let new = bundle(tmp.path(), "StatGlass.app");
        let path_env = std::env::join_paths([dir.clone()]).unwrap();

        let s = status(&dir, Some(&new), Some(&path_env));
        assert_eq!((s.state, s.on_path), (CliLinkState::Missing, true));
        install(&dir, &old).unwrap();
        assert_eq!(status(&dir, Some(&new), None).state, CliLinkState::Stale);
        assert!(
            needs_repair(&dir, &new),
            "a link to another bundle's CLI is ours"
        );
        install(&dir, &new).unwrap();
        assert_eq!(status(&dir, Some(&new), None).state, CliLinkState::Current);
        assert!(!needs_repair(&dir, &new));
        // Without a bundle there is nothing to compare against.
        assert_eq!(status(&dir, None, None).state, CliLinkState::Missing);

        std::fs::remove_file(dir.join(LINK_NAME)).unwrap();
        std::os::unix::fs::symlink("/usr/local/bin/other", dir.join(LINK_NAME)).unwrap();
        assert!(
            !needs_repair(&dir, &new),
            "a link we did not make is left alone"
        );

        std::fs::remove_file(dir.join(LINK_NAME)).unwrap();
        std::fs::write(dir.join(LINK_NAME), "#!/bin/sh\n").unwrap();
        assert_eq!(status(&dir, Some(&new), None).state, CliLinkState::Blocked);
        assert!(install(&dir, &new).is_err());
        assert_eq!(
            std::fs::read_to_string(dir.join(LINK_NAME)).unwrap(),
            "#!/bin/sh\n"
        );
    }
}
