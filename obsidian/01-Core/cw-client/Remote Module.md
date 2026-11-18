---
tags: [core, cw-client, module]
---

# Remote Module

> A backend on another machine, or in Docker, that the user chose in the app instead of naming it with `CODEWIKI_URL`.

## Source

- `crates/cw-client/src/remote.rs` — module `cw_client::remote`

## How it works

Main items:

- `Remote` — the saved backend, without its token.
- `FILE` — the file's name in the data directory.
- `file` — function
- `read` — function
- `saved_in` — the saved backend, when there is one.
- `write` — the new file replaces the old one in one step, so a reader never sees half of it.
- `normalized` — an address as it is saved and called: with its scheme, without a slash at the end.
- `save_in` — save a backend and make clients use it.

## Depends on

- [[Secret Module]] — uses `hint`, `protect`, `unprotect`

## Used by

- [[cw-client]] — from `check_remote_in`, `forget_remote`, `save_remote`, `saved_remote`
- [[Settings Pages Module]] — from `Model`
- [[Backend]] — from `named`

## See also

- [[_index]]
- [[cw-client]]
