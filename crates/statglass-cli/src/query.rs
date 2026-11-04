//! The commands that ask the app: build the [`Op`] from arguments, send it, and turn
//! the answer into output.

use statglass_query::{
    Answer, Body, Client, ErrorDetails, ErrorTag, Op, QueryRequest, REQUEST_TIMEOUT, UsageBy,
    suggest, time,
};
use statglass_schema::Module;

use crate::{
    Output,
    audience::Style,
    cli::{By, Command, RangeArgs},
    failure::Failure,
    format::{self, Clock},
    render,
};

/// Points per series `--json` asks for when `--points` is not given.
const JSON_DEFAULT_POINTS: u32 = 60;

/// Runs a query command. `cmd` is one of the commands that ask the app.
pub fn run(cmd: Command, style: Style) -> Result<Output, Failure> {
    let json = style.audience.is_json();
    let mut show_points = false;
    let mut all_gaps = false;
    let op = match cmd {
        Command::Hosts => Op::Hosts,
        Command::Now { host, all, detail } => Op::Now { host, all, detail },
        Command::Metrics { host, module } => Op::Metrics {
            host,
            module: module.as_deref().map(check_module).transpose()?,
        },
        Command::History {
            series,
            host,
            range,
            points,
            all_gaps: every_gap,
        } => {
            series.iter().try_for_each(|s| check_series(s))?;
            show_points = points.is_some();
            all_gaps = every_gap;
            let (from_ms, to_ms) = resolve_range(&range, format::now_ms())?;
            let points = points.or(Some(if json { JSON_DEFAULT_POINTS } else { 0 }));
            Op::History {
                host,
                series,
                from_ms,
                to_ms,
                points,
            }
        }
        Command::Events { host, range, kind } => {
            let (from_ms, to_ms) = resolve_range(&range, format::now_ms())?;
            Op::Events {
                host,
                from_ms,
                to_ms,
                kind,
            }
        }
        Command::Top {
            host,
            range,
            by,
            limit,
        } => {
            let (from_ms, to_ms) = resolve_range(&range, format::now_ms())?;
            Op::Top {
                host,
                from_ms,
                to_ms,
                by: usage_by(by),
                limit,
            }
        }
        other => return Err(Failure::defect(format!("{other:?} is not a query command"))),
    };
    let answer = send(op)?;
    if json {
        return answer_json(&answer);
    }
    let ctx = render::Ctx {
        style,
        clock: Clock::now(),
        show_points,
        all_gaps,
    };
    Ok(Output::Text(render::answer(&answer, &ctx)))
}

/// Sends one request and returns its answer; error answers become failures.
pub fn send(op: Op) -> Result<Answer, Failure> {
    let asked = op.name();
    let client = Client::discover()?;
    let response = client.request(&QueryRequest::new(op), REQUEST_TIMEOUT)?;
    match response.body {
        Body::Ok(answer) if answer.name() == asked => Ok(answer),
        Body::Ok(answer) => Err(Failure::defect(format!(
            "asked {asked}, StatGlass {} answered {}",
            response.app_version,
            answer.name()
        ))),
        Body::Error(e) => Err(e.into()),
    }
}

/// The inner answer object, without the `op` tag: what `statglass schema <op>` describes.
fn answer_json(answer: &Answer) -> Result<Output, Failure> {
    match answer {
        Answer::Status(a) => Output::json(a),
        Answer::Hosts(a) => Output::json(a),
        Answer::Now(a) => Output::json(a),
        Answer::Metrics(a) => Output::json(a),
        Answer::History(a) => Output::json(a),
        Answer::Events(a) => Output::json(a),
        Answer::Top(a) => Output::json(a),
    }
}

fn usage_by(by: By) -> UsageBy {
    match by {
        By::Cpu => UsageBy::Cpu,
        By::Memory => UsageBy::Memory,
        By::Energy => UsageBy::Energy,
    }
}

/// `--module` names a known module.
fn check_module(name: &str) -> Result<String, Failure> {
    let lower = name.to_lowercase();
    if Module::parse(&lower).is_some() {
        return Ok(lower);
    }
    Err(Failure::usage(
        Some(name),
        "unknown module",
        suggest::closest(name, Module::ALL.iter().map(|m| m.as_str())),
    ))
}

