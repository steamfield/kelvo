//! The byte cap trim (D-057, D-059): process detail first (D-122), then chart history,
//! oldest first, never into the last day.

use kelvo_schema::Tier;

use super::State;
use crate::db::{self, HistoryKind};
use crate::error::Result;
use crate::types::{CapTrim, Retention};

/// The byte cap never trims the most recent day: below that, history is not worth having
/// and the cap is reported unmet instead.
const CAP_TRIM_KEEP_MS: i64 = Retention::DAY_MS;
/// Smallest slice of history one cap-trim round removes.
const CAP_TRIM_MIN_STEP_MS: i64 = 3_600_000;
/// Bound on cap-trim rounds per prune; each round re-measures the file.
const CAP_TRIM_MAX_ROUNDS: usize = 64;

impl State {
    /// Deletes the oldest history on every host, a slice at a time, until the live pages
    /// fit in the low-water mark or only the last day is left. Process and app-usage
    /// detail goes first (D-122); chart history only once that alone cannot meet the
    /// mark. Each slice goes from the start of history to a cutoff, so the trimmed span
    /// is "before history began", never a hole in the middle; the `pruned` marks move to
    /// the cutoff so a cursor behind it gets `Truncated` (D-041).
    pub(super) fn trim_to_cap(
        &mut self,
        now: i64,
        retention: Retention,
        size_before: u64,
        mirror: bool,
    ) -> Result<CapTrim> {
        let low_water = retention.low_water_bytes();
        let keep = if mirror { 0 } else { CAP_TRIM_KEEP_MS };
        let floor = if mirror {
            now.saturating_add(1)
        } else {
            Tier::M15.bucket_start(now - keep).unwrap_or(now - keep)
        };
        let mut trim = CapTrim {
            size_before,
            ..CapTrim::default()
        };
        let mut used = self.used_bytes()?;
        if !mirror {
            used = self.trim_rounds(DETAIL_RANGE_SQL, true, floor, low_water, used, &mut trim)?;
        }
        used = self.trim_rounds(
            cap_range_sql(mirror),
            false,
            floor,
            low_water,
            used,
            &mut trim,
        )?;
        trim.cap_met = used <= low_water;
        self.checkpoint_truncate()?;
        Ok(trim)
    }

    /// Cap-trim rounds over the span `range_sql` reports. `detail_only` deletes process
    /// and usage tables alone and leaves the chart tiers and their `pruned` marks.
    /// Returns the live bytes after the last round.
    fn trim_rounds(
        &mut self,
        range_sql: &str,
        detail_only: bool,
        floor: i64,
        low_water: u64,
        mut used: u64,
        trim: &mut CapTrim,
    ) -> Result<u64> {
        for _ in 0..CAP_TRIM_MAX_ROUNDS {
            if used <= low_water {
                break;
            }
            let (oldest, newest): (Option<i64>, Option<i64>) =
                self.conn
                    .query_row(range_sql, [], |r| Ok((r.get(0)?, r.get(1)?)))?;
            let (Some(oldest), Some(newest)) = (oldest, newest) else {
                break;
            };
            if !detail_only {
                trim.earliest_ts_ms = trim.earliest_ts_ms.max(oldest);
            }
            // Remove the share of the span that is over the mark. It undershoots: S10,
            // snapshots and the recent minutes do not shrink with it, and the oldest span
            // is 15-minute rows, which cost a fifteenth of minutes. That only costs more
            // rounds.
            let span = (newest - oldest).max(0) + 60_000;
            let excess = used - low_water;
            let share = i128::from(span) * i128::from(excess) / i128::from(used.max(1));
            let step = i64::try_from(share)
                .unwrap_or(span)
                .max(CAP_TRIM_MIN_STEP_MS);
            let cutoff = Tier::M15
                .bucket_start(oldest + step)
                .unwrap_or(oldest + step)
                .min(floor);
            if cutoff <= oldest {
                break;
            }
            for host_ref in self.host_refs()? {
                self.trim_host(host_ref, cutoff, detail_only, trim)?;
            }
            if detail_only {
                trim.detail_before_ms = Some(cutoff);
            } else {
                trim.earliest_ts_ms = cutoff;
            }
            self.incremental_vacuum()?;
            used = self.used_bytes()?;
        }
        Ok(used)
    }

