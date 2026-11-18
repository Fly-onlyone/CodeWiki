---
tags: [core, cw-core, module]
---

# Platform Module

> Module `cw_core::secret::platform` defining 7 items, including `unprotect`, `protect`, and `blob`.

## Source

- `crates/cw-core/src/secret.rs` — module `cw_core::secret::platform`

## How it works

Main items:

- `unprotect` — function
- `protect` — function
- `blob` — function
- `failed` — function
- `take` — copy what the system returned, then give its memory back.
- `unavailable` — function
- `ENTROPY` — mixed into the encryption, so that another program's call with the same user does not decrypt a CodeWiki secret by accident.

## Depends on

- [[Error]] — uses `Error`, `external`, `invalid`
- [[Error Module]] — uses `Result`

## Used by

- [[Secret Module]] — from `protect`, `unprotect`

## See also

- [[_index]]
- [[cw-core]]
