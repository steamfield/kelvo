//! Persist every usage identity and sufficient statistic before interval ranking.
use std::collections::BTreeMap;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::{OptionalExtension, params};
use statglass_schema::{HostId, lock::LockExt};

use super::{Op, State, Writer, log_queued, widen};
use crate::usage::{UsageBucket, UsageCommits, UsageFacts, frames};
pub(super) type Pending = BTreeMap<(i64, i64), BTreeMap<i64, [Vec<(u64, UsageFacts)>; 3]>>;
use crate::{Result, StoreError};

/// Tickets for queued usage buckets, numbered in queue order, and how far they are on
/// disk. Lets the engine drop a recent bucket once it is durable instead of holding
/// every bucket for a fixed span.
#[derive(Default)]
pub(super) struct UsageTickets {
    next: Mutex<u64>,
    committed: AtomicU64,
    failed_through: AtomicU64,
}

impl UsageTickets {
    /// A write or the batch holding tickets up to `through` failed: those buckets are
    /// not on disk.
    fn failed(&self, through: u64) {
        self.failed_through.fetch_max(through, Ordering::Release);
    }
}

/// The writer's side of [`UsageTickets`]: the newest ticket in the open batch.
#[derive(Default)]
pub(super) struct Batch {
    pub(super) tickets: Arc<UsageTickets>,
    newest: Option<u64>,
}

impl Batch {
    /// The open batch committed (`true`) or was rolled back.
    pub(super) fn finish(&mut self, committed: bool) {
        if let Some(ticket) = self.newest.take() {
            if committed {
                self.tickets.committed.fetch_max(ticket, Ordering::Release);
            } else {
                self.tickets.failed(ticket);
            }
        }
    }
}

impl State {
    /// [`Op::Usage`]: writes a queued bucket into the open batch.
    pub(super) fn queued_usage(&mut self, host: HostId, bucket: &UsageBucket, ticket: u64) {
        let res = self.apply(|s| s.write_usage(host, bucket));
        if res.is_ok() {
            widen(&mut self.batch_span, host, bucket.from_ms, bucket.to_ms);
            self.usage_batch.newest = Some(ticket);
        } else {
            self.usage_batch.tickets.failed(ticket);
        }
        log_queued("usage", res);
    }

    pub(super) fn write_usage(&mut self, host: HostId, bucket: &UsageBucket) -> Result<()> {
        let host = self.host_ref(host)?;
        self.conn.execute("INSERT INTO usage_recording_start(host_id,from_ms) VALUES(?1,?2) ON CONFLICT(host_id) DO UPDATE SET from_ms=min(from_ms,excluded.from_ms)",params![host,bucket.from_ms])?;
        let mut app_ids = BTreeMap::new();
        let mut apps = BTreeMap::new();
        for app in &bucket.apps {
            let id = self.usage_app_id(host, &app.name, bucket.to_ms, &mut app_ids)?;
            apps.entry(id as u64)
                .or_insert_with(UsageFacts::default)
                .merge(&app.facts);
        }
        let mut processes = BTreeMap::new();
        let width = bucket.to_ms - bucket.from_ms;
        for p in bucket
            .processes
            .iter()
            .filter(|p| p.facts.is_active_process(width))
        {
            let app = self.usage_app_id(host, &p.app_name, bucket.to_ms, &mut app_ids)?;
            self.conn.prepare_cached("INSERT INTO usage_processes(host_id,app_id,pid,start_time_us,name,last_ms) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(host_id,app_id,pid,start_time_us) DO UPDATE SET last_ms=max(last_ms,excluded.last_ms)")?.execute(params![host,app,p.pid,p.start_time_us,&*p.name,bucket.to_ms])?;
            let id:i64 = self.conn.prepare_cached("SELECT id FROM usage_processes WHERE host_id=?1 AND pid=?2 AND start_time_us=?3 AND app_id=?4")?.query_row(params![host,p.pid,p.start_time_us,app],|r| r.get(0))?;
            processes
                .entry(id as u64)
                .or_insert_with(UsageFacts::default)
                .merge(&p.facts);
        }
        let source = UsageFacts {
            coverage: bucket.source,
            ..UsageFacts::default()
        };
        self.conn.execute(
            "DELETE FROM usage_uncertain WHERE host_id=?1 AND discovery_bucket_ts=?2",
            params![host, bucket.from_ms],
        )?;
        for u in crate::usage::grouped_uncertainty(&bucket.uncertain_network)? {
            if u.possible_to_ms <= u.possible_from_ms || u.records == 0 {
                return Err(StoreError::Usage(
                    "invalid uncertain network interval".into(),
                ));
            }
            let app = u
                .app_name
                .as_ref()
                .map(|n| self.usage_app_id(host, n, bucket.to_ms, &mut app_ids))
                .transpose()?;
            let seq = self.alloc_seq();
            self.conn.prepare_cached("INSERT INTO usage_uncertain(host_id,app_id,discovery_bucket_ts,possible_from_ms,possible_to_ms,rx_bytes,tx_bytes,records,seq) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)")?.execute(params![host,app,bucket.from_ms,u.possible_from_ms,u.possible_to_ms,signed(u.rx_bytes)?,signed(u.tx_bytes)?,signed(u.records)?,seq])?;
        }
        let batch = statglass_schema::floor_to(bucket.from_ms, frames::BATCH_MS);
        self.usage_pending.entry((host, batch)).or_default().insert(
            bucket.from_ms,
            [
                apps.into_iter().collect(),
                processes.into_iter().collect(),
                vec![(0, source)],
            ],
        );
        Ok(())
    }

