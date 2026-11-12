---
tags: [core, cw-index]
---

# Document

> One searchable unit.

## Source

- `crates/cw-index/src/search.rs` — definition of `Document`

## How it works

Declared as `pub struct Document`.

`kind` is `note`, `symbol` or `file`; `key` is the note path, qualified symbol name or file path.

## Used by

- [[Semantic Module]] — from `embed_documents`, `refresh`, `text_of`
- [[Service Module]] — from `index_notes`, `reindex`
- [[cw-index]] — from `replace_documents`

## See also

- [[_index]]
- [[Search Module]]
