---
tags: [core, cw-core]
---

# Confidence

> How sure the resolver is that an edge is real.

## Source

- `crates/cw-core/src/model.rs` — definition of `Confidence`

## How it works

Declared as `pub enum Confidence`.

Name resolution runs without a compiler, so every edge carries its confidence and consumers weight by it.

Public API:

- `pub fn parse(s: &str) -> Self`
- `pub fn weight(self) -> f64`
- `pub fn as_str(self) -> &'static str`

## Used by

- [[Resolver]] — from `by_bare_name`, `resolve`
- [[Model Module]] — from `Edge`
- [[Store Module]] — from `edge_from_row`
- [[Graph]] — from `from_edges`

## See also

- [[_index]]
- [[Model Module]]
