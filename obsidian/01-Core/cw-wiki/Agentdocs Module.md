---
tags: [core, cw-wiki, module]
---

# Agentdocs Module

> Keeps a compact project briefing inside `CLAUDE.md` and `AGENTS.md`.

## Source

- `crates/cw-wiki/src/agentdocs.rs` — module `cw_wiki::agentdocs`

## How it works

Main items:

- `render` — build the briefing from the vault and the index.
- `END` — const
- `START` — const
- `merge` — put `briefing` between the markers of `existing`, adding the block at the end when the file has none.
- `apply` — write the briefing into every target file.
- `TARGETS` — files the briefing is written to.
- `MAX_HOTSPOTS` — const
- `MAX_ROUTES` — const

## Depends on

- [[Gitintel Module]] — uses `hotspots`, `load`
- [[cw-core]] — uses `VAULT_DIR`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `io`
- [[Index]] — uses `Index`
- [[VaultMeta]] — uses `load`
- [[Note]] — uses `parse`

## Used by

- [[cw-wiki]] — from `apply_writes_both_files_and_preserves_the_authors_text`, `briefing_routes_agents_to_the_wiki`
- [[codewiki]] — from `run`

## See also

- [[_index]]
- [[cw-wiki]]
