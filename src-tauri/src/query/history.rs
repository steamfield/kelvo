//! `history`: per series, a summary over every stored bucket in the range at full tier
//! resolution, the range's gaps (see `gaps`), and optionally downsampled points.

use super::map::Fail;
use std::collections::BTreeSet;

use statglass_query::{
    ErrorDetails, ErrorTag, HistoryAnswer, PointDto, Range, SeriesHistory, Summary,
    suggest::closest,
};
use statglass_remote::SyncWatermark;
use statglass_schema::{Module, SeriesKey, SeriesSelector};
use statglass_store::{HistoryQuery, HistoryResult, SeriesSummary, TierChoice, series_summary};

use super::gaps::{self, Span};
use super::hosts::resolve_enabled;
use super::map::{
    CATALOG_INDEX, bad_range, err, err_with, freshness, from_command, host_ref, is_current, metric,
    metrics, natural_cmp, rfc, unit_of,
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

/// The start of the host's retained history, when it is more than `slack_ms` after
/// `from_ms`. Stored history begins on a bucket boundary, so a start within one bucket of
/// the request is not a clamp.
pub(super) fn retained_from(
    history: &History,
    view: &HostView,
    from_ms: i64,
    slack_ms: i64,
) -> Option<i64> {
    let id = view.record.id;
    let span = history.read(|r| r.retained_span(id)).ok().flatten()?;
    (span.from_ms > from_ms.saturating_add(slack_ms)).then_some(span.from_ms)
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
    let now = backend.host_now(id);
    let starts = retained_from(&history, &view, ask.from_ms, 0);
    // Read from where history begins: nothing is stored before it, and a later start can
    // let the read use a finer tier.
    let read_from = starts.filter(|&r| r < ask.to_ms).unwrap_or(ask.from_ms);
    // Enough points that no buckets merge: the summary needs full resolution.
    let unmerged =
        u32::try_from((ask.to_ms - read_from) / FINEST_BUCKET_MS + 2).unwrap_or(u32::MAX);
    let full = read(
        backend, &history, &view, &selectors, read_from, ask.to_ms, unmerged,
    )?;
    let watermark = view
        .remote
        .as_ref()
        .and_then(|_| backend.sync_watermark(id));
    let mut answer = HistoryAnswer {
        host: host_ref(&view.record),
        freshness: freshness(&view, watermark.as_ref()),
        range: Range {
            from: rfc(ask.from_ms),
            to: rfc(ask.to_ms),
            retained_from: None,
        },
        tier: full.tier.as_str().to_owned(),
        bucket_s: u32::try_from(full.bucket_ms / 1_000).unwrap_or(u32::MAX),
        series: Vec::new(),
        gaps: Vec::new(),
    };
    if let Some(start) = starts.filter(|&r| r >= ask.to_ms) {
        before_retention(&mut answer, &selectors, ask, start, now);
        return Ok(answer);
    }
    // Only a start more than one bucket late is a clamp worth reporting.
    let retained = (read_from - ask.from_ms > full.bucket_ms).then_some(read_from);
    let from_ms = retained.unwrap_or(ask.from_ms);
    let summaries = series_summary(&full, &CATALOG_INDEX, from_ms, ask.to_ms, now);
    let keep = selected(&selectors, summaries.series.iter().map(|s| &s.key));
    let points = ask.points.unwrap_or(0).min(MAX_POINTS);
    // The points come from the full read, merged in memory by the store's own rule.
    let sampled = (points > 0)
        .then(|| statglass_store::downsample(&full, from_ms, ask.to_ms, points))
        .transpose()
        .map_err(|e| from_command(e.into()))?;
    let bucket_s = u32::try_from(summaries.bucket_ms / 1_000).unwrap_or(u32::MAX);
    let window = summaries.to_ms.saturating_sub(summaries.from_ms);
    answer.series = summaries
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
    answer.series.sort_by(|a, b| natural_cmp(&a.key, &b.key));
    let range = Window {
        from_ms,
        to_ms: ask.to_ms,
        now,
    };
    let not_synced = not_synced(&view, watermark.as_ref(), &full, &range);
    answer.gaps = gaps(&full, &selectors, &keep, not_synced, &range)
        .iter()
        .map(Span::dto)
        .collect();
    answer.range.from = rfc(from_ms);
    answer.range.retained_from = retained.map(rfc);
    answer.bucket_s = bucket_s;
    Ok(answer)
}

/// Fills `answer` for a range that ends before retained history begins at `start`: one
/// empty series per selector and a `before_retention` gap over the whole range, so the
/// answer says the data was pruned rather than never recorded.
fn before_retention(
    answer: &mut HistoryAnswer,
    selectors: &[SeriesSelector],
    ask: &Ask<'_>,
    start: i64,
    now: i64,
) {
    answer.range.retained_from = Some(rfc(start));
    answer.series = selectors
        .iter()
        .map(|sel| {
            let key = SeriesKey::new(sel.metric.clone(), sel.labels.clone());
            SeriesHistory {
                key: key.to_string(),
                unit: unit_of(sel.metric.as_str()),
                summary: Summary {
                    min: None,
                    max: None,
                    max_at: None,
                    max_bucket_s: None,
                    mean: None,
                    mean_approximate: false,
                    last: None,
                    last_at: None,
                    measured_fraction: 0.0,
                },
                points: Vec::new(),
            }
        })
        .collect();
    answer.gaps = vec![
        Span {
            from: ask.from_ms,
            to: (ask.to_ms < now).then_some(ask.to_ms),
            reason: gaps::BEFORE_RETENTION.to_owned(),
            module: None,
        }
        .dto(),
    ];
}

/// The `not_synced` tail of a remote host whose mirror is not current: from where the
/// tier `full` read is synced through (rows past it in that tier have not arrived; a finer
/// or coarser tier being further along adds nothing to this read) to the range's end.
fn not_synced(
    view: &HostView,
    watermark: Option<&SyncWatermark>,
    full: &HistoryResult,
    w: &Window,
) -> Option<Span> {
    if view.remote.is_none() || is_current(view) {
        return None;
    }
    let synced = watermark.and_then(|w| w.tier(full.tier));
    synced.is_none_or(|s| s < w.to_ms).then(|| Span {
        from: synced.map_or(w.from_ms, |s| s.max(w.from_ms)),
        to: (w.to_ms < w.now).then_some(w.to_ms),
        reason: "not_synced".to_owned(),
        module: None,
    })
}

/// The range a history answer covers, and now on the host's clock.
struct Window {
    from_ms: i64,
    to_ms: i64,
    now: i64,
}

/// The stored gaps that apply to the selected series, the `not_synced` tail, and a
/// `no_data` gap for every span longer than two buckets, cut at now, where no kept series
/// has a point and no gap says why; merged.
fn gaps(
    full: &HistoryResult,
    selectors: &[SeriesSelector],
    keep: &BTreeSet<String>,
    not_synced: Option<Span>,
    w: &Window,
) -> Vec<Span> {
    let modules: BTreeSet<Module> = selectors
        .iter()
        .filter_map(|s| CATALOG_INDEX.get(s.metric.as_str()).map(|d| d.module))
        .collect();
    let mut spans: Vec<Span> = full
        .gaps
        .iter()
        .filter(|g| gaps::applies(g, &modules))
        .map(Span::from_gap)
        .chain(not_synced)
        .collect();
    let end = w.to_ms.min(w.now);
    let bucket = full.bucket_ms.max(1);
    let mut covered: Vec<(i64, i64)> = spans
        .iter()
        .map(|g| (g.from, g.to.unwrap_or(end)))
        .collect();
    for s in full
        .series
        .iter()
        .filter(|s| keep.contains(&s.key.to_string()))
    {
        // A series sampled less often than the bucket fills only some buckets; each point
        // stands for its sample period.
        let period = CATALOG_INDEX
            .get(s.key.metric.as_str())
            .map_or(0, |d| i64::from(d.period_s) * 1_000);
        let width = bucket.max(period);
        covered.extend(s.points.iter().map(|p| (p.t, p.t.saturating_add(width))));
    }
    spans.extend(
        gaps::uncovered(covered, w.from_ms, end, 2 * bucket)
            .into_iter()
            .map(|(from, to)| Span {
                from,
                to: (to < w.now).then_some(to),
                reason: gaps::NO_DATA.to_owned(),
                module: None,
            }),
    );
    gaps::merge(spans)
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
