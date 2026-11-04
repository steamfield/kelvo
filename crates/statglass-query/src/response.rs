//! What the app answers: `{"api", "app_version", "ok": {...}}` or `{..., "error": {...}}`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    EventsAnswer, HistoryAnswer, HostsAnswer, MetricsAnswer, NowAnswer, QueryError, StatusAnswer,
    TopAnswer,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct QueryResponse {
    pub api: u16,
    pub app_version: String,
    #[serde(flatten)]
    pub body: Body,
}

/// Exactly one of `ok` or `error`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Body {
    Ok(Answer),
    Error(QueryError),
}

/// One answer per [`crate::Op`], tagged with the same `op` text so a client can tell
/// them apart.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Answer {
    Status(StatusAnswer),
    Hosts(HostsAnswer),
    Now(NowAnswer),
    Metrics(MetricsAnswer),
    History(HistoryAnswer),
    Events(EventsAnswer),
    Top(TopAnswer),
}

impl Answer {
    /// The `op` tag text, which is also this answer's [`crate::schemas`] entry name.
    pub const fn name(&self) -> &'static str {
        match self {
            Answer::Status(_) => "status",
            Answer::Hosts(_) => "hosts",
            Answer::Now(_) => "now",
            Answer::Metrics(_) => "metrics",
            Answer::History(_) => "history",
            Answer::Events(_) => "events",
            Answer::Top(_) => "top",
        }
    }
}
