---
tags: [core, cw-core]
---

# Symbol

> A named code entity.

## What it does

`qualified` is unique within a repository, for example `cw_core::hash::sha256_hex` or `cw_core::model::Language::from_path`.

## Source

- `crates/cw-core/src/model.rs` — definition of `Symbol`

## Items

Declared as `pub struct Symbol`.

## Depends on

- [[SymbolKind]] — uses `SymbolKind`
- [[Visibility]] — uses `Visibility`

## Used by

- [[Explain Module]] — from `Callers`, `Step`, `Walk`, `callers_sentence`
- [[Resolver]] — from `Resolver`, `by_bare_name`, `lookup`, `lookup_from`
- [[Repo]] — from `Repo`, `build`, `children_of`, `is_test`
- [[Refactor Module]] — from `Facts`, `extract_types`, `lines_of`, `split_files`
- [[Index]] — from `find_symbols`, `symbol`, `symbols`, `symbols_in_file`
- [[Health Module]] — from `dead_code`, `file_health`, `is_test_symbol`
- [[Plan Module]] — from `enclosing_module`, `module_title`, `plan`
- [[Query Module]] — from `FileContext`, `SymbolDetail`, `symbol_detail`

## See also

- [[_index]]
- [[Model Module]]
