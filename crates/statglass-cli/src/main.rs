//! `statglass`: the read-only command-line tool for agents (D-126,
//! `plan/rfc-agent-cli.md`). It asks the running app over `cli/query.sock` and renders
//! the answer for one of three audiences; it never reads storage.
//!
//! Exit codes come from [`statglass_query::ErrorTag::exit_code`]. Ctrl+C uses the
//! default SIGINT action, which the shell reports as 130.

mod audience;
mod cli;
mod doctor;
mod failure;
mod format;
mod guide;
mod parse;
mod query;
mod render;
mod schema_cmd;
mod skill;

use std::{
    ffi::OsString,
    io::{self, Write},
    process::ExitCode,
};

use audience::{Inputs, Style};
use cli::{Cli, Command, SkillAction};
use failure::Failure;

/// What a successful command prints on stdout.
pub enum Output {
    Text(String),
    /// One JSON object, already serialized on one line, fields in declaration order.
    Json(String),
}

impl Output {
    pub fn json(value: &impl serde::Serialize) -> Result<Self, Failure> {
        serde_json::to_string(value)
            .map(Output::Json)
            .map_err(|e| Failure::defect(format!("result did not serialize: {e}")))
    }
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    let style = Inputs::from_process(parse::wants_json(&args)).style();
    let result = match parse::parse(args) {
        Ok(parse::Parsed::Run(cli)) => dispatch(cli, style),
        Ok(parse::Parsed::Print(text)) => Ok(Output::Text(text)),
        Err(f) => Err(f),
    };
    finish(result, style)
}

fn dispatch(cli: Cli, style: Style) -> Result<Output, Failure> {
    let json = style.audience.is_json();
    match cli.command {
        None => guide::run(None, json),
        Some(Command::Guide { topic }) => guide::run(topic.as_deref(), json),
        Some(Command::Doctor) => doctor::run(style),
        Some(Command::Schema { name }) => schema_cmd::run(name.as_deref(), json),
        Some(Command::Skill {
            action: SkillAction::Install { project, .. },
        }) => skill::install(project, json),
        Some(cmd) => query::run(cmd, style),
    }
}

/// Writes the result and picks the exit code. `--json` failures leave stdout empty and
/// end stderr with the envelope; a closed stdout exits 0 quietly.
fn finish(result: Result<Output, Failure>, style: Style) -> ExitCode {
    match result {
        Ok(Output::Text(text)) => write_stdout(&text),
        Ok(Output::Json(line)) => write_stdout(&format!("{line}\n")),
        Err(f) => {
            let mut stderr = io::stderr().lock();
            if style.audience.is_json() {
                let line = serde_json::to_string(&f.envelope()).unwrap_or_else(|_| {
                    r#"{"error":{"tag":"Defect","message":"Unexpected failure; this is a bug.","retryable":false}}"#
                        .to_owned()
                });
                let _ = writeln!(stderr, "{line}");
            } else {
                if let Some(preface) = &f.preface {
                    let _ = write_stdout(preface);
                }
                let _ = stderr.write_all(f.text(style).as_bytes());
            }
            ExitCode::from(f.exit_code())
        }
    }
}

/// Writes all of `text` to stdout. A reader that closed the pipe (`| head`) is not an
/// error: exit 0 without a message.
fn write_stdout(text: &str) -> ExitCode {
    let mut out = io::stdout().lock();
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(1),
    }
}
