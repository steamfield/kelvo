//! `statglass`, `statglass guide [topic]` and the static text the skill reuses.

use serde_json::json;
use statglass_query::suggest;

use crate::{Output, failure::Failure};

pub const GUIDE: &str = include_str!("../guide/guide.txt");

/// The guide's topics, in listing order.
pub const TOPICS: [(&str, &str); 3] = [
    ("series", include_str!("../guide/series.txt")),
    ("time", include_str!("../guide/time.txt")),
    ("errors", include_str!("../guide/errors.txt")),
];

/// The guide, or one topic of it.
pub fn run(topic: Option<&str>, json: bool) -> Result<Output, Failure> {
    let text = match topic {
        None => GUIDE,
        Some(name) => topic_text(name)?,
    };
    if json {
        Output::json(&json!({ "guide": text }))
    } else {
        Ok(Output::Text(text.to_owned()))
    }
}

fn topic_text(name: &str) -> Result<&'static str, Failure> {
    let lower = name.to_lowercase();
    TOPICS
        .iter()
        .find(|(t, _)| *t == lower)
        .map(|(_, text)| *text)
        .ok_or_else(|| {
            let names = TOPICS.iter().map(|(t, _)| *t);
            Failure::usage(
                Some(name),
                "unknown guide topic; topics are series, time and errors",
                suggest::closest(name, names),
            )
        })
}
