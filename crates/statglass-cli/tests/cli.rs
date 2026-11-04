//! The real binary against a fake query socket: every exit path, the stdout/stderr
//! contract, `--json` decoding and the agent text shapes.
// Test helpers outside #[test] functions are not covered by clippy.toml's test allowance.
#![allow(clippy::unwrap_used)]

mod support;

use statglass_query::{
    Answer, ErrorDetails, ErrorTag, EventDto, EventsAnswer, Freshness, GapDto, HistoryAnswer,
    HostDto, HostKind, HostRef, HostState, HostsAnswer, MetricDto, MetricsAnswer, NowAnswer,
    NowHost, Op, QueryError, Range, Reading, SeriesHistory, Summary, TopAnswer, TopApp, UsageBy,
    time::rfc3339_local,
};
use support::{Fake, Reply, err, ok, run_with};

/// `HH:MM` today as RFC 3339 in this Mac's offset, as the app would write it.
fn today(hhmm: &str) -> String {
    let now_s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let now = rfc3339_local(now_s * 1000);
    format!("{}T{hhmm}:00{}", &now[..10], &now[19..])
}

fn host_ref(name: &str) -> HostRef {
    HostRef {
        id: format!("id-{name}"),
        name: name.into(),
    }
}

fn range() -> Range {
    Range {
        from: today("13:00"),
        to: today("16:00"),
        retained_from: None,
    }
}

fn hosts_answer() -> HostsAnswer {
    HostsAnswer {
        hosts: vec![
            HostDto {
                id: "local".into(),
                name: "studio".into(),
                kind: HostKind::ThisMac,
                state: HostState::Local,
                last_seen: None,
                sync: None,
                synced_through: None,
                retained_from: Some(today("01:00")),
                retained_to: Some(today("16:00")),
                os_version: None,
                model: None,
                chip: None,
            },
            HostDto {
                id: "id-m1".into(),
                name: "m1-linux".into(),
                kind: HostKind::Linux,
                state: HostState::Offline,
                last_seen: Some(today("09:12")),
                sync: None,
                synced_through: Some(today("09:12")),
                retained_from: Some(today("02:00")),
                retained_to: Some(today("09:12")),
                os_version: None,
                model: None,
                chip: None,
            },
        ],
    }
}

fn history_answer() -> HistoryAnswer {
    HistoryAnswer {
        host: host_ref("m1-linux"),
        freshness: Some(Freshness {
            state: HostState::Connected,
            last_seen: None,
            synced_through: Some(today("16:00")),
        }),
        range: range(),
        tier: "m1".into(),
        bucket_s: 60,
        series: vec![SeriesHistory {
            key: "cpu.total".into(),
            unit: "percent".into(),
            summary: Summary {
                min: Some(2.1),
                max: Some(97.0),
                max_at: Some(today("15:12")),
                max_bucket_s: Some(60),
                mean: Some(18.4),
                mean_approximate: true,
                last: Some(6.3),
                last_at: Some(today("16:00")),
                measured_fraction: 0.98,
            },
            points: Vec::new(),
        }],
        gaps: vec![GapDto {
            from: today("14:31"),
            to: Some(today("14:37")),
            reason: "source_offline".into(),
            module: None,
        }],
    }
}

fn now_answer() -> NowAnswer {
    NowAnswer {
        hosts: vec![NowHost {
            host: host_ref("studio"),
            freshness: None,
            frame_at: Some(today("15:59")),
            age_ms: Some(800),
            readings: vec![Reading {
                series: "cpu.total".into(),
                value: 12.5,
                unit: "percent".into(),
                module: "cpu".into(),
            }],
            error: None,
        }],
    }
}

fn metrics_answer() -> MetricsAnswer {
    MetricsAnswer {
        host: None,
        metrics: vec![MetricDto {
            id: "cpu.load".into(),
            unit: "percent".into(),
            kind: "mean".into(),
            labels: vec!["core".into()],
            module: "cpu".into(),
            persisted: true,
            period_s: 1,
            max_cardinality: Some(64),
        }],
        series: None,
    }
}

fn events_answer() -> EventsAnswer {
    EventsAnswer {
        host: host_ref("studio"),
        freshness: None,
        range: range(),
        events: vec![EventDto {
            at: today("15:12"),
            started: None,
            kind: "thermal_state".into(),
            detail: serde_json::json!({ "state": "serious" }),
            processes: vec![statglass_query::EventProcess {
                name: "clang".into(),
                pid: None,
                value: None,
                unit: None,
            }],
        }],
        unreadable: 2,
    }
}

