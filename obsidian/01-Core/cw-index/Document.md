---
tags: [core, cw-index]
---

# Document

> One searchable unit.

## What it does

A `Document` is one piece of text that full-text search can find. It has a `kind`, a `key`, a `title` and a `body`. The `kind` is `note`, `symbol` or `file`. The `key` says where the thing lives: a note path, a qualified symbol name or a file path. Callers build documents from the wiki and the code, then hand them to the index. The same documents are read back when text is embedded for semantic search.

## Source

- `crates/cw-index/src/search.rs` — definition of `Document`

## How it works

`replace_documents` writes documents into the SQLite table `documents`. That table is an FTS5 table (SQLite full-text search) where only `title` and `body` are indexed; `kind` and `key` are stored but not searched.

`search` turns a query into hits. A `Hit` carries the same `kind`, `key` and `title`, plus a snippet cut from the body and a score. Title matches count five times as much as body matches.

`documents` reads stored documents back by kind, in the order they were written. Embedding them again needs no scan of the repository.

## Items

Declared as `pub struct Document`.

## Used by

- [[Semantic Module]] — from `embed_documents`, `embed_documents_with`, `embed_within`, `refresh`
- [[cw-index]] — from `documents`, `replace_documents`
- [[Service Module]] — from `index_notes`, `reindex`
- [[Run]] — from `plan`

## Gotchas

- `replace_documents` deletes every old document of the kinds you pass before it inserts. Kinds you do not pass stay.
- A document of a kind you did not list in `replace_documents` is still inserted. Pass every kind you write, or old rows of that kind pile up.
- `kind` and `key` are not searchable text. Put words you want found into `title` or `body`.

## See also

- [[_index]]
- [[Search Module]]
