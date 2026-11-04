//! Text rendering of answers for the agent and human audiences. Both print the same
//! content; human output aligns columns and may use color. Order is the server's.

mod events;
mod history;
mod hosts;
mod metrics;
mod now;
mod top;

use serde::Serialize;
use statglass_query::{Answer, Freshness, HostState, Range};

use crate::{audience::Style, format::Clock};

/// What a renderer needs besides the answer.
pub struct Ctx {
    pub style: Style,
    pub clock: Clock,
    /// Print history points (`--points N` given).
    pub show_points: bool,
    /// Print every history gap (`--all-gaps`), not only the longest.
    pub all_gaps: bool,
}

pub fn answer(answer: &Answer, ctx: &Ctx) -> String {
    match answer {
        Answer::Status(s) => format!(
            "StatGlass {} (api {}), {} hosts, timezone {}\n",
            s.app_version, s.api, s.host_count, s.timezone
        ),
        Answer::Hosts(a) => hosts::render(a, ctx),
        Answer::Now(a) => now::render(a, ctx),
        Answer::Metrics(a) => metrics::render(a, ctx),
        Answer::History(a) => history::render(a, ctx),
        Answer::Events(a) => events::render(a, ctx),
        Answer::Top(a) => top::render(a, ctx),
    }
}

/// The serialized text of a unit-variant enum (`HostState::Offline` is `offline`).
pub fn snake(value: &impl Serialize) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(s)) => s,
        _ => "unknown".to_owned(),
    }
}

/// Whether a host in this state serves current data.
fn is_current(state: HostState) -> bool {
    matches!(state, HostState::Local | HostState::Connected)
}

/// One line when a host's data is not current: `m1-linux offline since 09:12; history
/// through 09:12`.
pub fn freshness_line(name: &str, freshness: Option<&Freshness>, clock: &Clock) -> Option<String> {
    let f = freshness?;
    if is_current(f.state) {
        return None;
    }
    let mut line = format!("{name} {}", snake(&f.state));
    if let Some(seen) = &f.last_seen {
        line.push_str(&format!(" since {}", clock.short(seen)));
    }
    match &f.synced_through {
        Some(through) => line.push_str(&format!("; history through {}", clock.short(through))),
        None => line.push_str("; history sync point unknown"),
    }
    line.push('\n');
    Some(line)
}

/// The clock to print an answer covering `range` with: every time dated when the range
/// reaches past today.
pub fn range_clock(range: &Range, clock: &Clock) -> Clock {
    clock.for_range(&range.from, &range.to)
}

/// `13:00 to 16:00`.
pub fn range_text(range: &Range, clock: &Clock) -> String {
    format!("{} to {}", clock.short(&range.from), clock.short(&range.to))
}

/// A line saying the range was clamped to retained history, when it was.
pub fn retained_line(range: &Range, clock: &Clock) -> Option<String> {
    range.retained_from.as_ref().map(|from| {
        format!(
            "history starts {}; the range was clamped to it\n",
            clock.short(from)
        )
    })
}

/// Groups items by a key, keeping first-appearance order of keys and items.
pub fn group_by<T>(items: &[T], key: impl Fn(&T) -> &str) -> Vec<(String, Vec<&T>)> {
    let mut groups: Vec<(String, Vec<&T>)> = Vec::new();
    for item in items {
        let k = key(item);
        match groups.iter_mut().find(|(g, _)| g == k) {
            Some((_, members)) => members.push(item),
            None => groups.push((k.to_owned(), vec![item])),
        }
    }
    groups
}
