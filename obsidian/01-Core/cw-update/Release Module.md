---
tags: [core, cw-update, module]
---

# Release Module

> Asking GitHub for the latest release.

## What it does

Asks GitHub for the latest release of the CodeWiki repository and says whether it is newer than the running program. The answer is a `Check`: a newer [[Release]], up to date, or unavailable with a reason. It also holds the shared HTTP agent and User-Agent string that [[Download Module]] reuses.

## Source

- `crates/cw-update/src/release.rs` — module `cw_update::release`

## How it works

`check` parses the current version, then sends `GET /repos/<repository>/releases/latest` to `api_base`. The base is GitHub's API in production and a local stand-in in tests.

- 404 means the repository has no release yet: `UpToDate`.
- 403 or 429 means a rate limit: `Unavailable`.
- Any other failure, such as no network, is `Unavailable` with the error text.

On success `from_repository` wraps `parse`. `parse` reads the JSON. It returns `UpToDate` for a draft, a pre-release, a pre-release tag, or a version not above the current one. It strips a leading `v` from the tag. It then looks for two assets named `CodeWiki-<version>-windows-x64.zip` and the same name plus `.sha256`. Both must exist, or the result is `Unavailable`.

`from_repository` then checks the file addresses, see Gotchas.

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

## Gotchas

- With real GitHub as `api_base`, both file addresses must start with this repository's release download address (compared in lower case). Otherwise the release is `Unavailable`. The reason: the package is run before it is installed, and its checksum comes from the same answer, so the address is what proves whose package it is. A stand-in on another `api_base` is exempt.
- `agent` turns `https_only` on for every https address, so a redirect to plain http fails.
- The agent sets no total time limit, since a download may take minutes. It limits only connect (15 seconds) and each read (30 seconds).
- A pre-release is never offered, even if the answer marks it as a full release. The check looks at the tag too.
- An unreadable current version gives `Unavailable`, not a panic.

## See also

- [[_index]]
- [[cw-update]]