fn top_answer() -> TopAnswer {
    TopAnswer {
        host: host_ref("studio"),
        freshness: None,
        range: range(),
        by: UsageBy::Cpu,
        limit: 10,
        limit_reduced: false,
        apps: vec![TopApp {
            name: "Xcode".into(),
            value: 212.0,
            unit: "percent".into(),
            share: Some(0.41),
            processes: Vec::new(),
        }],
    }
}

fn assert_failure(run: &support::Run, code: i32, tag: ErrorTag) -> statglass_query::CliError {
    assert_eq!(run.code, code, "stderr: {}", run.stderr);
    assert!(run.stdout.is_empty(), "stdout on failure: {}", run.stdout);
    let env = run.envelope();
    assert_eq!(env.error.tag, tag);
    assert_eq!(env.error.retryable, tag == ErrorTag::Busy);
    assert!(env.error.next.is_some());
    env.error
}

#[test]
fn root_prints_the_guide() {
    let run = run_with(&[], |_| {});
    assert_eq!(run.code, 0);
    assert!(
        run.stdout.starts_with("statglass asks StatGlass"),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains("Rules:"));
    let run = run_with(&["--json"], |_| {});
    let v: serde_json::Value = run.json();
    assert!(v["guide"].as_str().unwrap().contains("The loop:"));
    let run = run_with(&["--help"], |_| {});
    assert_eq!(run.code, 0);
    assert!(run.stdout.contains("Start here (agents):"));
    assert!(run.stdout.contains("Usage: statglass "), "{}", run.stdout);
}

#[test]
fn every_subcommand_help_has_exits_and_examples() {
    let commands: [&[&str]; 11] = [
        &["guide"],
        &["hosts"],
        &["now"],
        &["metrics"],
        &["history"],
        &["events"],
        &["top"],
        &["doctor"],
        &["schema"],
        &["skill"],
        &["skill", "install"],
    ];
    for cmd in commands {
        let mut args = cmd.to_vec();
        args.push("--help");
        let run = run_with(&args, |_| {});
        assert_eq!(run.code, 0, "{cmd:?}");
        assert!(run.stdout.contains("EXAMPLES"), "{cmd:?}: {}", run.stdout);
        assert!(run.stdout.contains("Exits "), "{cmd:?}: {}", run.stdout);
        if cmd != ["skill"] {
            assert!(
                run.stdout.contains("--json\n"),
                "{cmd:?} has no --json example"
            );
        }
    }
}

#[test]
fn typo_is_usage_with_suggestions_in_text_and_json() {
    let run = run_with(&["hostz"], |_| {});
    assert_eq!(run.code, 2);
    assert!(run.stdout.is_empty());
    assert!(
        run.stderr.contains("  Did you mean: hosts?\n"),
        "{}",
        run.stderr
    );
    assert!(
        run.stderr
            .lines()
            .last()
            .unwrap()
            .starts_with("  Next: run statglass --help")
    );
    let run = run_with(&["hostz", "--json"], |_| {});
    let e = assert_failure(&run, 2, ErrorTag::Usage);
    assert_eq!(e.details.unwrap().suggestions, vec!["hosts"]);
}

