use statglass_query::TopAnswer;

use super::{Ctx, freshness_line, range_text, snake};
use crate::format;

/// Processes printed under each app; `--json` has them all.
const MAX_PROCESSES: usize = 3;

pub fn render(a: &TopAnswer, ctx: &Ctx) -> String {
    let clock = &ctx.clock;
    let mut out = String::new();
    if let Some(line) = freshness_line(&a.host.name, a.freshness.as_ref(), clock) {
        out.push_str(&line);
    }
    // Usage history, not metric history: the ring starts when StatGlass did, and durable
    // usage where the host's usage history begins.
    if let Some(from) = &a.range.retained_from {
        out.push_str(&format!(
            "usage data starts {} (StatGlass started then, or the host's usage history begins then)\n",
            clock.short(from)
        ));
    }
    let header = format!(
        "top {} apps by {} on {}, {}",
        a.apps.len(),
        snake(&a.by),
        a.host.name,
        range_text(&a.range, clock)
    );
    out.push_str(&ctx.style.bold(&header));
    out.push('\n');
    if a.limit_reduced {
        out.push_str(&format!(
            "limited to {} apps: this host serves no more\n",
            a.limit
        ));
    }
    if a.apps.is_empty() {
        out.push_str("no usage recorded in this range\n");
        return out;
    }
    let mut rows: Vec<Vec<String>> = Vec::new();
    for app in &a.apps {
        let share = app.share.map(format::percent).unwrap_or_default();
        rows.push(vec![
            app.name.clone(),
            format::value(app.value, &app.unit),
            share,
        ]);
        for p in app.processes.iter().take(MAX_PROCESSES) {
            let name = match p.pid {
                Some(pid) => format!("  {} (pid {pid})", p.name),
                None => format!("  {}", p.name),
            };
            rows.push(vec![name, format::value(p.value, &app.unit), String::new()]);
        }
        if let Some(more) = app
            .processes
            .len()
            .checked_sub(MAX_PROCESSES)
            .filter(|&n| n > 0)
        {
            rows.push(vec![
                format!("  and {more} more"),
                String::new(),
                String::new(),
            ]);
        }
    }
    out.push_str(&format::table(&rows, ctx.style.aligned(), ""));
    out
}
