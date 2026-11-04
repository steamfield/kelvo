//! `series_summary`: per series, what a history read measured over a range. Unlike
//! [`super::range_stats`] it keeps labelled series apart and reports min, max with the
//! bucket it fell in, and the last reading. Measured time follows the same rule: a bucket
//! counts the time it covers inside the range, cut at now, less any gap inside it that
//! applies to the metric's module; a bucket with no finite reading counts nothing.

use statglass_schema::{Catalog, SeriesKey, Tier};

use super::stats::{ModuleGaps, Window};
use crate::types::{HistoryResult, SeriesPoints};

/// A value and the start of the bucket it was read from, ms epoch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BucketValue {
    pub value: f64,
    /// Start of the bucket. The bucket covers `[t_ms, t_ms + bucket_ms)` of the
    /// [`SeriesSummaries`] it belongs to.
    pub t_ms: i64,
}

/// One series' summary over the [`SeriesSummaries`] range. Every stat is `None` when the
/// series measured nothing there; a non-finite value never becomes a stat.
#[derive(Clone, Debug, PartialEq)]
pub struct SeriesSummary {
    pub key: SeriesKey,
    /// Time inside the range with a reading, outside the gaps that apply to the metric.
    pub measured_ms: i64,
    /// The smallest bucket minimum among measured buckets.
    pub min: Option<f64>,
    /// The largest bucket maximum among measured buckets, with the start of the bucket it
    /// fell in (the earliest on a tie). The sample itself was somewhere in that bucket.
    pub max: Option<BucketValue>,
    /// Span-weighted mean of bucket means over `measured_ms`. Approximate when
    /// [`SeriesSummaries::mean_approximate`] is set.
    pub mean: Option<f64>,
    /// The mean of the newest measured bucket, with that bucket's start.
    pub last: Option<BucketValue>,
}

/// [`series_summary`]: the range widened to whole buckets and cut at now, and one entry
/// per series of the read, ordered by key text.
#[derive(Clone, Debug, PartialEq)]
pub struct SeriesSummaries {
    pub from_ms: i64,
    pub to_ms: i64,
    /// The tier the read came from.
    pub tier: Tier,
    /// The width of each bucket, ms.
    pub bucket_ms: i64,
    /// The read came from the 15-minute tier, whose buckets hold unweighted means of
    /// minute means (architecture.md, rollups), so `mean` is approximate.
    pub mean_approximate: bool,
    pub series: Vec<SeriesSummary>,
}

/// Summarizes `read` per series over `[from_ms, to_ms)`, cut at `now_ms`. `read` should
/// come from [`super::Reader::history`] (or the engine's merge of it with recent rows) at
/// full resolution: a `max_points` large enough that no buckets were merged, so `max`
/// keeps its own bucket. `catalog` gives each metric's module, for the gaps that apply to
/// it; a metric the catalog does not know is cut by host-wide gaps only.
pub fn series_summary(
    read: &HistoryResult,
    catalog: &Catalog,
    from_ms: i64,
    to_ms: i64,
    now_ms: i64,
) -> SeriesSummaries {
    let window = Window::new(read, from_ms, to_ms, now_ms);
    let mut series: Vec<SeriesSummary> = read
        .series
        .iter()
        .map(|s| {
            let module = catalog.get(s.key.metric.as_str()).map(|d| d.module);
            summarize(s, &window, &ModuleGaps::new(read, module, now_ms))
        })
        .collect();
    series.sort_by_cached_key(|s| s.key.to_string());
    SeriesSummaries {
        from_ms: window.from,
        to_ms: window.to,
        tier: read.tier,
        bucket_ms: window.width,
        mean_approximate: read.tier == Tier::M15,
        series,
    }
}

fn summarize(s: &SeriesPoints, window: &Window, gaps: &ModuleGaps) -> SeriesSummary {
    let mut measured_ms = 0_i64;
    let mut weighted = 0.0_f64;
    let mut min: Option<f64> = None;
    let mut max: Option<BucketValue> = None;
    let mut last: Option<BucketValue> = None;
    for p in &s.points {
        let ms = window.measured(gaps, p);
        if ms <= 0 {
            continue;
        }
        let avg = f64::from(p.avg);
        measured_ms += ms;
        weighted += avg * ms as f64;
        if p.min.is_finite() {
            let v = f64::from(p.min);
            min = Some(min.map_or(v, |m| m.min(v)));
        }
        if p.max.is_finite() {
            let v = f64::from(p.max);
            let earlier_tie = |m: &BucketValue| v == m.value && p.t < m.t_ms;
            if max.as_ref().is_none_or(|m| v > m.value || earlier_tie(m)) {
                max = Some(BucketValue {
                    value: v,
                    t_ms: p.t,
                });
            }
        }
        if last.as_ref().is_none_or(|l| p.t > l.t_ms) {
            last = Some(BucketValue {
                value: avg,
                t_ms: p.t,
            });
        }
    }
    SeriesSummary {
        key: s.key.clone(),
        measured_ms,
        min,
        max,
        mean: (measured_ms > 0).then(|| weighted / measured_ms as f64),
        last,
    }
}

#[cfg(test)]
mod tests {
    use statglass_schema::{Gap, GapReason, Labels, MetricId, Module};

    use super::*;
    use crate::types::Point;

    const S: i64 = 1_000;
    const W: i64 = 10 * S;

    fn key(metric: &'static str, labels: &[(&str, &str)]) -> SeriesKey {
        SeriesKey::new(
            MetricId::from_static(metric),
            Labels::from_pairs(labels.iter().copied()).expect("labels"),
        )
    }

