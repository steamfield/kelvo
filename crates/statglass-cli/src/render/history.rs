use statglass_query::{GapDto, HistoryAnswer, SeriesHistory, Summary};

use super::{Ctx, freshness_line, range_clock, range_text, retained_line};
use crate::format::{self, Clock};

pub fn render(a: &HistoryAnswer, ctx: &Ctx) -> String {
    let clock = &range_clock(&a.range, &ctx.clock);
    let secs = a.bucket_s < 60;
    let mut out = String::new();
    if let Some(line) = freshness_line(&a.host.name, a.freshness.as_ref(), clock) {
        out.push_str(&line);
    }
    if let Some(line) = retained_line(&a.range, clock) {
        out.push_str(&line);
    }
    let range = range_text(&a.range, clock);
    let bucket = format::bucket(a.bucket_s);
    for s in &a.series {
        let header = format!(
            "{} on {}, {range} ({bucket} buckets, {} measured)",
            s.key,
            a.host.name,
            format::percent(s.summary.measured_fraction)
        );
        out.push_str(&ctx.style.bold(&header));
        out.push('\n');
        out.push_str(&summary_line(&s.summary, &s.unit, clock, secs));
        if ctx.show_points {
            out.push_str(&points(s, a, clock, secs, ctx.style.aligned()));
        }
    }
    if a.series.is_empty() {
        // A mistyped series is an error; an empty answer means nothing stored in range.
        out.push_str(&format!(
            "no data for these series on {}, {range}\n",
            a.host.name
        ));
    }
    out.push_str(&gaps(a, clock, secs, ctx.all_gaps));
    out
}

/// Gap lines printed before the rest are summed up in one line.
const MAX_GAP_LINES: usize = 5;

/// Every gap with `all`; otherwise the longest [`MAX_GAP_LINES`] in time order and one
/// line for the rest: `12 more gaps, 3 h 10 min total (sleep: 9, host offline: 3)`.
fn gaps(a: &HistoryAnswer, clock: &Clock, secs: bool, all: bool) -> String {
    if all || a.gaps.len() <= MAX_GAP_LINES {
        return a.gaps.iter().map(|g| gap_line(g, clock, secs)).collect();
    }
    let at = |t: &str| statglass_query::time::parse_instant(t, 0).ok();
    let (from, to) = (at(&a.range.from), at(&a.range.to));
    // Length inside the range; an open gap runs to the range's end.
    let length = |g: &GapDto| -> i64 {
        let start = at(&g.from).max(from);
        let end = g.to.as_deref().and_then(at).or(to).min(to);
        match (start, end) {
            (Some(s), Some(e)) => e.saturating_sub(s).max(0),
            _ => 0,
        }
    };
    let mut order: Vec<usize> = (0..a.gaps.len()).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(a.gaps.get(i).map_or(0, length)), i));
    let (shown, rest) = order.split_at(MAX_GAP_LINES);
    let mut shown = shown.to_vec();
    shown.sort_unstable();
    let mut out: String = shown
        .iter()
        .filter_map(|&i| a.gaps.get(i))
        .map(|g| gap_line(g, clock, secs))
        .collect();
    let rest: Vec<&GapDto> = rest.iter().filter_map(|&i| a.gaps.get(i)).collect();
    let total: i64 = rest.iter().map(|g| length(g)).sum();
    let mut counts: Vec<(String, usize)> = Vec::new();
    for g in &rest {
        let r = reason(&g.reason);
        match counts.iter_mut().find(|(name, _)| *name == r) {
            Some((_, n)) => *n += 1,
            None => counts.push((r, 1)),
        }
    }
    counts.sort_by(|x, y| y.1.cmp(&x.1).then_with(|| x.0.cmp(&y.0)));
    let counts: Vec<String> = counts.iter().map(|(r, n)| format!("{r}: {n}")).collect();
    out.push_str(&format!(
        "{} more gaps, {} total ({}); --all-gaps lists every one\n",
        rest.len(),
        format::duration(total),
        counts.join(", ")
    ));
    out
}

/// `min 2.1%  mean 18.4%  max 97.0% in the minute from 15:12  last 6.3% at 16:00`.
fn summary_line(s: &Summary, unit: &str, clock: &Clock, secs: bool) -> String {
    let at = |t: &str| {
        if secs {
            clock.short_secs(t)
        } else {
            clock.short(t)
        }
    };
    let mut parts = Vec::new();
    if let Some(min) = s.min {
        parts.push(format!("min {}", format::value(min, unit)));
    }
    if let Some(mean) = s.mean {
        parts.push(format!("mean {}", format::value(mean, unit)));
    }
    if let Some(max) = s.max {
        let mut part = format!("max {}", format::value(max, unit));
        if let Some(t) = &s.max_at {
            part.push(' ');
            part.push_str(&bucket_phrase(s.max_bucket_s, &at(t)));
        }
        parts.push(part);
    }
    if let Some(last) = s.last {
        let mut part = format!("last {}", format::value(last, unit));
        if let Some(t) = &s.last_at {
            part.push_str(&format!(" at {}", at(t)));
        }
        parts.push(part);
    }
    if parts.is_empty() {
        return "no data measured in this range\n".to_owned();
    }
    if s.mean_approximate {
        parts.push("(mean approximate)".to_owned());
    }
    parts.join("  ") + "\n"
}

