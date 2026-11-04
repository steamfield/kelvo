//! How far a mirror's durable history reaches: the end of the newest bucket each tier
//! committed together with its sync cursor.

use statglass_schema::Tier;

use super::Mirror;
use crate::MirrorError;

/// Per persisted tier, the end (bucket start plus width, ms epoch on the remote host's
/// clock) of the newest bucket the mirror holds. Mirror rows are written only by
/// [`crate::MirrorSink::ingest`], in the same transaction as the tier's cursor, so this is
/// how far that tier is complete through: rows after it have not been synced, and the
/// in-memory recent snapshot is not counted. `None` when the tier holds no rows (never
/// synced, or everything pruned).
///
/// It is read from the mirror's database, so it survives a reconnect and a restart. A
/// truncation keeps the rows already held, so the watermark stays at their end until
/// pages from the remote's earliest row arrive; the span it skipped is a `truncated` gap
/// on M1 and M15. Retention pruning can lower it when a host has been offline longer
/// than a tier keeps rows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyncWatermark {
    pub s10_ms: Option<i64>,
    pub m1_ms: Option<i64>,
    pub m15_ms: Option<i64>,
}

impl SyncWatermark {
    /// The watermark of `tier`; `None` for a tier that is not persisted. A caller that read
    /// one tier should measure what is unsynced against that tier's watermark.
    pub fn tier(&self, tier: Tier) -> Option<i64> {
        match tier {
            Tier::S10 => self.s10_ms,
            Tier::M1 => self.m1_ms,
            Tier::M15 => self.m15_ms,
            Tier::Live1s | Tier::Unknown => None,
        }
    }

    /// `synced_through_ms` for freshness: the finest tier that holds rows. The 10 s tier
    /// syncs first and closes buckets soonest; once retention has emptied it, minutes,
    /// then 15-minute buckets.
    pub fn synced_through_ms(&self) -> Option<i64> {
        self.s10_ms.or(self.m1_ms).or(self.m15_ms)
    }
}

impl Mirror {
    /// Where this mirror's durable history is complete through, per tier. Blocking: two
    /// indexed lookups per tier on a pooled read connection.
    pub fn sync_watermark(&self) -> Result<SyncWatermark, MirrorError> {
        let spans = self.history.read(|r| r.tier_spans(self.host))?;
        let mut out = SyncWatermark::default();
        for (tier, span) in spans {
            match tier {
                Tier::S10 => out.s10_ms = Some(span.to_ms),
                Tier::M1 => out.m1_ms = Some(span.to_ms),
                Tier::M15 => out.m15_ms = Some(span.to_ms),
                Tier::Live1s | Tier::Unknown => {}
            }
        }
        Ok(out)
    }
}
