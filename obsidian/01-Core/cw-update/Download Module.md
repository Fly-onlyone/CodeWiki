---
tags: [core, cw-update, module]
---

# Download Module

> Downloading a package and checking it against its `.sha256` file.

## What it does

Fetches the update package of a [[Release]] from GitHub and proves it is the package the release workflow published. It checks the zip against the `.sha256` file that sits beside it. Only a verified zip reaches the next step, unpacking in [[Stage Module]].

## Source

- `crates/cw-update/src/download.rs` — module `cw_update::download`

## How it works

`download` calls `download_limited` with the size limit `MAX_PACKAGE` (1 GiB). `download_limited` works in this order:

1. Create the target folder.
2. Fetch the `.sha256` file with `get` and read the hash with `parse_sha256`. No hash means an error.
3. Stream the zip with `fetch` into a `.part` file. `fetch` reads 64 KB at a time, feeds a SHA-256 hasher and writes to disk. It stops with an error if the total passes the limit.
4. Compare the hash of what was written with the expected one.
5. On a match, rename the `.part` file to the real zip name and return its path. On any error, delete the `.part` file.

`get` builds the request through `agent` and `user_agent` from [[Release Module]]. It turns https-only on for every https address, so a redirect cannot lead to plain http. `limit` is a parameter of `download_limited` so a test can use a tiny limit.

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

## Gotchas

- The final zip name appears only after the hash matched. A zip with that name in the folder is therefore always verified.
- The size check runs while streaming, so an oversized download stops early. The server's length header is not trusted.
- `parse_sha256` takes only the first word of the file and accepts upper case, because PowerShell `Get-FileHash` prints upper case. Anything that is not 64 hex digits gives `None`, for example a "Not Found" page.
- The hash file and the zip are fetched in two requests from the same release. The check proves the two agree, not who made them. Origin is checked earlier, in [[Release Module]].

## See also

- [[_index]]
- [[cw-update]]
