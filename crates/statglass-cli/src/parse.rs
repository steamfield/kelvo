//! Argument parsing, with clap's errors mapped to `Usage` and our own did-you-mean
//! (`statglass_query::suggest::closest`), never clap's suggestions.

use std::ffi::OsString;

use clap::{
    CommandFactory, Parser,
    error::{ContextKind, ContextValue, ErrorKind},
};
use statglass_query::suggest;

use crate::{cli::Cli, failure::Failure};

pub enum Parsed {
    Run(Cli),
    /// `--help` or `--version`: print this and exit 0.
    Print(String),
}

/// Whether `--json` was asked for, read before parsing so a parse failure still gets
/// the envelope.
pub fn wants_json(args: &[OsString]) -> bool {
    args.iter()
        .skip(1)
        .take_while(|a| *a != "--")
        .any(|a| a == "--json")
}

pub fn parse(args: Vec<OsString>) -> Result<Parsed, Failure> {
    match Cli::try_parse_from(&args) {
        Ok(cli) => Ok(Parsed::Run(cli)),
        Err(e) => from_clap(&e, &args),
    }
}

fn from_clap(e: &clap::Error, args: &[OsString]) -> Result<Parsed, Failure> {
    match e.kind() {
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
            Ok(Parsed::Print(e.render().to_string()))
        }
        ErrorKind::InvalidSubcommand => {
            let name = context(e, ContextKind::InvalidSubcommand).unwrap_or_default();
            let root = Cli::command();
            let names: Vec<&str> = root.get_subcommands().map(|c| c.get_name()).collect();
            let suggestions = suggest::closest(&name, names);
            Err(Failure::usage(Some(&name), "no such command", suggestions))
        }
        ErrorKind::UnknownArgument => {
            let arg = context(e, ContextKind::InvalidArg).unwrap_or_default();
            let flags = flags_of(args);
            let suggestions = suggest::closest(&arg, flags.iter().map(String::as_str));
            Err(Failure::usage(Some(&arg), clap_summary(e), suggestions))
        }
        ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => Err(Failure::usage(
            None,
            "a subcommand is required; run statglass skill --help",
            Vec::new(),
        )),
        _ => {
            let input = context(e, ContextKind::InvalidValue)
                .filter(|v| !v.is_empty())
                .or_else(|| context(e, ContextKind::InvalidArg));
            Err(Failure::usage(
                input.as_deref(),
                clap_summary(e),
                Vec::new(),
            ))
        }
    }
}

fn context(e: &clap::Error, kind: ContextKind) -> Option<String> {
    match e.get(kind)? {
        ContextValue::String(s) => Some(s.clone()),
        ContextValue::Strings(v) => Some(v.join(", ")),
        _ => None,
    }
}

/// clap's message without its `error:` prefix, usage block or help pointer, on one line.
fn clap_summary(e: &clap::Error) -> String {
    let rendered = e.render().to_string();
    let lines: Vec<&str> = rendered
        .lines()
        .map(str::trim)
        .take_while(|l| !l.starts_with("Usage:") && !l.starts_with("For more information"))
        .filter(|l| !l.is_empty() && !l.starts_with("tip:"))
        .collect();
    lines.join(" ").trim_start_matches("error: ").to_owned()
}

/// The long flags of the subcommand named in `args` (or the root), for did-you-mean.
fn flags_of(args: &[OsString]) -> Vec<String> {
    let root = Cli::command();
    let sub = args
        .iter()
        .skip(1)
        .filter_map(|a| a.to_str())
        .find_map(|a| root.find_subcommand(a));
    let cmd = sub.unwrap_or(&root);
    cmd.get_arguments()
        .chain(root.get_arguments())
        .filter_map(|a| a.get_long())
        .map(|l| format!("--{l}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use statglass_query::ErrorTag;

    fn args(list: &[&str]) -> Vec<OsString> {
        std::iter::once("statglass")
            .chain(list.iter().copied())
            .map(OsString::from)
            .collect()
    }

    fn usage(list: &[&str]) -> Failure {
        match parse(args(list)) {
            Err(f) => f,
            Ok(_) => panic!("{list:?} parsed"),
        }
    }

    #[test]
    fn typo_suggests_a_subcommand() {
        let f = usage(&["hostz"]);
        assert_eq!(f.error.tag, ErrorTag::Usage);
        let d = f.error.details.unwrap();
        assert_eq!(d.suggestions, vec!["hosts"]);
        assert_eq!(d.input.as_deref(), Some("hostz"));
    }

    #[test]
    fn typo_in_a_flag_suggests_the_flag() {
        let d = usage(&["history", "cpu.total", "--sinse", "2h"])
            .error
            .details
            .unwrap();
        assert_eq!(d.suggestions, vec!["--since"]);
    }

    #[test]
    fn conflicts_and_bad_values_are_usage() {
        for list in [
            &["history", "cpu.total", "--since", "1h", "--from", "13:00"][..],
            &["history", "cpu.total", "--points", "1001"],
            &["history"],
            &["top", "--by", "disk"],
            &["now", "--all", "--host", "x"],
            &["skill"],
        ] {
            assert_eq!(usage(list).error.tag, ErrorTag::Usage, "{list:?}");
        }
    }

    #[test]
    fn help_and_version_print() {
        assert!(matches!(parse(args(&["--help"])), Ok(Parsed::Print(_))));
        assert!(matches!(parse(args(&["--version"])), Ok(Parsed::Print(_))));
        assert!(matches!(parse(args(&["top", "-h"])), Ok(Parsed::Print(_))));
        assert!(wants_json(&args(&["hostz", "--json"])));
        assert!(!wants_json(&args(&["history", "--", "--json"])));
    }
}
