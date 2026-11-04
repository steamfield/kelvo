//! `statglass events --summary`: the events answer counted by kind, by the processes
//! named and by day, so "how often, and because of what" takes one call instead of a
//! script over the event list. Counting only; every event still comes from the app.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{EventsAnswer, Freshness, HostRef, Range};

/// Processes listed per kind at most; the rest are summed in `other_processes`.
pub const MAX_SUMMARY_PROCESSES: usize = 10;

/// `events --summary`: every event in the range, counted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EventsSummary {
    pub host: HostRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freshness: Option<Freshness>,
    pub range: Range,
    /// Every event counted.
    pub total: u32,
    /// Events from a newer StatGlass that could not be read, so are not counted.
    pub unreadable: u32,
    /// Kinds by count descending, then name.
    pub kinds: Vec<KindSummary>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct KindSummary {
    pub kind: String,
    pub count: u32,
    /// When the first and last of these events were detected.
    pub first: String,
    pub last: String,
    /// Events naming each process, by count descending, then name. An event naming
    /// several processes counts once for each; an event naming none counts in
    /// `no_process`.
    pub processes: Vec<NameCount>,
    /// Events naming processes past the first [`MAX_SUMMARY_PROCESSES`], summed.
    pub other_processes: u32,
    /// Events that named no process.
    pub no_process: u32,
    /// Events per day (`YYYY-MM-DD` in this Mac's timezone), in date order. Days with
    /// none are absent.
    pub days: Vec<DayCount>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct NameCount {
    pub name: String,
    pub count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DayCount {
    pub day: String,
    pub count: u32,
}

#[derive(Default)]
struct Tally<'a> {
    count: u32,
    first: Option<&'a str>,
    last: Option<&'a str>,
    processes: BTreeMap<&'a str, u32>,
    no_process: u32,
    days: BTreeMap<&'a str, u32>,
}

impl EventsSummary {
    /// Counts `answer`'s events. Event times are RFC 3339 in this Mac's offset, so the
    /// date part is the local day.
    pub fn of(answer: &EventsAnswer) -> Self {
        let mut tallies: BTreeMap<&str, Tally<'_>> = BTreeMap::new();
        for e in &answer.events {
            let t = tallies.entry(e.kind.as_str()).or_default();
            t.count += 1;
            // Events arrive in time order; min/max keep this right if they do not.
            t.first = Some(t.first.map_or(e.at.as_str(), |f| f.min(e.at.as_str())));
            t.last = Some(t.last.map_or(e.at.as_str(), |l| l.max(e.at.as_str())));
            let mut names: Vec<&str> = e.processes.iter().map(|p| p.name.as_str()).collect();
            names.sort_unstable();
            names.dedup();
            if names.is_empty() {
                t.no_process += 1;
            }
            for name in names {
                *t.processes.entry(name).or_default() += 1;
            }
            *t.days.entry(e.at.get(..10).unwrap_or(&e.at)).or_default() += 1;
        }
        let mut kinds: Vec<KindSummary> = tallies
            .into_iter()
            .map(|(kind, t)| kind_summary(kind, t))
            .collect();
        kinds.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.kind.cmp(&b.kind)));
        Self {
            host: answer.host.clone(),
            freshness: answer.freshness.clone(),
            range: answer.range.clone(),
            total: u32::try_from(answer.events.len()).unwrap_or(u32::MAX),
            unreadable: answer.unreadable,
            kinds,
        }
    }
}

fn kind_summary(kind: &str, t: Tally<'_>) -> KindSummary {
    let mut processes: Vec<NameCount> = t
        .processes
        .into_iter()
        .map(|(name, count)| NameCount {
            name: name.to_owned(),
            count,
        })
        .collect();
    processes.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    let other_processes = processes
        .iter()
        .skip(MAX_SUMMARY_PROCESSES)
        .map(|p| p.count)
        .sum();
    processes.truncate(MAX_SUMMARY_PROCESSES);
    KindSummary {
        kind: kind.to_owned(),
        count: t.count,
        first: t.first.unwrap_or_default().to_owned(),
        last: t.last.unwrap_or_default().to_owned(),
        processes,
        other_processes,
        no_process: t.no_process,
        days: t
            .days
            .into_iter()
            .map(|(day, count)| DayCount {
                day: day.to_owned(),
                count,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EventDto, EventProcess};

    fn event(at: &str, kind: &str, processes: &[&str]) -> EventDto {
        EventDto {
            at: at.into(),
            started: None,
            kind: kind.into(),
            detail: serde_json::json!({}),
            processes: processes
                .iter()
                .map(|n| EventProcess {
                    name: (*n).into(),
                    pid: None,
                    value: None,
                    unit: None,
                })
                .collect(),
        }
    }

    #[test]
    fn counts_kinds_processes_and_days() {
        let answer = EventsAnswer {
            host: HostRef {
                id: "h".into(),
                name: "Mac".into(),
            },
            freshness: None,
            range: Range {
                from: "2026-11-01T00:00:00-06:00".into(),
                to: "2026-11-03T22:00:00-06:00".into(),
                retained_from: None,
            },
            events: vec![
                event(
                    "2026-11-02T09:00:00-06:00",
                    "fans_ramped",
                    &["node", "node"],
                ),
                event("2026-11-02T10:00:00-06:00", "thermal_state", &[]),
                event(
                    "2026-11-03T08:00:00-06:00",
                    "fans_ramped",
                    &["node", "Chrome"],
                ),
                event("2026-11-03T09:00:00-06:00", "fans_ramped", &[]),
            ],
            unreadable: 2,
        };
        let s = EventsSummary::of(&answer);
        assert_eq!((s.total, s.unreadable), (4, 2));
        let kinds: Vec<_> = s.kinds.iter().map(|k| (k.kind.as_str(), k.count)).collect();
        assert_eq!(kinds, [("fans_ramped", 3), ("thermal_state", 1)]);
        let fans = &s.kinds[0];
        assert_eq!(
            fans.processes,
            [
                NameCount {
                    name: "node".into(),
                    count: 2
                },
                NameCount {
                    name: "Chrome".into(),
                    count: 1
                },
            ],
            "a name repeated in one event counts once"
        );
        assert_eq!(fans.no_process, 1);
        assert_eq!(fans.first, "2026-11-02T09:00:00-06:00");
        assert_eq!(fans.last, "2026-11-03T09:00:00-06:00");
        let days: Vec<_> = fans
            .days
            .iter()
            .map(|d| (d.day.as_str(), d.count))
            .collect();
        assert_eq!(days, [("2026-11-02", 1), ("2026-11-03", 2)]);
    }

    #[test]
    fn processes_past_the_cap_are_summed() {
        let names: Vec<String> = (0..12).map(|i| format!("p{i:02}")).collect();
        let events = names
            .iter()
            .map(|n| event("2026-11-03T08:00:00-06:00", "fans_ramped", &[n.as_str()]))
            .collect();
        let answer = EventsAnswer {
            host: HostRef {
                id: "h".into(),
                name: "Mac".into(),
            },
            freshness: None,
            range: Range {
                from: "a".into(),
                to: "b".into(),
                retained_from: None,
            },
            events,
            unreadable: 0,
        };
        let k = &EventsSummary::of(&answer).kinds[0];
        assert_eq!(k.processes.len(), MAX_SUMMARY_PROCESSES);
        assert_eq!(k.other_processes, 2);
    }
}
