---
tags: [core, cw-core]
---

# Language

> Source language of a file.

## Source

- `crates/cw-core/src/model.rs` — definition of `Language`

## How it works

Declared as `pub enum Language`.

Languages without their own variant are indexed at file level as `Other`; see `Language::has_symbol_support` for the ones whose symbols are extracted.

Public API:

- `pub fn from_path(path: &str) -> Self` — detect the language from a repo-relative path.
- `pub fn has_symbol_support(self) -> bool` — true when symbols and edges are extracted for this language.
- `pub fn as_str(self) -> &'static str`
- `pub fn parse(s: &str) -> Self`

## Used by

- [[RenderCtx]] — from `composition`, `entry_label`, `render_pattern`, `render_startup_flow`
- [[Lang Module]] — from `link_imports`, `modules_of`, `resolve_ts_import`
- [[Health Module]] — from `dead_code`, `doc_drift`, `file_health`
- [[Render Module]] — from `external_noun`, `language_name`, `unit_noun`
- [[Units Module]] — from `discover`, `of`
- [[Resolver]] — from `by_bare_name`, `resolve`
- [[Model Module]] — from `FileRecord`
- [[Scan Module]] — from `scan_repo`

## See also

- [[_index]]
- [[Model Module]]
