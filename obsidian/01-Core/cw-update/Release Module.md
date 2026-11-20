---
tags: [core, cw-update, module]
---

# Release Module

> Asking GitHub for the latest release.

## Source

- `crates/cw-update/src/release.rs` — module `cw_update::release`

## How it works

1. `check` in [[Update Module]] and `a_newer_release_is_found_and_downloaded_through_a_redirect` in [[Wire Tests Module]], and 4 more, call `check`. `check` calls `agent`, then `user_agent`, then `from_repository`, then `parse`.
2. `get` in [[Download Module]] calls `agent`.
3. `get` in [[Download Module]] calls `user_agent`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- [[Release]] — A release newer than the running program.
- `pub enum Check` — the answer to a check.
- `pub(crate) fn agent(https_only: bool) -> ureq::Agent` — `https_only` refuses any request that is not https, a redirect included. It is on for every https address, so that an answer from GitHub cannot send a download on to plain http; a stand-in on http has it off.
- `pub(crate) fn user_agent() -> String` — internal function, returns `String`; called by `check`, `get`.
- `pub fn check(api_base: &str, current: &str) -> Check` — ask `api_base` (GitHub's API, or a stand-in) for the latest release of `REPOSITORY` and compare it with `current`.
- `fn from_repository(api_base: &str, check: Check) -> Check` — A release that GitHub described must have its files among this repository's release downloads at GitHub. The package is run before it is installed (`codewiki.exe --version`), and its checksum comes from the same answer, so the address is what says whose package it is. A stand-in (another `api_base`) may serve its files from anywhere.
- `pub(crate) fn parse(text: &str, current: &Version) -> Check` — read GitHub's answer to `GET /repos/{owner}/{repo}/releases/latest`.

And 2 more private items with no comment.

## Depends on

- [[cw-update]] — uses `GITHUB_API`, `REPOSITORY`
- [[Release]] — uses `Release`

## Used by

- [[Wire Tests Module]] — from `a_newer_release_is_found_and_downloaded_through_a_redirect`, `a_rate_limit_is_not_an_answer`, `no_network_is_not_an_answer`, `no_release_yet_is_up_to_date`
- [[Update Module]] — from `check`, `from_check`
- [[codewiki-app]] — from `show_check`
- [[Download Module]] — from `get`

## See also

- [[_index]]
- [[cw-update]]
