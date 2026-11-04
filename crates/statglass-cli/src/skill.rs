//! `statglass skill install`: writes the bundled skill, file by file, leaving identical
//! files alone.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::{Output, failure::Failure, guide};

const SKILL_MD: &str = include_str!("../skill/SKILL.md");

/// The skill's files, relative to its directory.
pub fn files() -> [(&'static str, String); 2] {
    [
        ("SKILL.md", SKILL_MD.to_owned()),
        ("reference/statglass.md", reference()),
    ]
}

/// `reference/statglass.md`: the guide and every topic, as one Markdown page.
fn reference() -> String {
    let mut out = String::from("# statglass reference\n\n## Guide\n\n");
    out.push_str(guide::GUIDE);
    for (_, text) in guide::TOPICS {
        let (title, body) = text.split_once('\n').unwrap_or((text, ""));
        out.push_str(&format!("\n## {title}\n{body}"));
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Created,
    Updated,
    Unchanged,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Created => "created",
            Status::Updated => "updated",
            Status::Unchanged => "unchanged",
        }
    }
}

pub fn install(project: bool, json: bool) -> Result<Output, Failure> {
    let base = if project {
        std::env::current_dir()
            .map_err(|e| Failure::defect(format!("cannot read the current directory: {e}")))?
    } else {
        std::env::var_os("HOME")
            .filter(|h| !h.is_empty())
            .map(PathBuf::from)
            .ok_or_else(|| Failure::defect("HOME is not set"))?
    };
    let dir = base.join(".claude/skills/statglass");
    let mut written = Vec::new();
    for (rel, content) in files() {
        let path = dir.join(rel);
        let status = write_if_changed(&path, &content).map_err(|e| unwritable(&path, &e))?;
        written.push((path, status));
    }
    if json {
        let files: Vec<_> = written
            .iter()
            .map(|(p, s)| json!({ "path": p.display().to_string(), "status": s.as_str() }))
            .collect();
        return Output::json(&json!({ "dir": dir.display().to_string(), "files": files }));
    }
    let text = written
        .iter()
        .map(|(p, s)| format!("{} {}\n", s.as_str(), p.display()))
        .collect();
    Ok(Output::Text(text))
}

/// A file the install could not read or write. Every such failure comes from the user's
/// filesystem (permissions, a read-only volume, a file where a directory should be), not
/// from a bug here, so it is `Usage` naming the path.
fn unwritable(path: &Path, e: &std::io::Error) -> Failure {
    let shown = path.display().to_string();
    let hint = format!("cannot write {shown}: {e}; check permissions or use --user/--project");
    Failure::usage(Some(&shown), hint, Vec::new())
}

fn write_if_changed(path: &Path, content: &str) -> std::io::Result<Status> {
    let status = match std::fs::read(path) {
        Ok(existing) if existing == content.as_bytes() => return Ok(Status::Unchanged),
        Ok(_) => Status::Updated,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Status::Created,
        Err(e) => return Err(e),
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_has_frontmatter_and_reference_has_every_topic() {
        let [(_, skill), (_, reference)] = files();
        assert!(skill.starts_with("---\nname: statglass\ndescription: "));
        for heading in ["## Guide", "## Series and quoting", "## Time", "## Errors"] {
            assert!(reference.contains(heading), "{heading}");
        }
    }
}
