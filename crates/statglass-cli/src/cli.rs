//! The command tree. Every flag and argument carries help, because an agent reads
//! `--help` cold; every subcommand's help states its exit codes and ends with examples,
//! including the `--json` form an agent runs.

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

const ROOT_ABOUT: &str = "Ask StatGlass what happened on this Mac and the user's other machines: \
current readings, history with explicit gaps, detector events and per-app usage.";

const ROOT_LONG: &str = "Ask StatGlass what happened on this Mac and the user's other machines: \
current readings, history with explicit gaps, detector events and per-app usage. Answers come \
from the running StatGlass app over a private socket; this tool never reads storage and never \
changes anything. Run statglass with no arguments for the agent guide.

Output: text for agents and pipes, aligned text at a terminal, one JSON object with --json \
(schema: statglass schema <command>). On failure stdout is empty and stderr ends with a Next: \
line, or with one JSON error envelope under --json.

Exit codes: 0 success, 1 bug, 2 fix the input, 4 retry shortly, 5 the data is not available \
(don't retry), 6 ask the user (StatGlass not running, access off, outdated), 130 interrupted.";

const ROOT_AFTER: &str = "Start here (agents): statglass hosts, then statglass events --since 3h \
and statglass top --since 1h to find what happened.
Then statglass history '<series>' --since 3h for the trend; quote series arguments.
Run statglass guide for the full guide; add --json for one JSON object.";

const HOST_HELP: &str = "Host display name, host id, or local; case-insensitive. Default: local \
(this Mac). Run statglass hosts for names.";

#[derive(Debug, Parser)]
#[command(
    name = "statglass",
    // The PATH link's name, not the bundled executable's (`statglass-cli`).
    bin_name = "statglass",
    version,
    about = ROOT_ABOUT,
    long_about = ROOT_LONG,
    after_help = ROOT_AFTER,
    disable_help_flag = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
    #[command(flatten)]
    pub global: Global,
}

#[derive(Debug, Args)]
pub struct Global {
    /// Write one JSON result object to stdout; a failure writes one JSON error envelope as
    /// the last stderr line and leaves stdout empty.
    #[arg(long, global = true)]
    pub json: bool,
    /// Print help: every flag, exit codes and examples.
    #[arg(short = 'h', long, global = true, action = ArgAction::HelpLong)]
    help: Option<bool>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print the agent guide, or one topic of it.
    #[command(
        long_about = "Print the agent guide (the same text statglass prints with no arguments), \
or one topic: series (series names and quoting), time (time flags and how times print), errors \
(exit codes, freshness and gaps). Exits 0, or 2 on an unknown topic.",
        after_help = "EXAMPLES
  # The guide
  statglass guide

  # How to write series arguments
  statglass guide series

