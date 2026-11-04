use statglass_query::{GapDto, HistoryAnswer, SeriesHistory, Summary};

use super::{Ctx, freshness_line, range_text, retained_line};
use crate::format::{self, Clock};

pub fn render(a: &HistoryAnswer, ctx: &Ctx) -> String {
    let clock = &ctx.clock;
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
            out.push_str(&points(s, clock, secs, ctx.style.aligned()));
        }
    }
    if a.series.is_empty() {
        out.push_str(&format!("no series matched on {}, {range}\n", a.host.name));
    }
    for gap in &a.gaps {
        out.push_str(&gap_line(gap, clock, secs));
    }
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

fn points(s: &SeriesHistory, clock: &Clock, secs: bool, aligned: bool) -> String {
    if s.points.is_empty() {
        return "  no points\n".to_owned();
    }
    let rows: Vec<Vec<String>> = s
        .points
        .iter()
        .map(|p| {
            vec![
                if secs {
                    clock.short_secs(&p.t)
                } else {
                    clock.short(&p.t)
                },
                format!("avg {}", format::value(p.avg, &s.unit)),
                format!("min {}", format::value(p.min, &s.unit)),
                format!("max {}", format::value(p.max, &s.unit)),
            ]
        })
        .collect();
    format::table(&rows, aligned, "  ")
}

/// `gap 14:31 to 14:37: host offline`.
fn gap_line(g: &GapDto, clock: &Clock, secs: bool) -> String {
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
        "module_disabled" => "module disabled".to_owned(),
        "truncated" => "pruned on the host before it synced".to_owned(),
        "clock_changed" => "clock changed".to_owned(),
        "write_failed" => "write failed".to_owned(),
        other => other.replace('_', " "),
    }
}
