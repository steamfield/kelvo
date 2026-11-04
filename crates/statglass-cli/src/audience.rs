//! Who reads this invocation's output, picked once (RFC "The DX/AX bar", following
//! stacks' `resolveAudience`). The three audiences differ only in rendering, never in
//! exit code or side effect.

use std::io::IsTerminal;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Audience {
    /// `--json`: one result object on stdout, the envelope as the last stderr line.
    Json,
    /// The default: terse text, no escapes, nothing a person needs but an agent doesn't.
    Agent,
    /// A person at a terminal: the same content aligned, with optional color.
    Human,
}

impl Audience {
    pub fn is_json(self) -> bool {
        self == Audience::Json
    }
}

/// Agent session markers: a variable set and non-empty (and equal to the value, when one
/// is given) means an agent is running this command. stacks' `AgentMarkers` plus the
/// Claude Code entrypoint and Gemini CLI variables.
const AGENT_MARKERS: &[(&str, Option<&str>)] = &[
    ("CLAUDECODE", None),
    ("CLAUDE_CODE", None),
    ("CLAUDE_CODE_ENTRYPOINT", None),
    ("CODEX_SANDBOX", None),
    ("CODEX_CI", None),
    ("CODEX_THREAD_ID", None),
    ("OPENCODE", None),
    ("OPENCODE_CLIENT", None),
    ("CURSOR_AGENT", None),
    ("CURSOR_EXTENSION_HOST_ROLE", Some("agent-exec")),
    ("AI_AGENT", None),
    ("GEMINI_CLI", None),
];

/// The inputs the audience and color rules read, so tests can drive them without
/// touching the process environment or a terminal.
pub struct Inputs<E: Fn(&str) -> Option<String>> {
    pub json: bool,
    pub stdout_tty: bool,
    pub stderr_tty: bool,
    pub env: E,
}

impl Inputs<fn(&str) -> Option<String>> {
    /// This process's flags, terminals and environment.
    pub fn from_process(json: bool) -> Self {
        fn var(name: &str) -> Option<String> {
            std::env::var(name).ok()
        }
        Inputs {
            json,
            stdout_tty: std::io::stdout().is_terminal(),
            stderr_tty: std::io::stderr().is_terminal(),
            env: var,
        }
    }
}

impl<E: Fn(&str) -> Option<String>> Inputs<E> {
    fn set(&self, name: &str) -> bool {
        (self.env)(name).is_some_and(|v| !v.is_empty())
    }

    fn agent_detected(&self) -> bool {
        AGENT_MARKERS.iter().any(|(name, equals)| match equals {
            None => self.set(name),
            Some(want) => (self.env)(name).as_deref() == Some(*want),
        })
    }

    /// `--json` is json; human only on a terminal with no agent marker and no `CI`;
    /// everything else (an agent's shell, a pipe, CI) is agent.
    pub fn audience(&self) -> Audience {
        if self.json {
            Audience::Json
        } else if self.stdout_tty && !self.agent_detected() && !self.set("CI") {
            Audience::Human
        } else {
            Audience::Agent
        }
    }

    /// Color for one stream: human audience only; a non-empty `NO_COLOR` wins, then a
    /// terminal or `FORCE_COLOR` enables it.
    fn color(&self, stream_tty: bool) -> bool {
        self.audience() == Audience::Human
            && !self.set("NO_COLOR")
            && (stream_tty || self.set("FORCE_COLOR"))
    }

    pub fn style(&self) -> Style {
        Style {
            audience: self.audience(),
            stdout_color: self.color(self.stdout_tty),
            stderr_color: self.color(self.stderr_tty),
        }
    }
}

/// The resolved rendering choices for one invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub audience: Audience,
    pub stdout_color: bool,
    pub stderr_color: bool,
}

impl Style {
    /// Align columns (human) or join cells with two spaces (agent).
    pub fn aligned(self) -> bool {
        self.audience == Audience::Human
    }

    /// `text` in bold on stdout when color is on.
    pub fn bold(self, text: &str) -> String {
        paint(self.stdout_color, "1", text)
    }

    /// `text` in red on stderr when color is on.
    pub fn error(self, text: &str) -> String {
        paint(self.stderr_color, "31", text)
    }
}

fn paint(on: bool, code: &str, text: &str) -> String {
    if on {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(
        json: bool,
        tty: bool,
        vars: &[(&str, &str)],
    ) -> Inputs<impl Fn(&str) -> Option<String>> {
        let vars: Vec<(String, String)> = vars
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Inputs {
            json,
            stdout_tty: tty,
            stderr_tty: tty,
            env: move |name: &str| vars.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone()),
        }
    }

    #[test]
    fn json_flag_wins_everywhere() {
        assert_eq!(inputs(true, true, &[]).audience(), Audience::Json);
        assert_eq!(
            inputs(true, false, &[("CLAUDECODE", "1")]).audience(),
            Audience::Json
        );
    }

    #[test]
    fn human_needs_a_terminal_no_agent_and_no_ci() {
        assert_eq!(inputs(false, true, &[]).audience(), Audience::Human);
        assert_eq!(inputs(false, false, &[]).audience(), Audience::Agent);
        assert_eq!(
            inputs(false, true, &[("CLAUDECODE", "1")]).audience(),
            Audience::Agent
        );
        assert_eq!(
            inputs(false, true, &[("CI", "true")]).audience(),
            Audience::Agent
        );
        assert_eq!(
            inputs(false, true, &[("CODEX_THREAD_ID", "x")]).audience(),
            Audience::Agent
        );
        assert_eq!(
            inputs(false, true, &[("GEMINI_CLI", "1")]).audience(),
            Audience::Agent
        );
    }

    #[test]
    fn blank_markers_and_wrong_values_do_not_count() {
        assert_eq!(
            inputs(false, true, &[("CLAUDECODE", "")]).audience(),
            Audience::Human
        );
        assert_eq!(
            inputs(false, true, &[("CI", "")]).audience(),
            Audience::Human
        );
        let cursor_person = [("CURSOR_EXTENSION_HOST_ROLE", "user")];
        assert_eq!(
            inputs(false, true, &cursor_person).audience(),
            Audience::Human
        );
        let cursor_agent = [("CURSOR_EXTENSION_HOST_ROLE", "agent-exec")];
        assert_eq!(
            inputs(false, true, &cursor_agent).audience(),
            Audience::Agent
        );
    }

    #[test]
    fn color_is_human_only_and_no_color_wins() {
        assert!(inputs(false, true, &[]).style().stdout_color);
        assert!(
            !inputs(false, true, &[("NO_COLOR", "1")])
                .style()
                .stdout_color
        );
        assert!(
            !inputs(false, false, &[("FORCE_COLOR", "1")])
                .style()
                .stdout_color
        );
        assert!(
            !inputs(true, true, &[("FORCE_COLOR", "1")])
                .style()
                .stdout_color
        );
        let agent = inputs(false, false, &[]).style();
        assert_eq!(agent.bold("x"), "x");
        assert_eq!(agent.error("x"), "x");
    }
}
