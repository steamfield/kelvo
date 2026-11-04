//! `history`: per series, a summary over every stored bucket in the range at full tier
//! resolution, the range's explicit gaps (plus a `not_synced` gap for a mirror behind the
//! range), and optionally downsampled points.

use super::map::Fail;
use std::cmp::Ordering;
use std::collections::BTreeSet;

use statglass_query::{
    ErrorDetails, ErrorTag, GapDto, HistoryAnswer, PointDto, Range, SeriesHistory, Summary,
    suggest::closest,
};
use statglass_schema::{SeriesKey, SeriesSelector};
use statglass_store::{HistoryQuery, HistoryResult, SeriesSummary, TierChoice, series_summary};

use super::hosts::resolve_enabled;
use super::map::{
    CATALOG_INDEX, bad_range, err, err_with, freshness, from_command, gap_dto, host_ref,
    is_current, metric, metrics, rfc, synced_through, unit_of,
};
use super::{Backend, HostView};
use crate::history::History;

const DAY_MS: i64 = 86_400_000;
/// The longest range `history` and `events` accept. Retention can be shorter; a range
/// starting before it is clamped, not refused.
pub(super) const MAX_RANGE_DAYS: u32 = 90;
/// Points per series at most.
const MAX_POINTS: u32 = 1_000;
/// The finest persisted bucket, for a `max_points` large enough that no buckets merge.
const FINEST_BUCKET_MS: i64 = 10_000;

pub(super) struct Ask<'a> {
    pub host: Option<&'a str>,
    pub series: &'a [String],
    pub from_ms: i64,
    pub to_ms: i64,
    pub points: Option<u32>,
}

/// `BadRange` unless `from < to` within [`MAX_RANGE_DAYS`].
pub(super) fn check_range(from_ms: i64, to_ms: i64, max_days: u32) -> Result<(), Fail> {
    if from_ms >= to_ms {
        return Err(bad_range(None, "from must be before to"));
    }
    if to_ms.saturating_sub(from_ms) > i64::from(max_days) * DAY_MS {
        return Err(bad_range(Some(max_days), "the range is too long"));
    }
    Ok(())
}

/// The host's history, `HistoryUnavailable` when its store could not be opened (the app
/// then serves only recent rows).
pub(super) fn available_history(
    backend: &dyn Backend,
    view: &HostView,
) -> Result<std::sync::Arc<History>, Fail> {
    let history = backend.history(view.record.id).map_err(from_command)?;
    if !history.is_available() {
        return Err(err(ErrorTag::HistoryUnavailable));
    }
    Ok(history)
}

/// The start of the host's retained history, when it is after `from_ms`.
pub(super) fn retained_from(history: &History, view: &HostView, from_ms: i64) -> Option<i64> {
    let id = view.record.id;
    let span = history.read(|r| r.retained_span(id)).ok().flatten()?;
    (span.from_ms > from_ms).then_some(span.from_ms)
}

