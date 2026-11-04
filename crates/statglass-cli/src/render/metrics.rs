use statglass_query::{MetricDto, MetricsAnswer};

use super::{Ctx, group_by};
use crate::format;

/// Labelled series printed per metric under `--host`; `--json` has every one.
const MAX_SERIES_PER_METRIC: usize = 6;

pub fn render(a: &MetricsAnswer, ctx: &Ctx) -> String {
    match &a.series {
        Some(series) => host_series(a, series, ctx),
        None => catalog(&a.metrics, ctx),
    }
}

fn catalog(metrics: &[MetricDto], ctx: &Ctx) -> String {
    let mut out = String::new();
    for (module, metrics) in group_by(metrics, |m| m.module.as_str()) {
        out.push_str(&format!("{module}\n"));
        let rows: Vec<Vec<String>> = metrics.iter().map(|m| row(m)).collect();
        out.push_str(&format::table(&rows, ctx.style.aligned(), "  "));
    }
    if metrics.is_empty() {
        out.push_str("no metrics\n");
    }
    out
}

/// With `--host`: only the metrics that host has stored, each with its catalog notes and
/// its series, at most [`MAX_SERIES_PER_METRIC`] of them, so a host with hundreds of
/// container series stays readable.
fn host_series(a: &MetricsAnswer, series: &[String], ctx: &Ctx) -> String {
    let host = a.host.as_ref().map_or("the host", |h| h.name.as_str());
    if series.is_empty() {
        return format!("no stored series on {host}\n");
    }
    let metric_of = |s: &str| s.split('{').next().unwrap_or(s).to_owned();
    let stored: Vec<&MetricDto> = a
        .metrics
        .iter()
        .filter(|m| series.iter().any(|s| metric_of(s) == m.id))
        .collect();
    let mut out = format!(
        "{} series of {} metrics stored on {host}\n",
        series.len(),
        stored.len()
    );
    for (module, metrics) in group_by(&stored, |m| m.module.as_str()) {
        out.push_str(&format!("{module}\n"));
        for m in metrics {
            out.push_str(&format::table(&[row(m)], ctx.style.aligned(), "  "));
            if m.labels.is_empty() {
                continue;
            }
            let of: Vec<&String> = series.iter().filter(|s| metric_of(s) == m.id).collect();
            for s in of.iter().take(MAX_SERIES_PER_METRIC) {
                out.push_str(&format!("    {s}\n"));
            }
            if let Some(more) = of
                .len()
                .checked_sub(MAX_SERIES_PER_METRIC)
                .filter(|&n| n > 0)
            {
                out.push_str(&format!("    and {more} more (--json lists every one)\n"));
            }
        }
    }
    let unstored = a.metrics.len() - stored.len();
    if unstored > 0 {
        out.push_str(&format!(
            "{unstored} catalog metrics have no stored series on {host}; statglass metrics lists the catalog\n"
        ));
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
