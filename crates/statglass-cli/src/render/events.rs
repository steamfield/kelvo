use serde_json::Value;
use statglass_query::{EventDto, EventsAnswer};

use super::{Ctx, freshness_line, range_text, retained_line};

/// At most this many detail fields per event line; the rest are in `--json`.
const MAX_DETAIL_FIELDS: usize = 6;

pub fn render(a: &EventsAnswer, ctx: &Ctx) -> String {
    let clock = &ctx.clock;
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
        out.push_str(&event_line(e, ctx));
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
fn event_line(e: &EventDto, ctx: &Ctx) -> String {
    let mut line = ctx.clock.short(&e.at);
    if let Some(started) = &e.started {
        line.push_str(&format!(" (from {})", ctx.clock.short(started)));
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
