//! "Allow command-line access" (D-126): the patch for `Settings::cli_access`.
use super::set;
use serde::Deserialize;
use statglass_schema::settings::CliAccessSettings;

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, specta::Type)]
#[serde(default)]
pub struct CliAccessPatch {
    #[specta(optional)]
    pub enabled: Option<bool>,
}

/// Applies `patch`, when there is one, to `s`.
pub(super) fn apply(patch: Option<CliAccessPatch>, s: &mut CliAccessSettings) {
    if let Some(p) = patch {
        set(&mut s.enabled, p.enabled);
    }
}
