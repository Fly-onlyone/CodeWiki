---
tags: [core, cw-core]
---

# Language

> Source language of a file.

## What it does

Languages without their own variant are indexed at file level as `Other`; see `Language::has_symbol_support` for the ones whose symbols are extracted.

## Source

- `crates/cw-core/src/model.rs` — definition of `Language`

## How it works

1. `link_imports` and `modules_of` in [[Lang Module]], and 19 more, call `from_path`. `from_path` calls `as_str`.
2. `Index::files` in [[Index]] calls `parse`.

## Items

Declared as `pub enum Language`.

Members:

- `pub fn from_path(path: &str) -> Self` — detect the language from a repo-relative path.
- `pub fn as_str(self) -> &'static str` — public method, returns `&'static str`; called by `Language::from_path`.
- `pub fn parse(s: &str) -> Self` — public method, returns `Self`; called by `Index::files`.
- `pub fn has_symbol_support(self) -> bool` — true when symbols and edges are extracted for this language.

## Used by

- [[RenderCtx]] — from `composition`, `entry_label`, `render_crate`, `render_module`
- [[Explain Module]] — from `facts`, `is_rust`, `items`, `return_type`
- [[Lang Module]] — from `link_imports`, `modules_of`, `resolve_ts_import`
- [[Doctext Module]] — from `blocks`, `doc_body`, `item_text`
- [[Health Module]] — from `dead_code`, `doc_drift`, `file_health`
- [[Render Module]] — from `external_noun`, `language_name`, `unit_noun`
- [[Units Module]] — from `discover`, `of`
- [[Resolver]] — from `by_bare_name`, `resolve`

## See also

- [[_index]]
- [[Model Module]]
