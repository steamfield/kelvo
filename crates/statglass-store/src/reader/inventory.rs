//! What a host's stored history holds, without reading it: the series it has rows for,
//! the span its buckets cover, and its events with a count of the ones this build cannot
//! decode.

use std::collections::BTreeMap;

use rusqlite::params;
use statglass_schema::{Event, HostId, SeriesKey, Tier};

use super::Reader;
use crate::db::{tier_table, tier_width};
use crate::error::Result;

/// The persisted bucket tiers, finest first.
const TIERS: [Tier; 3] = [Tier::S10, Tier::M1, Tier::M15];

/// The span a host's stored buckets cover, across tiers: from the start of the oldest
/// bucket to the end of the newest. Stored rows only; buckets the engine has not
/// committed yet are not in it. Gaps inside the span are not subtracted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetainedSpan {
    /// Start of the oldest stored bucket, ms epoch.
    pub from_ms: i64,
    /// End (start plus width) of the newest stored bucket, ms epoch.
    pub to_ms: i64,
}

/// [`Reader::events_counted`]: the events this build decoded, oldest first, and how many
/// stored events in the range it skipped because it could not decode them (a kind from a
/// newer build, D-083).
#[derive(Clone, Debug, PartialEq)]
pub struct EventsRead {
    pub events: Vec<Event>,
    pub unreadable: usize,
}

impl Reader {
    /// The series `host` has at least one stored bucket row for, in any tier, ordered by
    /// key text. A series is listed when it is in the layout of a stored row; that row
    /// may still hold no measured value for it. Metrics this build does not know (from a
    /// newer peer) are listed as stored. [`crate::StoreError::UnknownHost`] when the host
    /// was never registered.
    pub fn stored_series(&mut self, host: HostId) -> Result<Vec<SeriesKey>> {
        let host_ref = self.host_ref(host)?;
        let sql = TIERS
            .iter()
            .map(|&t| {
                Ok(format!(
                    "SELECT DISTINCT layout_id FROM {} WHERE host_id = ?1",
                    tier_table(t)?
                ))
            })
            .collect::<Result<Vec<_>>>()?
            .join(" UNION ");
        let layout_ids: Vec<u32> = self
            .conn
            .prepare_cached(&sql)?
            .query_map([host_ref], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut keys = BTreeMap::new();
        for id in layout_ids {
            for key in self.layout(id)?.iter() {
                keys.entry(key.to_string()).or_insert_with(|| key.clone());
            }
        }
        Ok(keys.into_values().collect())
    }

    /// The span `host`'s stored buckets cover across tiers, or `None` when it has none.
    /// [`crate::StoreError::UnknownHost`] when the host was never registered.
    pub fn retained_span(&mut self, host: HostId) -> Result<Option<RetainedSpan>> {
        Ok(self
            .tier_spans(host)?
            .into_iter()
            .map(|(_, s)| s)
            .reduce(|a, b| RetainedSpan {
                from_ms: a.from_ms.min(b.from_ms),
                to_ms: a.to_ms.max(b.to_ms),
            }))
    }

    /// The span each persisted tier's stored buckets cover for `host`, finest tier first;
    /// a tier with no rows is left out. Two indexed lookups per tier (`min` and `max`
    /// apart, so SQLite can answer each from the end of the index).
    /// [`crate::StoreError::UnknownHost`] when the host was never registered.
    pub fn tier_spans(&mut self, host: HostId) -> Result<Vec<(Tier, RetainedSpan)>> {
        let host_ref = self.host_ref(host)?;
        let mut out = Vec::new();
        for tier in TIERS {
            let (table, width) = (tier_table(tier)?, tier_width(tier)?);
            let sql = format!(
                "SELECT (SELECT min(bucket_ts) FROM {table} WHERE host_id = ?1),
                        (SELECT max(bucket_ts) FROM {table} WHERE host_id = ?1)"
            );
            let ends: (Option<i64>, Option<i64>) = self
                .conn
                .prepare_cached(&sql)?
                .query_row(params![host_ref], |r| Ok((r.get(0)?, r.get(1)?)))?;
            if let (Some(first), Some(last)) = ends {
                let span = RetainedSpan {
                    from_ms: first,
                    to_ms: last + width,
                };
                out.push((tier, span));
            }
        }
        Ok(out)
    }

    /// [`Reader::events`] with the count of the stored events in `[from_ms, to_ms)` it
    /// skipped because this build cannot decode them.
    pub fn events_counted(&mut self, host: HostId, from_ms: i64, to_ms: i64) -> Result<EventsRead> {
        let host_ref = self.host_ref(host)?;
        let mut stmt = self.conn.prepare_cached(
            "SELECT payload FROM events WHERE host_id = ?1 AND ts >= ?2 AND ts < ?3
             ORDER BY ts, kind",
        )?;
        let rows = stmt.query_map(params![host_ref, from_ms, to_ms], |r| {
            r.get::<_, Vec<u8>>(0)
        })?;
        let mut out = EventsRead {
            events: Vec::new(),
            unreadable: 0,
        };
        for payload in rows {
            match ciborium::from_reader::<Event, _>(payload?.as_slice()) {
                Ok(e) => out.events.push(e),
                Err(e) => {
                    out.unreadable += 1;
                    tracing::debug!("skipping an event this build cannot decode: {e}");
                }
            }
        }
        Ok(out)
    }
}
