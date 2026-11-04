//! Handler tests against a seeded local store and a seeded mirror, through the same
//! `answer` the socket calls, plus one round trip over a real socket with the CLI's
//! client.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use statglass_engine::{FrameLayout, LiveFrame};
use statglass_query::{
    API_VERSION, Answer, AppPaths, Body, Client, ErrorTag, HistoryAnswer, HostKind, HostState, Op,
    QueryError, QueryRequest, UsageBy,
};
use statglass_remote::{Mirror, RemoteError, SyncWatermark};
use statglass_schema::durable_usage::{HistoricalUsageReply, HistoricalUsageRequest};
use statglass_schema::lock::LockExt;
use statglass_schema::usage::{UsageByApp, UsageKey};
use statglass_schema::{
    Event, EventDetail, Gap, GapReason, HostId, HostInfo, HostRecord, Module, OsKind, SeriesKey,
    Tier,
};
use statglass_store::BucketRow;

use super::server::{Limits, answer};
use super::*;
use crate::remote::{RemoteState, RemoteSync};

const S: i64 = 1_000;
const MIN: i64 = 60 * S;
/// On every bucket grid, and the controller's (and the hosts') now.
const NOW: i64 = 1_800_000_000_000;
/// Where the offline mirror stops.
const SYNCED: i64 = NOW - 30 * MIN;

fn record(n: u128, name: &str, is_local: bool, os: OsKind) -> HostRecord {
    HostRecord {
        id: HostId(uuid::Uuid::from_u128(n)),
        is_local,
        display_name: name.into(),
        info: HostInfo {
            os,
            os_version: "27.0".into(),
            model: Some("Mac15,8".into()),
            chip: Some("Apple M3 Max".into()),
            chip_known: true,
            cpu_topology: Vec::new(),
            mem_total_bytes: 0,
            boot_time_ms: 0,
            gpu_dvfs_mhz: Vec::new(),
            kernel: None,
            boot_mounts: Vec::new(),
        },
    }
}

fn remote(record: HostRecord, enabled: bool, state: RemoteState) -> HostView {
    HostView {
        record,
        remote: Some(RemoteView {
            enabled,
            status: RemoteHostStatus {
                state,
                last_seen_ms: Some(SYNCED as f64),
                sync: RemoteSync::Idle,
                ..Default::default()
            },
        }),
    }
}