    /// One 10 s point per `(bucket, min, avg, max)`.
    fn series(key: SeriesKey, points: &[(i64, f32, f32, f32)]) -> SeriesPoints {
        SeriesPoints {
            key,
            points: points
                .iter()
                .map(|&(b, min, avg, max)| Point {
                    t: b * W,
                    min,
                    max,
                    avg,
                })
                .collect(),
        }
    }

    fn read(tier: Tier, series: Vec<SeriesPoints>, gaps: Vec<Gap>) -> HistoryResult {
        HistoryResult {
            tier,
            bucket_ms: W,
            series,
            gaps,
        }
    }

    #[test]
    fn labelled_series_stay_apart_with_min_max_time_and_last() {
        let core = |c: &str| key("cpu.load", &[("core", c)]);
        let r = read(
            Tier::S10,
            vec![
                series(
                    core("1"),
                    &[
                        (0, 5.0, 10.0, 20.0),
                        (1, 1.0, 30.0, 95.0),
                        (2, 2.0, 4.0, 6.0),
                    ],
                ),
                series(core("0"), &[(0, 50.0, 60.0, 70.0)]),
            ],
            vec![
                // CPU off over the second half of bucket 2: its 4% weighs half a bucket.
                Gap::module_disabled(2 * W + 5 * S, Some(3 * W), Module::Cpu),
                // Another module's gap cuts nothing here.
                Gap::module_disabled(0, Some(3 * W), Module::Disk),
            ],
        );
        let out = series_summary(&r, &Catalog::builtin(), 0, 3 * W, 100 * W);
        assert_eq!((out.from_ms, out.to_ms, out.bucket_ms), (0, 3 * W, W));
        assert!(!out.mean_approximate);
        let [c0, c1] = out.series.as_slice() else {
            panic!("two series: {out:?}")
        };
        assert_eq!(c0.key, core("0"), "ordered by key text");
        assert_eq!(c0.mean, Some(60.0));
        assert_eq!(c1.measured_ms, 25 * S);
        // (10 * 10 s + 30 * 10 s + 4 * 5 s) / 25 s.
        assert_eq!(c1.mean, Some(420.0 / 25.0));
        assert_eq!(c1.min, Some(1.0));
        assert_eq!(
            c1.max,
            Some(BucketValue {
                value: 95.0,
                t_ms: W
            })
        );
        assert_eq!(
            c1.last,
            Some(BucketValue {
                value: 4.0,
                t_ms: 2 * W
            })
        );
    }

    #[test]
    fn unmeasured_and_non_finite_buckets_never_become_stats() {
        let k = key("cpu.total", &[]);
        let r = read(
            Tier::S10,
            vec![series(
                k.clone(),
                &[
                    (0, 1.0, 2.0, 3.0),
                    // Inside a host-wide gap: its max must not win.
                    (1, 0.0, 50.0, 99.0),
                    // No finite reading: counts nothing.
                    (2, f32::NAN, f32::NAN, f32::NAN),
                    // A finite mean with a non-finite max keeps the old max.
                    (3, 1.0, 4.0, f32::INFINITY),
                    // Past now.
                    (5, 0.0, 70.0, 80.0),
                ],
            )],
            vec![Gap::host(W, Some(2 * W), GapReason::Sleep).expect("host gap")],
        );
        let out = series_summary(&r, &Catalog::builtin(), 0, 6 * W, 4 * W + 5 * S);
        assert_eq!(out.to_ms, 4 * W + 5 * S);
        let s = &out.series[0];
        assert_eq!(s.measured_ms, 20 * S);
        assert_eq!(s.mean, Some(3.0));
        assert_eq!(s.min, Some(1.0));
        assert_eq!(
            s.max,
            Some(BucketValue {
                value: 3.0,
                t_ms: 0
            })
        );
        assert_eq!(
            s.last,
            Some(BucketValue {
                value: 4.0,
                t_ms: 3 * W
            })
        );

        // A range with no points in it: an entry with nothing measured.
        let out = series_summary(&r, &Catalog::builtin(), 10 * W, 12 * W, 100 * W);
        let s = &out.series[0];
        assert_eq!(
            (s.measured_ms, s.min, s.max, s.mean, s.last),
            (0, None, None, None, None)
        );
    }

    #[test]
    fn ties_keep_the_earliest_max_and_m15_flags_the_mean() {
        let k = key("cpu.total", &[]);
        let r = HistoryResult {
            tier: Tier::M15,
            bucket_ms: 900 * S,
            series: vec![SeriesPoints {
                key: k,
                points: [0, 1, 2]
                    .iter()
                    .map(|&b| Point {
                        t: b * 900 * S,
                        min: 1.0,
                        max: if b == 1 { 5.0 } else { 9.0 },
                        avg: 2.0,
                    })
                    .collect(),
            }],
            gaps: Vec::new(),
        };
        let out = series_summary(&r, &Catalog::builtin(), 0, 2_700 * S, 10_000 * S);
        assert!(out.mean_approximate);
        assert_eq!(out.bucket_ms, 900 * S);
        assert_eq!(
            out.series[0].max,
            Some(BucketValue {
                value: 9.0,
                t_ms: 0
            })
        );
    }

    #[test]
    fn a_metric_the_catalog_does_not_know_takes_host_wide_gaps_only() {
        let k = key("future.metric", &[]);
        let r = read(
            Tier::S10,
            vec![series(k, &[(0, 1.0, 1.0, 1.0), (1, 1.0, 1.0, 1.0)])],
            vec![
                Gap::module_disabled(0, Some(W), Module::Cpu),
                Gap::host(W, Some(W + 5 * S), GapReason::Sleep).expect("host gap"),
            ],
        );
        let out = series_summary(&r, &Catalog::builtin(), 0, 2 * W, 100 * W);
        assert_eq!(out.series[0].measured_ms, 15 * S);
    }
}
