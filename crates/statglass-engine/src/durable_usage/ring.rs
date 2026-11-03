//! Recent complete and open facts, retained until the store has committed them.
use statglass_store::usage::UsageBucket;
use statglass_store::usage::UsageCommits;
use std::collections::BTreeMap;

/// The longest a closed bucket stays here: a commit that keeps failing, or no store.
const KEEP_MS: i64 = 900_000;

#[derive(Default)]
pub(crate) struct UsageRecent {
    /// Closed buckets by start, each with the store ticket it was queued under.
    buckets: BTreeMap<i64, (Option<u64>, UsageBucket)>,
    /// The commit watermark seen on the previous update. Dropping only what was durable
    /// then gives a reader whose disk snapshot predates a commit an update's time to
    /// take its recent facts.
    durable: UsageCommits,
    pub generation: u64,
    pub recorder: super::UsageRecorder,
}
impl UsageRecent {
    /// Adds closed buckets and drops those the store has committed. `commits` is the
    /// writer's watermark, `None` without a store.
    pub fn update_owned(
        &mut self,
        buckets: impl IntoIterator<Item = (Option<u64>, UsageBucket)>,
        now: i64,
        commits: Option<UsageCommits>,
    ) {
        for (ticket, bucket) in buckets {
            self.buckets.insert(bucket.from_ms, (ticket, bucket));
        }
        let durable = self.durable;
        let cutoff = now.saturating_sub(KEEP_MS);
        self.buckets.retain(|_, (ticket, b)| {
            // Uncertain network spans stay until they age out, whatever the store holds.
            if b.uncertain_network
                .iter()
                .any(|u| u.possible_to_ms >= cutoff)
            {
                return true;
            }
            b.to_ms >= cutoff && !ticket.is_some_and(|t| durable.durable(t))
        });
        if let Some(commits) = commits {
            self.durable = commits;
        }
    }
    pub fn snapshot(&self, from: i64, to: i64) -> Vec<UsageBucket> {
        self.buckets
            .values()
            .map(|(_, b)| b)
            .chain(self.recorder.buckets())
            .filter(|b| {
                (b.from_ms < to && b.to_ms > from)
                    || b.uncertain_network
                        .iter()
                        .any(|u| u.possible_from_ms < to && u.possible_to_ms > from)
            })
            .cloned()
            .collect()
    }
    pub fn apps(&self, from: i64, to: i64) -> Vec<UsageBucket> {
        self.project(from, to, None)
    }
    pub fn processes(&self, from: i64, to: i64, app: &str) -> Vec<UsageBucket> {
        self.project(from, to, Some(app))
    }
    fn project(&self, from: i64, to: i64, app: Option<&str>) -> Vec<UsageBucket> {
        self.buckets
            .values()
            .map(|(_, b)| b)
            .chain(self.recorder.buckets())
            .filter(|b| {
                (b.from_ms < to && b.to_ms > from)
                    || b.uncertain_network
                        .iter()
                        .any(|u| u.possible_from_ms < to && u.possible_to_ms > from)
            })
            .map(|b| UsageBucket {
                from_ms: b.from_ms,
                to_ms: b.to_ms,
                source: b.source,
                apps: b
                    .apps
                    .iter()
                    .filter(|a| app.is_none_or(|name| a.name.as_ref() == name))
                    .cloned()
                    .collect(),
                processes: app.map_or_else(Vec::new, |name| {
                    b.processes
                        .iter()
                        .filter(|p| p.app_name.as_ref() == name)
                        .cloned()
                        .collect()
                }),
                uncertain_network: b.uncertain_network.clone(),
            })
            .collect()
    }
    pub fn clear(&mut self) {
        self.buckets.clear();
        self.recorder.clear(0);
        self.generation = self.generation.wrapping_add(1);
    }
    pub fn drop_after(&mut self, at: i64) {
        self.buckets.retain(|_, (_, b)| b.to_ms <= at);
        self.recorder.drop_after(at);
    }
}