fn subdir(dir: &tempfile::TempDir, name: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn keys(keys: &[&str]) -> Arc<[SeriesKey]> {
    keys.iter()
        .map(|k| SeriesKey::parse(k).unwrap())
        .collect::<Vec<_>>()
        .into()
}

/// 10 s buckets over `[from, to)`, every series at `value(t)`: `(v - 1, v + 1, v)`.
fn write_s10(
    w: &statglass_store::Writer,
    host: HostId,
    layout: &Arc<[SeriesKey]>,
    from: i64,
    to: i64,
    value: impl Fn(i64) -> f32,
) {
    for t in (from..to).step_by(10_000) {
        let v = value(t);
        w.write_bucket(BucketRow {
            host,
            tier: Tier::S10,
            bucket_ts: t,
            series: layout.clone(),
            stats: layout.iter().flat_map(|_| [v - 1.0, v + 1.0, v]).collect(),
        })
        .unwrap();
    }
}

struct Fake {
    local: HostId,
    hosts: Vec<HostView>,
    histories: HashMap<HostId, Arc<History>>,
    mirrors: HashMap<HostId, Mirror>,
    frames: HashMap<HostId, Arc<LiveFrame>>,
    /// `hosts()` takes it: a test holding it stalls every handler that lists hosts.
    stall: Mutex<()>,
    remote: Box<dyn Fn() -> Result<RemoteAnswer, RemoteError> + Send + Sync>,
    _dir: tempfile::TempDir,
}

impl Backend for Fake {
    fn local(&self) -> HostId {
        self.local
    }
    fn hosts(&self) -> Vec<HostView> {
        let _stalled = self.stall.lock_ok();
        self.hosts.clone()
    }
    fn history(&self, host: HostId) -> Result<Arc<History>, CommandError> {
        self.histories
            .get(&host)
            .cloned()
            .ok_or(CommandError::UnknownHost { host })
    }
    fn recent_rows(&self, _: HostId, _: i64, _: i64) -> Vec<BucketRow> {
        Vec::new()
    }
    fn host_now(&self, _: HostId) -> i64 {
        NOW
    }
    fn latest_frame(&self, host: HostId) -> Option<Arc<LiveFrame>> {
        self.frames.get(&host).cloned()
    }
    fn sync_watermark(&self, host: HostId) -> Option<SyncWatermark> {
        self.mirrors.get(&host)?.sync_watermark().ok()
    }
    fn usage_ring(
        &self,
        _: HostId,
        from_ms: i64,
        to_ms: i64,
        _: UsageKey,
        _: usize,
    ) -> Result<UsageByApp, CommandError> {
        Ok(UsageByApp {
            from_ms,
            to_ms,
            ..Default::default()
        })
    }
    fn durable_usage(
        &self,
        _: &HistoricalUsageRequest,
    ) -> Result<HistoricalUsageReply, CommandError> {
        Err(CommandError::history_unavailable())
    }
    fn remote_usage(&self, _: HostId, _: RemoteAsk) -> RemoteFuture {
        let reply = (self.remote)();
        Box::pin(async move { reply })
    }
    fn socket_path(&self) -> PathBuf {
        self._dir.path().join("cli/query.sock")
    }
}

/// A frame over `layout` at `ts_ms` holding `held`.
fn frame(layout: &Arc<FrameLayout>, ts_ms: i64, held: &[f32]) -> Arc<LiveFrame> {
    Arc::new(LiveFrame {
        ts_ms,
        interval_ms: 1000,
        layout: layout.clone(),
        timeline: 0,
        values: held.into(),
        held: held.into(),
        holds: vec![2500; held.len()].into(),
    })
}

/// The series every fixture frame holds: headline, local-only headline and other ones.
const FRAME_SERIES: [&str; 6] = [
    "cpu.total",
    "cpu.load{core=0}",
    "mem.used",
    "power.system",
    "disk.used{vol=/}",
    "disk.used{vol=/Volumes/Backup}",
];

/// This Mac with an hour of CPU (a spike at NOW - 30 min, asleep NOW - 20 to - 15 min),
/// ten minutes of `gpu.util` (NOW - 40 to - 30 min), disk disabled NOW - 50 to - 45 min,
/// and two events, one from a newer build; `m1-linux`, offline, mirrored through
/// [`SYNCED`]; `studio`, disabled; `builder`, connected, which answers usage with
/// `Unsupported`.
fn fixture() -> Arc<Fake> {
    let dir = tempfile::Builder::new().prefix("sgq").tempdir().unwrap();
    let mac = record(1, "Mac", true, OsKind::MacOs);
    let linux = record(2, "m1-linux", false, OsKind::Linux);
    let studio = record(3, "studio", false, OsKind::MacOs);
    let builder = record(4, "builder", false, OsKind::MacOs);

    let local = Arc::new(History::open(&subdir(&dir, "local")));
    let w = local.register_host(&mac).unwrap();
    let cpu = keys(&[
        "cpu.total",
        "cpu.load{core=0}",
        "cpu.load{core=10}",
        "cpu.load{core=2}",
    ]);
    let spike = |t| if t == NOW - 30 * MIN { 90.0 } else { 10.0 };
    write_s10(&w, mac.id, &cpu, NOW - 60 * MIN, NOW - 20 * MIN, spike);
    write_s10(&w, mac.id, &cpu, NOW - 15 * MIN, NOW, spike);
    write_s10(
        &w,
        mac.id,
        &keys(&["gpu.util"]),
        NOW - 40 * MIN,
        NOW - 30 * MIN,
        |_| 5.0,
    );
    // One sleep stored as two touching rows, which the answer merges.
    for (from, to) in [
        (NOW - 20 * MIN, NOW - 18 * MIN),
        (NOW - 18 * MIN, NOW - 15 * MIN),
    ] {
        let sleep = Gap::host(from, Some(to), GapReason::Sleep).unwrap();
        w.write_gap(mac.id, sleep).unwrap();
    }
    let disk_off = Gap::module_disabled(NOW - 50 * MIN, Some(NOW - 45 * MIN), Module::Disk);
    w.write_gap(mac.id, disk_off).unwrap();
    let fans = Event {
        ts_ms: NOW - 10 * MIN,
        start_ms: NOW - 11 * MIN,
        processes: vec!["clang".into()],
        detail: EventDetail::FansRamped {
            from_rpm: 1200.0,
            to_rpm: 3400.0,
        },
    };
    w.record_event(mac.id, &fans).unwrap();
    w.write_event(mac.id, NOW - 5 * MIN, "power_spike", vec![0xff, 0x00])
        .unwrap();
    w.flush().unwrap();

    let mirror_history = Arc::new(History::open(&subdir(&dir, "mirror")));
    let w = mirror_history.register_host(&linux).unwrap();
    write_s10(
        &w,
        linux.id,
        &keys(&["cpu.total"]),
        NOW - 60 * MIN,
        SYNCED,
        |_| 20.0,
    );
    w.flush().unwrap();
    let mirror = Mirror::new(mirror_history.store(), linux.id, 100);

    let layout = Arc::new(FrameLayout {
        layout_no: 1,
        series: keys(&FRAME_SERIES),
    });
    let other = [9.0, 1e11, 2e11];
    Arc::new(Fake {
        local: mac.id,
        hosts: vec![
            HostView {
                record: mac.clone(),
                remote: None,
            },
            remote(builder.clone(), true, RemoteState::Connected),
            remote(linux.clone(), true, RemoteState::Offline),
            remote(studio.clone(), false, RemoteState::Disabled),
        ],
        histories: HashMap::from([(mac.id, local), (linux.id, mirror_history)]),
        mirrors: HashMap::from([(linux.id, mirror)]),
        frames: HashMap::from([
            (
                mac.id,
                frame(
                    &layout,
                    NOW - 1_000,
                    &[&[12.0, 30.0, f32::NAN], &other[..]].concat(),
                ),
            ),
            (
                builder.id,
                frame(
                    &layout,
                    NOW - 2_000,
                    &[&[40.0, 50.0, 1e9], &other[..]].concat(),
                ),
            ),
            (
                linux.id,
                frame(&layout, SYNCED, &[&[20.0, 20.0, 1e9], &other[..]].concat()),
            ),
        ]),
        stall: Mutex::new(()),
        remote: Box::new(|| Err(RemoteError::Unsupported("too old".into()))),
        _dir: dir,
    })
}

async fn ask(fake: &Arc<Fake>, op: Op) -> Result<Answer, Box<QueryError>> {
    let backend: Arc<dyn Backend> = fake.clone();
    match answer(backend, &Limits::default(), QueryRequest::new(op)).await {
        Body::Ok(a) => Ok(a),
        Body::Error(e) => Err(Box::new(e)),
    }
}

async fn history(
    fake: &Arc<Fake>,
    host: &str,
    series: &[&str],
) -> Result<HistoryAnswer, Box<QueryError>> {
    let op = Op::History {
        host: Some(host.into()),
        series: series.iter().map(|s| (*s).to_owned()).collect(),
        from_ms: NOW - 60 * MIN,
        to_ms: NOW,
        points: Some(6),
    };
    match ask(fake, op).await? {
        Answer::History(h) => Ok(h),
        other => panic!("{other:?}"),
    }
}

fn rfc(ms: i64) -> String {
    statglass_query::time::rfc3339_local(ms)
}

#[tokio::test]
async fn hosts_keep_registry_order_with_kind_state_and_mirror_extent() {
    let fake = fixture();
    let Answer::Hosts(h) = ask(&fake, Op::Hosts).await.unwrap() else {
        panic!()
    };
    let row: Vec<_> = h
        .hosts
        .iter()
        .map(|h| (h.name.as_str(), h.kind, h.state))
        .collect();
    assert_eq!(
        row,
        [
            ("Mac", HostKind::ThisMac, HostState::Local),
            ("builder", HostKind::Mac, HostState::Connected),
            ("m1-linux", HostKind::Linux, HostState::Offline),
            ("studio", HostKind::Mac, HostState::Disabled),
        ]
    );
    let mac = &h.hosts[0];
    assert_eq!(
        mac.retained_from.as_deref(),
        Some(rfc(NOW - 60 * MIN).as_str())
    );
    assert_eq!(mac.retained_to.as_deref(), Some(rfc(NOW).as_str()));
    assert_eq!((mac.sync, mac.synced_through.as_deref()), (None, None));
    let linux = &h.hosts[2];
    assert_eq!(linux.synced_through.as_deref(), Some(rfc(SYNCED).as_str()));
    assert_eq!(linux.last_seen.as_deref(), Some(rfc(SYNCED).as_str()));
    assert_eq!(linux.chip.as_deref(), Some("Apple M3 Max"));

    let Answer::Status(s) = ask(&fake, Op::Status).await.unwrap() else {
        panic!()
    };
    assert_eq!((s.api, s.host_count), (API_VERSION, 4));
    assert!(s.socket.ends_with("cli/query.sock"));
}

#[tokio::test]
async fn hosts_resolve_by_name_id_or_local_and_suggest_on_a_typo() {
    let fake = fixture();
    for input in [
        "LOCAL",
        "mac",
        &record(1, "", true, OsKind::MacOs).id.to_string(),
    ] {
        let h = history(&fake, input, &["cpu.total"]).await.unwrap();
        assert_eq!(h.host.name, "Mac", "{input}");
    }
    let e = history(&fake, "m1-linx", &["cpu.total"]).await.unwrap_err();
    assert_eq!(e.tag, ErrorTag::UnknownHost);
    let d = e.details.unwrap();
    assert_eq!(
        (d.host.as_deref(), d.suggestions),
        (Some("m1-linx"), vec!["m1-linux".to_owned()])
    );

    let e = history(&fake, "studio", &["cpu.total"]).await.unwrap_err();
    assert_eq!(e.tag, ErrorTag::HostDisabled);
    let events = Op::Events {
        host: Some("studio".into()),
        from_ms: NOW - MIN,
        to_ms: NOW,
        kind: None,
    };
    assert_eq!(
        ask(&fake, events).await.unwrap_err().tag,
        ErrorTag::HostDisabled
    );
}

#[tokio::test]
async fn history_summarizes_every_bucket_and_reports_explicit_gaps() {
    let fake = fixture();
    let h = history(&fake, "local", &["cpu.total"]).await.unwrap();
    assert_eq!((h.tier.as_str(), h.bucket_s), ("s10", 10));
    assert!(h.freshness.is_none(), "this Mac has no freshness");
    let [s] = h.series.as_slice() else {
        panic!("{h:?}")
    };
    assert_eq!((s.key.as_str(), s.unit.as_str()), ("cpu.total", "percent"));
    let sum = &s.summary;
    assert_eq!((sum.min, sum.max), (Some(9.0), Some(91.0)));
    assert_eq!(sum.max_at.as_deref(), Some(rfc(NOW - 30 * MIN).as_str()));
    assert_eq!(sum.max_bucket_s, Some(10));
    assert_eq!(sum.last_at.as_deref(), Some(rfc(NOW - 10 * S).as_str()));
    assert!(!sum.mean_approximate);
    assert!(
        (sum.measured_fraction - 55.0 / 60.0).abs() < 1e-9,
        "{sum:?}"
    );
    assert!(s.points.len() <= 6 && !s.points.is_empty());
    let gaps: Vec<_> = h.gaps.iter().map(|g| g.reason.as_str()).collect();
    assert_eq!(gaps, ["sleep"], "merged, and the disk gap is not cpu's");
    assert_eq!(h.gaps[0].from, rfc(NOW - 20 * MIN));
    assert_eq!(h.gaps[0].to, Some(rfc(NOW - 15 * MIN)));
    assert_eq!(h.range.retained_from, None);

    // Points are opt-in (absent and 0 both mean none), and the summary does not depend
    // on them.
    for points in [None, Some(0)] {
        let op = Op::History {
            host: None,
            series: vec!["cpu.total".into()],
            from_ms: NOW - 60 * MIN,
            to_ms: NOW,
            points,
        };
        let Answer::History(bare) = ask(&fake, op).await.unwrap() else {
            panic!()
        };
        assert!(bare.series[0].points.is_empty(), "{points:?}");
        assert_eq!(bare.series[0].summary, *sum);
    }
}

#[tokio::test]
async fn a_range_before_retention_is_clamped_and_reported() {
    let fake = fixture();
    let op = Op::History {
        host: None,
        series: vec!["cpu.total".into()],
        from_ms: NOW - 3 * 60 * MIN,
        to_ms: NOW,
        points: None,
    };
    let Answer::History(h) = ask(&fake, op).await.unwrap() else {
        panic!()
    };
    assert_eq!(h.range.from, rfc(NOW - 60 * MIN));
    assert_eq!(
        h.range.retained_from.as_deref(),
        Some(rfc(NOW - 60 * MIN).as_str())
    );
}

#[tokio::test]
async fn a_mirror_behind_the_range_reports_the_unsynced_tail() {
    let fake = fixture();
    let h = history(&fake, "m1-linux", &["cpu.total"]).await.unwrap();
    let f = h.freshness.unwrap();
    assert_eq!(f.state, HostState::Offline);
    assert_eq!(f.synced_through.as_deref(), Some(rfc(SYNCED).as_str()));
    let unsynced: Vec<_> = h.gaps.iter().filter(|g| g.reason == "not_synced").collect();
    let [g] = unsynced.as_slice() else {
        panic!("{:?}", h.gaps)
    };
    assert_eq!(
        (g.from.as_str(), g.to.as_deref()),
        (rfc(SYNCED).as_str(), None)
    );
    let measured = h.series[0].summary.measured_fraction;
    assert!(
        (measured - 0.5).abs() < 1e-9,
        "only the synced half: {measured}"
    );
}

#[tokio::test]
async fn a_connected_mirror_still_catching_up_reports_the_unsynced_tail() {
    // A host the sync loop calls current still lags by up to a sync interval; the read's
    // tail past the watermark is unsynced, not unrecorded.
    let Ok(mut fake) = Arc::try_unwrap(fixture()) else {
        panic!("fixture is shared")
    };
    let linux = fake.hosts[2].record.clone();
    let mut view = remote(linux, true, RemoteState::Connected);
    if let Some(r) = view.remote.as_mut() {
        r.status.sync = RemoteSync::Current;
    }
    fake.hosts[2] = view;
    let fake = Arc::new(fake);
    let h = history(&fake, "m1-linux", &["cpu.total"]).await.unwrap();
    assert_eq!(h.freshness.unwrap().state, HostState::Connected);
    let tail: Vec<_> = h
        .gaps
        .iter()
        .map(|g| (g.reason.as_str(), g.from.as_str(), g.to.as_deref()))
        .collect();
    assert_eq!(tail, [("not_synced", rfc(SYNCED).as_str(), None)]);

    // Once the mirror is within two buckets of now, the ordinary lag is not reported.
    let linux = fake.hosts[2].record.clone();
    let mirror = fake.histories.get(&linux.id).unwrap();
    let w = mirror.register_host(&linux).unwrap();
    write_s10(
        &w,
        linux.id,
        &keys(&["cpu.total"]),
        SYNCED,
        NOW - 10 * S,
        |_| 20.0,
    );
    w.flush().unwrap();
    let h = history(&fake, "m1-linux", &["cpu.total"]).await.unwrap();
    assert!(
        h.gaps.iter().all(|g| g.reason != "not_synced"),
        "{:?}",
        h.gaps
    );
}

#[tokio::test]
async fn a_mistyped_label_value_is_an_unknown_series_with_stored_suggestions() {
    let fake = fixture();
    let e = history(&fake, "local", &["cpu.load{core=3}"])
        .await
        .unwrap_err();
    assert_eq!(e.tag, ErrorTag::UnknownMetric);
    let d = e.details.unwrap_or_default();
    assert_eq!(d.series.as_deref(), Some("cpu.load{core=3}"));
    assert_eq!(
        d.suggestions,
        ["cpu.load{core=0}", "cpu.load{core=2}", "cpu.load{core=10}"]
    );
    assert!(
        d.hint
            .as_deref()
            .is_some_and(|h| h.contains("has stored 3 cpu.load series")),
        "{:?}",
        d.hint
    );

    // A metric the host never stored keeps the ordinary answer and its gaps.
    let disk = history(&fake, "local", &["disk.used{vol=/}"])
        .await
        .unwrap();
    assert!(disk.series.is_empty());
    assert!(disk.gaps.iter().any(|g| g.reason == "module_disabled"));
}

#[tokio::test]
async fn points_report_the_slot_they_cover_and_skip_empty_slots() {
    let fake = fixture();
    // An hour in 6 points over 10 s buckets: 10-minute slots.
    let h = history(&fake, "local", &["gpu.util"]).await.unwrap();
    assert_eq!(h.point_slot_s, Some(600));
    let times: Vec<_> = h.series[0].points.iter().map(|p| p.t.clone()).collect();
    assert_eq!(
        times,
        [rfc(NOW - 40 * MIN)],
        "gpu.util has data in one slot only"
    );

    let op = Op::History {
        host: None,
        series: vec!["gpu.util".into()],
        from_ms: NOW - 60 * MIN,
        to_ms: NOW,
        points: None,
    };
    let Answer::History(bare) = ask(&fake, op).await.unwrap() else {
        panic!()
    };
    assert_eq!(bare.point_slot_s, None, "no points asked, no slot");
}

#[tokio::test]
async fn a_bare_labelled_metric_expands_in_numeric_label_order() {
    let fake = fixture();
    let h = history(&fake, "local", &["cpu.load", "cpu.total"])
        .await
        .unwrap();
    let got: Vec<_> = h.series.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(
        got,
        [
            "cpu.load{core=0}",
            "cpu.load{core=2}",
            "cpu.load{core=10}",
            "cpu.total"
        ]
    );
    let one = history(&fake, "local", &["cpu.load{core=2}"])
        .await
        .unwrap();
    assert_eq!(one.series.len(), 1);
}

#[tokio::test]
async fn unknown_series_suggest_metrics_and_say_to_quote_brace_leftovers() {
    let fake = fixture();
    let e = history(&fake, "local", &["cpu.totl"]).await.unwrap_err();
    assert_eq!(e.tag, ErrorTag::UnknownMetric);
    let d = e.details.unwrap();
    assert!(d.suggestions.contains(&"cpu.total".to_owned()), "{d:?}");
    assert_eq!(d.hint, None);

    // zsh turned `cgroup.cpu{kind=container,name=postgres}` into two words.
    let e = history(&fake, "local", &["cgroup.cpukind=container"])
        .await
        .unwrap_err();
    assert_eq!(e.tag, ErrorTag::UnknownMetric);
    let d = e.details.unwrap();
    assert!(d.hint.unwrap().contains("quote"));
    assert_eq!(d.series.as_deref(), Some("cgroup.cpukind=container"));

    let e = history(&fake, "local", &["cpu.total{core=1}"])
        .await
        .unwrap_err();
    assert_eq!(e.tag, ErrorTag::UnknownMetric);
}

#[tokio::test]
async fn bad_ranges_are_refused_with_the_limit() {
    let fake = fixture();
    let op = |from_ms, to_ms| Op::History {
        host: None,
        series: vec!["cpu.total".into()],
        from_ms,
        to_ms,
        points: None,
    };
    let e = ask(&fake, op(NOW, NOW)).await.unwrap_err();
    assert_eq!(e.tag, ErrorTag::BadRange);
    let e = ask(&fake, op(NOW - 91 * 86_400_000, NOW))
        .await
        .unwrap_err();
    assert_eq!(
        (e.tag, e.details.unwrap().max_range_days),
        (ErrorTag::BadRange, Some(90))
    );
    let top = Op::Top {
        host: None,
        from_ms: NOW - 31 * 86_400_000,
        to_ms: NOW,
        by: UsageBy::Cpu,
        limit: 5,
    };
    let e = ask(&fake, top).await.unwrap_err();
    assert_eq!(
        (e.tag, e.details.unwrap().max_range_days),
        (ErrorTag::BadRange, Some(30))
    );
}

#[tokio::test]
async fn events_come_in_time_order_with_the_unreadable_count() {
    let fake = fixture();
    let op = |kind: Option<&str>| Op::Events {
        host: None,
        from_ms: NOW - 60 * MIN,
        to_ms: NOW,
        kind: kind.map(str::to_owned),
    };
    let Answer::Events(e) = ask(&fake, op(None)).await.unwrap() else {
        panic!()
    };
    assert_eq!(e.unreadable, 1);
    let [fans] = e.events.as_slice() else {
        panic!("{e:?}")
    };
    assert_eq!(fans.kind, "fans_ramped");
    assert_eq!(fans.started.as_deref(), Some(rfc(NOW - 11 * MIN).as_str()));
    assert_eq!(
        fans.detail,
        serde_json::json!({"from_rpm": 1200.0, "to_rpm": 3400.0})
    );
    assert_eq!(fans.processes[0].name, "clang");

    let Answer::Events(none) = ask(&fake, op(Some("alert"))).await.unwrap() else {
        panic!()
    };
    assert!(none.events.is_empty());
    let e = ask(&fake, op(Some("fans_rampd"))).await.unwrap_err();
    assert_eq!(e.tag, ErrorTag::Usage);
    assert_eq!(e.details.unwrap().suggestions, ["fans_ramped"]);
}

#[tokio::test]
async fn now_keeps_remote_hosts_to_headline_series_and_offline_hosts_empty() {
    let fake = fixture();
    let Answer::Now(n) = ask(
        &fake,
        Op::Now {
            host: None,
            all: true,
            detail: false,
        },
    )
    .await
    .unwrap() else {
        panic!()
    };
    let names: Vec<_> = n.hosts.iter().map(|h| h.host.name.as_str()).collect();
    assert_eq!(names, ["Mac", "builder", "m1-linux", "studio"]);
    let series = |i: usize| -> Vec<&str> {
        n.hosts[i]
            .readings
            .iter()
            .map(|r| r.series.as_str())
            .collect()
    };
    assert_eq!(
        series(0),
        ["cpu.total", "disk.used{vol=/}", "power.system"],
        "the headline set plus this Mac's extras; stale values are left out"
    );
    assert_eq!(n.hosts[0].age_ms, Some(1_000));
    assert_eq!(
        series(1),
        ["cpu.total", "mem.used"],
        "a remote host gets the headline set only, whatever its frame holds"
    );
    let linux = &n.hosts[2];
    assert!(linux.readings.is_empty());
    assert_eq!(linux.frame_at.as_deref(), Some(rfc(SYNCED).as_str()));
    assert_eq!(linux.freshness.as_ref().unwrap().state, HostState::Offline);
    assert!(n.hosts.iter().all(|h| h.error.is_none()));
}

#[tokio::test]
async fn top_on_a_remote_host_needs_it_connected_and_current() {
    let fake = fixture();
    let op = |host: &str, limit| Op::Top {
        host: Some(host.into()),
        from_ms: NOW - 30 * MIN,
        to_ms: NOW,
        by: UsageBy::Cpu,
        limit,
    };
    let e = ask(&fake, op("m1-linux", 5)).await.unwrap_err();
    assert_eq!(e.tag, ErrorTag::HostOffline);
    let d = e.details.unwrap();
    assert_eq!(d.host.as_deref(), Some("m1-linux"));
    assert_eq!(d.last_seen.as_deref(), Some(rfc(SYNCED).as_str()));
    assert_eq!(
        ask(&fake, op("builder", 5)).await.unwrap_err().tag,
        ErrorTag::PeerTooOld
    );

    let Answer::Top(t) = ask(&fake, op("local", 25)).await.unwrap() else {
        panic!()
    };
    assert_eq!((t.limit, t.limit_reduced), (25, false));
}

#[tokio::test]
async fn unknown_ops_and_newer_apis_are_unsupported() {
    let fake = fixture();
    let decoded: QueryRequest = serde_json::from_str(r#"{"api":1,"op":"frobnicate"}"#).unwrap();
    assert_eq!(decoded.op, Op::Unknown);
    for request in [
        decoded,
        QueryRequest {
            api: API_VERSION + 1,
            op: Op::Hosts,
        },
    ] {
        let backend: Arc<dyn Backend> = fake.clone();
        let Body::Error(e) = answer(backend, &Limits::default(), request).await else {
            panic!()
        };
        assert_eq!(e.tag, ErrorTag::Unsupported);
        assert_eq!(e.details.unwrap().api, Some(API_VERSION));
    }
}

#[tokio::test]
#[expect(
    clippy::await_holding_lock,
    reason = "the held lock is the stall under test"
)]
async fn a_fifth_request_is_busy_and_a_slot_outlives_its_deadline() {
    let fake = fixture();
    let limits = Limits::new(Duration::from_millis(50), Duration::from_millis(50));
    let held: Vec<_> = (0..4).map(|_| limits.try_slot().unwrap()).collect();
    let backend: Arc<dyn Backend> = fake.clone();
    let Body::Error(e) = answer(backend.clone(), &limits, QueryRequest::new(Op::Status)).await
    else {
        panic!()
    };
    assert_eq!(e.tag, ErrorTag::Busy);
    drop(held);

    // A handler stuck in blocking work answers Busy at the deadline but keeps its slot
    // until the work ends.
    // The test holds the lock across the await on purpose: it is what stalls the handler,
    // which runs on the blocking pool, not on this task.
    let stalled = fake.stall.lock_ok();
    let Body::Error(e) = answer(backend, &limits, QueryRequest::new(Op::Hosts)).await else {
        panic!()
    };
    assert_eq!(e.tag, ErrorTag::Busy);
    assert_eq!(
        limits.free_slots(),
        3,
        "the stuck read still holds its slot"
    );
    drop(stalled);
    for _ in 0..200 {
        if limits.free_slots() == 4 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the slot was not released when the work ended");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_cli_client_round_trips_over_the_socket() {
    let fake = fixture();
    let app_data = fake._dir.path().to_owned();
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let backend: Arc<dyn Backend> = fake.clone();
    let server = tokio::spawn(serve(
        AppPaths::new(&app_data).query_socket(),
        backend,
        Arc::new(Limits::default()),
        stopped,
    ));
    let socket = AppPaths::new(&app_data).query_socket();
    for _ in 0..200 {
        if socket.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let response = tokio::task::spawn_blocking(move || {
        Client::new(AppPaths::new(app_data))
            .request(&QueryRequest::new(Op::Hosts), Duration::from_secs(5))
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(response.api, API_VERSION);
    let Body::Ok(Answer::Hosts(h)) = response.body else {
        panic!("{response:?}")
    };
    assert_eq!(h.hosts.len(), 4);
    stop.send(true).unwrap();
    server.await.unwrap().unwrap();
    assert!(!socket.exists(), "the socket is removed when serving stops");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn turning_access_off_unbinds_the_socket_and_leaves_the_marker() {
    use std::os::unix::fs::PermissionsExt;
    let fake = fixture();
    let paths = AppPaths::new(fake._dir.path());
    let (socket, marker) = (paths.query_socket(), paths.access_disabled_marker());
    let (desired, rx) = tokio::sync::watch::channel(false);
    let backend: Arc<dyn Backend> = fake.clone();
    let owner = tokio::spawn(own_socket(paths, backend, rx));
    let settle = |want_socket: bool| {
        let (socket, marker) = (socket.clone(), marker.clone());
        async move {
            for _ in 0..200 {
                if socket.exists() == want_socket && marker.exists() != want_socket {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            panic!("socket {}, marker {}", socket.exists(), marker.exists());
        }
    };
    settle(false).await;
    let mode = std::fs::metadata(marker.parent().unwrap())
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700, "the marker's directory is private");
    desired.send(true).unwrap();
    settle(true).await;
    desired.send(false).unwrap();
    settle(false).await;
    drop(desired);
    owner.await.unwrap();
}

#[tokio::test]
async fn now_detail_reports_every_held_series() {
    let fake = fixture();
    let op = |host: &str| Op::Now {
        host: Some(host.into()),
        all: false,
        detail: true,
    };
    let series = |a: Answer| -> Vec<String> {
        let Answer::Now(n) = a else { panic!() };
        n.hosts[0]
            .readings
            .iter()
            .map(|r| r.series.clone())
            .collect()
    };
    assert_eq!(
        series(ask(&fake, op("local")).await.unwrap()),
        [
            "cpu.load{core=0}",
            "cpu.total",
            "disk.used{vol=/Volumes/Backup}",
            "disk.used{vol=/}",
            "power.system",
        ]
    );
    assert_eq!(series(ask(&fake, op("builder")).await.unwrap()).len(), 6);
}

#[tokio::test]
async fn history_explains_every_span_with_no_data() {
    let fake = fixture();
    let h = history(&fake, "local", &["gpu.util"]).await.unwrap();
    assert_eq!(h.range.retained_from, None, "history starts with the range");
    let gaps: Vec<_> = h
        .gaps
        .iter()
        .map(|g| (g.reason.as_str(), g.from.clone(), g.to.clone()))
        .collect();
    assert_eq!(
        gaps,
        [
            ("no_data", rfc(NOW - 60 * MIN), Some(rfc(NOW - 40 * MIN))),
            ("no_data", rfc(NOW - 30 * MIN), Some(rfc(NOW - 20 * MIN))),
            ("sleep", rfc(NOW - 20 * MIN), Some(rfc(NOW - 15 * MIN))),
            ("no_data", rfc(NOW - 15 * MIN), None),
        ],
        "leading, inner and trailing spans; the sleep explains its own"
    );

    // Any requested series' point covers the time: gpu.util's hole is cpu.total's data.
    let both = history(&fake, "local", &["cpu.total", "gpu.util"])
        .await
        .unwrap();
    let reasons: Vec<_> = both.gaps.iter().map(|g| g.reason.as_str()).collect();
    assert_eq!(reasons, ["sleep"]);

    // A gap for the requested series' module is kept.
    let disk = history(&fake, "local", &["disk.used"]).await.unwrap();
    let off: Vec<_> = disk
        .gaps
        .iter()
        .filter(|g| g.reason == "module_disabled")
        .collect();
    let [off] = off.as_slice() else {
        panic!("{:?}", disk.gaps)
    };
    assert_eq!(off.module.as_deref(), Some("disk"));
}

#[tokio::test]
async fn a_start_within_one_bucket_of_history_is_not_a_clamp() {
    let fake = fixture();
    let op = |from_ms| Op::History {
        host: None,
        series: vec!["cpu.total".into()],
        from_ms,
        to_ms: NOW,
        points: None,
    };
    let Answer::History(h) = ask(&fake, op(NOW - 60 * MIN - 4 * S)).await.unwrap() else {
        panic!()
    };
    assert_eq!(h.range.retained_from, None);
    assert_eq!(h.range.from, rfc(NOW - 60 * MIN - 4 * S));
    assert!(h.gaps.iter().all(|g| g.reason != "no_data"), "{:?}", h.gaps);
}

/// The fixture as a mutable value, for tests that change one host.
fn owned_fixture() -> Fake {
    Arc::try_unwrap(fixture()).unwrap_or_else(|_| panic!("the fixture is shared"))
}

async fn third_host_retained_to(fake: &Arc<Fake>) -> Option<String> {
    let Answer::Hosts(h) = ask(fake, Op::Hosts).await.unwrap() else {
        panic!()
    };
    h.hosts[2].retained_to.clone()
}

#[tokio::test]
async fn a_live_host_reports_history_through_its_newest_frame() {
    let mut fake = owned_fixture();
    let linux = fake.hosts[2].record.id;
    let layout = fake.frames[&linux].layout.clone();
    fake.frames
        .insert(linux, frame(&layout, NOW - S, &[1.0; FRAME_SERIES.len()]));
    let offline = Arc::new(fake);
    assert_eq!(
        third_host_retained_to(&offline).await,
        Some(rfc(SYNCED)),
        "an offline host's frame is not current"
    );
    let mut fake = Arc::try_unwrap(offline).unwrap_or_else(|_| panic!());
    if let Some(r) = fake.hosts[2].remote.as_mut() {
        r.status.state = RemoteState::Connected;
    }
    let connected = Arc::new(fake);
    assert_eq!(third_host_retained_to(&connected).await, Some(rfc(NOW - S)));
}

async fn top_retained_from(reply: RemoteAnswer, from_ms: i64) -> Option<String> {
    let mut fake = owned_fixture();
    fake.remote = Box::new(move || Ok(reply.clone()));
    let fake = Arc::new(fake);
    let op = Op::Top {
        host: Some("builder".into()),
        from_ms,
        to_ms: NOW,
        by: UsageBy::Cpu,
        limit: 5,
    };
    let Answer::Top(t) = ask(&fake, op).await.unwrap() else {
        panic!()
    };
    t.range.retained_from
}

#[tokio::test]
async fn top_reports_a_clamp_only_when_usage_starts_late() {
    use statglass_schema::durable_usage::{
        HistoricalUsageMetric, UsageHistoryRange, UsageHistorySpan,
    };
    let ring = |since_ms| {
        RemoteAnswer::Ring(UsageByApp {
            // Widened to whole buckets.
            from_ms: NOW - 30 * MIN - 5 * S,
            to_ms: NOW,
            since_ms: Some(since_ms),
            ..Default::default()
        })
    };
    let from = NOW - 30 * MIN;
    assert_eq!(top_retained_from(ring(from + 400), from).await, None);
    assert_eq!(
        top_retained_from(ring(NOW - 20 * MIN), from).await,
        Some(rfc(NOW - 20 * MIN))
    );

    let durable = |first_ms, from_ms| {
        RemoteAnswer::Durable(HistoricalUsageReply {
            range: UsageHistoryRange {
                requested_from_ms: from_ms,
                requested_to_ms: NOW,
                spans: vec![UsageHistorySpan {
                    from_ms: first_ms,
                    to_ms: NOW,
                    resolution_ms: 60_000,
                }],
                coverage: Vec::new(),
            },
            by: HistoricalUsageMetric::Cpu,
            apps: Vec::new(),
            ranking_complete: true,
            network_uncertainty: Vec::new(),
        })
    };
    let from = NOW - 180 * MIN - 27 * S;
    assert_eq!(
        top_retained_from(durable(NOW - 180 * MIN, from), from).await,
        None,
        "the first whole bucket after the start is not a clamp"
    );
    assert_eq!(
        top_retained_from(durable(NOW - 120 * MIN, from), from).await,
        Some(rfc(NOW - 120 * MIN))
    );
}

/// Serves `fake` on its app data dir's query socket; returns the stop sender and task.
async fn serving(
    fake: &Arc<Fake>,
) -> (
    tokio::sync::watch::Sender<bool>,
    tokio::task::JoinHandle<Result<(), RemoteError>>,
) {
    let socket = AppPaths::new(fake._dir.path()).query_socket();
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let backend: Arc<dyn Backend> = fake.clone();
    let server = tokio::spawn(serve(
        socket.clone(),
        backend,
        Arc::new(Limits::default()),
        stopped,
    ));
    for _ in 0..200 {
        if socket.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    (stop, server)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_connection_over_the_cap_is_answered_busy() {
    let fake = fixture();
    let app_data = fake._dir.path().to_owned();
    let (stop, server) = serving(&fake).await;
    let socket = AppPaths::new(&app_data).query_socket();
    // Connections that never send a request hold every place until the read deadline.
    let held: Vec<_> = (0..server::MAX_CONNECTIONS)
        .map(|_| std::os::unix::net::UnixStream::connect(&socket).unwrap())
        .collect();
    let response = tokio::task::spawn_blocking(move || {
        Client::new(AppPaths::new(app_data))
            .request(&QueryRequest::new(Op::Hosts), Duration::from_secs(5))
    })
    .await
    .unwrap()
    .unwrap();
    let Body::Error(e) = response.body else {
        panic!("{response:?}")
    };
    assert_eq!(e.tag, ErrorTag::Busy);
    drop(held);
    stop.send(true).unwrap();
    server.await.unwrap().unwrap();
}

#[test]
fn requests_from_a_newer_cli_are_unsupported_and_non_objects_are_usage() {
    let decode = |text: &str| super::server::decode_request(text.as_bytes(), || "1.2.3".into());
    for newer in [
        // A known op with a value added since.
        r#"{"api":1,"op":"top","from_ms":0,"to_ms":1,"by":"disk","limit":5}"#,
        r#"{"api":2,"op":"hosts"}"#,
        r#"{"api":2,"op":"top","by":"disk"}"#,
        r#"{"api":1,"op":"history","series":"cpu.total"}"#,
    ] {
        let e = decode(newer).unwrap_err();
        assert_eq!(e.tag, ErrorTag::Unsupported, "{newer}");
        let d = e.details.unwrap();
        assert_eq!(
            (d.api, d.app_version.as_deref()),
            (Some(API_VERSION), Some("1.2.3"))
        );
    }
    for garbage in ["[1]", "\"hosts\"", "{{", "7"] {
        assert_eq!(
            decode(garbage).unwrap_err().tag,
            ErrorTag::Usage,
            "{garbage}"
        );
    }
    assert_eq!(
        decode(r#"{"api":1,"op":"top","from_ms":0,"to_ms":1,"by":"cpu","limit":5}"#)
            .unwrap()
            .op
            .name(),
        "top"
    );
    assert_eq!(
        decode(r#"{"api":1,"op":"frobnicate"}"#).unwrap().op,
        Op::Unknown
    );
}

/// Polls from another thread, as fast as it can, until `done` holds, failing if the CLI
/// would ever find neither the socket nor the access-disabled marker. `marker_first`
/// orders the two checks so that a state passed through between them cannot read as
/// neither: the marker first while access turns on (it goes after the socket appears),
/// the socket first while it turns off (it goes after the marker is written).
fn never_neither(
    socket: PathBuf,
    marker: PathBuf,
    marker_first: bool,
    done: impl Fn(bool, bool) -> bool + Send + 'static,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let start = std::time::Instant::now();
        loop {
            let (s, m) = if marker_first {
                let m = marker.exists();
                (socket.exists(), m)
            } else {
                let s = socket.exists();
                (s, marker.exists())
            };
            assert!(s || m, "neither the socket nor the marker");
            if done(s, m) {
                return;
            }
            assert!(start.elapsed() < Duration::from_secs(5), "never settled");
        }
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_cli_always_finds_the_socket_or_the_access_disabled_marker() {
    let fake = fixture();
    let paths = AppPaths::new(fake._dir.path());
    let (socket, marker) = (paths.query_socket(), paths.access_disabled_marker());
    let (desired, rx) = tokio::sync::watch::channel(false);
    let backend: Arc<dyn Backend> = fake.clone();
    let owner = tokio::spawn(own_socket(paths, backend, rx));
    for _ in 0..200 {
        if marker.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    for _ in 0..20 {
        let watch = never_neither(socket.clone(), marker.clone(), true, |s, m| s && !m);
        desired.send(true).unwrap();
        tokio::task::spawn_blocking(move || watch.join().unwrap())
            .await
            .unwrap();
        let watch = never_neither(socket.clone(), marker.clone(), false, |s, m| m && !s);
        desired.send(false).unwrap();
        tokio::task::spawn_blocking(move || watch.join().unwrap())
            .await
            .unwrap();
    }
    drop(desired);
    owner.await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bind_failure_is_recorded_for_the_cli_until_the_socket_binds() {
    let fake = fixture();
    let paths = AppPaths::new(fake._dir.path());
    let (socket, failed) = (paths.query_socket(), paths.bind_failed_marker());
    // Something else already serves the socket path, so binding fails.
    let squatter = statglass_remote::bind_private(&socket).unwrap();
    let (desired, rx) = tokio::sync::watch::channel(true);
    let backend: Arc<dyn Backend> = fake.clone();
    let owner = tokio::spawn(own_socket(paths.clone(), backend, rx));
    let mut text = String::new();
    for _ in 0..200 {
        text = std::fs::read_to_string(&failed).unwrap_or_default();
        if !text.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(text.contains("already serving"), "{text:?}");
    // Off, then on again while the squatter stays: the CLI must read the bind failure,
    // not a stale access-disabled marker.
    let disabled = paths.access_disabled_marker();
    desired.send(false).unwrap();
    for _ in 0..200 {
        if disabled.exists() && !failed.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(
        disabled.exists() && !failed.exists(),
        "access off was not recorded"
    );
    desired.send(true).unwrap();
    for _ in 0..200 {
        if failed.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(failed.exists(), "the second bind failure was not recorded");
    assert!(
        !disabled.exists(),
        "access is on but the access-disabled marker remains"
    );
    // The squatter goes; the next attempt, after the first back-off, binds.
    drop(squatter);
    let mut answered = None;
    for _ in 0..400 {
        if !failed.exists() {
            let paths = paths.clone();
            answered = tokio::task::spawn_blocking(move || {
                Client::new(paths).request(&QueryRequest::new(Op::Hosts), Duration::from_secs(5))
            })
            .await
            .unwrap()
            .ok();
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!failed.exists(), "the marker outlived the failure");
    let answered = answered.expect("the socket answers once bound");
    assert!(matches!(answered.body, Body::Ok(Answer::Hosts(_))));
    drop(desired);
    owner.await.unwrap();
    assert!(!socket.exists());
}

#[tokio::test]
async fn a_range_wholly_before_retention_is_reported_as_before_retention() {
    let fake = fixture();
    let (from_ms, to_ms) = (NOW - 5 * 60 * MIN, NOW - 2 * 60 * MIN);
    let op = Op::History {
        host: None,
        series: vec!["cpu.total".into(), "cpu.load{core=2}".into()],
        from_ms,
        to_ms,
        points: Some(10),
    };
    let Answer::History(h) = ask(&fake, op).await.unwrap() else {
        panic!()
    };
    assert_eq!(h.range.from, rfc(from_ms));
    assert_eq!(
        h.range.retained_from.as_deref(),
        Some(rfc(NOW - 60 * MIN).as_str())
    );
    let keys: Vec<_> = h.series.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["cpu.total", "cpu.load{core=2}"]);
    // The tier an auto read of the range would use, though nothing is read.
    assert_eq!((h.tier.as_str(), h.bucket_s), ("s10", 10));
    assert!(h.series.iter().all(|s| s.points.is_empty()
        && s.summary.mean.is_none()
        && s.summary.measured_fraction == 0.0));
    let [gap] = h.gaps.as_slice() else {
        panic!("{:?}", h.gaps)
    };
    assert_eq!(
        (gap.reason.as_str(), gap.from.as_str(), gap.to.as_deref()),
        (
            "before_retention",
            rfc(from_ms).as_str(),
            Some(rfc(to_ms).as_str())
        )
    );

    // Events over the same range say the same: nothing, and where history starts.
    let events = Op::Events {
        host: None,
        from_ms,
        to_ms,
        kind: None,
    };
    let Answer::Events(e) = ask(&fake, events).await.unwrap() else {
        panic!()
    };
    assert!(e.events.is_empty());
    assert_eq!(e.range.from, rfc(from_ms));
    assert_eq!(e.range.retained_from, h.range.retained_from);
}

#[tokio::test]
async fn points_are_what_a_store_read_with_that_many_points_returns() {
    let fake = fixture();
    let h = history(&fake, "local", &["cpu.load", "cpu.total"])
        .await
        .unwrap();
    let mac = fake.local;
    for s in &h.series {
        let key = SeriesKey::parse(&s.key).unwrap();
        let query = statglass_store::HistoryQuery {
            host: mac,
            selectors: vec![statglass_schema::SeriesSelector {
                metric: key.metric.clone(),
                labels: key.labels.clone(),
            }],
            from_ms: NOW - 60 * MIN,
            to_ms: NOW,
            tier: statglass_store::TierChoice::Auto,
            max_points: 6,
        };
        let read = fake.histories[&mac].read(|r| r.history(&query)).unwrap();
        let want: Vec<_> = read.series[0]
            .points
            .iter()
            .map(|p| statglass_query::PointDto {
                t: rfc(p.t),
                min: f64::from(p.min),
                max: f64::from(p.max),
                avg: f64::from(p.avg),
            })
            .collect();
        assert!(!want.is_empty());
        assert_eq!(s.points, want, "{}", s.key);
    }
}

#[tokio::test]
async fn a_name_several_hosts_share_asks_for_the_id() {
    let mut fake = owned_fixture();
    let twin = record(9, "M1-Linux", false, OsKind::Linux);
    fake.hosts
        .push(remote(twin.clone(), true, RemoteState::Connected));
    let linux = fake.hosts[2].record.id;
    let fake = Arc::new(fake);
    let e = history(&fake, "m1-linux", &["cpu.total"])
        .await
        .unwrap_err();
    assert_eq!(e.tag, ErrorTag::UnknownHost);
    let d = e.details.unwrap();
    assert_eq!(d.suggestions, [linux.to_string(), twin.id.to_string()]);
    assert_eq!(
        d.hint.as_deref(),
        Some("several hosts have this name; pass the id")
    );
    let h = history(&fake, &linux.to_string(), &["cpu.total"])
        .await
        .unwrap();
    assert_eq!(h.host.id, linux.to_string());
}

#[tokio::test]
async fn now_orders_readings_with_numbers_compared_as_numbers() {
    let mut fake = owned_fixture();
    let layout = Arc::new(FrameLayout {
        layout_no: 2,
        series: keys(&["cpu.load{core=10}", "cpu.load{core=2}", "cpu.load{core=1}"]),
    });
    fake.frames
        .insert(fake.local, frame(&layout, NOW - S, &[1.0, 2.0, 3.0]));
    let fake = Arc::new(fake);
    let op = Op::Now {
        host: None,
        all: false,
        detail: true,
    };
    let Answer::Now(n) = ask(&fake, op).await.unwrap() else {
        panic!()
    };
    let got: Vec<_> = n.hosts[0]
        .readings
        .iter()
        .map(|r| r.series.as_str())
        .collect();
    assert_eq!(
        got,
        ["cpu.load{core=1}", "cpu.load{core=2}", "cpu.load{core=10}"]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_newer_cli_over_the_socket_gets_unsupported() {
    let fake = fixture();
    let socket = AppPaths::new(fake._dir.path()).query_socket();
    let (stop, server) = serving(&fake).await;
    let response = tokio::task::spawn_blocking(move || {
        use statglass_query::frame::{read_frame, write_frame};
        let mut stream = std::os::unix::net::UnixStream::connect(&socket).unwrap();
        let newer = serde_json::json!({
            "api": 1, "op": "top", "from_ms": 0, "to_ms": 1, "by": "disk", "limit": 5
        });
        write_frame(&mut stream, &newer, statglass_query::MAX_REQUEST_BYTES).unwrap();
        read_frame::<_, statglass_query::QueryResponse>(
            &mut stream,
            statglass_query::MAX_RESPONSE_BYTES,
        )
        .unwrap()
    })
    .await
    .unwrap();
    let Body::Error(e) = response.body else {
        panic!("{response:?}")
    };
    assert_eq!(e.tag, ErrorTag::Unsupported);
    stop.send(true).unwrap();
    server.await.unwrap().unwrap();
}
