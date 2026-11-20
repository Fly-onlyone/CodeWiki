---
tags: [core, cw-update, module]
---

# Download Module

> Downloading a package and checking it against its `.sha256` file.

## Source

- `crates/cw-update/src/download.rs` — module `cw_update::download`

## How it works

1. `install` in [[cw-update]] calls `download`. `download` calls `download_limited`.
2. `a_package_larger_than_the_limit_is_refused` in [[Wire Tests Module]] calls `download_limited`. `download_limited` calls `get`, then `parse_sha256`, then `fetch`. `get` calls `agent`, then `user_agent` (both in [[Release Module]]). `fetch` calls `get`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const MAX_PACKAGE: u64` — largest package accepted. A release is about 490 MB, nearly all of it the NVIDIA libraries beside the worker for the graphics card (`cuda\`); anything far larger is not one.
- `pub fn download(release: &Release, dir: &Path) -> Result<PathBuf, String>` — download the package of `release` into `dir` and verify it. Returns the path of the verified zip. A package that does not match its checksum is deleted.
- `download_limited` — internal function, returns `Result<PathBuf, String>`; called by `a_package_larger_than_the_limit_is_refused`, `download`.
- `fn get(url: &str) -> Result<ureq::http::Response<ureq::Body>, String>` — private function, returns `Result<ureq::http::Response<ureq::Body>, String>`; called by `download_limited`, `fetch`.
- `fn fetch(url: &str, to: &Path, limit: u64) -> Result<String, String>` — stream `url` into `to`, at most `limit` bytes. Returns the SHA-256 of what was written, in lowercase hex.
- `pub(crate) fn parse_sha256(text: &str) -> Option<String>` — the hash in a `.sha256` file: `<64 hex> <file name>`, as written by the release workflow and by `sha256sum`.

## Depends on

- [[Release Module]] — uses `agent`, `user_agent`
- [[Release]] — uses `Release`

## Used by

- [[cw-update]] — from `install`
- [[Wire Tests Module]] — from `a_package_larger_than_the_limit_is_refused`

## See also

- [[_index]]
- [[cw-update]]