    pub(super) fn flush_usage_pending(&mut self) -> Result<()> {
        let pending = std::mem::take(&mut self.usage_pending);
        let result = (|| {
            for (&(host, batch), updates) in &pending {
                for (index, table) in [
                    "usage_app_buckets",
                    "usage_process_buckets",
                    "usage_coverage",
                ]
                .into_iter()
                .enumerate()
                {
                    let mut frames = BTreeMap::new();
                    {
                        let mut stmt=self.conn.prepare_cached(&format!("SELECT bucket_ts,blob FROM {table} WHERE host_id=?1 AND width_ms=10000 AND bucket_ts>=?2 AND bucket_ts<?3 ORDER BY bucket_ts"))?;
                        let mut rows =
                            stmt.query(params![host, batch, batch + frames::BATCH_MS])?;
                        while let Some(row) = rows.next()? {
                            let at: i64 = row.get(0)?;
                            let blob: Vec<u8> = row.get(1)?;
                            frames.extend(frames::decode(&blob, at, None)?);
                        }
                    }
                    for (at, rows) in updates {
                        frames.insert(*at, rows[index].clone());
                    }
                    let blob = frames::encode(&frames.into_iter().collect::<Vec<_>>())?;
                    self.conn.execute(&format!("DELETE FROM {table} WHERE host_id=?1 AND width_ms=10000 AND bucket_ts>?2 AND bucket_ts<?3"),params![host,batch,batch+frames::BATCH_MS])?;
                    self.usage_upsert(table, host, 10_000, batch, &blob)?;
                }
            }
            Ok(())
        })();
        if result.is_err() {
            self.usage_pending = pending;
        }
        result
    }

    fn usage_app_id(
        &mut self,
        host: i64,
        name: &str,
        at: i64,
        ids: &mut BTreeMap<String, i64>,
    ) -> Result<i64> {
        if let Some(&id) = ids.get(name) {
            return Ok(id);
        }
        if name.is_empty() || name.len() > 1024 || name.contains('\0') {
            return Err(StoreError::Usage("invalid app identity".into()));
        }
        self.conn.prepare_cached("INSERT INTO usage_apps(host_id,name,last_ms) VALUES(?1,?2,?3) ON CONFLICT(host_id,name) DO UPDATE SET last_ms=max(last_ms,excluded.last_ms)")?.execute(params![host,name,at])?;
        let id = self
            .conn
            .prepare_cached("SELECT id FROM usage_apps WHERE host_id=?1 AND name=?2")?
            .query_row(params![host, name], |r| r.get(0))?;
        ids.insert(name.to_owned(), id);
        Ok(id)
    }

    pub(super) fn usage_upsert(
        &mut self,
        table: &str,
        host: i64,
        width: i64,
        at: i64,
        blob: &[u8],
    ) -> Result<()> {
        let previous: Option<Vec<u8>> = self
            .conn
            .prepare_cached(&format!(
                "SELECT blob FROM {table} WHERE host_id=?1 AND width_ms=?2 AND bucket_ts=?3"
            ))?
            .query_row(params![host, width, at], |r| r.get(0))
            .optional()?;
        if previous.as_deref() == Some(blob) {
            return Ok(());
        }
        let seq = self.alloc_seq();
        self.conn.prepare_cached(&format!("INSERT INTO {table}(host_id,width_ms,bucket_ts,seq,blob) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(host_id,width_ms,bucket_ts) DO UPDATE SET seq=excluded.seq,blob=excluded.blob"))?.execute(params![host,width,at,seq,blob])?;
        Ok(())
    }
}

fn signed(value: u64) -> Result<i64> {
    i64::try_from(value)
        .map_err(|_| StoreError::Usage("network count exceeds storage integer range".into()))
}

impl Writer {
    /// Queue complete finest-resolution usage facts, before any floor or ranking.
    /// Returns the bucket's ticket for [`Writer::usage_commits`].
    pub fn write_usage_bucket(
        &self,
        host: HostId,
        bucket: crate::usage::UsageBucket,
    ) -> Result<u64> {
        crate::usage::validate_bucket(&bucket)?;
        // Numbered and sent under one lock, so tickets reach the writer in order.
        let mut next = self.usage_tickets.next.lock_ok();
        *next += 1;
        self.queue(Op::Usage(host, bucket, *next))?;
        Ok(*next)
    }

    /// How far queued usage buckets are on disk.
    pub fn usage_commits(&self) -> UsageCommits {
        UsageCommits {
            committed: self.usage_tickets.committed.load(Ordering::Acquire),
            failed_through: self.usage_tickets.failed_through.load(Ordering::Acquire),
        }
    }
}
