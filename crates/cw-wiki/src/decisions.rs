//! Decision records and the "why" query.
//!
//! Decisions are read from two places: notes tagged `decision` in the vault,
//! and ADR files the repository already keeps (`docs/adr/`, `decisions/`).
//! A decision goes stale when the code it cites changes after it was written.

use std::path::Path;
use std::process::Command;

use cw_core::{Error, Result, VAULT_DIR};
use cw_index::Index;
use cw_vault::{Note, NoteKind, VaultMeta, sanitize_name};
use serde::{Deserialize, Serialize};

use crate::gitintel::{self, GitIntel};

/// Vault folder new decisions are written to.
pub const DECISIONS_ZONE: &str = "07-Decisions";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    /// Repo-relative path of the record.
    pub path: String,
    pub title: String,
    /// Lowercase status word: `accepted`, `proposed`, `superseded`, ...
    pub status: String,
    pub summary: String,
    /// Existing source files the record cites.
    pub cites: Vec<String>,
    /// Cited paths that are no longer in the repository.
    pub missing: Vec<String>,
    pub stale: bool,
    /// Why the record is considered stale; empty when it is not.
    pub stale_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rationale {
    /// Vault path of the note the text comes from.
    pub note: String,
    /// Section heading: `Why`, `Gotchas` or `Don't`.
    pub section: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub date: String,
    pub author: String,
    pub subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Why {
    /// The file the answer is about.
    pub file: String,
    pub decisions: Vec<Decision>,
    pub rationale: Vec<Rationale>,
    /// Most recent commits touching the file, newest first.
    pub commits: Vec<CommitInfo>,
}

/// Is this repo path an ADR file by the usual conventions?
fn is_adr_path(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".md")
        && lower.split('/').rev().skip(1).any(|dir| {
            matches!(
                dir,
                "adr" | "adrs" | "decisions" | "decision-records" | "architecture-decisions"
            )
        })
        && !lower.ends_with("readme.md")
        && !lower.ends_with("template.md")
}

/// Status from a `Status: accepted` line or the first line under `## Status`.
fn status_of(text: &str, note: &Note) -> String {
    let from_section = note.section("Status").and_then(|s| {
        s.body
            .lines()
            .find(|l| !l.trim().is_empty())
            .map(str::to_string)
    });
    let from_line = text.lines().find_map(|l| {
        let l = l
            .trim()
            .trim_start_matches(['-', '*', ' '])
            .trim_start_matches("**");
        l.strip_prefix("Status")
            .map(|rest| rest.trim_start_matches(['*', ':', ' ']).to_string())
            .filter(|rest| !rest.is_empty())
    });
    from_section
        .or(from_line)
        .and_then(|s| {
            s.split(|c: char| !c.is_alphabetic())
                .find(|w| !w.is_empty())
                .map(str::to_lowercase)
        })
        .unwrap_or_else(|| "accepted".to_string())
}

/// Backticked spans that name files in the repository, split into existing
/// and missing.
fn cited_paths(text: &str, index_files: &[String]) -> (Vec<String>, Vec<String>) {
    let (mut found, mut missing) = (Vec::new(), Vec::new());
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        for span in line.split('`').skip(1).step_by(2) {
            let path = span.trim().trim_end_matches('/');
            let file_like = path.contains('/')
                && !path.contains(' ')
                && !path.contains("::")
                && path.rsplit('/').next().is_some_and(|f| f.contains('.'));
            if !file_like {
                continue;
            }
            let bucket = if index_files.iter().any(|f| f == path) {
                &mut found
            } else {
                &mut missing
            };
            if !bucket.iter().any(|p: &String| p == path) {
                bucket.push(path.to_string());
            }
        }
    }
    (found, missing)
}

