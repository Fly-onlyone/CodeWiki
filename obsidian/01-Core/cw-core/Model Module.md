---
tags: [core, cw-core, module]
---

# Model Module

> Data model shared between the parser, index, graph and API layers.

## What it does

The plain data types of the code graph. The parser makes them, the index stores them, and the query, health and API layers read them. They are a file ([[Language]], `FileRecord`), a named thing in a file ([[Symbol]] with its `SymbolKind` and `Visibility`), and a relation between two nodes (`Edge` with its `EdgeKind` and [[Confidence]]). Nothing here does work. The module is the shared vocabulary.

## Source

- `crates/cw-core/src/model.rs` — module `cw_core::model`

## How it works

A graph node is the qualified name of a symbol, such as `cw_core::hash::sha256_hex`, or `file:<path>` for a file. `file_node` builds the second form.

Edge kinds are imports, calls, implements, references and declares. For a call, `site` holds the byte offset of the first such call in the caller's file; sorting a caller's edges by it restores source order. For other kinds it is 0.

Every enum has `as_str` and `parse` for the text stored in the index. They serialize with serde, in lower case or snake case.

`SymbolKind::is_type` marks structs, enums, unions, traits, classes and interfaces. Only those can earn their own concept note. `Visibility` has three levels: public, crate and private.

## Items

- [[Language]] — source language of a file.
- `pub struct FileRecord` — one indexed source file. `path` is repo-relative with forward slashes.
- [[SymbolKind]] — enum
- [[Visibility]] — enum
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
- [[Scan Module]] — from `RepoFacts`, `scan_repo`
- [[Query Module]] — from `context`, `related`
- [[Store Module]] — from `edge_from_row`
- [[Resolve Module]] — from `Resolved`
- [[Api Module]] — from `list_files`
- [[Analyze Module]] — from `DEPENDENCY_KINDS`

## Gotchas

- `Language`, `Visibility` and `Confidence` parse unknown text to a default (`Other`, `Private`, `Low`). `SymbolKind` and `EdgeKind` return `None`, so [[Store Module]] must decide what a bad row means.
- `Symbol.attrs` was added later with `serde(default)`, so older stored data still loads.
- Line numbers are 1-based and inclusive.
- File paths use forward slashes and are relative to the repository root.

## See also

- [[_index]]
- [[cw-core]]
