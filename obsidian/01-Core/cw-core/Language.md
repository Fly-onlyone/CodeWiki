---
tags: [core, cw-core]
---

# Language

> Source language of a file.

## What it does

Names the source language of an indexed file. It decides which parser runs, how a file is described in notes, and which health checks apply. Rust, Python, TypeScript, JavaScript and Java get symbols and edges. Markdown, TOML, JSON and YAML are recorded as files only. Everything else is `Other`.

## Source

- `crates/cw-core/src/model.rs` — definition of `Language`

## How it works

`from_path` looks only at the extension, lower-cased: for example `rs`, `py` and `pyi`, `ts` `tsx` `mts` `cts`, `js` `jsx` `mjs` `cjs`, `java`, `md`, `toml`, `json`, `yml` and `yaml`. No extension, or an unknown one, gives `Other`.

`as_str` and `parse` convert to and from the lowercase text stored in the index, where [[Index]] calls `parse`. `has_symbol_support` is true for the five code languages; callers use it to decide whether symbol-level work makes sense.

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

## Gotchas

- Detection ignores content and file names. A `Makefile` or a shebang script is `Other`.
- `parse` maps any unknown text to `Other` instead of failing. A language added to the enum but not to `parse` silently reads back as `Other`.
- Adding a variant means updating `from_path`, `as_str`, `parse` and `has_symbol_support` together.

## See also

- [[_index]]
- [[Model Module]]
