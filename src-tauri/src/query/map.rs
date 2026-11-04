//! Internal types to the CLI contract's DTOs and errors, in one place.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use statglass_query::{
    ErrorDetails, ErrorTag, Freshness, GapDto, HostKind, HostRef, HostState, MetricDto, QueryError,
    SyncState, time::rfc3339_local,
};
use statglass_remote::{RemoteError, SyncWatermark};
use statglass_schema::{CATALOG, Catalog, Gap, HostRecord, OsKind};

use super::HostView;
use crate::error::CommandError;
use crate::remote::{RemoteState, RemoteSync};

/// The built-in catalog, indexed once.
pub(super) static CATALOG_INDEX: LazyLock<Catalog> = LazyLock::new(Catalog::builtin);

/// Every catalog entry as its DTO, by metric id.
static METRICS: LazyLock<BTreeMap<&'static str, MetricDto>> = LazyLock::new(|| {
    CATALOG
        .iter()
        .map(|def| (def.id.as_str(), MetricDto::from_def(def)))
        .collect()
});

/// The catalog DTOs, sorted by id.
pub(super) fn metrics() -> impl Iterator<Item = &'static MetricDto> {
    METRICS.values()
}

pub(super) fn metric(id: &str) -> Option<&'static MetricDto> {
    METRICS.get(id)
}

/// A metric's unit text; `unknown` for a metric this build does not know.
pub(super) fn unit_of(id: &str) -> String {
    metric(id).map_or_else(|| "unknown".to_owned(), |m| m.unit.clone())
}

pub(super) fn rfc(ms: i64) -> String {
    rfc3339_local(ms)
}

/// A handler's error answer, boxed: `QueryError` carries every optional detail field.
pub(super) type Fail = Box<QueryError>;

pub(super) fn err(tag: ErrorTag) -> Fail {
    Box::new(QueryError::new(tag))
}

pub(super) fn err_with(tag: ErrorTag, details: ErrorDetails) -> Fail {
    Box::new(QueryError::new(tag).with_details(details))
}

pub(super) fn hint(tag: ErrorTag, hint: impl Into<String>) -> Fail {
    err_with(
        tag,
        ErrorDetails {
            hint: Some(hint.into()),
            ..Default::default()
        },
    )
}

pub(super) fn bad_range(max_range_days: Option<u32>, why: &str) -> Fail {
    err_with(
        ErrorTag::BadRange,
        ErrorDetails {
            max_range_days,
            hint: Some(why.to_owned()),
            ..Default::default()
        },
    )
}

/// A failure from the internals the Tauri commands share.
pub(super) fn from_command(e: CommandError) -> Fail {
    match e {
        CommandError::HistoryUnavailable { .. } => err(ErrorTag::HistoryUnavailable),
        CommandError::StoreBusy { .. } | CommandError::RemoteBusy { .. } => err(ErrorTag::Busy),
        CommandError::RemoteUnsupported { .. } | CommandError::RemoteIncompatible { .. } => {
            err(ErrorTag::PeerTooOld)
        }
        CommandError::InvalidArgument { message } => hint(ErrorTag::Usage, message),
        CommandError::UnknownHost { host } => err_with(
            ErrorTag::UnknownHost,
            ErrorDetails {
                host: Some(host.to_string()),
                ..Default::default()
            },
        ),
        other => hint(ErrorTag::Defect, other.to_string()),
    }
}

