---
tags: [core, cw-index]
---

# Document

> One searchable unit.

## What it does

`kind` is `note`, `symbol` or `file`; `key` is the note path, qualified symbol name or file path.

## Source

- `crates/cw-index/src/search.rs` — definition of `Document`

## Items

Declared as `pub struct Document`.

## Used by

- [[Semantic Module]] — from `embed_documents`, `embed_documents_with`, `embed_within`, `refresh`
- [[cw-index]] — from `documents`, `replace_documents`
- [[Service Module]] — from `index_notes`, `reindex`
- [[Run]] — from `plan`

## See also

- [[_index]]
- [[Search Module]]