fn build(path: &str, text: &str, index_files: &[String], intel: &GitIntel) -> Decision {
    let note = Note::parse(text);
    let stem = path
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .trim_end_matches(".md");
    let (cites, missing) = cited_paths(text, index_files);
    let written = intel.files.get(path).map(|h| h.last_changed);
    let mut stale_reasons: Vec<String> = missing
        .iter()
        .map(|p| format!("cites `{p}`, which no longer exists"))
        .collect();
    if let Some(written) = written {
        for cited in &cites {
            if let Some(history) = intel.files.get(cited)
                && history.last_changed > written
            {
                stale_reasons.push(format!(
                    "`{cited}` changed after this decision was last updated"
                ));
            }
        }
    }
    let status = status_of(text, &note);
    Decision {
        path: path.to_string(),
        title: note.title.clone().unwrap_or_else(|| stem.to_string()),
        summary: note.summary.clone().unwrap_or_default(),
        // A superseded decision is history; it cannot go stale.
        stale: !stale_reasons.is_empty() && status != "superseded" && status != "deprecated",
        status,
        cites,
        missing,
        stale_reasons,
    }
}

/// Every decision record in the repository and its vault, by path.
pub fn list(root: &Path, index: &Index) -> Result<Vec<Decision>> {
    let intel = gitintel::load(index)?;
    let files: Vec<String> = index.files()?.into_iter().map(|f| f.path).collect();
    let mut out = Vec::new();
    for path in files.iter().filter(|p| is_adr_path(p)) {
        if let Ok(text) = std::fs::read_to_string(root.join(path)) {
            out.push(build(path, &text, &files, &intel));
        }
    }
    let vault = root.join(VAULT_DIR);
    let meta = VaultMeta::load(&vault)?;
    for entry in meta.notes.values() {
        let Ok(text) = std::fs::read_to_string(vault.join(&entry.path)) else {
            continue;
        };
        let tagged = Note::parse(&text)
            .tags
            .iter()
            .any(|t| t == "decision" || t == "adr");
        if entry.kind == NoteKind::Decision || tagged {
            out.push(build(
                &format!("{VAULT_DIR}/{}", entry.path),
                &text,
                &files,
                &intel,
            ));
        }
    }
    // Decisions written but not yet adopted into the manifest.
    if let Ok(entries) = std::fs::read_dir(vault.join(DECISIONS_ZONE)) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let path = format!("{VAULT_DIR}/{DECISIONS_ZONE}/{name}");
            if name.ends_with(".md")
                && !name.starts_with('_')
                && !out.iter().any(|d| d.path == path)
                && let Ok(text) = std::fs::read_to_string(entry.path())
            {
                out.push(build(&path, &text, &files, &intel));
            }
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Create a new decision note from the standard template and return its
/// vault-relative path. The note is the author's to fill in.
pub fn create(root: &Path, title: &str) -> Result<String> {
    let title = sanitize_name(title);
    if title.is_empty() {
        return Err(Error::invalid("a decision needs a title"));
    }
    let dir = root.join(VAULT_DIR).join(DECISIONS_ZONE);
    std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    let mut highest = 0u32;
    for entry in std::fs::read_dir(&dir)
        .map_err(|e| Error::io(&dir, e))?
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(number) = name
            .strip_prefix("ADR-")
            .and_then(|rest| rest.split(' ').next())
            .and_then(|n| n.parse::<u32>().ok())
        {
            highest = highest.max(number);
        }
    }
    let name = format!("ADR-{:04} {title}", highest + 1);
    let text = format!(
        "---\ntags: [decision]\n---\n\n# {name}\n\n> One sentence: what was decided.\n\n## Status\n\nProposed\n\n## Context\n\nWhat forces are at play, and what made a decision necessary. Cite the code this is about as `path/to/file.rs`.\n\n## Decision\n\nWhat was chosen, stated as a fact.\n\n## Consequences\n\nWhat becomes easier, what becomes harder, and what must **not** be done as a result.\n\n## See also\n\n- [[_HOME]]\n"
    );
    let path = dir.join(format!("{name}.md"));
    std::fs::write(&path, text).map_err(|e| Error::io(&path, e))?;
    Ok(format!("{DECISIONS_ZONE}/{name}.md"))
}

