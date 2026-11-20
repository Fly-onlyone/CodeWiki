---
tags: [core, cw-update, module]
---

# Release Module

> Asking GitHub for the latest release.

## Source

- `crates/cw-update/src/release.rs` — module `cw_update::release`

## How it works

Main items:

- [[Release]] — A release newer than the running program.
- `check` — ask `api_base` (GitHub's API, or a stand-in) for the latest release of `REPOSITORY` and compare it with `current`.
- `Check` — the answer to a check.
- `agent` — `https_only` refuses any request that is not https, a redirect included.
- `user_agent` — function
- `ApiAsset` — struct
- `from_repository` — A release that GitHub described must have its files among this repository's release downloads at GitHub.
- `parse` — read GitHub's answer to `GET /repos/{owner}/{repo}/releases/latest`.

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