pub(super) fn answer(backend: &dyn Backend, ask: &Ask<'_>) -> Result<HistoryAnswer, Fail> {
    if ask.series.is_empty() {
        return Err(super::map::hint(
            ErrorTag::Usage,
            "name at least one series",
        ));
    }
    check_range(ask.from_ms, ask.to_ms, MAX_RANGE_DAYS)?;
    let selectors = ask
        .series
        .iter()
        .map(|s| selector(s))
        .collect::<Result<Vec<_>, _>>()?;
    let view = resolve_enabled(backend, ask.host)?;
    let history = available_history(backend, &view)?;
    let id = view.record.id;
    let retained = retained_from(&history, &view, ask.from_ms);
    let from_ms = retained.filter(|&r| r < ask.to_ms).unwrap_or(ask.from_ms);
    let span = ask.to_ms - from_ms;
    let full = read(backend, &history, &view, &selectors, from_ms, ask.to_ms, {
        u32::try_from(span / FINEST_BUCKET_MS + 2).unwrap_or(u32::MAX)
    })?;
    let now = backend.host_now(id);
    let summaries = series_summary(&full, &CATALOG_INDEX, from_ms, ask.to_ms, now);
    let keep = selected(&selectors, summaries.series.iter().map(|s| &s.key));
    let points = ask.points.unwrap_or(0).min(MAX_POINTS);
    let sampled = (points > 0)
        .then(|| {
            read(
                backend, &history, &view, &selectors, from_ms, ask.to_ms, points,
            )
        })
        .transpose()?;
    let bucket_s = u32::try_from(summaries.bucket_ms / 1_000).unwrap_or(u32::MAX);
    let window = summaries.to_ms.saturating_sub(summaries.from_ms);
    let mut series: Vec<SeriesHistory> = summaries
        .series
        .iter()
        .filter(|s| keep.contains(&s.key.to_string()))
        .map(|s| SeriesHistory {
            key: s.key.to_string(),
            unit: unit_of(s.key.metric.as_str()),
            summary: summary(s, bucket_s, window, summaries.mean_approximate),
            points: sampled
                .as_ref()
                .map_or_else(Vec::new, |r| points_of(r, &s.key)),
        })
        .collect();
    series.sort_by(|a, b| natural_cmp(&a.key, &b.key));
    let watermark = view
        .remote
        .as_ref()
        .and_then(|_| backend.sync_watermark(id));
    let mut gaps: Vec<GapDto> = full.gaps.iter().map(gap_dto).collect();
    if view.remote.is_some() && !is_current(&view) {
        let synced = watermark.as_ref().and_then(synced_through);
        if synced.is_none_or(|s| s < ask.to_ms) {
            gaps.push(GapDto {
                from: rfc(synced.map_or(from_ms, |s| s.max(from_ms))),
                to: (ask.to_ms < now).then(|| rfc(ask.to_ms)),
                reason: "not_synced".to_owned(),
                module: None,
            });
        }
    }
    Ok(HistoryAnswer {
        host: host_ref(&view.record),
        freshness: freshness(&view, watermark.as_ref()),
        range: Range {
            from: rfc(from_ms),
            to: rfc(ask.to_ms),
            retained_from: retained.map(rfc),
        },
        tier: full.tier.as_str().to_owned(),
        bucket_s,
        series,
        gaps,
    })
}

/// One read through now: stored rows plus the rows not yet committed.
fn read(
    backend: &dyn Backend,
    history: &History,
    view: &HostView,
    selectors: &[SeriesSelector],
    from_ms: i64,
    to_ms: i64,
    max_points: u32,
) -> Result<HistoryResult, Fail> {
    let id = view.record.id;
    let query = HistoryQuery {
        host: id,
        selectors: selectors.to_vec(),
        from_ms,
        to_ms,
        tier: TierChoice::Auto,
        max_points,
    };
    let recent_start = crate::commands::recent_from(from_ms);
    history
        .history_through_now(&query, || backend.recent_rows(id, recent_start, to_ms))
        .map_err(from_command)
}

/// A series argument as a selector. A bare labelled metric selects every series of it.
fn selector(input: &str) -> Result<SeriesSelector, Fail> {
    let unknown = |metric: Option<String>, hint: Option<String>| {
        let typed = metric.clone().unwrap_or_else(|| input.to_owned());
        err_with(
            ErrorTag::UnknownMetric,
            ErrorDetails {
                suggestions: closest(&typed, metrics().map(|m| m.id.as_str())),
                series: Some(input.to_owned()),
                metric,
                input: Some(input.to_owned()),
                hint,
                ..Default::default()
            },
        )
    };
    // `cgroup.cpu{kind=container,name=postgres}` unquoted is brace-expanded by the shell
    // into `cgroup.cpukind=container cgroup.cpuname=postgres`.
    let quote = (input.contains('=') && !input.contains('{'))
        .then(|| "quote series arguments: 'metric{label=value}'".to_owned());
    let key = SeriesKey::parse(input.trim()).map_err(|_| {
        let metric = input.split(['{', '=']).next().map(str::to_owned);
        unknown(metric, quote.clone())
    })?;
    let id = key.metric.as_str();
    let Some(def) = metric(id) else {
        return Err(unknown(Some(id.to_owned()), quote));
    };
    if let Some(stray) = key
        .labels
        .keys()
        .find(|k| !def.labels.iter().any(|l| l.as_str() == *k))
    {
        let hint = if def.labels.is_empty() {
            format!("{id} has no labels")
        } else {
            format!("{id} labels: {}", def.labels.join(", "))
        };
        return Err(unknown(
            Some(id.to_owned()),
            Some(format!("unknown label {stray}; {hint}")),
        ));
    }
    Ok(SeriesSelector {
        metric: key.metric,
        labels: key.labels,
    })
}

