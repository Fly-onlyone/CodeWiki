---
tags: [core, cw-wiki, module]
---

# Gitintel Module

> Git intelligence: what history says about each file.

## Source

- `crates/cw-wiki/src/gitintel.rs` — module `cw_wiki::gitintel`

## How it works

Main items:

- [[FileHistory]] — struct
- `load` — load mined history from the index; empty when none was stored.
- `Partner` — struct
- `RiskLevel` — enum
- `GitIntel` — struct
- `Owner` — struct
- `Hotspot` — struct
- `hotspots` — files that are both large and frequently changed, highest first.

## Depends on

- [[Index]] — uses `Index`, `edges`, `files`, `meta`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `external`
- [[FileHistory]] — uses `FileHistory`

## Used by

- [[Api Module]] — from `blast_radius`, `change_risk`, `file_risk`, `history`
- [[cw-wiki]] — from `blast_radius_follows_dependents`, `change_risk_flags_forgotten_partners_and_reads_the_working_tree`, `file_risk_explains_itself`, `mines_history_and_ranks_hotspots`
- [[Workspace Module (cw-server)]] — from `WorkspaceRepo`, `summarize`
- [[Decisions Module]] — from `build`, `list`
- [[codewiki]] — from `run`
- [[Agentdocs Module]] — from `render`
- [[Coverage Module]] — from `report`
- [[Refactor Module]] — from `load`

## See also

- [[_index]]
- [[cw-wiki]]
