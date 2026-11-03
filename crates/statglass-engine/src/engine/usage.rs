//! Recorder lifecycle and writes; collector cadences and interests stay unchanged.
use super::Engine;
use statglass_collect::{ProcessSample, Tick};
use statglass_schema::Module;
use statglass_store::Writer;
use statglass_store::usage::{USAGE_BUCKET_MS, UsageBucket};

pub(super) fn next_bucket(at: i64) -> i64 {
    at.saturating_add(USAGE_BUCKET_MS - 1)
        .div_euclid(USAGE_BUCKET_MS)
        * USAGE_BUCKET_MS
}
impl Engine {
    pub(super) fn account_usage(&mut self, tick: Tick, rows: &[ProcessSample], persist: bool) {
        if !persist {
            return;
        }
        let disabled = &self.settings.disabled;
        let mut enabled = [
            Module::Cpu,
            Module::Gpu,
            Module::Memory,
            Module::Power,
            Module::Network,
        ]
        .map(|m| !disabled.contains(&m));
        enabled[4] &= self
            .shared
            .net_history
            .load(std::sync::atomic::Ordering::Acquire);
        let caps = self.shared.caps.lock_ok();
        let processes = self
            .slots
            .iter()
            .any(|s| s.active && s.collector.id().0 == "processes");
        let supported = [
            processes,
            caps.process_gpu,
            processes,
            caps.process_energy,
            caps.process_network,
        ];
        drop(caps);
        let live = self.sink.live.clone();
        let mut usage = live.usage_recording();
        if usage.generation != self.usage_generation {
            return;
        }
        usage
            .recorder
            .record(tick, rows, &self.buf, enabled, supported);
        let grace = i64::from(self.interval_ms.max(statglass_collect::IDLE_MS)) * 3;
        let closed = usage
            .recorder
            .close_before(tick.wall_ms.saturating_sub(grace));
        // Hold the source lock through the queue write and the retain. Clear-history
        // takes this lock before its delete, so old writes precede it, and a reader
        // waits for the retain before it reads recent facts.
        let commits = self.sink.store.as_ref().map(Writer::usage_commits);
        let kept: Vec<_> = closed
            .into_iter()
            .map(|bucket| {
                (
                    self.write_usage_bucket(bucket.clone(), tick.wall_ms),
                    bucket,
                )
            })
            .collect();
        usage.update_owned(kept, tick.wall_ms, commits);
    }
    pub(super) fn flush_usage(&mut self, now: i64) {
        self.sync_usage_generation(now);
        let live = self.sink.live.clone();
        let usage = live.usage_recording();
        if usage.generation != self.usage_generation {
            return;
        }
        if self.persist_from.is_none() {
            for bucket in usage.recorder.snapshots() {
                self.write_usage_bucket(bucket, now);
            }
        }
    }
    pub(super) fn sync_usage_generation(&mut self, now: i64) {
        let generation = self.sink.live.usage_generation();
        if generation != self.usage_generation {
            self.sink
                .live
                .usage_recording()
                .recorder
                .clear(next_bucket(now));
            self.usage_generation = generation;
            self.reprobe.all = true;
            for slot in &mut self.slots {
                if slot.collector.id().0 == "net_per_process"
                    || slot.collector.id().0 == "gpu_per_process"
                {
                    slot.collector.release();
                    slot.demanded = false;
                }
            }
        }
    }
    /// Queues `bucket`, returning its ticket once the writer accepted it.
    fn write_usage_bucket(&mut self, bucket: UsageBucket, now: i64) -> Option<u64> {
        let store = self.sink.store.clone()?;
        let result = store.write_usage_bucket(self.host, bucket);
        let ticket = result.as_ref().ok().copied();
        self.store_result("usage bucket", result.map(drop), now);
        ticket
    }
}
use statglass_schema::lock::LockExt;

/// Failed attribution sources lose their baseline and mark resource coverage incomplete.
pub(super) fn failed_read(
    collector: &mut dyn statglass_collect::Collector,
    buf: &mut statglass_collect::SampleBuf,
) {
    let resources: &[usize] = match collector.id().0 {
        "processes" => &[0, 2, 3],
        "gpu_per_process" => &[1],
        "net_per_process" => &[4],
        _ => &[],
    };
    if !resources.is_empty() {
        collector.release();
    }
    for &resource in resources {
        buf.set_usage_observation(
            resource,
            statglass_collect::UsageObservation {
                span_ms: 0,
                causes: [
                    Some(statglass_schema::durable_usage::UsageIncompleteCause::ReadFailed),
                    None,
                    None,
                    None,
                ],
            },
        );
    }
}