/// The keys to report: every match, except that a selector naming a labelled metric with
/// no labels keeps at most the metric's `max_cardinality`, first in natural order.
fn selected<'a>(
    selectors: &[SeriesSelector],
    keys: impl Iterator<Item = &'a SeriesKey>,
) -> BTreeSet<String> {
    let mut keys: Vec<&SeriesKey> = keys.collect();
    keys.sort_by(|a, b| natural_cmp(&a.to_string(), &b.to_string()));
    let mut out = BTreeSet::new();
    for sel in selectors {
        let cap = metric(sel.metric.as_str())
            .filter(|_| sel.labels.is_empty())
            .and_then(|m| m.max_cardinality)
            .map_or(usize::MAX, usize::from);
        out.extend(
            keys.iter()
                .filter(|k| sel.matches(k))
                .take(cap)
                .map(ToString::to_string),
        );
    }
    out
}

fn summary(s: &SeriesSummary, bucket_s: u32, window_ms: i64, approximate: bool) -> Summary {
    #[expect(
        clippy::cast_precision_loss,
        reason = "ms spans below 2^53 are exact in f64"
    )]
    let measured_fraction = if window_ms > 0 {
        (s.measured_ms as f64 / window_ms as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };
    Summary {
        min: s.min,
        max: s.max.map(|m| m.value),
        max_at: s.max.map(|m| rfc(m.t_ms)),
        max_bucket_s: s.max.map(|_| bucket_s),
        mean: s.mean,
        mean_approximate: approximate,
        last: s.last.map(|l| l.value),
        last_at: s.last.map(|l| rfc(l.t_ms)),
        measured_fraction,
    }
}

/// A series' downsampled points; a slot with any non-finite value is left out.
fn points_of(read: &HistoryResult, key: &SeriesKey) -> Vec<PointDto> {
    read.series
        .iter()
        .find(|s| &s.key == key)
        .map_or_else(Vec::new, |s| {
            s.points
                .iter()
                .filter(|p| p.min.is_finite() && p.max.is_finite() && p.avg.is_finite())
                .map(|p| PointDto {
                    t: rfc(p.t),
                    min: f64::from(p.min),
                    max: f64::from(p.max),
                    avg: f64::from(p.avg),
                })
                .collect()
        })
}

/// Text order with runs of digits compared as numbers: `core=2` before `core=10`.
pub(super) fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut a, mut b) = (a, b);
    loop {
        let (Some(ca), Some(cb)) = (a.chars().next(), b.chars().next()) else {
            return a.len().cmp(&b.len());
        };
        if ca.is_ascii_digit() && cb.is_ascii_digit() {
            let da = a.len() - a.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            let db = b.len() - b.trim_start_matches(|c: char| c.is_ascii_digit()).len();
            let (na, ra) = a.split_at(da);
            let (nb, rb) = b.split_at(db);
            let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
            let ord = ta
                .len()
                .cmp(&tb.len())
                .then_with(|| ta.cmp(tb))
                .then(da.cmp(&db));
            if ord != Ordering::Equal {
                return ord;
            }
            (a, b) = (ra, rb);
        } else {
            if ca != cb {
                return ca.cmp(&cb);
            }
            (a, b) = (&a[ca.len_utf8()..], &b[cb.len_utf8()..]);
        }
    }
}
