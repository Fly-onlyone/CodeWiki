---
tags: [core, cw-server]
---

# Host

> What the background embedding job asks of the machine.

## What it does

A test replaces it with a stand-in, so that it calls no model server.

## Source

- `crates/cw-server/src/lib.rs` — definition of `Host`

## How it works

1. `run` in [[Serve Module]] calls `default`.

## Items

Declared as `pub struct Host`.

Members:

- `fn default() -> Self` — public method, returns `Self`; called by `run`.

## Depends on

- [[Semantic Module]] — uses `embedder`
- [[Embedder]] — uses `Embedder`

## Used by

- [[cw-server]] — from `ServerConfig`, `config`, `shutdown_route_ends_serve`, `with_config`
- [[Serve Module]] — from `run`

## See also

- [[_index]]
- [[cw-server]]
