---
tags: [core, cw-update, module]
---

# Wire Tests Module

> Drives the check and the download against a stand-in for GitHub, so no network is needed.

## Source

- `crates/cw-update/src/wire_tests.rs` — module `cw_update::wire_tests`

## How it works

Main items:

- `Reply` — enum
- `Routes` — type alias
- `serve` — answer requests by path until the test ends.
- `github` — A stand-in that offers 0.2.0: `package`, behind a redirect, and a `.sha256` file naming `sha256`.
- `release_at` — function
- `sha256_hex` — function
- `LATEST` — const
- `ZIP` — const

## Depends on

- [[Logs Module]] — uses `flush`
- [[Download Module]] — uses `download_limited`
- [[Release Module]] — uses `check`
- [[Release]] — uses `Release`

## See also

- [[_index]]
- [[cw-update]]
