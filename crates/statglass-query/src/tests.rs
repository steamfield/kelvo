//! Contract tests: serde round-trips, forward-compatible fallbacks, JSON shapes and the
//! schema registry.

use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use statglass_schema::CATALOG;

use crate::*;

fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let text = serde_json::to_string(value).unwrap();
    let back: T = serde_json::from_str(&text).unwrap();
    assert_eq!(&back, value, "{text}");
}

fn host() -> HostRef {
    HostRef {
        id: "8c1f".into(),
        name: "m1-linux".into(),
    }
}

fn range() -> Range {
    Range {
        from: "2026-11-03T13:00:00-08:00".into(),
        to: "2026-11-03T16:00:00-08:00".into(),
        retained_from: Some("2026-10-04T00:00:00-07:00".into()),
    }
}

fn freshness() -> Option<Freshness> {
    Some(Freshness {
        state: HostState::Offline,
        last_seen: Some("2026-11-03T09:12:00-08:00".into()),
        synced_through: Some("2026-11-03T09:12:00-08:00".into()),
    })
}

/// One sample of every answer variant. The match in `every_answer_has_a_schema` keeps
/// this list honest when a variant is added.
fn answers() -> Vec<Answer> {
    vec![
        Answer::Status(StatusAnswer {
            app_version: "0.5.0".into(),
            api: 1,
            timezone: "America/Los_Angeles".into(),
            socket: "/Users/x/Library/Application Support/com.tryopendata.statglass/cli/query.sock"
                .into(),
            host_count: 2,
        }),
        Answer::Hosts(HostsAnswer {
            hosts: vec![
                HostDto {
                    id: "a1".into(),
                    name: "MacBook Pro".into(),
                    kind: HostKind::ThisMac,
                    state: HostState::Local,
                    last_seen: None,
                    sync: None,
                    synced_through: None,
                    retained_from: Some("2026-10-04T00:00:00-07:00".into()),
                    retained_to: Some("2026-11-03T16:00:00-08:00".into()),
                    os_version: Some("27.0".into()),
                    model: Some("Mac15,8".into()),
                    chip: Some("Apple M3 Max".into()),
                },
                HostDto {
                    id: "8c1f".into(),
                    name: "m1-linux".into(),
                    kind: HostKind::Linux,
                    state: HostState::Offline,
                    last_seen: Some("2026-11-03T09:12:00-08:00".into()),
                    sync: Some(SyncState::CatchingUp),
                    synced_through: Some("2026-11-03T09:12:00-08:00".into()),
                    retained_from: None,
                    retained_to: None,
                    os_version: None,
                    model: None,
                    chip: None,
                },
            ],
        }),
        Answer::Now(NowAnswer {
            hosts: vec![
                NowHost {
                    host: host(),
                    freshness: None,
                    frame_at: Some("2026-11-03T16:00:00-08:00".into()),
                    age_ms: Some(420),
                    readings: vec![Reading {
                        series: "cpu.load{core=3}".into(),
                        value: 12.5,
                        unit: "percent".into(),
                        module: "cpu".into(),
                    }],
                    error: None,
                },
                NowHost {
                    host: host(),
                    freshness: freshness(),
                    frame_at: None,
                    age_ms: None,
                    readings: vec![],
                    error: Some(QueryError::new(ErrorTag::HostOffline)),
                },
            ],
        }),
        Answer::Metrics(MetricsAnswer {
            host: Some(host()),
            metrics: CATALOG.iter().take(3).map(MetricDto::from_def).collect(),
            series: Some(vec!["cpu.total".into()]),
        }),
        Answer::History(HistoryAnswer {
            host: host(),
            freshness: freshness(),
            range: range(),
            tier: "m1".into(),
            bucket_s: 60,
            series: vec![SeriesHistory {
                key: "cpu.total".into(),
                unit: "percent".into(),
                summary: Summary {
                    min: Some(2.1),
                    max: Some(97.0),
                    max_at: Some("2026-11-03T15:12:00-08:00".into()),
                    max_bucket_s: Some(60),
                    mean: Some(18.4),
                    mean_approximate: false,
                    last: Some(6.3),
                    last_at: Some("2026-11-03T16:00:00-08:00".into()),
                    measured_fraction: 0.98,
                },
                points: vec![PointDto {
                    t: "2026-11-03T13:00:00-08:00".into(),
                    min: 2.1,
                    max: 30.0,
                    avg: 10.0,
                }],
            }],
            gaps: vec![
                GapDto {
                    from: "2026-11-03T14:31:00-08:00".into(),
                    to: Some("2026-11-03T14:37:00-08:00".into()),
                    reason: "source_offline".into(),
                    module: None,
                },
                GapDto {
                    from: "2026-11-03T09:12:00-08:00".into(),
                    to: None,
                    reason: "not_synced".into(),
                    module: None,
                },
            ],
        }),
        Answer::Events(EventsAnswer {
            host: host(),
            freshness: None,
            range: range(),
            events: vec![EventDto {
                at: "2026-11-03T15:12:00-08:00".into(),
                started: Some("2026-11-03T15:10:00-08:00".into()),
                kind: "fans_ramped".into(),
                detail: json!({"from_rpm": 1200.0, "to_rpm": 3400.0}),
                processes: vec![EventProcess {
                    name: "clang".into(),
                    pid: None,
                    value: None,
                    unit: None,
                }],
            }],
            unreadable: 1,
        }),
        Answer::Top(TopAnswer {
            host: host(),
            freshness: None,
            range: range(),
            by: UsageBy::Cpu,
            limit: 10,
            limit_reduced: true,
            apps: vec![TopApp {
                name: "Xcode".into(),
                value: 250.0,
                unit: "percent".into(),
                share: Some(0.4),
                processes: vec![TopProcess {
                    name: "SourceKitService".into(),
                    pid: Some(812),
                    value: 120.0,
                }],
            }],
        }),
    ]
}