#[test]
fn every_query_answer_decodes_from_json_stdout() {
    let fake = Fake::serving(ok(Answer::Hosts(hosts_answer())));
    let run = fake.run(&["hosts", "--json"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.json::<HostsAnswer>(), hosts_answer());
    assert_eq!(fake.request().op, Op::Hosts);

    let fake = Fake::serving(ok(Answer::Now(now_answer())));
    assert_eq!(
        fake.run(&["now", "--json"]).json::<NowAnswer>(),
        now_answer()
    );
    let fake = Fake::serving(ok(Answer::Metrics(metrics_answer())));
    assert_eq!(
        fake.run(&["metrics", "--json"]).json::<MetricsAnswer>(),
        metrics_answer()
    );
    let fake = Fake::serving(ok(Answer::Events(events_answer())));
    assert_eq!(
        fake.run(&["events", "--json"]).json::<EventsAnswer>(),
        events_answer()
    );
    let fake = Fake::serving(ok(Answer::Top(top_answer())));
    let run = fake.run(&["top", "--by", "memory", "--limit", "5", "--json"]);
    assert_eq!(run.json::<TopAnswer>(), top_answer());
    match fake.request().op {
        Op::Top { by, limit, .. } => assert_eq!((by, limit), (UsageBy::Memory, 5)),
        other => panic!("{other:?}"),
    }

    let fake = Fake::serving(ok(Answer::History(history_answer())));
    let run = fake.run(&["history", "cpu.total", "--json"]);
    assert_eq!(run.json::<HistoryAnswer>(), history_answer());
    match fake.request().op {
        Op::History {
            series,
            points,
            from_ms,
            to_ms,
            host,
        } => {
            assert_eq!(series, vec!["cpu.total"]);
            assert_eq!(points, Some(60));
            assert_eq!(to_ms - from_ms, 3_600_000);
            assert_eq!(host, None);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn history_agent_text_matches_the_rfc_shape() {
    let fake = Fake::serving(ok(Answer::History(history_answer())));
    let run = fake.run(&[
        "history",
        "cpu.total",
        "--host",
        "m1-linux",
        "--from",
        "13:00",
        "--to",
        "16:00",
    ]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        run.stdout,
        "cpu.total on m1-linux, 13:00 to 16:00 (1 min buckets, 98% measured)\n\
         min 2.1%  mean 18.4%  max 97.0% in the minute from 15:12  last 6.3% at 16:00  (mean approximate)\n\
         gap 14:31 to 14:37: host offline\n"
    );
    assert!(!run.stdout.contains('\x1b'));
    match fake.request().op {
        Op::History { points, host, .. } => {
            assert_eq!(points, Some(0), "text asks for no points by default");
            assert_eq!(host.as_deref(), Some("m1-linux"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn other_agent_text_shapes() {
    let fake = Fake::serving(ok(Answer::Hosts(hosts_answer())));
    let out = fake.run(&["hosts"]).stdout;
    assert!(
        out.contains("studio  this Mac  local  history 01:00 to 16:00\n"),
        "{out}"
    );
    assert!(
        out.contains("m1-linux  Linux  offline  last seen 09:12, synced through 09:12"),
        "{out}"
    );

    let fake = Fake::serving(ok(Answer::Events(events_answer())));
    let out = fake.run(&["events"]).stdout;
    assert!(
        out.contains("15:12 thermal_state state=serious; processes: clang\n"),
        "{out}"
    );
    assert!(
        out.contains("2 events from a newer StatGlass could not be read"),
        "{out}"
    );

    let fake = Fake::serving(ok(Answer::Top(top_answer())));
    let out = fake.run(&["top"]).stdout;
    assert!(
        out.contains("top 1 apps by average CPU on studio, 13:00 to 16:00\n"),
        "{out}"
    );
    assert!(out.contains("Xcode  212.0%  41% of host\n"), "{out}");

    let fake = Fake::serving(ok(Answer::Now(now_answer())));
    let out = fake.run(&["now"]).stdout;
    assert!(
        out.starts_with("studio at 15:59:00 (0.8s old)\ncpu\n  cpu.total  12.5%\n"),
        "{out}"
    );
}

#[test]
fn a_range_reaching_past_today_dates_every_time() {
    // The week before today: a bare 15:12 next to dated times would read as an earlier day.
    let week_ago = rfc3339_local(
        (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64)
            - 7 * 86_400_000,
    );
    let mut answer = events_answer();
    answer.range.from = week_ago;
    let fake = Fake::serving(ok(Answer::Events(answer)));
    let out = fake.run(&["events", "--since", "7d"]).stdout;
    let event = out.lines().nth(1).unwrap_or_default();
    assert!(
        event.contains(" 15:12 thermal_state") && !event.starts_with("15:12"),
        "{out}"
    );
}

#[test]
fn metrics_for_a_host_lists_only_stored_metrics_with_capped_series() {
    let mut answer = metrics_answer();
    answer.host = Some(host_ref("m1-linux"));
    answer.metrics.push(MetricDto {
        id: "battery.charge".into(),
        unit: "percent".into(),
        kind: "gauge".into(),
        labels: Vec::new(),
        module: "battery".into(),
        persisted: true,
        period_s: 10,
        max_cardinality: None,
    });
    answer.series = Some((0..9).map(|c| format!("cpu.load{{core={c}}}")).collect());
    let fake = Fake::serving(ok(Answer::Metrics(answer)));
    let out = fake.run(&["metrics", "--host", "m1-linux"]).stdout;
    assert!(
        out.starts_with("9 series of 1 metrics stored on m1-linux\ncpu\n  cpu.load{core}"),
        "{out}"
    );
    assert!(out.contains("    cpu.load{core=5}\n"), "{out}");
    assert!(!out.contains("cpu.load{core=6}"), "{out}");
    assert!(
        out.contains("    and 3 more (--json lists every one)\n"),
        "{out}"
    );
    assert!(!out.contains("battery.charge"), "{out}");
    assert!(
        out.ends_with("1 catalog metrics have no stored series on m1-linux; statglass metrics lists the catalog\n"),
        "{out}"
    );
}

#[test]
fn a_gap_within_one_minute_prints_seconds() {
    let mut answer = history_answer();
    answer.gaps[0].from = today("14:31")
        .replace(":00-", ":05-")
        .replace(":00+", ":05+");
    answer.gaps[0].to = Some(
        today("14:31")
            .replace(":00-", ":40-")
            .replace(":00+", ":40+"),
    );
    let fake = Fake::serving(ok(Answer::History(answer)));
    let out = fake.run(&["history", "cpu.total"]).stdout;
    assert!(
        out.contains("gap 14:31:05 to 14:31:40: host offline\n"),
        "{out}"
    );
}

#[test]
fn unknown_host_from_the_app_is_exit_2_with_details() {
    let error = QueryError::new(ErrorTag::UnknownHost).with_details(ErrorDetails {
        host: Some("m1-linx".into()),
        suggestions: vec!["m1-linux".into()],
        ..ErrorDetails::default()
    });
    let fake = Fake::serving(err(error.clone()));
    let run = fake.run(&["now", "--host", "m1-linx"]);
    assert_eq!(run.code, 2);
    assert!(run.stdout.is_empty());
    assert!(run.stderr.contains("  Host: m1-linx\n"), "{}", run.stderr);
    assert!(run.stderr.contains("  Did you mean: m1-linux?\n"));
    assert!(
        run.stderr
            .ends_with("  Next: run statglass hosts to see host names\n")
    );

    let fake = Fake::serving(err(error));
    let e = assert_failure(
        &fake.run(&["now", "--host", "m1-linx", "--json"]),
        2,
        ErrorTag::UnknownHost,
    );
    assert_eq!(e.details.unwrap().suggestions, vec!["m1-linux"]);
}

#[test]
fn busy_is_exit_4_and_retryable() {
    let fake = Fake::serving(err(QueryError::new(ErrorTag::Busy)));
    assert_failure(&fake.run(&["hosts", "--json"]), 4, ErrorTag::Busy);
}

#[test]
fn app_states_without_a_query_socket_are_exit_6() {
    let fake = Fake::empty();
    assert_failure(&fake.run(&["hosts", "--json"]), 6, ErrorTag::AppNotRunning);
    let run = fake.run(&["hosts"]);
    assert!(run.stderr.contains("open -a StatGlass"), "{}", run.stderr);

    let fake = Fake::control_only();
    assert_failure(&fake.run(&["hosts", "--json"]), 6, ErrorTag::AccessDisabled);

    let missing = run_with(&["hosts", "--json"], |c| {
        c.env("STATGLASS_APP_DATA", "/nonexistent/statglass-test");
    });
    assert_failure(&missing, 6, ErrorTag::AppNotInstalled);
}

#[test]
fn garbage_frame_is_a_defect() {
    let fake = Fake::serving(Reply::Garbage);
    assert_failure(&fake.run(&["hosts", "--json"]), 1, ErrorTag::Defect);
}

#[test]
fn client_side_input_errors_never_connect() {
    let fake = Fake::empty();
    assert_failure(
        &fake.run(&["history", "cpu.total", "--since", "2w", "--json"]),
        2,
        ErrorTag::BadRange,
    );
    let e = assert_failure(
        &fake.run(&["history", "cgroup.cpukind=container", "--json"]),
        2,
        ErrorTag::UnknownMetric,
    );
    assert!(e.details.unwrap().hint.unwrap().contains("single-quote"));
    assert_failure(
        &fake.run(&["history", "cpu.total", "--points", "5000", "--json"]),
        2,
        ErrorTag::Usage,
    );
    assert_failure(
        &fake.run(&["metrics", "--module", "dsk", "--json"]),
        2,
        ErrorTag::Usage,
    );
}

#[test]
fn doctor_reports_problems_with_exit_0() {
    let fake = Fake::control_only();
    let run = fake.run(&["doctor", "--json"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let report: statglass_query::DoctorReport = run.json();
    assert_eq!(report.access, statglass_query::AccessState::Off);
    assert!(report.app_running);
    assert_eq!(report.problems[0].tag, ErrorTag::AccessDisabled);
    assert!(
        report.problems[0]
            .next
            .as_deref()
            .unwrap()
            .contains("Allow command-line access")
    );
    let run = fake.run(&["doctor"]);
    assert!(
        run.stdout.contains("problem AccessDisabled"),
        "{}",
        run.stdout
    );

    let fake = Fake::serving(Reply::Garbage);
    let run = fake.run(&["doctor", "--json"]);
    assert_failure(&run, 1, ErrorTag::Defect);
}

#[test]
fn doctor_reports_access_on_when_the_app_could_not_bind() {
    let fake = Fake::control_only();
    let cli = fake.app_data().join("cli");
    std::fs::remove_file(cli.join("access-disabled")).unwrap();
    std::fs::write(cli.join("bind-failed"), "socket already serving").unwrap();
    let run = fake.run(&["doctor", "--json"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let report: statglass_query::DoctorReport = run.json();
    assert_eq!(report.problems[0].tag, ErrorTag::AccessUnavailable);
    assert_eq!(report.access, statglass_query::AccessState::On);
}

#[test]
fn doctor_reports_an_unsafe_socket_as_a_problem() {
    use std::os::unix::fs::PermissionsExt;
    let fake = Fake::serving(Reply::Garbage);
    let socket = fake.app_data().join("cli/query.sock");
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o666)).unwrap();
    let run = fake.run(&["doctor", "--json"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let report: statglass_query::DoctorReport = run.json();
    let [problem] = report.problems.as_slice() else {
        panic!("{:?}", report.problems)
    };
    assert_eq!(problem.tag, ErrorTag::AccessUnavailable);
    // Nothing says whether access is on.
    assert_eq!(report.access, statglass_query::AccessState::Unknown);
    let detail = problem.detail.as_deref().unwrap();
    assert!(detail.contains("socket mode is not 0600"), "{detail}");
    assert!(detail.contains("query.sock"), "{detail}");
    assert_failure(
        &fake.run(&["hosts", "--json"]),
        6,
        ErrorTag::AccessUnavailable,
    );
}

#[test]
fn schema_lists_prints_and_suggests() {
    let run = run_with(&["schema"], |_| {});
    assert_eq!(run.code, 0);
    assert!(
        run.stdout.contains("history  statglass history --json\n"),
        "{}",
        run.stdout
    );
    assert!(
        run.stdout.contains("doctor   statglass doctor --json\n"),
        "{}",
        run.stdout
    );
    let run = run_with(&["schema", "doctor"], |_| {});
    let v: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(v["title"], "DoctorReport");
    let run = run_with(&["schema", "history"], |_| {});
    let v: serde_json::Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(v["title"], "HistoryAnswer");
    let e = assert_failure(
        &run_with(&["schema", "histroy", "--json"], |_| {}),
        2,
        ErrorTag::Usage,
    );
    assert_eq!(e.details.unwrap().suggestions, vec!["history"]);
}

#[test]
fn skill_install_creates_then_leaves_unchanged() {
    let home = tempfile::tempdir().unwrap();
    let install = || {
        run_with(&["skill", "install"], |c| {
            c.env("HOME", home.path());
        })
    };
    let first = install();
    assert_eq!(first.code, 0, "{}", first.stderr);
    assert_eq!(
        first.stdout.matches("created ").count(),
        2,
        "{}",
        first.stdout
    );
    let skill = home.path().join(".claude/skills/statglass/SKILL.md");
    assert!(
        std::fs::read_to_string(&skill)
            .unwrap()
            .contains("name: statglass")
    );
    assert!(
        home.path()
            .join(".claude/skills/statglass/reference/statglass.md")
            .is_file()
    );
    let second = install();
    assert_eq!(
        second.stdout.matches("unchanged ").count(),
        2,
        "{}",
        second.stdout
    );
    std::fs::write(&skill, "old").unwrap();
    assert!(
        install()
            .stdout
            .contains(&format!("updated {}", skill.display()))
    );

    let project = tempfile::tempdir().unwrap();
    let run = run_with(&["skill", "install", "--project", "--json"], |c| {
        c.current_dir(project.path());
    });
    let v: serde_json::Value = run.json();
    assert_eq!(v["files"][0]["status"], "created");
    assert!(
        project
            .path()
            .join(".claude/skills/statglass/SKILL.md")
            .is_file()
    );
}

#[cfg(unix)]
#[test]
fn skill_install_into_an_unwritable_directory_is_a_usage_error_naming_the_path() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    std::fs::set_permissions(home.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    let run = run_with(&["skill", "install", "--json"], |c| {
        c.env("HOME", home.path());
    });
    std::fs::set_permissions(home.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let e = assert_failure(&run, 2, ErrorTag::Usage);
    let skill = home.path().join(".claude/skills/statglass/SKILL.md");
    let details = e.details.unwrap();
    assert_eq!(
        details.input.as_deref(),
        Some(&*skill.display().to_string())
    );
    let hint = details.hint.unwrap();
    assert!(hint.contains("check permissions"), "{hint}");
}

#[test]
fn now_asks_for_the_headline_set_unless_detail_is_given() {
    for (args, want) in [(&["now"][..], false), (&["now", "--detail"][..], true)] {
        let fake = Fake::serving(ok(Answer::Now(now_answer())));
        assert_eq!(fake.run(args).code, 0);
        match fake.request().op {
            Op::Now { detail, .. } => assert_eq!(detail, want, "{args:?}"),
            other => panic!("{other:?}"),
        }
    }
}

fn gap(from: &str, to: &str, reason: &str) -> GapDto {
    GapDto {
        from: today(from),
        to: Some(today(to)),
        reason: reason.into(),
        module: None,
    }
}

/// Eight gaps: five long ones and three short sleeps, the 15:50 one still open.
fn gappy_history() -> HistoryAnswer {
    let mut a = history_answer();
    a.gaps = vec![
        gap("13:00", "13:20", "no_data"),
        gap("13:30", "13:31", "sleep"),
        gap("13:40", "14:00", "sleep"),
        gap("14:10", "14:12", "sleep"),
        gap("14:20", "14:40", "source_offline"),
        gap("14:50", "14:53", "sleep"),
        gap("15:00", "15:30", "sleep"),
        GapDto {
            to: None,
            ..gap("15:50", "15:50", "not_synced")
        },
    ];
    a
}

#[test]
fn history_prints_the_longest_gaps_and_sums_up_the_rest() {
    let fake = Fake::serving(ok(Answer::History(gappy_history())));
    let run = fake.run(&["history", "cpu.total", "--from", "13:00", "--to", "16:00"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let gaps: Vec<&str> = run.stdout.lines().skip(2).collect();
    assert_eq!(
        gaps,
        [
            "gap 13:00 to 13:20: no data recorded",
            "gap 13:40 to 14:00: sleep",
            "gap 14:20 to 14:40: host offline",
            "gap 15:00 to 15:30: sleep",
            "gap 15:50 to now (open): not synced yet",
            "3 more gaps, 6 min total (sleep: 3); --all-gaps lists every one",
        ]
    );

    let fake = Fake::serving(ok(Answer::History(gappy_history())));
    let run = fake.run(&["history", "cpu.total", "--all-gaps"]);
    assert_eq!(run.stdout.matches("\ngap ").count(), 8, "{}", run.stdout);
    assert!(!run.stdout.contains("more gaps"));

    let fake = Fake::serving(ok(Answer::History(gappy_history())));
    let run = fake.run(&["history", "cpu.total", "--json"]);
    assert_eq!(
        run.json::<HistoryAnswer>().gaps.len(),
        8,
        "JSON keeps every gap"
    );
}

#[test]
fn top_text_caps_processes_per_app_and_says_where_usage_starts() {
    let mut answer = top_answer();
    answer.range.retained_from = Some(today("13:20"));
    answer.apps[0].processes = (1..=5)
        .map(|n| statglass_query::TopProcess {
            name: format!("node{n}"),
            pid: Some(100 + n),
            value: 40.0 - f64::from(n),
        })
        .collect();
    let fake = Fake::serving(ok(Answer::Top(answer.clone())));
    let out = fake.run(&["top"]).stdout;
    assert!(
        out.starts_with(
            "usage data starts 13:20 (StatGlass started then, or the host's usage history begins then)\n"
        ),
        "{out}"
    );
    assert!(out.contains("  node3 (pid 103)  37.0%\n"), "{out}");
    assert!(!out.contains("node4"), "{out}");
    assert!(out.ends_with("  and 2 more\n"), "{out}");
    assert!(!out.contains("clamped"), "{out}");

    let fake = Fake::serving(ok(Answer::Top(answer.clone())));
    let json: TopAnswer = fake.run(&["top", "--json"]).json();
    assert_eq!(json.apps[0].processes.len(), 5, "JSON keeps every process");

    answer.range.retained_from = None;
    let fake = Fake::serving(ok(Answer::Top(answer)));
    assert!(!fake.run(&["top"]).stdout.contains("usage data starts"));
}
