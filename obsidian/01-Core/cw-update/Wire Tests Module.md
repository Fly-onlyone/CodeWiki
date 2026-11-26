---
tags: [core, cw-update, module]
---

# Wire Tests Module

> Drives the check and the download against a stand-in for GitHub, so no network is needed.

## What it does

Tests for the check and the download that run through real HTTP, but against a small server in the test process instead of GitHub. No network is needed. The module exists only in test builds.

## Source

- `crates/cw-update/src/wire_tests.rs` — module `cw_update::wire_tests`

## How it works

`serve` binds a local port, spawns a thread, and answers each request by looking up its path in a shared route table. A route is either a status with a body or a 302 redirect. Unknown paths get 404. Routes can be added after the server starts, because the test needs the address first.

`github` builds a stand-in that offers version 0.2.0. Its `releases/latest` answer names a zip and a `.sha256` file under the stand-in's address. The zip sits behind a redirect, as GitHub sends assets to another host. `release_at` runs `check` against it.

The tests cover:

- a newer release found and downloaded through the redirect, leaving no `.part` file;
- a wrong checksum, which deletes the download;
- a package over a tiny limit (through `download_limited`), which is refused;
- no release yet (404), a rate limit (403) and no listener at all, which are up to date or unavailable.

## Items

- `enum Reply` — private enum; used by `Routes`.
- `type Routes = Arc<Mutex<HashMap<String, Reply>>>` — private type alias; used by `serve`.
- `fn serve() -> (String, Routes)` — answer requests by path until the test ends. Routes can be added after the server is bound, once its address is known.
- `const LATEST: &str` — private constant; used by `a_rate_limit_is_not_an_answer`, `github`.
- `const ZIP: &str` — private constant; used by `a_newer_release_is_found_and_downloaded_through_a_redirect`, `github`.
- `fn sha256_hex(bytes: &[u8]) -> String` — private function, returns `String`; called by `a_newer_release_is_found_and_downloaded_through_a_redirect`, `a_package_larger_than_the_limit_is_refused`, `a_package_that_does_not_match_its_checksum_is_deleted`.
- `fn github(package: &[u8], sha256: &str) -> String` — A stand-in that offers 0.2.0: `package`, behind a redirect, and a `.sha256` file naming `sha256`.
- `fn release_at(url: &str) -> Release` — private function, returns `Release`; called by `a_newer_release_is_found_and_downloaded_through_a_redirect`, `a_package_larger_than_the_limit_is_refused`, `a_package_that_does_not_match_its_checksum_is_deleted`.

And 6 more private items with no comment.

## Depends on

- [[Download Module]] — uses `download_limited`
- [[Release Module]] — uses `check`
- [[Release]] — uses `Release`

## Gotchas

- The stand-in uses http, so `agent` turns `https_only` off for it. The redirect, https-only and repository-address checks are not exercised here. `from_repository` skips any `api_base` other than GitHub's, so the stand-in may serve files from anywhere.
- The server thread never stops. It lives until the test process ends.
- Each `github` call starts its own server and port, so tests do not share routes.
- The repository name in the `LATEST` path is hard-coded. If `REPOSITORY` changes, these tests fail with a 404 answer, which reads as up to date.

## See also

- [[_index]]
- [[cw-update]]
