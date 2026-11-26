---
tags: [core, cw-core]
---

# Confidence

> How sure the resolver is that an edge is real.

## What it does

A three-level grade on every edge of the code graph. CodeWiki resolves names without a compiler, so a call or reference may point at the wrong target. Each edge records how sure the [[Resolver]] was, and queries and risk scores weight edges by it.

## Source

- `crates/cw-core/src/model.rs` — definition of `Confidence`

## How it works

The levels:

- `High`: resolved through an explicit path or import. Weight 1.0.
- `Medium`: matched by bare name, with one candidate in scope. Weight 0.6.
- `Low`: matched by bare name, with several candidates. Weight 0.25.

`weight` gives that number. The type derives ordering, so `High > Low`. `as_str` and `parse` convert to and from the text stored in the index; [[Store Module]] calls `parse` when it reads an edge row.

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

## Gotchas

- `parse` turns any unknown text into `Low`, the safest reading. A typo in stored data never raises confidence.
- The weights are plain numbers in `weight`; changing them changes every ranking built from edges.

## See also

- [[_index]]
- [[Model Module]]
