//! Typed errors: a tag with a fixed message, an exit code and a retry rule, plus a closed
//! set of optional details. Messages never interpolate, so an agent can match them;
//! context goes in [`ErrorDetails`].

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// What went wrong. Serialized as PascalCase text (`"UnknownHost"`). A published tag
/// never changes meaning or exit code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ErrorTag {
    Usage,
    UnknownHost,
    UnknownMetric,
    BadRange,
    Busy,
    HostOffline,
    HostDisabled,
    PeerTooOld,
    HistoryUnavailable,
    AccessDisabled,
    /// The app is running with command-line access on but could not open its socket.
    AccessUnavailable,
    AppNotRunning,
    AppNotInstalled,
    AppOutdated,
    Unsupported,
    Interrupted,
    Defect,
    /// A tag from a newer app this build does not know. Handled like `Defect`.
    #[serde(other)]
    Unknown,
}

impl ErrorTag {
    /// The fixed message for this tag.
    pub const fn message(self) -> &'static str {
        match self {
            ErrorTag::Usage => "Invalid arguments.",
            ErrorTag::UnknownHost => "No host by that name or id.",
            ErrorTag::UnknownMetric => "No series by that name.",
            ErrorTag::BadRange => "The time range is invalid.",
            ErrorTag::Busy => "StatGlass is busy; retry shortly.",
            ErrorTag::HostOffline => "The host is offline.",
            ErrorTag::HostDisabled => "The host is disabled in StatGlass.",
            ErrorTag::PeerTooOld => "The host runs a StatGlass version that cannot answer this.",
            ErrorTag::HistoryUnavailable => "History is unavailable; only recent data is served.",
            ErrorTag::AccessDisabled => "Command-line access is turned off in StatGlass Settings.",
            ErrorTag::AccessUnavailable => "StatGlass could not open its command-line socket.",
            ErrorTag::AppNotRunning => "StatGlass is not running.",
            ErrorTag::AppNotInstalled => "StatGlass has never run for this user.",
            ErrorTag::AppOutdated => "The running StatGlass is older than this command-line tool.",
            ErrorTag::Unsupported => "StatGlass does not support this request.",
            ErrorTag::Interrupted => "Interrupted.",
            ErrorTag::Defect => "Unexpected failure; this is a bug.",
            ErrorTag::Unknown => "Unrecognized error from StatGlass.",
        }
    }

    /// The process exit code: each code means one next action (RFC "Errors and exit
    /// codes"). 1 report a bug, 2 fix the input, 4 retry, 5 the data is not available,
    /// 6 ask the human, 130 interrupted.
    pub const fn exit_code(self) -> u8 {
        match self {
            ErrorTag::Defect | ErrorTag::Unknown => 1,
            ErrorTag::Usage
            | ErrorTag::UnknownHost
            | ErrorTag::UnknownMetric
            | ErrorTag::BadRange => 2,
            ErrorTag::Busy => 4,
            ErrorTag::HostOffline
            | ErrorTag::HostDisabled
            | ErrorTag::PeerTooOld
            | ErrorTag::HistoryUnavailable => 5,
            ErrorTag::AccessDisabled
            | ErrorTag::AccessUnavailable
            | ErrorTag::AppNotRunning
            | ErrorTag::AppNotInstalled
            | ErrorTag::AppOutdated
            | ErrorTag::Unsupported => 6,
            ErrorTag::Interrupted => 130,
        }
    }

    /// Whether retrying the same request can succeed.
    pub const fn retryable(self) -> bool {
        matches!(self, ErrorTag::Busy)
    }
}

/// An error answer from the app, or one the CLI raised itself.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct QueryError {
    pub tag: ErrorTag,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<ErrorDetails>,
}

impl QueryError {
    /// An error with the tag's fixed message and no details.
    pub fn new(tag: ErrorTag) -> Self {
        Self {
            tag,
            message: tag.message().to_owned(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: ErrorDetails) -> Self {
        self.details = Some(details);
        self
    }
}

/// Context for an error: a closed set of optional fields. Fields are added, never
/// renamed; unknown fields from a newer app are ignored on decode.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ErrorDetails {
    /// Did-you-mean candidates, closest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metric: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,
    /// The argument as given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_range_days: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api: Option<u16>,
    /// One line on what to do instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// The CLI's failure output: the last stderr line under `--json`, stdout left empty.
/// `{"error": {"tag", "message", "retryable", "next"?, "details"?}}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CliErrorEnvelope {
    pub error: CliError,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CliError {
    pub tag: ErrorTag,
    pub message: String,
    pub retryable: bool,
    /// A real command to run next, or an instruction for the human.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<ErrorDetails>,
}

impl CliErrorEnvelope {
    pub fn new(error: QueryError, next: Option<String>) -> Self {
        Self {
            error: CliError {
                retryable: error.tag.retryable(),
                tag: error.tag,
                message: error.message,
                next,
                details: error.details,
            },
        }
    }
}
