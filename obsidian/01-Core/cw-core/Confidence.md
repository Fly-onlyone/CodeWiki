---
tags: [core, cw-core]
---

# Confidence

> How sure the resolver is that an edge is real.

## What it does

Name resolution runs without a compiler, so every edge carries its confidence and consumers weight by it.

## Source

- `crates/cw-core/src/model.rs` — definition of `Confidence`

## How it works

1. `edge_from_row` in [[Store Module]] calls `parse`.

## Items

Declared as `pub enum Confidence`.

Members:

- `pub fn weight(self) -> f64` — public method, returns `f64`.
- `pub fn as_str(self) -> &'static str` — public method, returns `&'static str`.
- `pub fn parse(s: &str) -> Self` — public method, returns `Self`; called by `edge_from_row`.

## Used by

- [[Resolver]] — from `by_bare_name`, `resolve`
- [[Model Module]] — from `Edge`
- [[Store Module]] — from `edge_from_row`

## See also

- [[_index]]
- [[Model Module]]
