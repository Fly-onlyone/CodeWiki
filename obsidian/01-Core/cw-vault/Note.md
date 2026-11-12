---
tags: [core, cw-vault]
---

# Note

> `Note` struct defined in `cw_vault::note` with 2 methods.

## Source

- `crates/cw-vault/src/note.rs` — definition of `Note`

## How it works

Declared as `pub struct Note`.

Public API:

- `pub fn parse(text: &str) -> Self`
- `pub fn section(&self, heading: &str) -> Option<&Section>`

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
