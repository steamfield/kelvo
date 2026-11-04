//! The one rule history reads merge buckets into points by: equal slots of whole buckets,
//! as few buckets per slot as keep the count at or under `max_points`; per slot the min of
//! mins, the max of maxes and the weighted mean of averages. [`crate::Reader::history`]
//! applies it to stored rows; [`downsample`] applies it to a full-resolution result, so a
//! caller holding one can get the points without reading the range again.

use std::collections::BTreeMap;

use statglass_schema::floor_to;

use crate::db::tier_width;
use crate::error::Result;
use crate::types::{HistoryResult, Point, SeriesPoints};

/// The slot grid of a read of `[from_ms, to_ms)` over buckets `width` wide.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Slots {
    /// Start of the bucket holding `from_ms`.
    pub base: i64,
    /// The width each point averages over: whole buckets.
    pub slot_w: i64,
    /// Source weights are minutes for M15 reads, otherwise one source bucket.
    pub unit_w: i64,
}

impl Slots {
    pub(crate) fn new(from_ms: i64, to_ms: i64, width: i64, max_points: u32) -> Self {
        let base = floor_to(from_ms, width);
        let buckets = (to_ms - base + width - 1) / width;
        let max = i64::from(max_points.max(1));
        let per_slot = (buckets + max - 1) / max;
        Self {
            base,
            slot_w: width * per_slot.max(1),
            unit_w: if width == 900_000 { 60_000 } else { width },
        }
    }

    pub(crate) fn index(&self, ts: i64) -> i64 {
        (ts - self.base) / self.slot_w
    }

    pub(crate) fn start(&self, slot: i64) -> i64 {
        self.base + slot * self.slot_w
    }

    /// The point a slot's accumulator makes for a series of `metric`.
    pub(crate) fn point(&self, slot: i64, a: &Acc, metric: &str) -> Point {
        let divisor = if statglass_schema::disk_io::preserves_request_integral(metric) {
            (self.slot_w / self.unit_w) as f64
        } else {
            f64::from(a.weight)
        };
        Point {
            t: self.start(slot),
            min: a.min,
            max: a.max,
            avg: (a.sum / divisor) as f32,
        }
    }
}

/// One slot's stats in progress.
#[derive(Default)]
pub(crate) struct Acc {
    pub(crate) min: f32,
    pub(crate) max: f32,
    /// Sum of averages times their weights.
    pub(crate) sum: f64,
    pub(crate) weight: u32,
}

impl Acc {
    /// Adds one bucket's stats; `weight` is its width in tier units (minutes for an M15
    /// read, 1 otherwise).
    pub(crate) fn add(&mut self, min: f32, max: f32, avg: f32, weight: u32) {
        if self.weight == 0 {
            self.min = min;
            self.max = max;
        } else {
            self.min = self.min.min(min);
            self.max = self.max.max(max);
        }
        self.sum += f64::from(avg) * f64::from(weight);
        self.weight += weight;
    }
}

/// `full`, a read of `[from_ms, to_ms)` at full resolution (no buckets merged), merged
/// down to at most `max_points` per series by the same rule a read with that `max_points`
/// uses. Each input point weighs its whole width in tier units, so the result equals the
/// store's own merge wherever every point is one bucket's row: always for the 10 s and
/// minute tiers, and for 15-minute points not still made of loose minutes.
pub fn downsample(
    full: &HistoryResult,
    from_ms: i64,
    to_ms: i64,
    max_points: u32,
) -> Result<HistoryResult> {
    let slots = Slots::new(from_ms, to_ms, full.bucket_ms.max(1), max_points);
    let tier_w = tier_width(full.tier)?;
    let unit_w = if tier_w == 900_000 { 60_000 } else { tier_w };
    let slots = Slots { unit_w, ..slots };
    let weight = u32::try_from((full.bucket_ms / unit_w).max(1)).unwrap_or(u32::MAX);
    let series = full
        .series
        .iter()
        .map(|s| {
            let mut accs: BTreeMap<i64, Acc> = BTreeMap::new();
            for p in s.points.iter().filter(|p| p.t >= slots.base && p.t < to_ms) {
                accs.entry(slots.index(p.t))
                    .or_default()
                    .add(p.min, p.max, p.avg, weight);
            }
            SeriesPoints {
                key: s.key.clone(),
                points: accs
                    .iter()
                    .map(|(&slot, a)| slots.point(slot, a, s.key.metric.as_str()))
                    .collect(),
            }
        })
        .filter(|s| !s.points.is_empty())
        .collect();
    Ok(HistoryResult {
        tier: full.tier,
        bucket_ms: slots.slot_w,
        series,
        gaps: full.gaps.clone(),
    })
}
