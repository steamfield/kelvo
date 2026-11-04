//! A `history` selector whose labels match no series the host has ever stored, although
//! the host has stored that metric under other labels: a mistyped label value such as
//! `disk.read{dev=disk9}`. Without this check the answer is an empty series list and a
//! `no_data` gap over the whole range, which reads as "the host recorded nothing" rather
//! than "that series does not exist here". A metric the host never stored at all keeps
//! the ordinary answer, whose gaps say why (a disabled module, for one).

use statglass_query::{ErrorDetails, ErrorTag, suggest::closest};
use statglass_schema::{SeriesKey, SeriesSelector};

use super::HostView;
use super::map::{Fail, err_with, from_command, metric};
use crate::history::History;

/// `UnknownMetric` for the first labelled selector in `selectors` that matched nothing in
/// the read (`read`), names a persisted metric, and matches none of the host's stored
/// series of that metric while some exist, with the closest of them as suggestions. Reads the host's series
/// inventory only when some selector matched nothing, so an ordinary read pays nothing.
pub(super) fn check(
    history: &History,
    view: &HostView,
    selectors: &[SeriesSelector],
    read: &[&SeriesKey],
) -> Result<(), Fail> {
    // A metric that keeps only the last hour has no inventory to check against.
    let candidates: Vec<&SeriesSelector> = selectors
        .iter()
        .filter(|s| !s.labels.is_empty() && !read.iter().any(|k| s.matches(k)))
        .filter(|s| metric(s.metric.as_str()).is_some_and(|m| m.persisted))
        .collect();
    if candidates.is_empty() {
        return Ok(());
    }
    let id = view.record.id;
    let stored = history
        .read(|r| r.stored_series(id))
        .map_err(from_command)?;
    let same_metric = |s: &SeriesSelector| -> Vec<String> {
        stored
            .iter()
            .filter(|k| k.metric == s.metric)
            .map(ToString::to_string)
            .collect()
    };
    let Some((missing, others)) = candidates
        .into_iter()
        .filter(|s| !stored.iter().any(|k| s.matches(k)))
        .map(|s| (s, same_metric(s)))
        .find(|(_, others)| !others.is_empty())
    else {
        return Ok(());
    };
    let metric_id = missing.metric.as_str();
    let typed = SeriesKey::new(missing.metric.clone(), missing.labels.clone()).to_string();
    let hint = format!(
        "{} has stored {} {metric_id} series; statglass metrics --host lists them",
        view.record.display_name,
        others.len()
    );
    Err(err_with(
        ErrorTag::UnknownMetric,
        ErrorDetails {
            suggestions: closest(&typed, others.iter().map(String::as_str)),
            series: Some(typed.clone()),
            metric: Some(metric_id.to_owned()),
            input: Some(typed),
            hint: Some(hint),
            ..Default::default()
        },
    ))
}
