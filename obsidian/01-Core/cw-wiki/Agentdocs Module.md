---
tags: [core, cw-wiki, module]
---

# Agentdocs Module

> Keeps a compact project briefing inside `CLAUDE.md` and `AGENTS.md`.

## What it does

Agent instruction files are loaded into every session, so the briefing is short: where the wiki is, which notes to read for which area, and which files need care. It lives between two marker comments; everything outside the markers belongs to the repository's authors and is never changed.

## Source

- `crates/cw-wiki/src/agentdocs.rs` — module `cw_wiki::agentdocs`

## How it works

1. `run` in [[codewiki]] calls `apply`. `apply` calls `render`, then `Error::io` in [[Error]], then `merge`. `render` calls `VaultMeta::load` in [[VaultMeta]], then `Note::parse` in [[Note Module]], then `load`, then `hotspots` (both in [[Gitintel Module]]).

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[cw-wiki]]
