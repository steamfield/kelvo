//! `top`: the apps that used the most over a range. This Mac answers from its usage ring
//! for ranges inside the last hour and from durable usage beyond (30 days at most). A
//! remote host is asked live at every range (durable usage is not mirrored, D-120), so it
//! must be connected.

use super::map::Fail;
use std::sync::Arc;

use statglass_query::{
    Answer, ErrorDetails, ErrorTag, Op, Range, TopAnswer, TopApp, TopProcess, UsageBy,
};
use statglass_schema::durable_usage::{
    HistoricalUsageMetric, HistoricalUsageReply, HistoricalUsageRequest,
};
use statglass_schema::usage::{UsageByApp, UsageKey};
use tokio::sync::OwnedSemaphorePermit;

use super::history::check_range;
use super::hosts::resolve_enabled;
use super::map::{
    err, err_with, freshness, from_command, from_remote, host_ref, last_seen_ms, rfc,
};
use super::server::{Limits, run_async, run_blocking};
use super::{Backend, HostView, RemoteAnswer, RemoteAsk};
use crate::remote::RemoteState;

const HOUR_MS: i64 = 3_600_000;
/// Durable usage's longest range (`USAGE_HISTORY_MAX_RANGE_MS`).
const MAX_DURABLE_DAYS: u32 = 30;
/// Durable replies carry at most ten app groups (D-120).
const DURABLE_APPS: u32 = 10;
/// How much later than the requested start the ring may begin before it is a clamp.
const RING_SLACK_MS: i64 = 1_000;

/// Which source answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    /// The last-hour usage ring.
    Ring,
    /// Durable usage.
    Durable,
}

struct Ask {
    from_ms: i64,
    to_ms: i64,
    by: UsageBy,
    limit: u32,
    source: Source,
}

impl Ask {
    /// The limit the source can serve, and whether it is below the one asked for.
    fn applied_limit(&self, remote: bool) -> (u32, bool) {
        let asked = self.limit.max(1);
        if (remote || self.source == Source::Durable) && asked > DURABLE_APPS {
            (DURABLE_APPS, true)
        } else {
            (asked, false)
        }
    }
}

pub(super) async fn answer(
    backend: Arc<dyn Backend>,
    slot: OwnedSemaphorePermit,
    limits: &Limits,
    op: Op,
) -> Result<Answer, Fail> {
    let Op::Top {
        host,
        from_ms,
        to_ms,
        by,
        limit,
    } = op
    else {
        return Err(err(ErrorTag::Defect));
    };
    if from_ms >= to_ms {
        return Err(super::map::bad_range(None, "from must be before to"));
    }
    // Host lookup and the clock read in-memory state only, so they run here.
    let view = resolve_enabled(backend.as_ref(), host.as_deref())?;
    let now = backend.host_now(view.record.id);
    let source = if from_ms >= now.saturating_sub(HOUR_MS) {
        Source::Ring
    } else {
        check_range(from_ms, to_ms, MAX_DURABLE_DAYS)?;
        Source::Durable
    };
    let ask = Ask {
        from_ms,
        to_ms,
        by,
        limit,
        source,
    };
    if view.remote.is_none() {
        return run_blocking(slot, limits.local_deadline, move || {
            local(backend.as_ref(), &view, &ask)
        })
        .await;
    }
    let connected = view
        .remote
        .as_ref()
        .is_some_and(|r| matches!(r.status.state, RemoteState::Connected));
    if !connected {
        return Err(err_with(
            ErrorTag::HostOffline,
            ErrorDetails {
                host: Some(view.record.display_name.clone()),
                last_seen: last_seen_ms(&view).map(rfc),
                hint: Some(
                    "per-app usage lives on the host; use `events` and `cpu.*` or `cgroup.*` history"
                        .to_owned(),
                ),
                ..Default::default()
            },
        ));
    }
    run_async(slot, limits.remote_deadline, remote(backend, view, ask)).await
}

fn local(backend: &dyn Backend, view: &HostView, ask: &Ask) -> Result<Answer, Fail> {
    let id = view.record.id;
    let (limit, reduced) = ask.applied_limit(false);
    let (range, apps) = match ask.source {
        Source::Ring => {
            let usage = backend
                .usage_ring(
                    id,
                    ask.from_ms,
                    ask.to_ms,
                    usage_key(ask.by),
                    limit as usize,
                )
                .map_err(from_command)?;
            from_ring(&usage, ask)
        }
        Source::Durable => {
            let request = durable_request(view, ask);
            request
                .validate()
                .map_err(|m| super::map::hint(ErrorTag::Usage, m))?;
            let reply = backend.durable_usage(&request).map_err(from_command)?;
            from_durable(&reply, ask.by)
        }
    };
    Ok(top_answer(view, ask, range, apps, limit, reduced))
}

async fn remote(backend: Arc<dyn Backend>, view: HostView, ask: Ask) -> Result<Answer, Fail> {
    let (limit, reduced) = ask.applied_limit(true);
    let question = match ask.source {
        Source::Ring => RemoteAsk::Ring {
            from_ms: ask.from_ms,
            to_ms: ask.to_ms,
            by: usage_key(ask.by),
            limit,
        },
        Source::Durable => RemoteAsk::Durable(durable_request(&view, &ask)),
    };
    let reply = backend
        .remote_usage(view.record.id, question)
        .await
        .map_err(|e| from_remote(&e, &view.record.display_name, last_seen_ms(&view).map(rfc)))?;
    let (range, apps) = match reply {
        RemoteAnswer::Ring(usage) => from_ring(&usage, &ask),
        RemoteAnswer::Durable(reply) => from_durable(&reply, ask.by),
    };
    Ok(top_answer(&view, &ask, range, apps, limit, reduced))
}

