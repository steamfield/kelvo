//! `statglass doctor`: why the CLI can or cannot reach StatGlass. Problems are findings
//! with their next step, not failures; only a defect fails the command.

use std::os::unix::net::UnixStream;

use statglass_query::{
    AccessState, Answer, Client, DoctorProblem, DoctorReport as Findings, ErrorTag, Op, time,
};

use crate::{
    Output,
    audience::Style,
    failure::{Failure, next_for},
    format, query,
};

pub fn run(style: Style) -> Result<Output, Failure> {
    let client = Client::discover()?;
    let paths = client.paths();
    let control_answers = UnixStream::connect(paths.control_socket()).is_ok();
    let mut findings = Findings {
        cli_version: env!("CARGO_PKG_VERSION").to_owned(),
        executable: std::env::current_exe()
            .ok()
            .map(|p| p.display().to_string()),
        bundle_id: paths
            .app_data()
            .file_name()
            .map(|n| n.to_string_lossy().into_owned()),
        app_data: paths.app_data().display().to_string(),
        app_data_exists: paths.app_data().is_dir(),
        query_socket: paths.query_socket().display().to_string(),
        query_socket_exists: paths.query_socket().exists(),
        app_running: control_answers,
        access: AccessState::Unknown,
        timezone: time::timezone_name(),
        app: None,
        problems: Vec::new(),
    };
    match query::send(Op::Status) {
        Ok(Answer::Status(status)) => {
            findings.app_running = true;
            findings.access = AccessState::On;
            findings.app = Some(status);
        }
        Ok(other) => {
            return Err(Failure::defect(format!(
                "status answered as {}",
                other.name()
            )));
        }
        Err(f) if matches!(f.error.tag, ErrorTag::Defect | ErrorTag::Unknown) => {
            return Err(Failure {
                preface: Some(text(&findings, style)),
                ..f
            });
        }
        Err(f) => {
            match f.error.tag {
                ErrorTag::AccessDisabled => findings.access = AccessState::Off,
                // The app writes this marker only while access is on and binding fails;
                // an unsafe socket or a refused connect says nothing about the setting.
                ErrorTag::AccessUnavailable if paths.bind_failed_marker().exists() => {
                    findings.access = AccessState::On;
                }
                _ => {}
            }
            findings.problems.push(DoctorProblem {
                tag: f.error.tag,
                message: f.error.message,
                next: Some(next_for(f.error.tag).to_owned()),
                detail: f.error.details.and_then(|d| d.hint),
            });
        }
    }
    if style.audience.is_json() {
        return Output::json(&findings);
    }
    Ok(Output::Text(text(&findings, style)))
}

fn text(f: &Findings, style: Style) -> String {
    let yes_no = |b: bool| if b { "yes" } else { "no" };
    let found = |b: bool| if b { "exists" } else { "missing" };
    let mut rows = vec![
        vec![
            "statglass".to_owned(),
            format!(
                "{} ({})",
                f.cli_version,
                f.executable.as_deref().unwrap_or("unknown path")
            ),
        ],
        vec![
            "bundle id".to_owned(),
            f.bundle_id.clone().unwrap_or_else(|| "unknown".to_owned()),
        ],
        vec![
            "app data".to_owned(),
            format!("{} ({})", f.app_data, found(f.app_data_exists)),
        ],
        vec![
            "query socket".to_owned(),
            format!("{} ({})", f.query_socket, found(f.query_socket_exists)),
        ],
        vec!["running".to_owned(), yes_no(f.app_running).to_owned()],
        vec!["access".to_owned(), crate::render::snake(&f.access)],
        vec!["timezone".to_owned(), f.timezone.clone()],
    ];
    if let Some(app) = &f.app {
        rows.push(vec![
            "StatGlass".to_owned(),
            format!(
                "{}, api {}, {} hosts",
                app.app_version, app.api, app.host_count
            ),
        ]);
    }
    let mut out = format::table(&rows, true, "");
    if f.problems.is_empty() {
        out.push_str("no problems found\n");
    }
    for p in &f.problems {
        out.push_str(&style.bold(&format!("problem {:?}: {}", p.tag, p.message)));
        out.push('\n');
        if let Some(detail) = &p.detail {
            out.push_str(&format!("  Detail: {detail}\n"));
        }
        if let Some(next) = &p.next {
            out.push_str(&format!("  Next: {next}\n"));
        }
    }
    out
}
