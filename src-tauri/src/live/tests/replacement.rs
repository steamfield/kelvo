use super::*;

fn replacement(feed: &FakeFeed) -> Arc<FakeFeed> {
    let mut next = FakeFeed::new();
    Arc::get_mut(&mut next).unwrap().host = feed.host;
    next
}

#[tokio::test(start_paused = true)]
async fn a_subscription_racing_registration_fails_explicitly_then_recovers() {
    let a = FakeFeed::new();
    let b = replacement(&a);
    let reg = registry(&["dashboard"]);
    reg.replace_host(a.clone());
    reg.remove_host(a.host);
    let (before, _before_rx, _) = sink();
    assert!(
        reg.subscribe("dashboard", b.clone(), before, req(0))
            .is_none()
    );
    reg.window_closed("dashboard");
    reg.replace_host(b.clone());
    assert_eq!(b.detail.load(Ordering::SeqCst), 0);
    assert!(!reg.is_visible("dashboard"));
    reg.window_visible("dashboard", true);
    let (after, after_rx, _) = sink();
    reg.subscribe("dashboard", b.clone(), after, req(0))
        .unwrap();
    drain_raw(&after_rx).await;
    b.tick(&layout(1, 1), 1_000, 10.0);
    assert!(drain(&after_rx).await.iter().any(|s| s == "frame 1 @1000"));
    reg.window_closed("dashboard");
    assert_eq!(b.detail.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn source_replacement_reconnects_with_identical_process_interest() {
    let a = FakeFeed::new();
    let b = replacement(&a);
    let l = layout(1, 1);
    a.tick(&l, 1_000, 10.0);
    b.tick(&l, 2_000, 20.0);
    b.background_tick.store(true, Ordering::SeqCst);
    b.set_status(|s| s.interval_ms = 2000);
    let reg = registry(&["dashboard", "popover"]);
    let (sink_a, rx_a, _) = sink();
    let (sink_b, rx_b, _) = sink();
    reg.subscribe("dashboard", a.clone(), sink_a, req(60_000))
        .unwrap();
    reg.subscribe("popover", a.clone(), sink_b, req(60_000))
        .unwrap();
    let mut wanted = view(None, &[], Some(1000));
    wanted.network = true;
    wanted.gpu = true;
    wanted.ports = true;
    reg.set_process_interest("dashboard", a.clone(), Some(wanted.clone()), None);
    reg.replace_host(b.clone());
    settle().await;
    assert_eq!(a.detail.load(Ordering::SeqCst), 0);
    assert_eq!(a.period(), None);
    assert_eq!(a.network.lock().unwrap().last(), Some(&false));
    assert_eq!(a.hub.bus().subscriber_count(), 0);
    assert_eq!(b.detail.load(Ordering::SeqCst), 0, "awaiting reconnect");
    let (new_a, new_rx_a, _) = sink();
    let (new_b, new_rx_b, _) = sink();
    reg.subscribe("dashboard", b.clone(), new_a, req(60_000))
        .unwrap();
    reg.subscribe("popover", b.clone(), new_b, req(60_000))
        .unwrap();
    reg.set_process_interest("dashboard", b.clone(), Some(wanted), None);
    assert_eq!(b.detail.load(Ordering::SeqCst), 1);
    assert_eq!(b.period(), Some(1000));
    assert_eq!(b.network.lock().unwrap().last(), Some(&true));
    assert_eq!(b.gpu.lock().unwrap().last(), Some(&true));
    assert_eq!(b.ports.lock().unwrap().last(), Some(&true));
    let received = drain_raw(&new_rx_a).await;
    assert_eq!(status_intervals(&received).last(), Some(&1000));
    drain_raw(&new_rx_b).await;
    b.tick(&l, 3_000, 30.0);
    assert_eq!(drain(&new_rx_a).await, vec!["frame 1 @3000"]);
    assert_eq!(drain(&new_rx_b).await, vec!["frame 1 @3000"]);
    rx_a.try_iter().count();
    rx_b.try_iter().count();
    a.tick(&l, 4_000, 40.0);
    assert!(drain(&rx_a).await.is_empty());
    assert!(drain(&rx_b).await.is_empty());
    reg.window_visible("popover", false);
    assert_eq!(b.detail.load(Ordering::SeqCst), 1);
    reg.window_closed("dashboard");
    assert_eq!(b.detail.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn retired_feed_cannot_reinstall_a_channel_after_host_removal() {
    let a = FakeFeed::new();
    let l = layout(1, 1);
    a.tick(&l, 1_000, 1.0);
    let reg = Arc::new(registry(&["dashboard"]));
    let (sink_a, rx, _) = sink();
    let hook = {
        let reg = reg.clone();
        let host = a.host;
        Box::new(move || reg.remove_host(host)) as Box<dyn FnOnce() + Send>
    };
    reg.subscribe(
        "dashboard",
        a.clone(),
        Arc::new(ReentrantSink {
            inner: sink_a,
            hook: Mutex::new(Some(hook)),
        }),
        req(60_000),
    );
    settle().await;
    assert_eq!(a.detail.load(Ordering::SeqCst), 0);
    assert_eq!(a.hub.bus().subscriber_count(), 0);
    rx.try_iter().count();
    let (stale, _, _) = sink();
    assert!(
        reg.subscribe("dashboard", a.clone(), stale, req(0))
            .is_none()
    );
    assert_eq!(a.detail.load(Ordering::SeqCst), 0);
    let b = replacement(&a);
    reg.replace_host(b.clone());
    let (current, _current_rx, _) = sink();
    reg.subscribe("dashboard", b.clone(), current, req(0))
        .unwrap();
    assert_eq!(b.detail.load(Ordering::SeqCst), 1);
    let (stale, _, _) = sink();
    assert!(
        reg.subscribe("dashboard", a.clone(), stale, req(0))
            .is_none()
    );
    assert_eq!(b.detail.load(Ordering::SeqCst), 1);
    assert_eq!(a.detail.load(Ordering::SeqCst), 0);
}

#[tokio::test(start_paused = true)]
async fn hidden_replacement_stays_idle_until_shown_and_releases_on_removal() {
    let a = FakeFeed::new();
    let b = replacement(&a);
    let reg = registry(&[]);
    let (old_sink, _rx, _) = sink();
    reg.subscribe("dashboard", a.clone(), old_sink, req(0))
        .unwrap();
    reg.replace_host(b.clone());
    let (sink, _current_rx, _) = sink();
    reg.subscribe("dashboard", b.clone(), sink, req(0)).unwrap();
    assert_eq!(b.detail.load(Ordering::SeqCst), 0);
    reg.window_visible("dashboard", true);
    assert_eq!(b.detail.load(Ordering::SeqCst), 1);
    reg.remove_host(b.host);
    assert_eq!(b.detail.load(Ordering::SeqCst), 0);
    reg.window_visible("dashboard", false);
    reg.window_visible("dashboard", true);
    assert_eq!(b.detail.load(Ordering::SeqCst), 0);
}

#[derive(Clone)]
struct RecordingSource {
    record: statglass_schema::HostRecord,
    detail: Arc<Mutex<Vec<bool>>>,
}

#[tokio::test(start_paused = true)]
async fn a_remote_screen_lock_does_not_suspend_the_controllers_live_frames() {
    for is_local in [false, true] {
        let record: statglass_schema::HostRecord = serde_json::from_value(serde_json::json!({
            "id": Uuid::new_v4(), "is_local": is_local, "display_name": "test",
            "info": {"os": "mac_os", "os_version": "test", "model": null, "chip": null,
            "chip_known": false, "cpu_topology": [], "mem_total_bytes": 1024, "boot_time_ms": 0}
        }))
        .unwrap();
        let source = Arc::new(RecordingSource {
            record: record.clone(),
            detail: Arc::default(),
        });
        let hosts = crate::hosts::HostRegistry::new();
        let host = hosts.insert(record, source);
        host.hub().publish(BusMsg::Status(EngineStatus {
            interval_ms: 1000,
            ..Default::default()
        }));
        let reg = registry(&["dashboard"]);
        let (sink, rx, _) = sink();
        reg.subscribe("dashboard", host.clone(), sink, req(0))
            .unwrap();
        drain_raw(&rx).await;
        host.hub().publish(BusMsg::Status(EngineStatus {
            interval_ms: 1000,
            display_idle: true,
            ..Default::default()
        }));
        settle().await;
        host.hub().publish(BusMsg::Frame(Arc::new(LiveFrame {
            ts_ms: 1000,
            interval_ms: 1000,
            layout: layout(1, 1),
            timeline: 0,
            values: vec![42.0].into(),
            held: vec![42.0].into(),
            holds: vec![2500].into(),
        })));
        let messages = drain_raw(&rx).await;
        assert_eq!(
            messages
                .iter()
                .any(|m| matches!(m, LiveMsg::Frame { ts_ms: 1000, .. })),
            !is_local,
            "only this Mac's display may suspend a visible window's frames"
        );
        assert_eq!(
            messages
                .iter()
                .any(|m| matches!(m, LiveMsg::Status(s) if s.display_idle)),
            is_local
        );
        assert!(
            host.hub().status().display_idle,
            "the source's actual state stays intact"
        );
        reg.window_closed("dashboard");
    }
}

impl statglass_engine::Source for RecordingSource {
    fn host(&self) -> statglass_schema::HostRecord {
        self.record.clone()
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }
    fn start(
        self: Arc<Self>,
        _: statglass_engine::SourceSink,
    ) -> Result<statglass_engine::SourceHandle, statglass_engine::SourceError> {
        Ok(statglass_engine::SourceHandle::new(Box::new(
            (*self).clone(),
        )))
    }
}

impl statglass_engine::SourceControl for RecordingSource {
    fn set_paused(&self, _: bool) {}
    fn set_process_interest(&self, _: Option<u32>) {}
    fn set_network_process_interest(&self, _: bool) {}
    fn set_gpu_process_interest(&self, _: bool) {}
    fn set_port_process_interest(&self, _: bool) {}
    fn set_network_history(&self, _: bool) {}
    fn set_detail_interest(&self, interested: bool) {
        self.detail.lock().unwrap().push(interested);
    }
    fn apply_settings(&self, _: &statglass_schema::Settings) {}
    fn set_store(&self, _: Option<statglass_store::Writer>) {}
    fn status(&self) -> EngineStatus {
        EngineStatus::default()
    }
    fn stop(&mut self) {}
}

#[tokio::test(start_paused = true)]
async fn real_host_entry_balances_engine_detail_across_windows_and_process_updates() {
    let record: statglass_schema::HostRecord = serde_json::from_value(serde_json::json!({
        "id": Uuid::new_v4(), "is_local": false, "display_name": "test",
        "info": {"os": "linux", "os_version": "test", "model": null, "chip": null,
        "chip_known": false, "cpu_topology": [], "mem_total_bytes": 1024, "boot_time_ms": 0}
    }))
    .unwrap();
    let detail = Arc::new(Mutex::new(Vec::new()));
    let source = Arc::new(RecordingSource {
        record: record.clone(),
        detail: detail.clone(),
    });
    let hosts = crate::hosts::HostRegistry::new();
    let host = hosts.insert(record, source);
    host.start(None).unwrap();
    let reg = registry(&["dashboard", "popover"]);
    let mut receivers = Vec::new();
    for label in ["dashboard", "popover"] {
        let (sink, rx, _) = sink();
        receivers.push(rx);
        reg.subscribe(label, host.clone(), sink, req(0)).unwrap();
    }
    reg.set_process_interest(
        "dashboard",
        host.clone(),
        Some(view(None, &[], Some(1000))),
        None,
    );
    reg.window_visible("popover", false);
    assert_eq!(*detail.lock().unwrap(), vec![true]);
    reg.window_closed("dashboard");
    assert_eq!(*detail.lock().unwrap(), vec![true, false]);
}
