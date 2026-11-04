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
    Event, EventDetail, Gap, GapReason, HostId, HostInfo, HostRecord, OsKind, SeriesKey, Tier,
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

/// This Mac with an hour of CPU (a spike at NOW - 30 min, asleep NOW - 20 to - 15 min)
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
    let sleep = Gap::host(NOW - 20 * MIN, Some(NOW - 15 * MIN), GapReason::Sleep).unwrap();
    w.write_gap(mac.id, sleep).unwrap();
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
        series: keys(&["cpu.total", "cpu.load{core=0}", "mem.used"]),
    });
    let frame = |ts_ms, held: &[f32]| {
        Arc::new(LiveFrame {
            ts_ms,
            interval_ms: 1000,
            layout: layout.clone(),
            timeline: 0,
            values: held.into(),
            held: held.into(),
            holds: vec![2500; held.len()].into(),
        })
    };
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
            (mac.id, frame(NOW - 1_000, &[12.0, 30.0, f32::NAN])),
            (builder.id, frame(NOW - 2_000, &[40.0, 50.0, 1e9])),
            (linux.id, frame(SYNCED, &[20.0, 20.0, 1e9])),
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
    assert_eq!(gaps, ["sleep"]);
    assert_eq!(h.gaps[0].from, rfc(NOW - 20 * MIN));
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
        ["cpu.load{core=0}", "cpu.total"],
        "stale values are left out"
    );
    assert_eq!(n.hosts[0].age_ms, Some(1_000));
    assert_eq!(series(1), ["cpu.total", "mem.used"], "headline series only");
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
