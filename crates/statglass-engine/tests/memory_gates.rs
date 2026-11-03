//! Retained-heap gate for a host with heavy process churn (CI runners, containers),
//! with a window watching it: 1 s ticks, process interest and durable usage on. The
//! engine and a real store run on the fake ticker with fake collectors, so the gate is
//! the same on macOS and Linux. Thresholds live in `perf-budget.json`, section
//! `engine.memory`; raising one needs a decision entry (D-060).
//!
//! A global allocator tracks live Rust heap bytes across every thread (the store writer
//! included). The gate is the peak over a 30-minute run after the first 2 minutes, and
//! the growth over the last 10 minutes, once every 15-minute window has filled.

#![allow(clippy::unwrap_used, clippy::print_stdout)]

mod common;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicI64, Ordering};

use common::*;
use serde_json::Value;
use statglass_collect::{
    Cadence, CollectError, Collector, CollectorId, Interest, Probe, ProcessSample, SampleBuf, Tick,
    UsageObservation,
};
use statglass_schema::durable_usage::UsageIncompleteCause;
use statglass_schema::{Capabilities, Entitlement, Module};

struct Live;

static LIVE: AtomicI64 = AtomicI64::new(0);

#[allow(clippy::cast_possible_wrap)]
unsafe impl GlobalAlloc for Live {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        LIVE.fetch_add(layout.size() as i64, Ordering::Relaxed);
        // SAFETY: forwarded unchanged to the system allocator.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        LIVE.fetch_add(layout.size() as i64, Ordering::Relaxed);
        // SAFETY: forwarded unchanged to the system allocator.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        LIVE.fetch_add(new_size as i64 - layout.size() as i64, Ordering::Relaxed);
        // SAFETY: forwarded unchanged to the system allocator.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size() as i64, Ordering::Relaxed);
        // SAFETY: forwarded unchanged to the system allocator.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Live = Live;

fn live() -> i64 {
    LIVE.load(Ordering::Relaxed)
}

/// Long-lived processes on the host.
const STEADY: i32 = 300;
/// New short-lived processes per second, each living `CHURN_LIFE_S`.
const CHURN_PER_S: i64 = 8;
const CHURN_LIFE_S: i64 = 3;
const NAMES: [&str; 8] = ["sh", "git", "node", "cc1", "ld", "rustc", "python3", "make"];

/// A processes collector shaped like the Linux one under CI churn: `STEADY` processes
/// that never exit, plus `CHURN_PER_S` new ones a second. Each CI job (one a minute)
/// runs in its own systemd scope, so its processes belong to a new app.
struct Churn {
    previous: Option<u64>,
}

impl Collector for Churn {
    fn id(&self) -> CollectorId {
        CollectorId("processes")
    }
    fn cadence(&self) -> Cadence {
        Cadence::Adaptive {
            idle_ms: 10_000,
            interest: Interest::Processes,
        }
    }
    fn required_entitlements(&self) -> &'static [Entitlement] {
        &[Entitlement::None]
    }
    fn probe(&mut self) -> Probe {
        self.previous = None;
        Probe::Supported(Vec::new())
    }
    fn fill_capabilities(&self, caps: &mut Capabilities) {
        caps.process_disk_io = true;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    fn sample(&mut self, t: &Tick, out: &mut SampleBuf) -> Result<(), CollectError> {
        let Some(previous) = self.previous.replace(t.continuous_ns) else {
            out.set_usage_observation(
                0,
                UsageObservation {
                    span_ms: 0,
                    causes: [Some(UsageIncompleteCause::NoBaseline), None, None, None],
                },
            );
            return Ok(());
        };
        let secs = (t.continuous_ns - previous) as f32 / 1e9;
        let now_s = (t.continuous_ns / 1_000_000_000) as i64;
        let row = |pid: i32, start_s: i64, name: &str, app: String| ProcessSample {
            pid,
            start_time_us: start_s * 1_000_000,
            name: name.into(),
            cpu_pct: 5.0,
            mem_bytes: 8 << 20,
            compressed_bytes: None,
            threads: 1,
            idle_wakeups_per_s: 0.0,
            energy: 0.0,
            energy_j: 0.0,
            interval_s: secs,
            app: Some(app.into()),
            app_main: false,
            disk_read_bps: 4096.0,
            disk_write_bps: 0.0,
            net_rx_bps: None,
            net_tx_bps: None,
            gpu_pct: None,
            ports: None,
            user: "root".into(),
        };
        for pid in 0..STEADY {
            let name = NAMES[pid as usize % NAMES.len()];
            out.push_process(row(
                pid + 1,
                0,
                name,
                format!("service-{}.service", pid % 40),
            ));
        }
        for start_s in (now_s - CHURN_LIFE_S + 1)..=now_s {
            for k in 0..CHURN_PER_S {
                let n = start_s * CHURN_PER_S + k;
                let pid = 10_000 + (n % 4_000_000) as i32;
                let name = NAMES[(n as usize) % NAMES.len()];
                out.push_process(row(
                    pid,
                    start_s,
                    name,
                    format!("run-job-{}.scope", start_s / 60),
                ));
            }
        }
        let rows = (STEADY + (CHURN_LIFE_S * CHURN_PER_S) as i32) as u32;
        out.observe_processes((secs * 1000.0).round() as u64, rows, rows, false);
        Ok(())
    }
}

fn budget(path: &str) -> f64 {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../perf-budget.json"
    ))
    .unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    path.split('.')
        .fold(&v, |v, k| &v[k])
        .as_f64()
        .unwrap_or_else(|| panic!("perf-budget.json has no number at {path}"))
}

