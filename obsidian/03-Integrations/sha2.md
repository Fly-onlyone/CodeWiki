---
tags: [integration]
---

# sha2

> External crate `sha2`, imported by 4 source files.

## Used for

Imported by 4 source files. Main users:

- [[Download Module]]
- [[Hash Module]]
- [[Store Module (cw-models)]]
- [[Wire Tests Module]]

## Configuration

- `crates/cw-core/Cargo.toml` — declares the dependency
- `crates/cw-models/Cargo.toml` — declares the dependency
- `crates/cw-update/Cargo.toml` — declares the dependency

## Wire-up

- `crates/cw-core/src/hash.rs` — imports `sha2`
- `crates/cw-models/src/store.rs` — imports `sha2`
- `crates/cw-update/src/download.rs` — imports `sha2`
- `crates/cw-update/src/wire_tests.rs` — imports `sha2`

## See also

- [[_index]]
