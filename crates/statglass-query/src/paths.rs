//! Where the running app's sockets are.
//!
//! The app data directory is `$HOME/Library/Application Support/<bundle id>`. The bundle
//! id comes from, in order: `STATGLASS_APP_ID`; the `CFBundleIdentifier` of the app
//! bundle this executable sits in (so a dev build's CLI finds the dev app); otherwise
//! [`DEFAULT_BUNDLE_ID`]. `STATGLASS_APP_DATA` replaces the whole directory (tests).

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

/// The release app's bundle id.
pub const DEFAULT_BUNDLE_ID: &str = "com.tryopendata.statglass";

const ENV_APP_ID: &str = "STATGLASS_APP_ID";
const ENV_APP_DATA: &str = "STATGLASS_APP_DATA";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppPaths {
    app_data: PathBuf,
}

impl AppPaths {
    pub fn new(app_data: impl Into<PathBuf>) -> Self {
        Self {
            app_data: app_data.into(),
        }
    }

    /// The app data directory for `bundle_id` under `home`.
    pub fn for_bundle(home: &Path, bundle_id: &str) -> Self {
        Self::new(home.join("Library/Application Support").join(bundle_id))
    }

    /// Resolves the paths from this process's environment and executable. `None` when
    /// neither `STATGLASS_APP_DATA` nor `HOME` is set.
    pub fn discover() -> Option<Self> {
        let exe = std::env::current_exe().and_then(std::fs::canonicalize).ok();
        Self::resolve(|k| std::env::var_os(k), exe.as_deref())
    }

    /// [`AppPaths::discover`] over an explicit environment lookup and executable path.
    pub(crate) fn resolve(
        env: impl Fn(&str) -> Option<OsString>,
        exe: Option<&Path>,
    ) -> Option<Self> {
        if let Some(dir) = env(ENV_APP_DATA).filter(|d| !d.is_empty()) {
            return Some(Self::new(dir));
        }
        let home = env("HOME").filter(|h| !h.is_empty())?;
        let bundle_id = env(ENV_APP_ID)
            .and_then(|id| id.into_string().ok())
            .filter(|id| !id.is_empty())
            .or_else(|| exe.and_then(bundle_id_for_exe))
            .unwrap_or_else(|| DEFAULT_BUNDLE_ID.to_owned());
        Some(Self::for_bundle(Path::new(&home), &bundle_id))
    }

    pub fn app_data(&self) -> &Path {
        &self.app_data
    }

    /// `<app_data>/cli/query.sock`.
    pub fn query_socket(&self) -> PathBuf {
        self.app_data.join("cli").join("query.sock")
    }

    /// `<app_data>/cli/access-disabled`: present while command-line access is turned
    /// off, so the CLI can tell that from an app that predates the query socket.
    pub fn access_disabled_marker(&self) -> PathBuf {
        self.app_data.join("cli").join("access-disabled")
    }

    /// `<app_data>/cli/bind-failed`: present while access is on but the query socket
    /// cannot be bound, holding the error, so the CLI can say why instead of guessing.
    pub fn bind_failed_marker(&self) -> PathBuf {
        self.app_data.join("cli").join("bind-failed")
    }

    /// `<app_data>/remote/control.sock`, the remote helper's socket. The CLI only checks
    /// whether it answers, to tell "access disabled" from "not running".
    pub fn control_socket(&self) -> PathBuf {
        self.app_data.join("remote").join("control.sock")
    }
}

/// The `CFBundleIdentifier` of the bundle `exe` lives in, when `exe` is
/// `<Name>.app/Contents/MacOS/<binary>`.
fn bundle_id_for_exe(exe: &Path) -> Option<String> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    if macos.file_name()? != "MacOS" || contents.file_name()? != "Contents" {
        return None;
    }
    let plist = std::fs::read_to_string(contents.join("Info.plist")).ok()?;
    plist_string(&plist, "CFBundleIdentifier")
}

/// The `<string>` after `<key>name</key>` in an XML plist. A string search is enough for
/// the one key the bundle's own Info.plist carries; no plist parser is linked.
fn plist_string(xml: &str, name: &str) -> Option<String> {
    let key = format!("<key>{name}</key>");
    let after = &xml[xml.find(&key)? + key.len()..];
    let open = after.trim_start().strip_prefix("<string>")?;
    let value = open[..open.find("</string>")?].trim();
    (!value.is_empty()).then(|| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let map: HashMap<String, OsString> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), OsString::from(v)))
            .collect();
        move |k| map.get(k).cloned()
    }

    const PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
  <key>CFBundleName</key>
  <string>StatGlass Dev</string>
  <key>CFBundleIdentifier</key>
  <string>com.tryopendata.statglass.dev</string>
</dict></plist>"#;

    fn bundle(dir: &Path) -> PathBuf {
        let macos = dir.join("StatGlass Dev.app/Contents/MacOS");
        std::fs::create_dir_all(&macos).unwrap();
        std::fs::write(macos.parent().unwrap().join("Info.plist"), PLIST).unwrap();
        macos.join("statglass-cli")
    }

    #[test]
    fn app_data_env_wins() {
        let p =
            AppPaths::resolve(env(&[("STATGLASS_APP_DATA", "/x"), ("HOME", "/h")]), None).unwrap();
        assert_eq!(p.query_socket(), Path::new("/x/cli/query.sock"));
        assert_eq!(p.control_socket(), Path::new("/x/remote/control.sock"));
    }

    #[test]
    fn bundle_id_order_is_env_then_bundle_then_default() {
        let tmp = tempfile::tempdir().unwrap();
        let exe = bundle(tmp.path());
        let support = Path::new("/h/Library/Application Support");
        let p = AppPaths::resolve(
            env(&[("HOME", "/h"), ("STATGLASS_APP_ID", "a.b")]),
            Some(&exe),
        )
        .unwrap();
        assert_eq!(p.app_data(), support.join("a.b"));
        let p = AppPaths::resolve(env(&[("HOME", "/h")]), Some(&exe)).unwrap();
        assert_eq!(p.app_data(), support.join("com.tryopendata.statglass.dev"));
        let p = AppPaths::resolve(
            env(&[("HOME", "/h")]),
            Some(Path::new("/usr/local/bin/statglass")),
        )
        .unwrap();
        assert_eq!(p.app_data(), support.join(DEFAULT_BUNDLE_ID));
        assert!(AppPaths::resolve(env(&[]), None).is_none());
    }

    #[test]
    fn plist_lookup_needs_the_key_and_a_string() {
        assert_eq!(
            plist_string(PLIST, "CFBundleName").as_deref(),
            Some("StatGlass Dev")
        );
        assert_eq!(plist_string(PLIST, "Missing"), None);
        assert_eq!(
            plist_string("<key>CFBundleIdentifier</key><true/>", "CFBundleIdentifier"),
            None
        );
    }
}
