use statglass_query::{NowAnswer, NowHost};

use super::{Ctx, freshness_line, group_by};
use crate::{failure::next_for, format};

pub fn render(a: &NowAnswer, ctx: &Ctx) -> String {
    let blocks: Vec<String> = a.hosts.iter().map(|h| host(h, ctx)).collect();
    if blocks.is_empty() {
        return "no hosts\n".to_owned();
    }
    blocks.join("\n")
}

fn host(h: &NowHost, ctx: &Ctx) -> String {
    let name = &h.host.name;
    let mut out = ctx.style.bold(name);
    match (&h.frame_at, h.age_ms) {
        (Some(at), Some(age)) => out.push_str(&format!(
            " at {} ({} old)",
            ctx.clock.short_secs(at),
            age_text(age)
        )),
        (Some(at), None) => out.push_str(&format!(" at {}", ctx.clock.short_secs(at))),
        _ => out.push_str(": no frame"),
    }
    out.push('\n');
    if let Some(line) = freshness_line(name, h.freshness.as_ref(), &ctx.clock) {
        out.push_str(&line);
    }
    if let Some(e) = &h.error {
        out.push_str(&format!(
            "  error {:?}: {} Next: {}\n",
            e.tag,
            e.message,
            next_for(e.tag)
        ));
        return out;
    }
    if h.readings.is_empty() {
        out.push_str("  no readings\n");
        return out;
    }
    for (module, readings) in group_by(&h.readings, |r| r.module.as_str()) {
        out.push_str(&format!("{module}\n"));
        let rows: Vec<Vec<String>> = readings
            .iter()
            .map(|r| vec![r.series.clone(), format::value(r.value, &r.unit)])
            .collect();
        out.push_str(&format::table(&rows, ctx.style.aligned(), "  "));
    }
    out
}

/// `0.8s`, `12s`, `4 min`, `3 h`.
fn age_text(ms: i64) -> String {
    match ms {
        ..=9_999 => format!("{:.1}s", ms as f64 / 1000.0),
        10_000..=119_999 => format!("{}s", ms / 1000),
        120_000..=7_199_999 => format!("{} min", ms / 60_000),
        _ => format!("{} h", ms / 3_600_000),
    }
}
