//! What a host's stored history holds: its series, the span it covers and the events
//! this build cannot read.

#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use statglass_schema::{Event, EventDetail, Tier};
use statglass_store::{RetainedSpan, StoreError};

#[test]
fn stored_series_lists_each_key_once_across_tiers_and_hosts_stay_apart() {
    let dir = temp_dir("inventory-series");
    let store = open(&dir, "h.sqlite");
    let w = store.writer();
    let (a, b) = (host(1), host(2));
    w.upsert_host(a.clone()).unwrap();
    w.upsert_host(b.clone()).unwrap();
    let cpu = layout(&["cpu.total", "cpu.load{core=1}", "cpu.load{core=0}"]);
    let mem = layout(&["mem.used", "cpu.total"]);
    w.write_bucket(bucket(a.id, Tier::S10, T0, &cpu, 5.0))
        .unwrap();
    w.write_bucket(bucket(a.id, Tier::M1, T0, &mem, 5.0))
        .unwrap();
    w.write_bucket(bucket(b.id, Tier::S10, T0, &layout(&["gpu.busy"]), 5.0))
        .unwrap();
    w.flush().unwrap();

    let mut r = store.reader().unwrap();
    let keys: Vec<String> = r
        .stored_series(a.id)
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        keys,
        [
            "cpu.load{core=0}",
            "cpu.load{core=1}",
            "cpu.total",
            "mem.used"
        ]
    );
    assert_eq!(r.stored_series(b.id).unwrap(), vec![key("gpu.busy")]);
    assert!(matches!(
        r.stored_series(host(9).id),
        Err(StoreError::UnknownHost(_))
    ));
}

#[test]
fn stored_series_leaves_out_a_layout_whose_rows_are_all_gone() {
    let dir = temp_dir("inventory-stale-layout");
    let store = open(&dir, "h.sqlite");
    let w = store.writer();
    let h = host(1);
    w.upsert_host(h.clone()).unwrap();
    let old = layout(&["cpu.total", "net.rx{iface=en5}"]);
    let current = layout(&["cpu.total"]);
    w.write_bucket(bucket(h.id, Tier::M1, T0 - DAY, &old, 5.0))
        .unwrap();
    w.write_bucket(bucket(h.id, Tier::M1, T0, &current, 5.0))
        .unwrap();
    w.flush().unwrap();
    // Pruning removes rows and keeps their layout registered.
    let conn = rusqlite::Connection::open(store.path()).unwrap();
    conn.execute("DELETE FROM tier_1m WHERE bucket_ts < ?1", [T0])
        .unwrap();
    assert_eq!(count(&conn, "SELECT count(*) FROM layouts"), 2);

    let mut r = store.reader().unwrap();
    assert_eq!(r.stored_series(h.id).unwrap(), vec![key("cpu.total")]);
}

#[test]
fn retained_span_runs_from_the_oldest_bucket_to_the_end_of_the_newest() {
    let dir = temp_dir("inventory-span");
    let store = open(&dir, "h.sqlite");
    let w = store.writer();
    let h = host(1);
    w.upsert_host(h.clone()).unwrap();
    w.flush().unwrap();
    let mut r = store.reader().unwrap();
    assert_eq!(r.retained_span(h.id).unwrap(), None, "no buckets yet");

    let cpu = layout(&["cpu.total"]);
    w.write_bucket(bucket(h.id, Tier::M15, T0 - 10 * DAY, &cpu, 5.0))
        .unwrap();
    w.write_bucket(bucket(h.id, Tier::M1, T0 - DAY, &cpu, 5.0))
        .unwrap();
    w.write_bucket(bucket(h.id, Tier::M1, T0, &cpu, 5.0))
        .unwrap();
    w.write_bucket(bucket(h.id, Tier::S10, T0 - 30_000, &cpu, 5.0))
        .unwrap();
    w.flush().unwrap();
    let span = |from_ms, to_ms| RetainedSpan { from_ms, to_ms };
    assert_eq!(
        r.tier_spans(h.id).unwrap(),
        vec![
            (Tier::S10, span(T0 - 30_000, T0 - 20_000)),
            (Tier::M1, span(T0 - DAY, T0 + MIN)),
            (Tier::M15, span(T0 - 10 * DAY, T0 - 10 * DAY + 15 * MIN)),
        ]
    );
    // The newest minute ends after the newest 10 s bucket.
    assert_eq!(
        r.retained_span(h.id).unwrap(),
        Some(RetainedSpan {
            from_ms: T0 - 10 * DAY,
            to_ms: T0 + MIN,
        })
    );
}

#[test]
fn events_counted_reports_rows_this_build_cannot_decode() {
    let dir = temp_dir("inventory-events");
    let store = open(&dir, "h.sqlite");
    let w = store.writer();
    let h = host(1);
    w.upsert_host(h.clone()).unwrap();
    let fans = Event {
        ts_ms: T0 + MIN,
        start_ms: T0 + 30_000,
        processes: vec!["clang".into()],
        detail: EventDetail::FansRamped {
            from_rpm: 1200.0,
            to_rpm: 3400.0,
        },
    };
    w.record_event(h.id, &fans).unwrap();
    // A kind from a newer build, and one outside the range.
    w.write_event(h.id, T0 + 2 * MIN, "power_spike", vec![0xff, 0x00])
        .unwrap();
    w.write_event(h.id, T0 + HOUR, "power_spike", vec![0xff, 0x00])
        .unwrap();
    w.flush().unwrap();

    let mut r = store.reader().unwrap();
    let got = r.events_counted(h.id, T0, T0 + HOUR).unwrap();
    assert_eq!(got.events, vec![fans.clone()]);
    assert_eq!(got.unreadable, 1);
    assert_eq!(r.events(h.id, T0, T0 + HOUR).unwrap(), vec![fans]);
    assert_eq!(r.events_counted(h.id, 0, T0).unwrap().unreadable, 0);
}