fn ops() -> Vec<Op> {
    vec![
        Op::Status,
        Op::Hosts,
        Op::Now {
            host: Some("m1-linux".into()),
            all: false,
            detail: false,
        },
        Op::Now {
            host: None,
            all: true,
            detail: true,
        },
        Op::Metrics {
            host: Some("local".into()),
            module: Some("cpu".into()),
        },
        Op::History {
            host: None,
            series: vec!["cpu.total".into(), "cpu.load{core=3}".into()],
            from_ms: 1,
            to_ms: 2,
            points: Some(60),
        },
        Op::Events {
            host: None,
            from_ms: 1,
            to_ms: 2,
            kind: Some("thermal_state".into()),
        },
        Op::Top {
            host: None,
            from_ms: 1,
            to_ms: 2,
            by: UsageBy::Energy,
            limit: 5,
        },
    ]
}

#[test]
fn every_op_round_trips() {
    for op in ops() {
        round_trip(&QueryRequest::new(op));
    }
}

#[test]
fn every_answer_round_trips_in_a_response() {
    for answer in answers() {
        let v = serde_json::to_value(&answer).unwrap();
        assert_eq!(v["op"], answer.name());
        round_trip(&QueryResponse {
            api: API_VERSION,
            app_version: "0.5.0".into(),
            body: Body::Ok(answer),
        });
    }
}

#[test]
fn unknown_op_decodes_to_the_fallback() {
    let req: QueryRequest =
        serde_json::from_value(json!({"api": 2, "op": "containers", "host": "x", "deep": true}))
            .unwrap();
    assert_eq!(req.api, 2);
    assert_eq!(req.op, Op::Unknown);
    // Omitted optional fields take their defaults.
    let req: QueryRequest = serde_json::from_value(json!({"api": 1, "op": "now"})).unwrap();
    assert_eq!(
        req.op,
        Op::Now {
            host: None,
            all: false,
            detail: false
        }
    );
}

#[test]
fn unknown_values_from_a_newer_app_decode() {
    let e: QueryError = serde_json::from_value(json!({
        "tag": "QuotaExceeded",
        "message": "x",
        "details": {"suggestions": ["a"], "future_field": 3}
    }))
    .unwrap();
    assert_eq!(e.tag, ErrorTag::Unknown);
    assert_eq!(e.tag.exit_code(), 1);
    assert_eq!(e.details.unwrap().suggestions, vec!["a"]);

    let h: HostDto = serde_json::from_value(json!({
        "id": "1", "name": "n", "kind": "windows", "state": "hibernating",
        "sync": "rewinding", "new_field": {"x": 1}
    }))
    .unwrap();
    assert_eq!(
        (h.kind, h.state, h.sync),
        (
            HostKind::Unknown,
            HostState::Unknown,
            Some(SyncState::Unknown)
        )
    );
}

#[test]
fn request_json_shape() {
    let req = QueryRequest::new(Op::History {
        host: Some("m1-linux".into()),
        series: vec!["cpu.total".into()],
        from_ms: 1_000,
        to_ms: 2_000,
        points: None,
    });
    assert_eq!(
        serde_json::to_value(&req).unwrap(),
        json!({
            "api": 1,
            "op": "history",
            "host": "m1-linux",
            "series": ["cpu.total"],
            "from_ms": 1000,
            "to_ms": 2000,
            "points": null
        })
    );
}

