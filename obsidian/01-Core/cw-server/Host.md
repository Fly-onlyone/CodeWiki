---
tags: [core, cw-server]
---

# Host

> What the work on built-in models asks of the machine.

## Source

- `crates/cw-server/src/lib.rs` — definition of `Host`

## How it works

Declared as `pub struct Host`.

A test replaces each with a stand-in, so that it starts no model, reads no hardware and waits for no game.

Public API:

- `fn default() -> Self`

## Depends on

- [[Game Module]] — uses `busy`
- [[Hardware Module]] — uses `probe`
- [[Semantic Module]] — uses `embedder`
- [[Embedder]] — uses `Embedder`
- [[Hardware]] — uses `Hardware`

## Used by

- [[cw-server]] — from `ServerConfig`, `host`, `new`, `shutdown_route_ends_serve`
- [[Serve Module]] — from `run`

## See also

- [[_index]]
- [[cw-server]]
