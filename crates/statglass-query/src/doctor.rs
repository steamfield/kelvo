//! `statglass doctor --json`: what the CLI found about its own install and the running
//! app. Built by the CLI from local checks plus one `status` request; the app never
//! sends it. Evolves additively like the answers.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ErrorTag, StatusAnswer};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DoctorReport {
    /// This command-line tool's version.
    pub cli_version: String,
    /// The tool's resolved executable path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
    /// The bundle id the app data directory is named for. With `STATGLASS_APP_DATA` set
    /// it is just that directory's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bundle_id: Option<String>,
    pub app_data: String,
    pub app_data_exists: bool,
    pub query_socket: String,
    pub query_socket_exists: bool,
    /// The app's control socket or query socket answered.
    pub app_running: bool,
    pub access: AccessState,
    /// This Mac's IANA timezone, which bare times and output offsets use.
    pub timezone: String,
    /// The app's own status, when it answered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<StatusAnswer>,
    /// Each problem found, with its next step. Empty when everything works.
    pub problems: Vec<DoctorProblem>,
}

/// Whether the app accepts command-line queries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessState {
    On,
    Off,
    /// The app did not answer, so access could not be told; also a value from a newer
    /// build.
    #[serde(other)]
    Unknown,
}

/// A problem doctor found: the error the status request reported, and what to do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DoctorProblem {
    pub tag: ErrorTag,
    pub message: String,
    /// A real command to run next, or an instruction for the human.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    /// What the check saw, such as the socket path that did not answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_tolerates_newer_access_values() {
        let report = DoctorReport {
            cli_version: "0.4.1".into(),
            executable: None,
            bundle_id: Some("com.tryopendata.statglass".into()),
            app_data: "/a".into(),
            app_data_exists: true,
            query_socket: "/a/cli/query.sock".into(),
            query_socket_exists: false,
            app_running: true,
            access: AccessState::Off,
            timezone: "UTC".into(),
            app: None,
            problems: vec![DoctorProblem {
                tag: ErrorTag::AccessDisabled,
                message: ErrorTag::AccessDisabled.message().into(),
                next: Some("ask the user".into()),
                detail: None,
            }],
        };
        let text = serde_json::to_string(&report).unwrap();
        assert!(text.contains(r#""access":"off""#), "{text}");
        assert_eq!(serde_json::from_str::<DoctorReport>(&text).unwrap(), report);
        let newer = text.replace(r#""access":"off""#, r#""access":"partial""#);
        let back: DoctorReport = serde_json::from_str(&newer).unwrap();
        assert_eq!(back.access, AccessState::Unknown);
    }
}