#[test]
fn response_json_shapes() {
    let ok = QueryResponse {
        api: 1,
        app_version: "0.5.0".into(),
        body: Body::Ok(Answer::Events(EventsAnswer {
            host: host(),
            freshness: None,
            range: Range {
                from: "a".into(),
                to: "b".into(),
                retained_from: None,
            },
            events: vec![],
            unreadable: 0,
        })),
    };
    assert_eq!(
        serde_json::to_value(&ok).unwrap(),
        json!({
            "api": 1,
            "app_version": "0.5.0",
            "ok": {
                "op": "events",
                "host": {"id": "8c1f", "name": "m1-linux"},
                "range": {"from": "a", "to": "b"},
                "events": [],
                "unreadable": 0
            }
        })
    );
    let err = QueryResponse {
        api: 1,
        app_version: "0.5.0".into(),
        body: Body::Error(
            QueryError::new(ErrorTag::UnknownHost).with_details(ErrorDetails {
                suggestions: vec!["m1-linux".into()],
                input: Some("m1-linxu".into()),
                ..ErrorDetails::default()
            }),
        ),
    };
    assert_eq!(
        serde_json::to_value(&err).unwrap(),
        json!({
            "api": 1,
            "app_version": "0.5.0",
            "error": {
                "tag": "UnknownHost",
                "message": "No host by that name or id.",
                "details": {"suggestions": ["m1-linux"], "input": "m1-linxu"}
            }
        })
    );
    round_trip(&err);
}

#[test]
fn cli_envelope_shape() {
    let env = CliErrorEnvelope::new(
        QueryError::new(ErrorTag::Busy),
        Some("retry in a second".into()),
    );
    assert_eq!(
        serde_json::to_value(&env).unwrap(),
        json!({"error": {
            "tag": "Busy",
            "message": "StatGlass is busy; retry shortly.",
            "retryable": true,
            "next": "retry in a second"
        }})
    );
    round_trip(&env);
}

#[test]
fn exit_codes_follow_the_rfc_table() {
    use ErrorTag::*;
    let table = [
        (Defect, 1),
        (Usage, 2),
        (UnknownHost, 2),
        (UnknownMetric, 2),
        (BadRange, 2),
        (Busy, 4),
        (HostOffline, 5),
        (HostDisabled, 5),
        (PeerTooOld, 5),
        (HistoryUnavailable, 5),
        (Unsupported, 5),
        (AccessDisabled, 6),
        (AccessUnavailable, 6),
        (AppNotRunning, 6),
        (AppNotInstalled, 6),
        (AppOutdated, 6),
        (Interrupted, 130),
    ];
    for (tag, code) in table {
        assert_eq!(tag.exit_code(), code, "{tag:?}");
        assert_eq!(tag.retryable(), tag == Busy, "{tag:?}");
        assert!(!tag.message().is_empty());
        let text = serde_json::to_value(tag).unwrap();
        assert_eq!(text, json!(format!("{tag:?}")));
        assert_eq!(QueryError::new(tag).message, tag.message());
    }
}

#[test]
fn metric_dto_uses_catalog_text() {
    let def = CATALOG
        .iter()
        .find(|d| d.id.as_str() == "cpu.load")
        .unwrap();
    let dto = MetricDto::from_def(def);
    assert_eq!(
        (dto.unit.as_str(), dto.kind.as_str(), dto.module.as_str()),
        ("percent", "mean", "cpu")
    );
    assert_eq!(dto.labels, vec!["core"]);
    for def in CATALOG {
        let dto = MetricDto::from_def(def);
        assert_ne!(dto.unit, "unknown", "{}", dto.id);
        assert_ne!(dto.kind, "unknown", "{}", dto.id);
    }
}

#[test]
fn every_answer_has_a_schema() {
    let entries = schemas();
    let names: Vec<&str> = entries.iter().map(|e| e.name).collect();
    for answer in answers() {
        // Exhaustive: a new Answer variant fails to compile here until it has a sample
        // above and a schema entry.
        let name = match &answer {
            Answer::Status(_) => "status",
            Answer::Hosts(_) => "hosts",
            Answer::Now(_) => "now",
            Answer::Metrics(_) => "metrics",
            Answer::History(_) => "history",
            Answer::Events(_) => "events",
            Answer::Top(_) => "top",
        };
        assert_eq!(name, answer.name());
        assert!(names.contains(&name), "no schema entry for {name}");
    }
    assert_eq!(answers().len(), 7);
    for extra in ["error", "request", "doctor"] {
        assert!(names.contains(&extra), "no schema entry for {extra}");
    }
    let mut unique = names.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), names.len());
}

#[test]
fn schemas_describe_the_json() {
    for entry in schemas() {
        let v = entry.schema.as_value();
        assert!(v.get("$schema").is_some(), "{}", entry.name);
        assert!(!entry.describes.is_empty());
    }
    let history = schemas().into_iter().find(|e| e.name == "history").unwrap();
    let props = &history.schema.as_value()["properties"];
    for field in ["host", "range", "tier", "bucket_s", "series", "gaps"] {
        assert!(props.get(field).is_some(), "history schema lacks {field}");
    }
}
