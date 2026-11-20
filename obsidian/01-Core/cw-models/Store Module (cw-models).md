---
tags: [core, cw-models, module]
---

# Store Module (cw-models)

> The models on disk.

## What it does

An installed model is the folder `<data dir>\models\<id>` with the files the catalog names and a `manifest.json`. A download is made in `<data dir>\models\.partial\<id>` and moved into place whole, after every file matched its SHA-256, so a folder under `models` is never half a model. A download that broke off, or was cancelled, goes on from the bytes it has.

## Source

- `crates/cw-models/src/store.rs` — module `cw_models::store`

## How it works

1. `Store::install` in [[Store]] calls `io`. `io` calls `Error::io` in [[Error]].
2. `Store::install` in [[Store]] calls `fetch`. `fetch` calls `io`, then `sha256_of`, then `agent`, then `Error::external`, then `Error::invalid` (both in [[Error]]). `sha256_of` calls `io`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const DEFAULT_BASE: &str` — where the models are fetched from when nothing else is named.
- `pub const BASE_ENV: &str` — environment variable that names another host, for tests and mirrors.
- `pub enum Outcome` — how an install ended when it did not fail.
- [[Store]] — struct
- `fn io(path: &Path) -> impl Fn(std::io::Error) -> Error + '_` — private function, returns `impl Fn(std::io::Error) -> Error + '_`; called by `Store::install`, `fetch`, `sha256_of`.
- `fn agent(https_only: bool) -> ureq::Agent` — `https_only` refuses any request that is not https, a redirect included, so that an answer cannot send a download on to plain http. A stand-in on http has it off.
- `fn sha256_of(path: &Path, hasher: &mut Sha256, cancel: &AtomicBool) -> Result<bool>` — private function, returns `Result<bool>`; called by `fetch`.
- `fetch` — bring `part` to the `size` bytes of `url`, going on from the bytes it holds. Returns the SHA-256 of the whole file, or `None` when cancelled. `moved` hears each change of the byte count: positive as bytes arrive, negative when a server that cannot resume makes the file start over.

And 3 more private items with no comment.

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
