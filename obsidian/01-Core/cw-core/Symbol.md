---
tags: [core, cw-core]
---

# Symbol

> A named code entity.

## Source

- `crates/cw-core/src/model.rs` — definition of `Symbol`

## How it works

Declared as `pub struct Symbol`.

`qualified` is unique within a repository, for example `cw_core::hash::sha256_hex` or `cw_core::model::Language::from_path`.

## Depends on

- [[Model Module]] — uses `Visibility`
- [[SymbolKind]] — uses `SymbolKind`

## Used by

- [[Resolver]] — from `Resolver`, `by_bare_name`, `lookup`, `lookup_from`
- [[Repo]] — from `Repo`, `build`, `children_of`, `is_test`
- [[Refactor Module]] — from `Facts`, `extract_types`, `lines_of`, `split_files`
- [[Index]] — from `find_symbols`, `symbol`, `symbols`, `symbols_in_file`
- [[RenderCtx]] — from `illustration`, `render_crate`, `render_pattern`, `render_type`
- [[Health Module]] — from `dead_code`, `file_health`, `is_test_symbol`
- [[Plan Module]] — from `enclosing_module`, `module_title`, `plan`
- [[Query Module]] — from `FileContext`, `SymbolDetail`, `symbol_detail`

## See also

- [[_index]]
- [[Model Module]]
