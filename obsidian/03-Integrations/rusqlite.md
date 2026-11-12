---
tags: [integration]
---

# rusqlite

> External crate `rusqlite`, imported by 3 source files.

## Used for

Imported by 3 source files. Main users:

- [[Search Module]]
- [[Store Module]]
- [[Vectors Module]]

## Configuration

- `crates/cw-index/Cargo.toml` — declares the dependency

## Wire-up

- `crates/cw-index/src/search.rs` — imports `rusqlite`
- `crates/cw-index/src/store.rs` — imports `rusqlite`
- `crates/cw-index/src/vectors.rs` — imports `rusqlite`

## See also

- [[_index]]
