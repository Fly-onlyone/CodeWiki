---
tags: [core, cw-update, module]
---

# Wire Tests Module

> Drives the check and the download against a stand-in for GitHub, so no network is needed.

## Source

- `crates/cw-update/src/wire_tests.rs` — module `cw_update::wire_tests`

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

## See also

- [[_index]]
- [[cw-update]]
