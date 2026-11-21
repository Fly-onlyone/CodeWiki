---
tags: [core, cw-core]
---

# Key

> One setting: the environment variable that names it everywhere, and where it sits in the file.

## Source

- `crates/cw-core/src/settings.rs` — definition of `Key`

## Items

Declared as `pub struct Key`.

Members:

- `pub fn path(&self) -> String` — the setting's name in the file and over HTTP, as `section.field`.
- `pub fn section(&self) -> &'static str` — public method, returns `&'static str`.
- `pub fn field(&self) -> &'static str` — public method, returns `&'static str`.

## Used by

- [[Settings Module (cw-core)]] — from `ALL`, `EMBED_MODEL`, `EMBED_PROVIDER`, `LEMONADE_KEY`
- [[Settings]] — from `clear`, `field`, `put`, `secret`
- [[Settings Module (cw-server)]] — from `ADDRESS_KEYS`, `EMBEDDER_KEYS`, `apply`, `checked`
- [[Resolved]] — from `saved_secret`, `secret_state`, `source`

## See also

- [[_index]]
- [[Settings Module (cw-core)]]
