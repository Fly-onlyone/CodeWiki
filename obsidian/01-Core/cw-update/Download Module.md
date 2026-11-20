---
tags: [core, cw-update, module]
---

# Download Module

> Downloading a package and checking it against its `.sha256` file.

## Source

- `crates/cw-update/src/download.rs` — module `cw_update::download`

## How it works

Main items:

- `download` — download the package of `release` into `dir` and verify it.
- `get` — function
- `download_limited` — function
- `fetch` — stream `url` into `to`, at most `limit` bytes.
- `parse_sha256` — the hash in a `.sha256` file: `<64 hex> <file name>`, as written by the release workflow and by `sha256sum`.
- `MAX_PACKAGE` — largest package accepted.

## Depends on

- [[Release Module]] — uses `agent`, `user_agent`
- [[Release]] — uses `Release`

## Used by

- [[cw-update]] — from `install`
- [[Wire Tests Module]] — from `a_package_larger_than_the_limit_is_refused`

## See also

- [[_index]]
- [[cw-update]]
