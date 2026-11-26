---
tags: [core, cw-index, module]
---

# Search Module

> Full-text search over notes, symbols and files (SQLite FTS5, BM25).

## What it does

This module is keyword search over everything CodeWiki indexes: wiki notes, symbols and files. It stores [[Document]] rows in a SQLite FTS5 table and ranks matches with BM25. The query box, the answer tool and the API use it. Semantic (meaning-based) search lives apart, in [[Vectors Module]]; both return the same `Hit` type, so callers can mix them.

## Source

- `crates/cw-index/src/search.rs` — module `cw_index::search`

## How it works

`search` first runs `fts_query` on the user's text. That function splits the text on anything that is not a letter, digit or underscore. It wraps each word in quotes, adds a star so it matches as a prefix, and joins the words with OR. If no word is left it returns nothing, and `search` returns an empty list at once.

The SQL query then matches the FTS table, optionally filters by `kind`, and sorts by score and then key. The score is the negated BM25 value, so higher is better. The weights give a title match five times the weight of a body match. The snippet comes from the body column, about 24 tokens long.

`replace_documents` and `documents` write and read the rows. See [[Document]].

## Items

- [[Document]] — one searchable unit.
- `pub struct Hit` — public struct; used by `Answer`, `Index::nearest`, `Index::search`, `nearest` and 2 more.
- `fn fts_query(text: &str) -> Option<String>` — turn free text into an FTS5 query: each word becomes a quoted prefix term, joined with OR so partial matches still rank. Quoting keeps user input from being read as FTS5 syntax.

## Used by

- [[Semantic Module]] — from `nearest`, `search`, `search_with`
- [[cw-index]] — from `nearest`, `search`
- [[Api Module]] — from `search`
- [[Query Module]] — from `Answer`

## Gotchas

- Quoting every word is what keeps input like `vault AND (` or a lone quote from being read as FTS5 syntax. Do not pass raw text to MATCH.
- Words are joined with OR, so a long question matches many documents. Ranking, not filtering, decides what comes first.
- Underscores count as word characters, so `sha256_hex` is one term and `sha256` still finds it by prefix.
- A hit from semantic search has an empty snippet. Use `document_opening` to fill it.

## See also

- [[_index]]
- [[cw-index]]
