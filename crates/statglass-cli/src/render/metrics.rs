use statglass_query::{MetricDto, MetricsAnswer};

use super::{Ctx, group_by};
use crate::format;

pub fn render(a: &MetricsAnswer, ctx: &Ctx) -> String {
    let mut out = String::new();
    for (module, metrics) in group_by(&a.metrics, |m| m.module.as_str()) {
        out.push_str(&format!("{module}\n"));
        let rows: Vec<Vec<String>> = metrics.iter().map(|m| row(m)).collect();
        out.push_str(&format::table(&rows, ctx.style.aligned(), "  "));
    }
    if a.metrics.is_empty() {
        out.push_str("no metrics\n");
    }
    if let Some(series) = &a.series {
        let host = a.host.as_ref().map_or("the host", |h| h.name.as_str());
        if series.is_empty() {
            out.push_str(&format!("\nno stored series on {host}\n"));
        } else {
            out.push_str(&format!("\n{} series stored on {host}:\n", series.len()));
            for s in series {
                out.push_str(&format!("  {s}\n"));
            }
        }
    }
    out
}

fn row(m: &MetricDto) -> Vec<String> {
    let id = if m.labels.is_empty() {
        m.id.clone()
    } else {
        format!("{}{{{}}}", m.id, m.labels.join(","))
    };
    let mut notes = vec![
        m.unit.clone(),
        m.kind.clone(),
        format!("every {}s", m.period_s),
        if m.persisted {
            "history"
        } else {
            "last hour only"
        }
        .to_owned(),
    ];
    if let Some(n) = m.max_cardinality {
        notes.push(format!("up to {n} series"));
    }
    let mut row = vec![id];
    row.append(&mut notes);
    row
}
