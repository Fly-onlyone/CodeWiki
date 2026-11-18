---
tags: [core, cw-core, module]
---

# Secret Module

> Secrets kept in a settings file: API keys and the token of a remote backend.

## Source

- `crates/cw-core/src/secret.rs` — module `cw_core::secret`

## How it works

Main items:

- `unprotect` — the secret behind text that `protect` returned, when the same user on the same machine asks.
- `protect` — encrypt `plain` for the current user and return the text to store.
- `available` — whether this system can save a secret.
- `hint` — the end of a secret, shown so that its owner can tell which one is saved.
- `HINT_CHARS` — characters of a secret's end that the hint shows.
- `HINT_FLOOR` — A secret shorter than this shows no hint: its last characters would be too large a part of it.

Submodules: [[Platform Module]].

## Depends on

- [[Platform Module]] — uses `protect`, `unprotect`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `invalid`

## Used by

- [[Remote Module]] — from `save_in`, `token_in`
- [[Settings Module (cw-server)]] — from `apply`, `describe`
- [[Resolved]] — from `saved_secret`
- [[Settings]] — from `set`

## See also

- [[_index]]
- [[cw-core]]
