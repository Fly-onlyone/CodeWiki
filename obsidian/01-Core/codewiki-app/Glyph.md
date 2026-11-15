---
tags: [core, codewiki-app]
---

# Glyph

> An icon of the app: the path its SVG is served under.

## Source

- `crates/codewiki-app/src/icons.rs` — definition of `Glyph`

## How it works

Declared as `pub struct Glyph`.

Public API:

- `fn path(self) -> SharedString`

## Used by

- [[cw-wiki]] — from `a_decision_goes_stale_when_its_code_changes_later`, `a_folder_without_git_is_not_called_stale`, `a_healthy_repository_gets_no_suggestions`, `a_type_that_moves_between_crates_keeps_the_vault_consistent`
- [[cw-server]] — from `analysis_routes`, `full_repository_flow`, `git_webhook_updates_the_matching_repository`, `shutdown_route_ends_serve`
- [[Icons Module]] — from `for_command`, `for_theme`, `hue`, `tint`
- [[codewiki]] — from `briefs_an_agent_in_a_repository_without_a_wiki`, `briefs_from_a_subfolder_of_a_repository_with_a_wiki`, `never_fails_and_prints_nothing_on_an_error`
- [[Wire Tests Module]] — from `a_newer_release_is_found_and_downloaded_through_a_redirect`, `a_package_larger_than_the_limit_is_refused`, `a_package_that_does_not_match_its_checksum_is_deleted`
- [[Doc Module]] — from `Row`, `new`
- [[Button Module]] — from `IconButton`, `new`
- [[List Module]] — from `ListFolder`, `new`

## See also

- [[_index]]
- [[Icons Module]]
