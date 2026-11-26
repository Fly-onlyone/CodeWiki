---
tags: [core, cw-wiki, module]
---

# Agentdocs Module

> Keeps a compact project briefing inside `CLAUDE.md` and `AGENTS.md`.

## What it does

AI agents read `CLAUDE.md` and `AGENTS.md` at the start of every session. This module writes a short briefing into both files. The briefing says where the wiki is, which notes to read for which area, and which files change most. The text sits between two marker comments. Everything outside the markers belongs to the repository authors and is never touched.

## Source

- `crates/cw-wiki/src/agentdocs.rs` — module `cw_wiki::agentdocs`

## How it works

`apply` is the entry point. It calls `render` once to build the briefing, then loops over `TARGETS` (`CLAUDE.md` and `AGENTS.md`).

`render` loads the vault metadata and picks the area notes: flow, pattern, operations and decision notes, plus crate notes (a concept note named like its folder). It parses each note and keeps only those with a summary. Rows are sorted so flows come first, then crates, patterns, decisions, the rest. The table shows at most 14 rows; a line says how many more exist. It then asks [[Gitintel Module]] for the top 5 hotspots and lists them with commit and line counts.

`merge` puts the briefing between `START` and `END`. If both markers exist in order, only the text between them is replaced. If not, the block is appended. An empty file gets just the block.

A file is written only when the merged text differs from the old text. `apply` returns the names of the files it changed. A missing file counts as empty and is created.

## Items

- `pub const START: &str` — public constant; used by `merge`.
- `pub const END: &str` — public constant; used by `merge`.
- `pub const TARGETS: [&str; 2]` — files the briefing is written to.
- `pub fn render(root: &Path, index: &Index) -> Result<String>` — build the briefing from the vault and the index.
- `pub fn merge(existing: &str, briefing: &str) -> String` — put `briefing` between the markers of `existing`, adding the block at the end when the file has none. Text outside the markers is returned unchanged.
- `pub fn apply(root: &Path, index: &Index) -> Result<Vec<String>>` — write the briefing into every target file. Returns the files that changed.

And 2 more private items with no comment.

## Depends on

- [[Gitintel Module]] — uses `hotspots`, `load`
- [[cw-core]] — uses `VAULT_DIR`
- [[Error Module]] — uses `Result`
- [[Note Module]] — uses `parse`
- [[Error]] — uses `io`
- [[Index]] — uses `Index`
- [[VaultMeta]] — uses `load`

## Used by

- [[cw-wiki]] — from `apply_writes_both_files_and_preserves_the_authors_text`, `briefing_routes_agents_to_the_wiki`
- [[codewiki]] — from `run`

## Gotchas

- Pipes in summaries are replaced with slashes, since a pipe would break the Markdown table cell.
- If the markers are in the wrong order (END before START), `merge` does not replace anything. It appends a new block and keeps the old text.
- Running `apply` twice with the same briefing changes nothing, so it is safe to call on every sync.
- A note with an empty summary never appears in the table.

## See also

- [[_index]]
- [[cw-wiki]]
