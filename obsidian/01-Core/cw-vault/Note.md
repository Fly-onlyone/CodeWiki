---
tags: [core, cw-vault]
---

# Note

> `Note` struct defined in `cw_vault::note` with 2 methods.

## Source

- `crates/cw-vault/src/note.rs` — definition of `Note`

## How it works

1. `link_resolution_rate` and `lint` in [[Lint Module]], and 8 more, call `parse`. `parse` calls `parse_tags`, then `push_line`, then `extract_links`, then `count_words` (all in [[Note Module]]).

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct Note`.

Members:

- `pub fn parse(text: &str) -> Self` — public method, returns `Self`; called by `adopt_existing`, `answer`, `build`, `doc_drift` and 6 more.
- `pub fn section(&self, heading: &str) -> Option<&Section>` — public method, returns `Option<&Section>`.

## Depends on

- [[Note Module]] — uses `Section`, `WikiLink`, `count_words`, `extract_links`

## Used by

- [[Decisions Module]] — from `build`, `list`, `status_of`, `why`
- [[Generate Module]] — from `adopt_existing`, `adopted_kind`, `cited_sources`
- [[Lint Module]] — from `link_resolution_rate`, `lint`
- [[Agentdocs Module]] — from `render`
- [[Health Module]] — from `doc_drift`
- [[Query Module]] — from `answer`
- [[Service Module]] — from `index_notes`

## See also

- [[_index]]
- [[Note Module]]
