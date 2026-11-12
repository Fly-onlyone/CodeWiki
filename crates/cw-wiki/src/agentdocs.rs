//! Keeps a compact project briefing inside `CLAUDE.md` and `AGENTS.md`.
//!
//! Agent instruction files are loaded into every session, so the briefing is
//! short: where the wiki is, which notes to read for which area, and which
//! files need care. It lives between two marker comments; everything outside
//! the markers belongs to the repository's authors and is never changed.

use std::path::Path;

use cw_core::{Error, Result, VAULT_DIR};
use cw_index::Index;
use cw_vault::{Note, NoteKind, VaultMeta};

use crate::gitintel;

pub const START: &str = "<!-- codewiki:start -->";
pub const END: &str = "<!-- codewiki:end -->";

/// Files the briefing is written to.
pub const TARGETS: [&str; 2] = ["CLAUDE.md", "AGENTS.md"];

const MAX_ROUTES: usize = 14;
const MAX_HOTSPOTS: usize = 5;

/// Build the briefing from the vault and the index.
pub fn render(root: &Path, index: &Index) -> Result<String> {
    let vault = root.join(VAULT_DIR);
    let meta = VaultMeta::load(&vault)?;
    let mut out = String::new();
    out.push_str("## Codebase wiki\n\n");
    out.push_str(&format!(
        "This repository has a maintained wiki in `{VAULT_DIR}/`, kept current by CodeWiki. Read `{VAULT_DIR}/_HOME.md` before exploring code; it maps the architecture and links every area.\n"
    ));
    out.push_str(
        "\nIf the `codewiki` MCP server is connected, prefer its tools: `get_overview` first, `get_context` before editing a file or symbol, `get_why` before changing behavior that looks deliberate, `get_change_risk` before finishing a change.\n",
    );

    // One line per area: the note to read and what it covers.
    let mut routes: Vec<(String, String, String)> = Vec::new();
    for entry in meta.notes.values() {
        let is_area = match entry.kind {
            NoteKind::Flow | NoteKind::Pattern | NoteKind::Operations | NoteKind::Decision => true,
            // Crate notes: a concept note named like its folder.
            NoteKind::Concept => {
                let mut parts = entry.path.rsplit('/');
                let file = parts.next().unwrap_or("").trim_end_matches(".md");
                parts.next() == Some(file)
            }
            _ => false,
        };
        if !is_area {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(vault.join(&entry.path)) else {
            continue;
        };
        let note = Note::parse(&text);
        let summary = note.summary.unwrap_or_default();
        if summary.is_empty() {
            continue;
        }
        let order = match entry.kind {
            NoteKind::Flow => "0",
            NoteKind::Concept => "1",
            NoteKind::Pattern => "2",
            NoteKind::Decision => "3",
            _ => "4",
        };
        routes.push((
            format!("{order}{}", entry.path),
            entry.path.clone(),
            summary,
        ));
    }
    routes.sort();
    if !routes.is_empty() {
        out.push_str("\n| Read | For |\n|---|---|\n");
        for (_, path, summary) in routes.iter().take(MAX_ROUTES) {
            // Table cells cannot hold a pipe.
            out.push_str(&format!(
                "| `{VAULT_DIR}/{path}` | {} |\n",
                summary.replace('|', "/")
            ));
        }
        if routes.len() > MAX_ROUTES {
            out.push_str(&format!(
                "\n{} more area notes are linked from `{VAULT_DIR}/_HOME.md`.\n",
                routes.len() - MAX_ROUTES
            ));
        }
    }

    let intel = gitintel::load(index)?;
    let hotspots = gitintel::hotspots(index, &intel, MAX_HOTSPOTS)?;
    if !hotspots.is_empty() {
        out.push_str("\nFiles that change most and deserve extra care:\n\n");
        for h in &hotspots {
            out.push_str(&format!(
                "- `{}` — {} commits, {} lines\n",
                h.path, h.commits, h.lines
            ));
        }
    }
    out.push_str(&format!(
        "\nNotes you edit by hand are never overwritten; CodeWiki proposes changes to them for review. Do not edit `{VAULT_DIR}/.codewiki/`.\n"
    ));
    Ok(out)
}

/// Put `briefing` between the markers of `existing`, adding the block at the
/// end when the file has none. Text outside the markers is returned unchanged.
pub fn merge(existing: &str, briefing: &str) -> String {
    let block = format!("{START}\n{}\n{END}", briefing.trim_end());
    if let (Some(start), Some(end)) = (existing.find(START), existing.find(END))
        && start < end
    {
        return format!(
            "{}{block}{}",
            &existing[..start],
            &existing[end + END.len()..]
        );
    }
    if existing.trim().is_empty() {
        return format!("{block}\n");
    }
    format!("{}\n\n{block}\n", existing.trim_end())
}

/// Write the briefing into every target file. Returns the files that changed.
pub fn apply(root: &Path, index: &Index) -> Result<Vec<String>> {
    let briefing = render(root, index)?;
    let mut changed = Vec::new();
    for target in TARGETS {
        let path = root.join(target);
        let existing = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(Error::io(&path, e)),
        };
        let merged = merge(&existing, &briefing);
        if merged != existing {
            std::fs::write(&path, merged).map_err(|e| Error::io(&path, e))?;
            changed.push(target.to_string());
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_the_block_and_keeps_existing_text() {
        let existing = "# Project rules\n\nAlways run tests.\n";
        let merged = merge(existing, "## Codebase wiki\n\nRead it.\n");
        assert!(
            merged.starts_with("# Project rules\n\nAlways run tests.\n\n<!-- codewiki:start -->\n")
        );
        assert!(merged.ends_with("Read it.\n<!-- codewiki:end -->\n"));
    }

    #[test]
    fn replaces_only_what_is_between_the_markers() {
        let existing = format!("Top.\n\n{START}\nold briefing\n{END}\n\nBottom stays.\n");
        let merged = merge(&existing, "new briefing");
        assert_eq!(
            merged,
            format!("Top.\n\n{START}\nnew briefing\n{END}\n\nBottom stays.\n")
        );
        // Applying the same briefing again changes nothing.
        assert_eq!(merge(&merged, "new briefing"), merged);
    }

    #[test]
    fn creates_the_block_in_an_empty_file() {
        assert_eq!(merge("", "hello"), format!("{START}\nhello\n{END}\n"));
        assert_eq!(merge("  \n", "hello"), format!("{START}\nhello\n{END}\n"));
    }

    #[test]
    fn misordered_markers_do_not_destroy_text() {
        let existing = format!("{END}\nkeep me\n{START}\n");
        let merged = merge(&existing, "x");
        assert!(merged.contains("keep me"));
        assert!(merged.starts_with(&existing.trim_end().to_string()));
    }
}