/// A failure asking a remote host. `host` is its display name.
pub(super) fn from_remote(e: &RemoteError, host: &str, last_seen: Option<String>) -> Fail {
    let tag = match e {
        RemoteError::Unsupported(_)
        | RemoteError::Incompatible(_)
        | RemoteError::IncompatiblePeer { .. } => ErrorTag::PeerTooOld,
        RemoteError::Busy(_) => ErrorTag::Busy,
        RemoteError::HistoryUnavailable(_) => ErrorTag::HistoryUnavailable,
        RemoteError::InvalidArgument(_) => ErrorTag::Usage,
        RemoteError::Transport(_)
        | RemoteError::Io(_)
        | RemoteError::AgentNotRunning(_)
        | RemoteError::Authentication(_)
        | RemoteError::HostKey(_)
        | RemoteError::Identity(_)
        | RemoteError::IdentityMismatch { .. } => ErrorTag::HostOffline,
        RemoteError::Codec(_) | RemoteError::Protocol(_) | RemoteError::Refused(_) => {
            ErrorTag::Defect
        }
    };
    let offline = tag == ErrorTag::HostOffline;
    err_with(
        tag,
        ErrorDetails {
            host: Some(host.to_owned()),
            last_seen: last_seen.filter(|_| offline),
            hint: Some(e.to_string()),
            ..Default::default()
        },
    )
}

pub(super) fn host_ref(record: &HostRecord) -> HostRef {
    HostRef {
        id: record.id.to_string(),
        name: record.display_name.clone(),
    }
}

pub(super) fn host_kind(record: &HostRecord) -> HostKind {
    match (record.is_local, record.info.os) {
        (true, _) => HostKind::ThisMac,
        (false, OsKind::MacOs) => HostKind::Mac,
        (false, OsKind::Linux) => HostKind::Linux,
        (false, OsKind::Unknown) => HostKind::Unknown,
    }
}

pub(super) fn host_state(view: &HostView) -> HostState {
    let Some(remote) = &view.remote else {
        return HostState::Local;
    };
    if !remote.enabled {
        return HostState::Disabled;
    }
    match remote.status.state {
        RemoteState::Connecting => HostState::Connecting,
        RemoteState::Connected => HostState::Connected,
        RemoteState::Offline => HostState::Offline,
        RemoteState::Disabled => HostState::Disabled,
        RemoteState::Error => HostState::Error,
    }
}

pub(super) fn sync_state(sync: &RemoteSync) -> SyncState {
    match sync {
        RemoteSync::Idle => SyncState::Idle,
        RemoteSync::CatchingUp => SyncState::CatchingUp,
        RemoteSync::Current => SyncState::Current,
        RemoteSync::Unavailable => SyncState::Unavailable,
    }
}

/// Whether a remote host's mirror is as current as the session can make it: connected
/// and caught up, so the in-memory recent rows cover what the store has not committed.
pub(super) fn is_current(view: &HostView) -> bool {
    view.remote.as_ref().is_none_or(|r| {
        r.enabled
            && matches!(r.status.state, RemoteState::Connected)
            && matches!(r.status.sync, RemoteSync::Current)
    })
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "a status time is a ms epoch below 2^53, carried as f64 for the webview"
)]
pub(super) fn last_seen_ms(view: &HostView) -> Option<i64> {
    let ms = view.remote.as_ref()?.status.last_seen_ms?;
    ms.is_finite().then_some(ms as i64)
}

/// How far a mirror is complete through. The 10 s tier normally leads; when it is behind
/// the minute tier (an S10 truncation can leave a hole there with no gap, and pruning
/// during a long offline spell can empty or lower it), the minute tier's watermark is
/// the truer one. The 15-minute tier counts only when neither finer tier holds rows.
pub(super) fn synced_through(w: &SyncWatermark) -> Option<i64> {
    match (w.s10_ms, w.m1_ms) {
        (Some(s10), Some(m1)) => Some(s10.max(m1)),
        (s10, m1) => s10.or(m1).or(w.m15_ms),
    }
}

/// `freshness` for a remote host; `None` for this Mac.
pub(super) fn freshness(view: &HostView, watermark: Option<&SyncWatermark>) -> Option<Freshness> {
    view.remote.as_ref()?;
    Some(Freshness {
        state: host_state(view),
        last_seen: last_seen_ms(view).map(rfc),
        synced_through: watermark.and_then(synced_through).map(rfc),
    })
}

pub(super) fn gap_dto(gap: &Gap) -> GapDto {
    GapDto {
        from: rfc(gap.start_ms),
        to: gap.end_ms.map(rfc),
        reason: gap.reason.as_str().to_owned(),
        module: gap.module.map(|m| m.as_str().to_owned()),
    }
}
