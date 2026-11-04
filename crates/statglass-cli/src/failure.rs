//! A failed invocation: the typed error, its `next` instruction, and how each audience
//! sees it. `--json` leaves stdout empty and writes one [`CliErrorEnvelope`] as the last
//! stderr line; text modes write the fixed message, detail lines and `Next:`.

use statglass_query::{CliErrorEnvelope, ClientError, ErrorDetails, ErrorTag, QueryError};

use crate::audience::Style;

#[derive(Debug)]
pub struct Failure {
    pub error: Box<QueryError>,
    /// Text a text-mode run prints on stdout before failing (`doctor`'s findings). Never
    /// written under `--json`, where stdout stays empty on failure.
    pub preface: Option<String>,
}

impl Failure {
    pub fn new(tag: ErrorTag, details: ErrorDetails) -> Self {
        Self::from(QueryError::new(tag).with_details(details))
    }

    /// A `Usage` failure about `input`, with `hint` saying what was wrong.
    pub fn usage(input: Option<&str>, hint: impl Into<String>, suggestions: Vec<String>) -> Self {
        Self::new(
            ErrorTag::Usage,
            ErrorDetails {
                suggestions,
                input: input.map(str::to_owned),
                hint: Some(hint.into()),
                ..ErrorDetails::default()
            },
        )
    }

    /// A `Defect`, with what went wrong as the hint.
    pub fn defect(hint: impl Into<String>) -> Self {
        Self::new(
            ErrorTag::Defect,
            ErrorDetails {
                hint: Some(hint.into()),
                ..ErrorDetails::default()
            },
        )
    }

    pub fn exit_code(&self) -> u8 {
        self.error.tag.exit_code()
    }

    pub fn envelope(&self) -> CliErrorEnvelope {
        CliErrorEnvelope::new(
            (*self.error).clone(),
            Some(next_for(self.error.tag).to_owned()),
        )
    }

    /// The stderr text for the text audiences, ending with the `Next:` line.
    pub fn text(&self, style: Style) -> String {
        let mut out = style.error(&self.error.message);
        out.push('\n');
        if let Some(d) = &self.error.details {
            for (label, value) in detail_lines(d) {
                out.push_str(&format!("  {label}: {value}\n"));
            }
        }
        out.push_str(&format!("  Next: {}\n", next_for(self.error.tag)));
        out
    }
}

impl From<QueryError> for Failure {
    fn from(error: QueryError) -> Self {
        Self {
            error: Box::new(error),
            preface: None,
        }
    }
}

impl From<ClientError> for Failure {
    fn from(e: ClientError) -> Self {
        let mut details = ErrorDetails {
            hint: Some(e.to_string()),
            ..ErrorDetails::default()
        };
        if let ClientError::AppOutdated { app_version, api } = &e {
            details.app_version = Some(app_version.clone());
            details.api = Some(*api);
        }
        Self::new(e.tag(), details)
    }
}

/// The detail lines text modes print, in a fixed order.
fn detail_lines(d: &ErrorDetails) -> Vec<(&'static str, String)> {
    let mut lines = Vec::new();
    let mut push = |label, value: &Option<String>| {
        if let Some(v) = value {
            lines.push((label, v.clone()));
        }
    };
    push("Input", &d.input);
    push("Host", &d.host);
    push("Metric", &d.metric);
    push("Series", &d.series);
    push("Last seen", &d.last_seen);
    push("App version", &d.app_version);
    push("Detail", &d.hint);
    if let Some(days) = d.max_range_days {
        lines.push(("Longest range", format!("{days} days")));
    }
    if !d.suggestions.is_empty() {
        lines.push(("Did you mean", format!("{}?", d.suggestions.join(", "))));
    }
    lines
}

/// What to do next for each tag: the envelope's `next` and the text modes' `Next:` line.
/// Every arm names a real command or tells the agent what to ask the user.
pub const fn next_for(tag: ErrorTag) -> &'static str {
    match tag {
        ErrorTag::Usage => "run statglass --help, or statglass <command> --help",
        ErrorTag::UnknownHost => "run statglass hosts to see host names",
        ErrorTag::UnknownMetric => {
            "run statglass metrics to see metric ids; quote series arguments, e.g. \
             'cgroup.cpu{kind=container,name=postgres}'"
        }
        ErrorTag::BadRange => "use --since 2h, or --from/--to with RFC 3339 or HH:MM",
        ErrorTag::Busy => "retry in a few seconds",
        ErrorTag::HostOffline => {
            "use statglass events or statglass history for this host; per-app usage needs it \
             connected"
        }
        ErrorTag::HostDisabled => "ask the user to enable the host in StatGlass Settings",
        ErrorTag::PeerTooOld => {
            "ask the user to update StatGlass on that host; until then use statglass events or \
             statglass history for it"
        }
        ErrorTag::HistoryUnavailable => {
            "ask the user to quit and reopen StatGlass; until then statglass now still answers"
        }
        ErrorTag::AppNotRunning => "ask the user to open StatGlass (open -a StatGlass), then rerun",
        ErrorTag::AccessDisabled => {
            "ask the user to turn on Allow command-line access in StatGlass Settings"
        }
        ErrorTag::AppNotInstalled => {
            "ask the user to install StatGlass and open it once (open -a StatGlass), then rerun"
        }
        ErrorTag::AppOutdated | ErrorTag::Unsupported => {
            "ask the user to quit and reopen StatGlass"
        }
        ErrorTag::Interrupted => "rerun the command",
        ErrorTag::Defect | ErrorTag::Unknown => {
            "rerun with the same arguments; if it repeats, report it with statglass doctor --json"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audience::Audience;

    /// Every tag. `next_for`'s exhaustive match forces a string for a new tag; this list
    /// makes the test check it is a real instruction.
    const ALL_TAGS: [ErrorTag; 17] = [
        ErrorTag::Usage,
        ErrorTag::UnknownHost,
        ErrorTag::UnknownMetric,
        ErrorTag::BadRange,
        ErrorTag::Busy,
        ErrorTag::HostOffline,
        ErrorTag::HostDisabled,
        ErrorTag::PeerTooOld,
        ErrorTag::HistoryUnavailable,
        ErrorTag::AccessDisabled,
        ErrorTag::AppNotRunning,
        ErrorTag::AppNotInstalled,
        ErrorTag::AppOutdated,
        ErrorTag::Unsupported,
        ErrorTag::Interrupted,
        ErrorTag::Defect,
        ErrorTag::Unknown,
    ];

    #[test]
    fn every_tag_has_an_actionable_next() {
        for tag in ALL_TAGS {
            let next = next_for(tag);
            assert!(
                next.contains("statglass")
                    || next.contains(" --")
                    || next.starts_with("ask the user")
                    || next.contains("retry")
                    || next.contains("rerun"),
                "{tag:?}: {next}"
            );
        }
    }

    #[test]
    fn text_ends_with_next_and_lists_suggestions() {
        let style = Style {
            audience: Audience::Agent,
            stdout_color: false,
            stderr_color: false,
        };
        let f = Failure::usage(Some("hostz"), "no such command", vec!["hosts".into()]);
        let text = f.text(style);
        assert!(text.starts_with("Invalid arguments.\n"), "{text}");
        assert!(text.contains("  Did you mean: hosts?\n"), "{text}");
        let last = text.lines().last().unwrap();
        assert_eq!(
            last,
            "  Next: run statglass --help, or statglass <command> --help"
        );
        let env = f.envelope();
        assert_eq!(env.error.details.unwrap().suggestions, vec!["hosts"]);
        assert!(!env.error.retryable);
    }
}