    fn trim_host(
        &mut self,
        host_ref: i64,
        cutoff: i64,
        detail_only: bool,
        trim: &mut CapTrim,
    ) -> Result<()> {
        // Highest deleted `seq` per `pruned` mark: M1 also covers gaps and events.
        let (mut m1, mut m15, mut s10) = (None, None, None);
        for table in db::HISTORY_TABLES {
            let Some(col) = table.trim_col else {
                continue;
            };
            if detail_only && !matches!(table.kind, HistoryKind::Proc | HistoryKind::Net) {
                continue;
            }
            let (n, max) = self.delete_batched(table.name, col, host_ref, cutoff)?;
            match table.kind {
                HistoryKind::M1 => {
                    trim.m1_rows += n;
                    m1 = m1.max(max);
                }
                HistoryKind::M15 => {
                    trim.m15_rows += n;
                    m15 = m15.max(max);
                }
                HistoryKind::S10 => {
                    trim.s10_rows += n;
                    s10 = s10.max(max);
                }
                HistoryKind::Gaps => {
                    trim.gaps += n;
                    m1 = m1.max(max);
                }
                HistoryKind::Events => {
                    trim.events += n;
                    m1 = m1.max(max);
                }
                HistoryKind::Proc => trim.proc_rows += n,
                HistoryKind::Net => trim.net_rows += n,
                HistoryKind::Cursors => {}
            }
        }
        self.apply(|s| s.usage_trim_identities(host_ref, cutoff))?;
        self.commit()?;
        self.mark_pruned(host_ref, Tier::M1, m1, cutoff)?;
        self.mark_pruned(host_ref, Tier::M15, m15, cutoff)?;
        self.mark_pruned(host_ref, Tier::S10, s10, cutoff)?;
        Ok(())
    }
}

/// Span of the process and app-usage detail the cap trims before chart history (D-122).
const DETAIL_RANGE_SQL: &str = "SELECT min(t), max(t) FROM (
           SELECT min(ts) AS t FROM proc_snap UNION ALL SELECT max(ts) FROM proc_snap UNION ALL
           SELECT min(bucket_ts) FROM proc_top_1m UNION ALL SELECT max(bucket_ts) FROM proc_top_1m UNION ALL
           SELECT min(bucket_ts) FROM proc_top_15m UNION ALL SELECT max(bucket_ts) FROM proc_top_15m UNION ALL
           SELECT min(bucket_ts) FROM proc_net_10s UNION ALL SELECT max(bucket_ts) FROM proc_net_10s UNION ALL
           SELECT min(bucket_ts) FROM proc_net_1m UNION ALL SELECT max(bucket_ts) FROM proc_net_1m UNION ALL
           SELECT min(bucket_ts) FROM proc_net_15m UNION ALL SELECT max(bucket_ts) FROM proc_net_15m UNION ALL
           SELECT min(bucket_ts) FROM usage_coverage UNION ALL SELECT max(bucket_ts) FROM usage_coverage
         )";

fn cap_range_sql(mirror: bool) -> &'static str {
    if mirror {
        "SELECT min(t), max(t) FROM (
                   SELECT min(bucket_ts) AS t FROM tier_10s UNION ALL
                   SELECT max(bucket_ts) FROM tier_10s UNION ALL
                   SELECT min(bucket_ts) FROM tier_1m UNION ALL
                   SELECT max(bucket_ts) FROM tier_1m UNION ALL
                   SELECT min(bucket_ts) FROM tier_15m UNION ALL
                   SELECT max(bucket_ts) FROM tier_15m UNION ALL
                   SELECT min(ts) FROM events UNION ALL SELECT max(ts) FROM events UNION ALL
                   SELECT min(end_ts) FROM gaps UNION ALL SELECT max(end_ts) FROM gaps
                 )"
    } else {
        "SELECT min(t), max(t) FROM (
                   SELECT min(bucket_ts) AS t FROM tier_1m UNION ALL
                   SELECT max(bucket_ts) FROM tier_1m UNION ALL
                   SELECT min(bucket_ts) FROM tier_15m UNION ALL
                   SELECT max(bucket_ts) FROM tier_15m UNION ALL
                   SELECT min(bucket_ts) FROM usage_coverage UNION ALL
                   SELECT max(bucket_ts) FROM usage_coverage
                 )"
    }
}