  # As JSON: {\"guide\": \"...\"}
  statglass guide errors --json"
    )]
    Guide {
        /// series, time or errors. Omit for the whole guide.
        topic: Option<String>,
    },

    /// List every host StatGlass knows, with state and retained history.
    #[command(
        long_about = "List every host StatGlass knows: this Mac first, then remote Macs and Linux \
hosts by name. Each line shows the name, kind, state (local, connected, offline, disabled, \
error), when it was last seen if not now, how far its mirror is synced, and the span of \
history it retains. Offline and disabled hosts are listed and keep their history. Exits 0 \
with the list, 1 on a bug, 4 when StatGlass is busy, 6 when StatGlass is not running, not \
installed, outdated or has command-line access off (ask the user).",
        after_help = "EXAMPLES
  # Every host
  statglass hosts

  # One HostsAnswer object, as an agent would
  statglass hosts --json"
    )]
    Hosts,

    /// Current readings of one host, or every host.
    #[command(
        long_about = "Current readings from the newest frame StatGlass holds for a host, grouped \
by module, with the frame's age. Remote hosts report their headline series; for more detail on \
a remote host use statglass history '<series>' --since 2m. An offline host shows when it was \
last seen and no readings. With --all, every host is listed, and a host that fails shows its \
error while the command still exits 0. Exits 0 with the readings, 1 on a bug, 2 on bad \
arguments or an unknown host, 4 when StatGlass is busy, 6 when StatGlass is not running, not \
installed, outdated or has command-line access off (ask the user).",
        after_help = "EXAMPLES
  # This Mac now
  statglass now

  # A remote host
  statglass now --host m1-linux

  # Every host, as one NowAnswer object
  statglass now --all --json"
    )]
    Now {
        #[arg(long, value_name = "HOST", help = HOST_HELP)]
        host: Option<String>,
        /// Every host, offline and disabled ones included.
        #[arg(long, conflicts_with = "host")]
        all: bool,
    },

    /// The metric catalog, and the series one host has.
    #[command(
        long_about = "The metric catalog: each metric's id, label keys, unit, kind, sample period \
and whether it keeps history beyond the last hour, grouped by module. With --host, also the \
series that host has stored, which are the names statglass history takes. Exits 0 with the \
catalog, 1 on a bug, 2 on bad arguments, an unknown module or an unknown host, 4 when StatGlass \
is busy, 5 when the host's history is not available, 6 when StatGlass is not running, not \
installed, outdated or has command-line access off (ask the user).",
        after_help = "EXAMPLES
  # The whole catalog
  statglass metrics

  # The disk series m1-linux has
  statglass metrics --host m1-linux --module disk

  # One MetricsAnswer object
  statglass metrics --module cpu --json"
    )]
    Metrics {
        #[arg(long, value_name = "HOST", help = HOST_HELP)]
        host: Option<String>,
        /// One module: cpu, gpu, memory, power, sensors, network, disk, battery, containers.
        #[arg(long, value_name = "MODULE")]
        module: Option<String>,
    },

    /// Summary, gaps and optional points for series over a time range.
    #[command(
        long_about = HISTORY_LONG,
        after_help = "EXAMPLES
  # CPU on this Mac over the last 3 hours
  statglass history cpu.total --since 3h

  # Every core this afternoon, with 12 points each
  statglass history cpu.load --from 13:00 --to 16:00 --points 12

  # A container on a Linux host; quote braces
  statglass history 'cgroup.cpu{kind=container,name=postgres}' --host m1-linux --since 1d

  # One HistoryAnswer object (60 points per series by default)
  statglass history cpu.total mem.used --since 2h --json"
    )]
    History {
        /// Series in catalog form: cpu.total, 'cpu.load{core=3}'. A bare labelled metric
        /// (cpu.load) selects every series of it. Single-quote anything with braces.
        #[arg(required = true, value_name = "SERIES")]
        series: Vec<String>,
        #[arg(long, value_name = "HOST", help = HOST_HELP)]
        host: Option<String>,
        #[command(flatten)]
        range: RangeArgs,
        /// Downsampled points per series, 0 to 1000. Text output shows none unless given;
        /// --json defaults to 60.
        #[arg(long, value_name = "N", value_parser = clap::value_parser!(u32).range(0..=1000))]
        points: Option<u32>,
    },

    /// Detector and alert events over a time range, with the processes involved.
    #[command(
        long_about = "Detector and alert events over a time range in time order: fans ramping, \
thermal state changes, sustained heavy processes, power spikes and user alerts, each with the \
processes held responsible. Events are mirrored, so offline hosts have them too. Run this before \
history when asking what caused something. Events a newer StatGlass recorded that this one \
cannot read are counted, never silently dropped. Exits 0 with the events (none is a valid \
answer once freshness says the data is current), 1 on a bug, 2 on bad arguments, an unknown \
host or a bad range, 4 when StatGlass is busy, 5 when the host is disabled or history is not \
available, 6 when StatGlass is not running, not installed, outdated or has command-line access \
off (ask the user).",
        after_help = "EXAMPLES
  # Events on this Mac today since 9am
  statglass events --from 09:00

  # Thermal events on m1-linux over two days
  statglass events --host m1-linux --kind thermal_state --since 2d

  # One EventsAnswer object
  statglass events --since 3h --json"
    )]
    Events {
        #[arg(long, value_name = "HOST", help = HOST_HELP)]
        host: Option<String>,
        #[command(flatten)]
        range: RangeArgs,
        /// One event kind: fans_ramped, thermal_state, sustained_process, power_spike, alert.
        #[arg(long, value_name = "KIND")]
        kind: Option<String>,
    },

    /// Apps and processes that used the most CPU, memory or energy over a range.
    #[command(
        long_about = "The apps that used the most over a time range, by value descending, each \
with its share of the host's total and its top processes. This Mac answers from the last hour \
at full detail and from 30 days of durable usage beyond it. A remote host is asked live, so it \
must be connected; remote hosts serve at most 10 apps, and a larger --limit is reduced and \
says so. Exits 0 with the apps, 1 on a bug, 2 on bad arguments, an unknown host or a bad \
range, 4 when StatGlass or the host is busy, 5 when the host is offline or too old to answer \
(use statglass events or statglass history instead; don't retry), 6 when StatGlass is not \
running, not installed, outdated or has command-line access off (ask the user).",
        after_help = "EXAMPLES
  # What used the most CPU on this Mac in the last hour
  statglass top

  # Memory on m1-linux since 13:00, 5 apps
  statglass top --host m1-linux --by memory --from 13:00 --limit 5

  # One TopAnswer object
  statglass top --by energy --since 1d --json"
    )]
    Top {
        #[arg(long, value_name = "HOST", help = HOST_HELP)]
        host: Option<String>,
        #[command(flatten)]
        range: RangeArgs,
        /// What to rank by.
        #[arg(long, value_enum, default_value_t = By::Cpu)]
        by: By,
        /// How many apps, 1 to 100.
        #[arg(long, value_name = "N", default_value_t = 10,
              value_parser = clap::value_parser!(u32).range(1..=100))]
        limit: u32,
    },

    /// Check why statglass can or cannot reach StatGlass.
    #[command(
        long_about = "Report this tool's version, the StatGlass bundle id and data directory it \
resolved, the query socket and whether it exists, whether StatGlass is running, whether \
command-line access is on, this Mac's timezone, and, when StatGlass answers, its version, api \
and host count. Problems are reported with the step to fix each, not raised. Exits 0 with the \
findings, even when StatGlass is unreachable, or 1 when the check itself hits a bug.",
        after_help = "EXAMPLES
  # Why doesn't statglass work?
  statglass doctor

  # The same findings as one JSON object
  statglass doctor --json"
    )]
    Doctor,

    /// Print the JSON Schema of a --json result, or list them.
    #[command(
        long_about = "With no name, list every published JSON Schema: one per --json result, \
the error envelope and the socket request. With a name, print that schema. Exits 0, or 2 on an \
unknown name.",
        after_help = "EXAMPLES
  # What schemas exist
  statglass schema

  # The shape of statglass history --json
  statglass schema history

  # The error envelope on the last stderr line
  statglass schema error

  # The list as one JSON object
  statglass schema --json"
    )]
    Schema {
        /// A schema name from the list (hosts, now, history, error, ...).
        name: Option<String>,
    },

    /// Install the bundled agent skill.
    #[command(
        long_about = "Manage the bundled agent skill, which tells coding agents when and how to \
use statglass. Exits 2 without a subcommand; see statglass skill install --help.",
        after_help = "EXAMPLES
  # Install for every project of this user
  statglass skill install"
    )]
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
}