fn recent_commits(root: &Path, file: &str, limit: usize) -> Vec<CommitInfo> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "log",
            &format!("-n{limit}"),
            "--date=short",
            "--format=%h%x02%ad%x02%an%x02%s",
            "--",
            file,
        ])
        .output();
    let Ok(out) = output else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(4, '\u{2}');
            Some(CommitInfo {
                hash: parts.next()?.to_string(),
                date: parts.next()?.to_string(),
                author: parts.next()?.to_string(),
                subject: parts.next()?.to_string(),
            })
        })
        .collect()
}

/// Why is this code the way it is? `target` is a file path or a symbol name.
/// Collects the decisions that cite the file, the rationale recorded in notes
/// about it, and the latest commits that touched it.
pub fn why(root: &Path, index: &Index, target: &str) -> Result<Why> {
    let normalized = target.replace('\\', "/");
    let file = if index.files()?.iter().any(|f| f.path == normalized) {
        normalized
    } else {
        let symbol = match index.symbol(target)? {
            Some(s) => s,
            None => index
                .find_symbols(target, 1)?
                .into_iter()
                .next()
                .ok_or_else(|| Error::not_found(format!("file or symbol '{target}'")))?,
        };
        symbol.file
    };
    let decisions = list(root, index)?
        .into_iter()
        .filter(|d| d.cites.contains(&file))
        .collect();
    let vault = root.join(VAULT_DIR);
    let meta = VaultMeta::load(&vault)?;
    let mut rationale = Vec::new();
    for entry in meta.notes_for_source(&file) {
        let Ok(text) = std::fs::read_to_string(vault.join(&entry.path)) else {
            continue;
        };
        let note = Note::parse(&text);
        for heading in ["Why", "Gotchas", "Don't"] {
            if let Some(section) = note.section(heading) {
                rationale.push(Rationale {
                    note: entry.path.clone(),
                    section: heading.to_string(),
                    text: section.body.clone(),
                });
            }
        }
    }
    rationale.sort_by(|a, b| a.note.cmp(&b.note).then(a.section.cmp(&b.section)));
    Ok(Why {
        commits: recent_commits(root, &file, 8),
        file,
        decisions,
        rationale,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_adr_paths() {
        for yes in [
            "docs/adr/0001-use-rust.md",
            "decisions/x.md",
            "doc/ADRs/A.md",
        ] {
            assert!(is_adr_path(yes), "{yes}");
        }
        for no in [
            "docs/adr/README.md",
            "docs/adr/template.md",
            "src/adr.md",
            "docs/adr/x.txt",
            "adr.md",
        ] {
            assert!(!is_adr_path(no), "{no}");
        }
    }

    #[test]
    fn reads_status_in_common_forms() {
        let status = |text: &str| status_of(text, &Note::parse(text));
        assert_eq!(
            status("# T\n\n## Status\n\nAccepted on 2026-01-02\n"),
            "accepted"
        );
        assert_eq!(status("# T\n\nStatus: Superseded by ADR-9\n"), "superseded");
        assert_eq!(status("# T\n\n- **Status**: proposed\n"), "proposed");
        assert_eq!(status("# T\n\nNo status here.\n"), "accepted");
    }

    #[test]
    fn splits_cited_paths_into_existing_and_missing() {
        let files = vec!["src/core.rs".to_string()];
        let text = "Uses `src/core.rs` and `src/gone.rs`, not `cw_core::hash` or `a/b`.\n```\n`src/in_code.rs`\n```\n";
        let (found, missing) = cited_paths(text, &files);
        assert_eq!(found, ["src/core.rs"]);
        assert_eq!(missing, ["src/gone.rs"]);
    }
}
