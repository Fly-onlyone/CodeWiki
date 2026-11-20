---
tags: [core, cw-core]
---

# Key

> One setting: the environment variable that names it everywhere, and where it sits in the file.

## Source

- `crates/cw-core/src/settings.rs` — definition of `Key`

## How it works

Declared as `pub struct Key`.

Public API:

- `pub fn path(&self) -> String` — the setting's name in the file and over HTTP, as `section.field`.
- `pub fn section(&self) -> &'static str`
- `pub fn field(&self) -> &'static str`

## Used by

- [[Settings Module (cw-core)]] — from `ALL`, `ANTHROPIC_API_KEY`, `EMBED_DEVICE`, `EMBED_MODEL`
- [[Settings]] — from `clear`, `field`, `put`, `secret`
- [[Settings Module (cw-server)]] — from `EMBEDDER_KEYS`, `apply`, `checked`, `describe`
- [[Provider Module]] — from `from_config`, `missing`, `setting`
- [[Resolved]] — from `saved_secret`, `secret_state`, `source`

## See also

- [[_index]]
- [[Settings Module (cw-core)]]
