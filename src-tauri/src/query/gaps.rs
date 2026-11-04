//! The gaps `history` reports: stored gaps that apply to the requested series, the
//! `not_synced` tail, and `no_data` spans nothing explains, merged so that runs of
//! touching gaps read as one.

use std::collections::BTreeSet;

use statglass_query::GapDto;
use statglass_schema::{Gap, Module};

use super::map::rfc;

/// The reason for a span with no point and no stored gap.
pub(super) const NO_DATA: &str = "no_data";
/// The reason for a range that ends before the host's retained history begins: pruned,
/// not unrecorded.
pub(super) const BEFORE_RETENTION: &str = "before_retention";

/// A gap in milliseconds; `to` is `None` while it is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Span {
    pub from: i64,
    pub to: Option<i64>,
    pub reason: String,
    pub module: Option<Module>,
}

impl Span {
    pub(super) fn from_gap(gap: &Gap) -> Self {
        Self {
            from: gap.start_ms,
            to: gap.end_ms,
            reason: gap.reason.as_str().to_owned(),
            module: gap.module,
        }
    }

    pub(super) fn dto(&self) -> GapDto {
        GapDto {
            from: rfc(self.from),
            to: self.to.map(rfc),
            reason: self.reason.clone(),
            module: self.module.map(|m| m.as_str().to_owned()),
        }
    }
}

/// Whether a stored gap applies to a read of series in `modules`: host-wide, or one of
/// their modules.
pub(super) fn applies(gap: &Gap, modules: &BTreeSet<Module>) -> bool {
    gap.module.is_none_or(|m| modules.contains(&m))
}

/// Spans of `[from, to)` longer than `min_ms` that no interval in `covered` touches.
/// `covered` is `[start, end)` pairs in any order.
pub(super) fn uncovered(
    mut covered: Vec<(i64, i64)>,
    from: i64,
    to: i64,
    min_ms: i64,
) -> Vec<(i64, i64)> {
    covered.sort_unstable();
    let mut out = Vec::new();
    let mut cursor = from;
    let mut push = |a: i64, b: i64| {
        if b.saturating_sub(a) > min_ms {
            out.push((a, b));
        }
    };
    for (start, end) in covered {
        if cursor >= to {
            break;
        }
        if start > cursor {
            push(cursor, start.min(to));
        }
        cursor = cursor.max(end);
    }
    if cursor < to {
        push(cursor, to);
    }
    out
}

/// Gaps with the same reason and module that touch or overlap become one; the result is
/// in time order (then reason, then module).
pub(super) fn merge(mut spans: Vec<Span>) -> Vec<Span> {
    spans.sort_by(|a, b| (&a.reason, a.module, a.from).cmp(&(&b.reason, b.module, b.from)));
    let mut out: Vec<Span> = Vec::with_capacity(spans.len());
    for span in spans {
        if let Some(last) = out.last_mut()
            && last.reason == span.reason
            && last.module == span.module
            && last.to.is_none_or(|to| span.from <= to)
        {
            last.to = match (last.to, span.to) {
                (Some(a), Some(b)) => Some(a.max(b)),
                _ => None,
            };
            continue;
        }
        out.push(span);
    }
    out.sort_by(|a, b| (a.from, &a.reason, a.module).cmp(&(b.from, &b.reason, b.module)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(from: i64, to: Option<i64>, reason: &str) -> Span {
        Span {
            from,
            to,
            reason: reason.to_owned(),
            module: None,
        }
    }

    #[test]
    fn touching_and_overlapping_gaps_of_one_reason_merge() {
        let merged = merge(vec![
            span(30, Some(40), "sleep"),
            span(0, Some(10), "sleep"),
            span(10, Some(20), "sleep"),
            span(15, Some(18), "sleep"),
            span(5, Some(25), "paused"),
            span(50, None, "sleep"),
            span(60, Some(70), "sleep"),
        ]);
        assert_eq!(
            merged,
            [
                span(0, Some(20), "sleep"),
                span(5, Some(25), "paused"),
                span(30, Some(40), "sleep"),
                span(50, None, "sleep"),
            ]
        );
        let mut disk = span(10, Some(20), "module_disabled");
        disk.module = Some(Module::Disk);
        let merged = merge(vec![span(0, Some(10), "module_disabled"), disk.clone()]);
        assert_eq!(merged.len(), 2, "different modules stay apart");
    }

    #[test]
    fn uncovered_finds_leading_inner_and_trailing_spans_over_the_minimum() {
        let covered = vec![(40, 60), (100, 110), (55, 70), (108, 120)];
        assert_eq!(
            uncovered(covered.clone(), 0, 200, 20),
            [(0, 40), (70, 100), (120, 200)]
        );
        assert_eq!(uncovered(covered, 0, 130, 20), [(0, 40), (70, 100)]);
        assert_eq!(uncovered(Vec::new(), 0, 30, 20), [(0, 30)]);
        assert_eq!(uncovered(vec![(0, 30)], 0, 30, 20), []);
    }
}
