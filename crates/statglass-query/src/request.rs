//! What the CLI asks. One request per connection.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::API_VERSION;

/// One request: `{"api": 1, "op": "history", ...}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct QueryRequest {
    pub api: u16,
    #[serde(flatten)]
    pub op: Op,
}

impl QueryRequest {
    /// A request at this build's [`API_VERSION`].
    pub fn new(op: Op) -> Self {
        Self {
            api: API_VERSION,
            op,
        }
    }
}

/// The operation. `host` is a display name, a host id or `"local"`, matched
/// case-insensitively by the app; `None` means `local`. Times are millisecond epochs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// App version, api, timezone and socket path, for `statglass doctor`.
    Status,
    /// Every host the app knows.
    Hosts,
    /// The latest frame of one host, or of every host with `all`.
    Now {
        #[serde(default)]
        host: Option<String>,
        #[serde(default)]
        all: bool,
    },
    /// The catalog, optionally one module's, plus the series `host` has rows for.
    Metrics {
        #[serde(default)]
        host: Option<String>,
        #[serde(default)]
        module: Option<String>,
    },
    /// Summaries, gaps and points for series in catalog text form. A bare labelled
    /// metric selects every series of that metric.
    History {
        #[serde(default)]
        host: Option<String>,
        series: Vec<String>,
        from_ms: i64,
        to_ms: i64,
        /// Points per series; the app applies its default and cap.
        #[serde(default)]
        points: Option<u32>,
    },
    /// Detector and alert events in a range, optionally of one kind.
    Events {
        #[serde(default)]
        host: Option<String>,
        from_ms: i64,
        to_ms: i64,
        #[serde(default)]
        kind: Option<String>,
    },
    /// The apps and processes that used the most over a range.
    Top {
        #[serde(default)]
        host: Option<String>,
        from_ms: i64,
        to_ms: i64,
        by: UsageBy,
        limit: u32,
    },
    /// An operation this build does not know, sent by a newer CLI. The app answers it
    /// with `Unsupported`; never sent by this build.
    #[serde(other)]
    Unknown,
}

impl Op {
    /// The `op` tag text.
    pub const fn name(&self) -> &'static str {
        match self {
            Op::Status => "status",
            Op::Hosts => "hosts",
            Op::Now { .. } => "now",
            Op::Metrics { .. } => "metrics",
            Op::History { .. } => "history",
            Op::Events { .. } => "events",
            Op::Top { .. } => "top",
            Op::Unknown => "unknown",
        }
    }
}

/// What `top` ranks by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum UsageBy {
    Cpu,
    Memory,
    Energy,
}
