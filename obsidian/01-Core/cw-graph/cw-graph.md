---
tags: [core, cw-graph]
---

# cw-graph

> Graph construction, metrics, community detection and layout for CodeWiki.

## What it does

Every algorithm here is deterministic: the same input always yields the same output, so generated notes and stored layouts do not churn.

## Source

- `crates/cw-graph/Cargo.toml` — package manifest
- `crates/cw-graph/src/lib.rs` — crate root

## How it works

Builds a library.

## Items

Modules:

- [[Community Module]] — community detection.
- [[Graph Module]] — directed weighted graph over string-named nodes.
- [[Layout Module]] — Force-directed layout.
- [[Metrics Module]] — centrality and connectivity metrics.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