/// Where a max fell: the bucket it is the start of, not the instant.
fn bucket_phrase(bucket_s: Option<u32>, start: &str) -> String {
    match bucket_s {
        None | Some(0 | 1) => format!("at {start}"),
        Some(60) => format!("in the minute from {start}"),
        Some(s) if s.is_multiple_of(60) => format!("in the {} minutes from {start}", s / 60),
        Some(s) => format!("in the {s} s from {start}"),
    }
}

/// The points, one row each, with a `no data` row wherever slots on the point grid have
/// no point (before the first, between two, after the last), so a missing slot never
/// reads as the next point following on. A run of empty slots is one row.
fn points(
    s: &SeriesHistory,
    a: &HistoryAnswer,
    clock: &Clock,
    secs: bool,
    aligned: bool,
) -> String {
    if s.points.is_empty() {
        return "  no points\n".to_owned();
    }
    let t = |rfc: &str| {
        if secs {
            clock.short_secs(rfc)
        } else {
            clock.short(rfc)
        }
    };
    let slot_ms = a
        .point_slot_s
        .map(|s| i64::from(s) * 1_000)
        .filter(|&w| w > 0);
    let ms = |rfc: &str| statglass_query::time::parse_instant(rfc, 0).ok();
    let empty = |from: i64, slots: i64| -> Vec<String> {
        let when = t(&statglass_query::time::rfc3339_local(from));
        let what = if slots == 1 {
            "no data".to_owned()
        } else {
            format!("no data ({slots} points)")
        };
        vec![when, what, String::new(), String::new()]
    };
    let mut rows: Vec<Vec<String>> = Vec::new();
    // Where the next point is due: the range start, then one slot past each point.
    let mut due = ms(&a.range.from);
    for p in &s.points {
        let at = ms(&p.t);
        if let (Some(w), Some(due_ms), Some(at_ms)) = (slot_ms, due, at) {
            // Slots are on the point grid, so step back to the slot holding `due_ms`.
            let first = at_ms - (at_ms - due_ms).div_euclid(w) * w;
            let missing = (at_ms - first) / w;
            if missing > 0 {
                rows.push(empty(first, missing));
            }
        }
        rows.push(vec![
            t(&p.t),
            format!("avg {}", format::value(p.avg, &s.unit)),
            format!("min {}", format::value(p.min, &s.unit)),
            format!("max {}", format::value(p.max, &s.unit)),
        ]);
        due = at.zip(slot_ms).map(|(at_ms, w)| at_ms + w);
    }
    if let (Some(w), Some(due_ms), Some(to)) = (slot_ms, due, ms(&a.range.to)) {
        let missing = (to - due_ms) / w;
        if missing > 0 {
            rows.push(empty(due_ms, missing));
        }
    }
    format::table(&rows, aligned, "  ")
}

/// `gap 14:31 to 14:37: host offline`. A gap within one minute prints seconds, so it
/// never reads `14:31 to 14:31`.
fn gap_line(g: &GapDto, clock: &Clock, secs: bool) -> String {
    let secs = secs
        || g.to
            .as_deref()
            .is_some_and(|to| clock.short(to) == clock.short(&g.from));
    let t = |s: &str| {
        if secs {
            clock.short_secs(s)
        } else {
            clock.short(s)
        }
    };
    let to = g.to.as_deref().map_or_else(|| "now (open)".to_owned(), t);
    let module = g
        .module
        .as_ref()
        .map(|m| format!(" ({m} only)"))
        .unwrap_or_default();
    format!(
        "gap {} to {to}: {}{module}\n",
        t(&g.from),
        reason(&g.reason)
    )
}

/// A gap reason in words. An unfamiliar reason is shown as it is; it is still a gap.
fn reason(r: &str) -> String {
    match r {
        "source_offline" => "host offline".to_owned(),
        "app_not_running" => "StatGlass not running".to_owned(),
        "not_synced" => "not synced yet".to_owned(),
        "no_data" => "no data recorded".to_owned(),
        "module_disabled" => "module disabled".to_owned(),
        "truncated" => "pruned on the host before it synced".to_owned(),
        "clock_changed" => "clock changed".to_owned(),
        "write_failed" => "write failed".to_owned(),
        other => other.replace('_', " "),
    }
}
