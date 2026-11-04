//! `events`: detector and alert events in a range, with the count of stored rows this
//! build cannot decode.

use super::map::Fail;
use statglass_query::{
    ErrorDetails, ErrorTag, EventDto, EventProcess, EventsAnswer, Range, suggest::closest,
};
use statglass_schema::{Event, EventDetail};

use super::Backend;
use super::history::{MAX_RANGE_DAYS, available_history, check_range, retained_from};
use super::hosts::resolve_enabled;
use super::map::{err_with, freshness, from_command, host_ref, rfc};

/// Retained history begins on a bucket boundary; a start less than a minute after the
/// request's is not a clamp worth reporting.
const MINUTE_MS: i64 = 60_000;

pub(super) fn answer(
    backend: &dyn Backend,
    host: Option<&str>,
    from_ms: i64,
    to_ms: i64,
    kind: Option<&str>,
) -> Result<EventsAnswer, Fail> {
    check_range(from_ms, to_ms, MAX_RANGE_DAYS)?;
    let kind = kind.map(parse_kind).transpose()?;
    let view = resolve_enabled(backend, host)?;
    let history = available_history(backend, &view)?;
    let id = view.record.id;
    let read = history
        .read(|r| r.events_counted(id, from_ms, to_ms))
        .map_err(from_command)?;
    let mut events: Vec<&Event> = read
        .events
        .iter()
        .filter(|e| kind.is_none_or(|k| e.detail.kind() == k))
        .collect();
    events.sort_by_key(|e| (e.ts_ms, e.detail.kind()));
    let watermark = view
        .remote
        .as_ref()
        .and_then(|_| backend.sync_watermark(id));
    Ok(EventsAnswer {
        host: host_ref(&view.record),
        freshness: freshness(&view, watermark.as_ref()),
        range: Range {
            from: rfc(from_ms),
            to: rfc(to_ms),
            retained_from: retained_from(&history, &view, from_ms, MINUTE_MS).map(rfc),
        },
        events: events.into_iter().map(event_dto).collect(),
        unreadable: u32::try_from(read.unreadable).unwrap_or(u32::MAX),
    })
}

fn parse_kind(input: &str) -> Result<&'static str, Fail> {
    let kinds = EventDetail::KINDS;
    kinds
        .iter()
        .copied()
        .find(|k| k.eq_ignore_ascii_case(input.trim()))
        .ok_or_else(|| {
            err_with(
                ErrorTag::Usage,
                ErrorDetails {
                    suggestions: closest(input, kinds.iter().copied()),
                    input: Some(input.to_owned()),
                    hint: Some(format!("kinds: {}", kinds.join(", "))),
                    ..Default::default()
                },
            )
        })
}

fn event_dto(event: &Event) -> EventDto {
    // The kind's own fields, without the tag `kind` already carries.
    let mut detail = serde_json::to_value(&event.detail).unwrap_or(serde_json::Value::Null);
    if let Some(fields) = detail.as_object_mut() {
        fields.remove("kind");
    }
    EventDto {
        at: rfc(event.ts_ms),
        started: (event.start_ms != event.ts_ms).then(|| rfc(event.start_ms)),
        kind: event.detail.kind().to_owned(),
        detail,
        processes: event
            .processes
            .iter()
            .map(|name| EventProcess {
                name: name.clone(),
                pid: None,
                value: None,
                unit: None,
            })
            .collect(),
    }
}