fn top_answer(
    view: &HostView,
    ask: &Ask,
    range: Range,
    mut apps: Vec<TopApp>,
    limit: u32,
    limit_reduced: bool,
) -> Answer {
    apps.sort_by(|a, b| {
        b.value
            .total_cmp(&a.value)
            .then_with(|| a.name.cmp(&b.name))
    });
    apps.truncate(limit as usize);
    Answer::Top(TopAnswer {
        host: host_ref(&view.record),
        freshness: freshness(view, None),
        range,
        by: ask.by,
        limit,
        limit_reduced,
        apps,
    })
}

fn durable_request(view: &HostView, ask: &Ask) -> HistoricalUsageRequest {
    HistoricalUsageRequest {
        host: view.record.id,
        from_ms: ask.from_ms,
        to_ms: ask.to_ms,
        by: match ask.by {
            UsageBy::Cpu => HistoricalUsageMetric::Cpu,
            UsageBy::Memory => HistoricalUsageMetric::Memory,
            UsageBy::Energy => HistoricalUsageMetric::Energy,
        },
        app_names: Vec::new(),
    }
}

/// The key `query_usage_by_app` sorts by.
fn usage_key(by: UsageBy) -> UsageKey {
    match by {
        UsageBy::Cpu => UsageKey::Cpu,
        UsageBy::Memory => UsageKey::Memory,
        UsageBy::Energy => UsageKey::Energy,
    }
}

/// The unit of a `top` value: average percent of one core, peak footprint, joules.
fn unit(by: UsageBy) -> &'static str {
    match by {
        UsageBy::Cpu => "percent",
        UsageBy::Memory => "bytes",
        UsageBy::Energy => "joules",
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "byte counts below 2^53 are exact in f64"
)]
fn from_ring(usage: &UsageByApp, ask: &Ask) -> (Range, Vec<TopApp>) {
    let by = ask.by;
    let total = match by {
        UsageBy::Cpu => Some(usage.total.cpu_avg_pct),
        UsageBy::Energy => Some(usage.total.energy_j),
        UsageBy::Memory => None,
    }
    .filter(|t| *t > 0.0);
    let apps = usage
        .apps
        .iter()
        .map(|app| {
            let value = match by {
                UsageBy::Cpu => app.cpu_avg_pct,
                UsageBy::Memory => app.mem_peak_b as f64,
                UsageBy::Energy => app.energy_j,
            };
            TopApp {
                name: app.name.clone(),
                value,
                unit: unit(by).to_owned(),
                share: total.map(|t| (value / t).clamp(0.0, 1.0)),
                processes: app
                    .processes
                    .iter()
                    .map(|p| TopProcess {
                        name: p.name.clone(),
                        pid: u32::try_from(p.pid).ok(),
                        value: match by {
                            UsageBy::Cpu => p.cpu_avg_pct,
                            UsageBy::Memory => p.mem_peak_b as f64,
                            UsageBy::Energy => p.energy_j,
                        },
                    })
                    .collect(),
            }
        })
        .collect();
    let range = Range {
        from: rfc(usage.from_ms),
        to: rfc(usage.to_ms),
        // The ring widens the request to whole buckets; only a start more than a second
        // after the one asked for is a clamp.
        retained_from: usage
            .since_ms
            .filter(|&since| since > ask.from_ms.saturating_add(RING_SLACK_MS))
            .map(rfc),
    };
    (range, apps)
}

/// Durable app groups. A group whose value is unavailable is left out (never zero);
/// durable replies have no per-process breakdown without a further query, and no total.
#[expect(
    clippy::cast_precision_loss,
    reason = "byte counts below 2^53 are exact in f64"
)]
fn from_durable(reply: &HistoricalUsageReply, by: UsageBy) -> (Range, Vec<TopApp>) {
    let apps = reply
        .apps
        .iter()
        .filter_map(|app| {
            let f = &app.figures;
            let value = match by {
                UsageBy::Cpu => f.cpu_avg_pct,
                UsageBy::Memory => f.mem_peak_bytes.map(|b| b as f64),
                UsageBy::Energy => f.energy_j,
            }?;
            Some(TopApp {
                name: app.name.clone(),
                value,
                unit: unit(by).to_owned(),
                share: None,
                processes: Vec::new(),
            })
        })
        .collect();
    let r = &reply.range;
    // Spans are whole buckets, so a first span within one bucket of the request is not a
    // clamp.
    let first_span = r.spans.iter().min_by_key(|s| s.from_ms);
    let late = first_span.filter(|s| {
        let slack = i64::try_from(s.resolution_ms).unwrap_or(i64::MAX);
        s.from_ms > r.requested_from_ms.saturating_add(slack)
    });
    let range = Range {
        from: rfc(r.requested_from_ms),
        to: rfc(r.requested_to_ms),
        retained_from: late.map(|s| rfc(s.from_ms)),
    };
    (range, apps)
}
