---
tags: [core, cw-core, module]
---

# Model Module

> Data model shared between the parser, index, graph and API layers.

## Source

- `crates/cw-core/src/model.rs` — module `cw_core::model`

## How it works

Main items:

- [[Language]] — source language of a file.
- [[Symbol]] — A named code entity.
- `Visibility` — enum
- [[SymbolKind]] — enum
- `EdgeKind` — enum
- [[Confidence]] — how sure the resolver is that an edge is real.
- `Edge` — A directed relation between two nodes.
- `FileRecord` — one indexed source file.

## Depends on

- [[Confidence]] — uses `Confidence`
- [[Language]] — uses `Language`

## Used by

- [[Index]] — from `edges`, `edges_from`, `edges_to`, `files`
- [[Store Module]] — from `edge_from_row`, `symbol_from_row`
- [[Scan Module]] — from `RepoFacts`, `scan_repo`
- [[Typescript Module]] — from `member_visibility`, `visibility`
- [[Query Module]] — from `context`, `related`
- [[Repo]] — from `Repo`, `build`
- [[Java Module]] — from `visibility`
- [[Python Module]] — from `visibility`

## See also

- [[_index]]
- [[cw-core]]