/// A series argument with `=` but no braces is what the shell's brace expansion leaves
/// of an unquoted `metric{a=b,c=d}`: refuse it with the quoting fix rather than asking.
fn check_series(arg: &str) -> Result<(), Failure> {
    if arg.contains('=') && !arg.contains('{') {
        return Err(Failure::new(
            ErrorTag::UnknownMetric,
            ErrorDetails {
                series: Some(arg.to_owned()),
                hint: Some(
                    "this looks like an unquoted series the shell split at its braces; \
                     single-quote it: 'metric{key=value,key=value}'"
                        .to_owned(),
                ),
                ..ErrorDetails::default()
            },
        ));
    }
    Ok(())
}

/// `--since` (default 1h), or `--from`/`--to` (`--to` defaults to now, `--from` to an
/// hour before `--to`). clap already refuses `--since` with either.
pub fn resolve_range(args: &RangeArgs, now_ms: i64) -> Result<(i64, i64), Failure> {
    let instant =
        |arg: &str| time::parse_instant(arg, now_ms).map_err(|e| bad_range(arg, e.to_string()));
    let to = args
        .to
        .as_deref()
        .map(instant)
        .transpose()?
        .unwrap_or(now_ms);
    let from = match (&args.from, &args.since) {
        (Some(from), _) => instant(from)?,
        (None, since) => {
            let since = since.as_deref().unwrap_or("1h");
            let back = time::parse_duration(since).map_err(|e| bad_range(since, e.to_string()))?;
            to.saturating_sub(back)
        }
    };
    if from >= to {
        let input = args.from.as_deref().unwrap_or_default();
        return Err(bad_range(
            input,
            "the start must be before the end".to_owned(),
        ));
    }
    Ok((from, to))
}

fn bad_range(input: &str, hint: String) -> Failure {
    Failure::new(
        ErrorTag::BadRange,
        ErrorDetails {
            input: (!input.is_empty()).then(|| input.to_owned()),
            hint: Some(hint),
            ..ErrorDetails::default()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(since: Option<&str>, from: Option<&str>, to: Option<&str>) -> RangeArgs {
        RangeArgs {
            since: since.map(str::to_owned),
            from: from.map(str::to_owned),
            to: to.map(str::to_owned),
        }
    }

    const NOW: i64 = 1_800_000_000_000;
    const HOUR: i64 = 3_600_000;

    #[test]
    fn default_is_the_last_hour() {
        assert_eq!(
            resolve_range(&range(None, None, None), NOW).unwrap(),
            (NOW - HOUR, NOW)
        );
        assert_eq!(
            resolve_range(&range(Some("2h"), None, None), NOW).unwrap(),
            (NOW - 2 * HOUR, NOW)
        );
        assert_eq!(
            resolve_range(&range(None, Some("3h"), Some("1h")), NOW).unwrap(),
            (NOW - 3 * HOUR, NOW - HOUR)
        );
        assert_eq!(
            resolve_range(&range(None, None, Some("1h")), NOW).unwrap(),
            (NOW - 2 * HOUR, NOW - HOUR)
        );
    }

    #[test]
    fn bad_times_and_inverted_ranges_are_bad_range() {
        for r in [
            range(Some("2w"), None, None),
            range(None, Some("yesterday"), None),
            range(None, Some("1h"), Some("3h")),
        ] {
            let f = resolve_range(&r, NOW).unwrap_err();
            assert_eq!(f.error.tag, ErrorTag::BadRange);
        }
    }

    #[test]
    fn brace_leftovers_ask_for_quotes() {
        let f = check_series("cgroup.cpukind=container").unwrap_err();
        assert_eq!(f.error.tag, ErrorTag::UnknownMetric);
        assert!(check_series("cgroup.cpu{kind=container,name=pg}").is_ok());
        assert!(check_series("cpu.total").is_ok());
        let f = check_module("dsk").unwrap_err();
        assert_eq!(f.error.details.unwrap().suggestions, vec!["disk"]);
    }
}
