---
tags: [core, cw-models, module]
---

# Store Module (cw-models)

> The models on disk.

## Source

- `crates/cw-models/src/store.rs` — module `cw_models::store`

## How it works

Main items:

- [[Store]] — struct
- `DEFAULT_BASE` — where the models are fetched from when nothing else is named.
- `BASE_ENV` — environment variable that names another host, for tests and mirrors.
- `Outcome` — how an install ended when it did not fail.
- `PARTIAL` — const
- `io` — function
- `MANIFEST` — const
- `CHUNK` — const

## Depends on

- [[Error]] — uses `Error`, `external`, `invalid`, `io`
- [[Error Module]] — uses `Result`
- [[Logs Module]] — uses `flush`
- [[Supervisor]] — uses `status`

## Used by

- [[Store]] — from `install`, `is_installed`, `new`, `partial`
- [[Model Module (codewiki)]] — from `store`
- [[Models Module]] — from `store`

## See also

- [[_index]]
- [[cw-models]]