const HISTORY_LONG: &str = "For each series over a time range: min, span-weighted mean, max with \
the bucket it fell in, last value, the share of the range that was measured, and every gap with \
its reason (sleep, host offline, not synced, ...). A gap is unknown data, never zero. The \
summary covers every stored bucket in the range, so it does not change with --points. The time \
of the max is the start of its bucket (1 min up to 7 days back, 15 min beyond), and means over \
15-minute buckets are approximate, marked (mean approximate). A range before the host's \
retained history is clamped and says where history starts. Exits 0 with the history, 1 on a \
bug, 2 on bad arguments, an unknown host or series (quote series with braces) or a bad range, 4 \
when StatGlass is busy, 5 when the host is disabled or history is not available, 6 when \
StatGlass is not running, not installed, outdated or has command-line access off (ask the user).";

#[derive(Debug, Subcommand)]
pub enum SkillAction {
    /// Write the skill to ~/.claude/skills/statglass/ or ./.claude/skills/statglass/.
    #[command(
        long_about = "Write the bundled skill (SKILL.md and reference/statglass.md) to \
~/.claude/skills/statglass/ (--user, the default) or ./.claude/skills/statglass/ (--project). \
Identical files are left alone and changed ones replaced; each path is reported as created, \
updated or unchanged. Exits 0 when written, 1 when a file cannot be written.",
        after_help = "EXAMPLES
  # For every project of this user
  statglass skill install

  # For this repository only
  statglass skill install --project

  # Report each file as JSON
  statglass skill install --json"
    )]
    Install {
        /// Install under ~/.claude/skills/statglass/ (the default).
        #[arg(long, conflicts_with = "project")]
        user: bool,
        /// Install under ./.claude/skills/statglass/ in the current directory.
        #[arg(long)]
        project: bool,
    },
}

/// `--since`, or `--from`/`--to`.
#[derive(Debug, Args)]
pub struct RangeArgs {
    /// How far back from now: 90s, 15m, 2h, 3d. Default 1h. Not combined with --from/--to.
    #[arg(long, value_name = "DURATION", conflicts_with_all = ["from", "to"])]
    pub since: Option<String>,
    /// Start: RFC 3339 (2026-11-03T15:00:00-08:00), a time today in this Mac's timezone
    /// (15:00), or a duration ago (3h).
    #[arg(long, value_name = "TIME")]
    pub from: Option<String>,
    /// End, in the same forms as --from. Default now; with --to alone the range is the
    /// hour before it.
    #[arg(long, value_name = "TIME")]
    pub to: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum By {
    Cpu,
    Memory,
    Energy,
}
