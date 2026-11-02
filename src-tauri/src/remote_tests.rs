//! Shell configuration and cancellation behavior.
use super::*;
#[test]
fn invalid_host_configuration_is_rejected_before_ssh_or_storage() {
    let valid = RemoteHostConfig {
        id: String::new(),
        name: "Other Mac".into(),
        lan: Some("build-mac".into()),
        tailscale: None,
        enabled: true,
        mirror_budget_mb: 100,
    };
    assert!(validate(&valid).is_ok());
    for alias in ["-oProxyCommand=bad", "mac --verbose", "mac\nwhoami"] {
        let mut c = valid.clone();
        c.lan = Some(alias.into());
        assert!(validate(&c).is_err());
    }
    let mut c = valid.clone();
    c.mirror_budget_mb = 0;
    assert!(validate(&c).is_err());
    c = valid;
    c.lan = None;
    assert!(validate(&c).is_err());
}

#[tokio::test]
async fn cancellation_drops_inflight_setup_before_it_can_register_a_host() {
    use std::sync::atomic::{AtomicBool, Ordering};
    struct Cleanup(Arc<AtomicBool>);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let dropped = Arc::new(AtomicBool::new(false));
    let (tx, mut rx) = tokio::sync::watch::channel(false);
    let (started, ready) = tokio::sync::oneshot::channel();
    let observed = dropped.clone();
    let task = tokio::spawn(async move {
        setup_step(&mut rx, async move {
            let _cleanup = Cleanup(observed);
            let _ = started.send(());
            std::future::pending::<()>().await;
            Ok::<(), CommandError>(())
        })
        .await
    });
    ready.await.unwrap();
    tx.send(true).unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(1), task)
            .await
            .expect("setup cancellation must settle promptly")
            .unwrap()
            .is_err()
    );
    assert!(dropped.load(Ordering::SeqCst));
}

#[test]
fn agent_update_states_compare_with_this_build() {
    use commands::agent_update_against as state;
    let this = "0.3.1";
    assert_eq!(state(this, true, None, false), AgentUpdate::Unknown);
    assert_eq!(
        state(this, true, Some("0.3.0"), false),
        AgentUpdate::Available {
            installed: "0.3.0".into(),
            target: "0.3.1".into(),
            via: AgentUpdateVia::Push
        }
    );
    assert_eq!(
        state(this, false, Some("0.3.0"), false),
        AgentUpdate::Available {
            installed: "0.3.0".into(),
            target: "0.3.1".into(),
            via: AgentUpdateVia::AddHost
        }
    );
    assert_eq!(
        state(this, true, Some("0.2.0"), true),
        AgentUpdate::Required {
            installed: "0.2.0".into(),
            target: "0.3.1".into(),
            via: AgentUpdateVia::Push
        }
    );
    assert_eq!(
        state(this, true, Some("0.3.1"), false),
        AgentUpdate::UpToDate {
            installed: "0.3.1".into()
        }
    );
    // A newer host is never offered this build, even when it can't connect.
    for incompatible in [false, true] {
        assert_eq!(
            state(this, true, Some("0.4.0"), incompatible),
            AgentUpdate::NewerThanThis {
                installed: "0.4.0".into()
            }
        );
    }
    // Fail-open like "Update agent" always was: a malformed version is offered it.
    assert!(matches!(
        state(this, true, Some("garbage"), false),
        AgentUpdate::Available { .. }
    ));
}

#[test]
fn offline_hosts_keep_their_last_reported_version() {
    let offline = statglass_remote::ConnectionStatus::default();
    let status = registration::connection_status(offline, true, Some("0.0.1".into()));
    assert_eq!(status.agent_version.as_deref(), Some("0.0.1"));
    assert!(matches!(status.agent_update, AgentUpdate::Available { .. }));

    let incompatible = statglass_remote::ConnectionStatus {
        remote_version: Some("0.0.2".into()),
        incompatible: true,
        error: Some("incompatible remote".into()),
        ..Default::default()
    };
    let status = registration::connection_status(incompatible, false, Some("0.0.1".into()));
    assert_eq!(status.agent_version.as_deref(), Some("0.0.2"));
    assert!(matches!(
        status.agent_update,
        AgentUpdate::Required {
            via: AgentUpdateVia::AddHost,
            ..
        }
    ));
}

#[test]
fn saved_hosts_without_a_reported_version_still_load() {
    let id = HostId(Uuid::new_v4());
    let mut record = statglass_engine::host_record(id);
    record.is_local = false;
    let host = serde_json::json!({
        "config": {"id": id.to_string(), "name": "Box", "lan": "box", "tailscale": null,
                   "enabled": true, "mirror_budget_mb": 100},
        "record": record,
    });
    let bytes = serde_json::to_vec(&serde_json::json!({ id.to_string(): host })).unwrap();
    let (saved, errors) = parse_saved(&bytes);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(saved[&id].last_agent_version, None);
}

#[test]
fn concurrent_saves_leave_one_whole_file_with_every_host() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("remote-hosts.json");
    let lock = Mutex::new(());
    let saved = Mutex::new(BTreeMap::new());
    std::thread::scope(|scope| {
        for i in 0..16 {
            let (lock, saved, path) = (&lock, &saved, &path);
            scope.spawn(move || {
                let id = HostId(Uuid::new_v4());
                let mut record = statglass_engine::host_record(id);
                record.is_local = false;
                let host = SavedHost {
                    config: RemoteHostConfig {
                        id: id.to_string(),
                        name: format!("host-{i}"),
                        lan: Some(format!("host-{i}")),
                        tailscale: None,
                        enabled: true,
                        mirror_budget_mb: 100,
                    },
                    record,
                    command: RemoteCommand::default(),
                    last_agent_version: Some("0.3.1".into()),
                };
                saved.lock_ok().insert(id, host);
                write_saved(lock, saved, path).unwrap();
            });
        }
    });
    let (hosts, errors) = parse_saved(&std::fs::read(&path).unwrap());
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(hosts.len(), 16);
}