const MIB: f64 = 1024.0 * 1024.0;

#[test]
fn churning_host_with_a_window_open_keeps_retained_heap_within_budget() {
    let keys: Vec<String> = (0..285)
        .map(|i| format!("cgroup.cpu_pct{{unit=u{i}}}"))
        .collect();
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    let (series, _) = fake("cgroups", Cadence::Every(1), &[Module::Cpu], &keys);
    let collectors: Vec<Box<dyn Collector>> = vec![series, Box::new(Churn { previous: None })];
    let mut h = Harness::new("memory-churn", collectors, &settings_all_on());
    h.visible();
    h.ctl.set_process_interest(Some(0));
    let writer = h.store.as_ref().unwrap().writer();
    let commit_s = statglass_store::DEFAULT_COMMIT_INTERVAL.as_secs() as i64;

    let start = h.clock.now().wall_ms;
    let elapsed_s = |h: &Harness| (h.clock.now().wall_ms - start) / 1_000;
    let mut committed_s = 0;
    let (mut warm, mut at_20, mut peak) = (0, 0, 0);
    while elapsed_s(&h) < 30 * 60 {
        assert!(h.tick());
        while h.sub.try_recv().is_some() {}
        let s = elapsed_s(&h);
        if s >= committed_s + commit_s {
            committed_s = s / commit_s * commit_s;
            writer.flush().unwrap();
        }
        let now = live();
        if s == 120 && warm == 0 {
            warm = now;
        }
        if s == 20 * 60 && at_20 == 0 {
            at_20 = now;
        }
        if s >= 120 {
            peak = peak.max(now);
        }
    }
    let end = live();
    let recent = h.live.recent_usage_buckets(start, start + 31 * 60_000);
    let procs: usize = recent.iter().map(|b| b.processes.len()).sum();
    println!(
        "[perf] memory churn: {} ring rows, {} recent usage buckets with {procs} process facts",
        h.live
            .backfill(start)
            .iter()
            .map(|s| s.rows.len())
            .sum::<usize>(),
        recent.len()
    );
    let retained = (peak - warm) as f64 / MIB;
    let growth = (end - at_20) as f64 / MIB;
    println!(
        "[perf] memory churn: live heap {:.1} MiB at 2 min, peak {:.1} MiB, {:.1} MiB at 20 min, {:.1} MiB at 30 min; retained over warm-up {retained:.1} MiB, last-10-min growth {growth:.1} MiB",
        warm as f64 / MIB,
        peak as f64 / MIB,
        at_20 as f64 / MIB,
        end as f64 / MIB,
    );
    let max_retained = budget("engine.memory.churnRetainedMiB");
    let max_growth = budget("engine.memory.churnLateGrowthMiB");
    assert!(
        retained <= max_retained,
        "retained heap {retained:.1} MiB > {max_retained} MiB"
    );
    assert!(
        growth <= max_growth,
        "late growth {growth:.1} MiB > {max_growth} MiB"
    );
}
