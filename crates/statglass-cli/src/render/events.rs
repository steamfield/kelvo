use serde_json::Value;
use statglass_query::{EventDto, EventsAnswer, EventsSummary, KindSummary};

use super::{Ctx, freshness_line, range_clock, range_text, retained_line};
use crate::format::Clock;

/// At most this many detail fields per event line; the rest are in `--json`.
const MAX_DETAIL_FIELDS: usize = 6;

pub fn render(a: &EventsAnswer, ctx: &Ctx) -> String {
    let clock = &range_clock(&a.range, &ctx.clock);
    let mut out = String::new();
    if let Some(line) = freshness_line(&a.host.name, a.freshness.as_ref(), clock) {
        out.push_str(&line);
    }
    if let Some(line) = retained_line(&a.range, clock) {
        out.push_str(&line);
    }
    let range = range_text(&a.range, clock);
    let count = match a.events.len() {
        0 => "no events".to_owned(),
        1 => "1 event".to_owned(),
        n => format!("{n} events"),
    };
    out.push_str(
        &ctx.style
            .bold(&format!("{count} on {}, {range}", a.host.name)),
    );
    out.push('\n');
    for e in &a.events {
        out.push_str(&event_line(e, clock));
    }
    if a.unreadable > 0 {
        out.push_str(&format!(
            "{} events from a newer StatGlass could not be read\n",
            a.unreadable
        ));
    }
    out
}

/// `15:12 thermal_state state=serious; processes: clang, rustc`.
fn event_line(e: &EventDto, clock: &Clock) -> String {
    let mut line = clock.short(&e.at);
    if let Some(started) = &e.started {
        line.push_str(&format!(" (from {})", clock.short(started)));
    }
    line.push(' ');
    line.push_str(&e.kind);
    let detail = detail_summary(&e.detail);
    if !detail.is_empty() {
        line.push(' ');
        line.push_str(&detail);
    }
    if !e.processes.is_empty() {
        let names: Vec<&str> = e.processes.iter().map(|p| p.name.as_str()).collect();
        line.push_str(&format!("; processes: {}", names.join(", ")));
    }
    line.push('\n');
    line
}

/// The detail object's scalar fields as `key=value`, in the order the app sent them.
/// Nested objects and arrays are left to `--json`.
fn detail_summary(detail: &Value) -> String {
    let scalar = |v: &Value| match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(
            n.as_f64()
                .map_or_else(|| n.to_string(), crate::format::number),
        ),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    };
    match detail {
        Value::Object(map) => map
            .iter()
            .filter_map(|(k, v)| scalar(v).map(|v| format!("{k}={v}")))
            .take(MAX_DETAIL_FIELDS)
            .collect::<Vec<_>>()
            .join(" "),
        other => scalar(other).unwrap_or_default(),
    }
}

/// Days printed per kind before the rest are counted; `--json` has every day.
const MAX_SUMMARY_DAYS: usize = 14;

/// `events --summary`: one block per kind, most frequent first.
pub fn summary(a: &EventsSummary, ctx: &Ctx) -> String {
    let clock = &range_clock(&a.range, &ctx.clock);
    let mut out = String::new();
    if let Some(line) = freshness_line(&a.host.name, a.freshness.as_ref(), clock) {
        out.push_str(&line);
    }
    if let Some(line) = retained_line(&a.range, clock) {
        out.push_str(&line);
    }
    let count = match a.total {
        0 => "no events".to_owned(),
        1 => "1 event".to_owned(),
        n => format!("{n} events"),
    };
    let range = range_text(&a.range, clock);
    out.push_str(
        &ctx.style
            .bold(&format!("{count} on {}, {range}", a.host.name)),
    );
    out.push('\n');
    for k in &a.kinds {
        out.push_str(&kind_block(k, clock));
    }
    if a.unreadable > 0 {
        out.push_str(&format!(
            "{} events from a newer StatGlass could not be read and are not counted\n",
            a.unreadable
        ));
    }
    out
}

/// `fans_ramped 115, Oct 29 11:42 to Nov 3 21:16` then its processes and days.
fn kind_block(k: &KindSummary, clock: &Clock) -> String {
    let mut out = format!(
        "{} {}, {} to {}\n",
        k.kind,
        k.count,
        clock.short(&k.first),
        clock.short(&k.last)
    );
    let mut named: Vec<String> = k
        .processes
        .iter()
        .map(|p| format!("{} {}", p.name, p.count))
        .collect();
    if k.other_processes > 0 {
        named.push(format!("others {}", k.other_processes));
    }
    if k.no_process > 0 {
        named.push(format!("none named {}", k.no_process));
    }
    if !named.is_empty() {
        out.push_str(&format!("  processes: {}\n", named.join(", ")));
    }
    let mut days: Vec<String> = k
        .days
        .iter()
        .rev()
        .take(MAX_SUMMARY_DAYS)
        .rev()
        .map(|d| format!("{} {}", clock.day(&d.day), d.count))
        .collect();
    if let Some(more) = k
        .days
        .len()
        .checked_sub(MAX_SUMMARY_DAYS)
        .filter(|&n| n > 0)
    {
        days.insert(0, format!("{more} earlier days"));
    }
    out.push_str(&format!("  per day: {}\n", days.join(", ")));
    out
}
