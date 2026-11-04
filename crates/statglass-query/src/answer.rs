//! Result DTOs, one `*Answer` per operation.
//!
//! Timestamps are RFC 3339 strings in the controller Mac's local offset
//! ([`crate::time::rfc3339_local`]). Optional fields are skipped when absent, and every
//! field added after api 1 must be an `Option` with
//! `#[serde(default, skip_serializing_if = "Option::is_none")]` so older clients ignore
//! it. Values are always finite: a value the app does not have is absent, never NaN.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use statglass_schema::MetricDef;

use crate::{QueryError, UsageBy};

/// `status`: what `statglass doctor` reports about the running app.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StatusAnswer {
    pub app_version: String,
    pub api: u16,
    /// The controller's IANA timezone, which bare times and output offsets use.
    pub timezone: String,
    /// The query socket's path.
    pub socket: String,
    pub host_count: u32,
}

/// `hosts`: every host, this Mac first, then by name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HostsAnswer {
    pub hosts: Vec<HostDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HostDto {
    pub id: String,
    pub name: String,
    pub kind: HostKind,
    pub state: HostState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync: Option<SyncState>,
    /// The mirror holds every bucket up to this time; later spans are `not_synced`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synced_through: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retained_from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retained_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chip: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HostKind {
    ThisMac,
    Mac,
    Linux,
    /// A kind this build does not know, or an OS the app could not identify.
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HostState {
    Local,
    Connected,
    Connecting,
    Offline,
    Disabled,
    Error,
    /// A state from a newer app; treat it as "other".
    #[serde(other)]
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SyncState {
    Idle,
    CatchingUp,
    Current,
    Unavailable,
    /// A sync state from a newer app; treat it as "other".
    #[serde(other)]
    Unknown,
}

/// Which host an answer is about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct HostRef {
    pub id: String,
    pub name: String,
}

/// How current a remote host's data is. Read it before concluding nothing happened.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Freshness {
    pub state: HostState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synced_through: Option<String>,
}

/// The range an answer covers, after clamping to retained history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Range {
    pub from: String,
    pub to: String,
    /// Where the host's retained data begins, when it begins more than one bucket (1 s for
    /// the usage ring) after the requested start: the range was clamped to it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retained_from: Option<String>,
}

/// `now`: one entry per host asked about.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct NowAnswer {
    pub hosts: Vec<NowHost>,
}

/// One host's latest frame. An offline host has `frame_at` and no readings; a host that
/// failed carries `error` while the answer as a whole succeeds.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct NowHost {
    pub host: HostRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freshness: Option<Freshness>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_at: Option<String>,
    /// The frame's age. Gauges sampled every 5 to 60 s can be older than the frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age_ms: Option<i64>,
    pub readings: Vec<Reading>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<QueryError>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Reading {
    /// Catalog text form, for example `cpu.load{core=3}`.
    pub series: String,
    pub value: f64,
    pub unit: String,
    pub module: String,
}

/// `metrics`: the catalog and, with a host, the series that host has rows for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MetricsAnswer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<HostRef>,
    pub metrics: Vec<MetricDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub series: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MetricDto {
    pub id: String,
    /// The catalog unit in snake_case (`percent`, `bytes_per_sec`).
    pub unit: String,
    /// `gauge`, `rate`, `mean` or `counter`.
    pub kind: String,
    /// The label keys every series of this metric carries, sorted. Empty when unlabelled.
    pub labels: Vec<String>,
    pub module: String,
    /// Whether the metric has history beyond the last hour.
    pub persisted: bool,
    pub period_s: u16,
    /// At most this many series per host; a bare metric selects up to this many.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_cardinality: Option<u16>,
}

impl MetricDto {
    /// The DTO for a catalog entry.
    pub fn from_def(def: &MetricDef) -> Self {
        Self {
            id: def.id.as_str().to_owned(),
            unit: snake_text(&def.unit),
            kind: snake_text(&def.kind),
            labels: def.label_keys.iter().map(|k| (*k).to_owned()).collect(),
            module: def.module.as_str().to_owned(),
            persisted: def.persisted,
            period_s: def.period_s,
            max_cardinality: def.max_cardinality,
        }
    }
}

/// The serialized text of a unit-variant enum (`Unit::BytesPerSec` is `bytes_per_sec`).
fn snake_text(value: &impl Serialize) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => s,
        _ => "unknown".to_owned(),
    }
}

/// `history`: per series, a summary over every stored bucket in the range, gaps, and
/// up to the requested number of points.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HistoryAnswer {
    pub host: HostRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freshness: Option<Freshness>,
    pub range: Range,
    /// The tier the summary and points come from (`live1s`, `s10`, `m1`, `m15`).
    pub tier: String,
    /// The tier's bucket width in seconds.
    pub bucket_s: u32,
    pub series: Vec<SeriesHistory>,
    /// Host-wide gaps and gaps of the requested series' modules, merged, in time order.
    pub gaps: Vec<GapDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SeriesHistory {
    pub key: String,
    pub unit: String,
    pub summary: Summary,
    pub points: Vec<PointDto>,
}

/// A series over the range. Every value is absent when nothing was measured.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Summary {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// The start of the bucket the max fell in, not the instant of the max.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_at: Option<String>,
    /// The width of that bucket.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_bucket_s: Option<u32>,
    /// Span-weighted mean over measured time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean: Option<f64>,
    /// True when the mean includes 15-minute buckets, whose values are unweighted means
    /// of minute means.
    pub mean_approximate: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_at: Option<String>,
    /// Measured time over the range, 0 to 1.
    pub measured_fraction: f64,
}

/// A span with no data. `to` is absent while the gap is open. Gaps with the same reason
/// and module that touch or overlap are merged into one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GapDto {
    pub from: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    /// The stored reason (`sleep`, `source_offline`, ...), `not_synced` for a span the
    /// mirror has not caught up on, or `no_data` for a span longer than two buckets where
    /// no requested series has a point and no stored gap says why. Treat an unfamiliar
    /// reason as a gap all the same.
    pub reason: String,
    /// Set when the gap covers one module only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
}

/// One downsampled bucket.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PointDto {
    pub t: String,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
}

/// `events`: detector and alert events in time order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EventsAnswer {
    pub host: HostRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freshness: Option<Freshness>,
    pub range: Range,
    pub events: Vec<EventDto>,
    /// Rows written by a newer build that this app could not read.
    pub unreadable: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EventDto {
    /// When the event was detected.
    pub at: String,
    /// Where the episode began, when it differs from `at`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started: Option<String>,
    /// The event kind (`fans_ramped`, `thermal_state`, `sustained_process`,
    /// `power_spike`, `alert`, or a newer one).
    pub kind: String,
    /// The kind's own fields, as the app records them.
    pub detail: serde_json::Value,
    /// Processes held responsible, most responsible first.
    pub processes: Vec<EventProcess>,
}

/// A process named by an event. Events record names only (a pid means nothing once the
/// process is gone), so `pid`, `value` and `unit` are absent today.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EventProcess {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// `top`: apps by value descending, then name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TopAnswer {
    pub host: HostRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freshness: Option<Freshness>,
    pub range: Range,
    pub by: UsageBy,
    /// The limit applied, which may be lower than the one asked for.
    pub limit: u32,
    /// True when the host could not serve the limit asked for (remote hosts serve 10).
    pub limit_reduced: bool,
    pub apps: Vec<TopApp>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TopApp {
    pub name: String,
    pub value: f64,
    pub unit: String,
    /// This app's share of the host's total over the range, 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share: Option<f64>,
    pub processes: Vec<TopProcess>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TopProcess {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    pub value: f64,
}
