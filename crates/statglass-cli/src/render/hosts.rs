use statglass_query::{HostDto, HostKind, HostsAnswer, SyncState};

use super::{Ctx, is_current, snake};
use crate::format;

pub fn render(a: &HostsAnswer, ctx: &Ctx) -> String {
    if a.hosts.is_empty() {
        return "no hosts\n".to_owned();
    }
    let rows: Vec<Vec<String>> = a.hosts.iter().map(|h| row(h, ctx)).collect();
    format::table(&rows, ctx.style.aligned(), "")
}

fn row(h: &HostDto, ctx: &Ctx) -> Vec<String> {
    let kind = match h.kind {
        HostKind::ThisMac => "this Mac",
        HostKind::Mac => "Mac",
        HostKind::Linux => "Linux",
        HostKind::Unknown => "unknown",
    };
    let clock = &ctx.clock;
    let mut notes = Vec::new();
    let current = is_current(h.state);
    if !current && let Some(seen) = &h.last_seen {
        notes.push(format!("last seen {}", clock.short(seen)));
    }
    let behind = matches!(
        h.sync,
        Some(SyncState::CatchingUp | SyncState::Unavailable | SyncState::Unknown)
    );
    if let Some(sync) = h.sync.filter(|_| behind) {
        notes.push(format!("sync {}", snake(&sync)));
    }
    if (!current || behind)
        && let Some(through) = &h.synced_through
    {
        notes.push(format!("synced through {}", clock.short(through)));
    }
    match (&h.retained_from, &h.retained_to) {
        (Some(from), Some(to)) => notes.push(format!(
            "history {} to {}",
            clock.short(from),
            clock.short(to)
        )),
        (Some(from), None) => notes.push(format!("history from {}", clock.short(from))),
        _ => notes.push("no history".to_owned()),
    }
    vec![
        h.name.clone(),
        kind.to_owned(),
        snake(&h.state),
        notes.join(", "),
    ]
}
