//! `now`: the newest frame's held values, the headline set by default.
//!
//! Every host starts from the series a remote host serves while unselected
//! ([`statglass_remote::headline_metrics`]: `cpu.total`, `mem.used`, `mem.pressure`,
//! `gpu.util`, `thermal.hottest`, `net.rx`, `net.tx`, `disk.read`, `disk.write`, every
//! label of each), so local and remote answers have one shape. This Mac adds
//! `power.system`, `battery.charge` and the root volume's `disk.used{vol=/}` and
//! `disk.free{vol=/}`; a remote host gets no extras, because it serves them only while
//! a window shows it, and the answer must not depend on that. A series the frame does
//! not hold (no battery) is simply absent. `detail` drops the filter and reports every
//! held series, which for a remote host is still what it serves at the time.

use statglass_query::{NowAnswer, NowHost, Reading};
use statglass_remote::headline_metrics;
use statglass_schema::SeriesKey;

use super::hosts::resolve;
use super::map::{Fail, freshness, host_ref, is_live, metric, natural_cmp, rfc};
use super::{Backend, HostView};

/// This Mac's headline series beyond [`headline_metrics`]: `(metric, labels)`, where
/// labels are matched exactly and an empty list matches the unlabelled series.
const LOCAL_HEADLINE: &[(&str, &[(&str, &str)])] = &[
    ("power.system", &[]),
    ("battery.charge", &[]),
    ("disk.used", &[("vol", "/")]),
    ("disk.free", &[("vol", "/")]),
];

/// Whether `key` is in the headline set: `base` by metric id, plus
/// [`LOCAL_HEADLINE`] on this Mac.
fn is_headline(key: &SeriesKey, base: &[&str], local: bool) -> bool {
    let id = key.metric.as_str();
    base.contains(&id)
        || local
            && LOCAL_HEADLINE.iter().any(|(m, labels)| {
                *m == id
                    && key.labels.len() == labels.len()
                    && labels.iter().all(|(k, v)| key.labels.get(k) == Some(*v))
            })
}

/// `now`: one host's latest frame, or every host's with `all`.
pub(super) fn now(
    backend: &dyn Backend,
    host: Option<&str>,
    all: bool,
    detail: bool,
) -> Result<NowAnswer, Fail> {
    let hosts = if all {
        backend.hosts()
    } else {
        vec![resolve(backend, host)?]
    };
    Ok(NowAnswer {
        hosts: hosts
            .iter()
            .map(|view| now_host(backend, view, detail))
            .collect(),
    })
}

/// One host's latest held values. A host that is not connected reports its last frame's
/// time and no values.
fn now_host(backend: &dyn Backend, view: &HostView, detail: bool) -> NowHost {
    let id = view.record.id;
    let frame = backend.latest_frame(id);
    let base = headline_metrics(view.record.info.os);
    let local = view.remote.is_none();
    let mut readings: Vec<Reading> = match (&frame, is_live(view)) {
        (Some(frame), true) => frame
            .layout
            .series
            .iter()
            .zip(frame.held.iter())
            .filter(|(key, v)| v.is_finite() && (detail || is_headline(key, base, local)))
            .map(|(key, v)| {
                let dto = metric(key.metric.as_str());
                Reading {
                    series: key.to_string(),
                    value: f64::from(*v),
                    unit: dto.map_or_else(|| "unknown".to_owned(), |m| m.unit.clone()),
                    module: dto.map_or_else(|| "unknown".to_owned(), |m| m.module.clone()),
                }
            })
            .collect(),
        _ => Vec::new(),
    };
    readings.sort_by(|a, b| natural_cmp(&a.series, &b.series));
    let watermark = view
        .remote
        .as_ref()
        .and_then(|_| backend.sync_watermark(id));
    NowHost {
        host: host_ref(&view.record),
        freshness: freshness(view, watermark.as_ref()),
        frame_at: frame.as_ref().map(|f| rfc(f.ts_ms)),
        age_ms: frame
            .as_ref()
            .map(|f| backend.host_now(id).saturating_sub(f.ts_ms).max(0)),
        readings,
        error: None,
    }
}
