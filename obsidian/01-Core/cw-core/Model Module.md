---
tags: [core, cw-core, module]
---

# Model Module

> Data model shared between the parser, index, graph and API layers.

## Source

- `crates/cw-core/src/model.rs` — module `cw_core::model`

## How it works

1. `context` in [[Query Module]] and `Resolver::resolve` in [[Resolver]] call `file_node`.
2. `symbol_from_row` in [[Store Module]] calls `Visibility::parse`.
3. `edge_from_row` in [[Store Module]] calls `EdgeKind::parse`.

## Items

- [[Language]] — source language of a file.
- `pub struct FileRecord` — one indexed source file. `path` is repo-relative with forward slashes.
- [[SymbolKind]] — enum
- `pub enum Visibility` — public enum; used by `Extractor::member_visibility`, `Extractor::visibility`, `Modifiers::visibility`, `Symbol` and 1 more.
- [[Symbol]] — A named code entity.
- `pub enum EdgeKind` — public enum; used by `DEPENDENCY_KINDS`, `Edge`, `Graph::from_edges`, `Resolver::resolve`.
- [[Confidence]] — how sure the resolver is that an edge is real.
- `pub struct Edge` — A directed relation between two nodes. A node is a symbol's qualified name, or a repo-relative file path prefixed with `file:` for file-level nodes.
- `pub fn file_node(path: &str) -> String` — node id for a file-level node.

## Depends on

- [[Confidence]] — uses `Confidence`
- [[Language]] — uses `Language`

## Used by

- [[Index]] — from `edges`, `edges_from`, `edges_to`, `files`
- [[Repo]] — from `Repo`, `build`, `calls_from`
- [[Store Module]] — from `edge_from_row`, `symbol_from_row`
- [[Scan Module]] — from `RepoFacts`, `scan_repo`
- [[Typescript Module]] — from `member_visibility`, `visibility`
- [[Query Module]] — from `context`, `related`
- [[Java Module]] — from `visibility`
- [[Python Module]] — from `visibility`

## See also

- [[_index]]
- [[cw-core]]
