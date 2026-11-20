---
tags: [integration]
---

# serde

> External crate `serde`, imported by 33 source files.

## Used for

Imported by 33 source files. Main users:

- [[Api Module]]
- [[Catalog Module]]
- [[Ci Module]]
- [[Contracts Module]]
- [[Coverage Module]]
- [[Decisions Module]]
- [[Generate Module]]
- [[Gitintel Module]]

## Configuration

- `crates/codewiki-mcp/Cargo.toml` — declares the dependency
- `crates/cw-core/Cargo.toml` — declares the dependency
- `crates/cw-index/Cargo.toml` — declares the dependency
- `crates/cw-models/Cargo.toml` — declares the dependency
- `crates/cw-parse/Cargo.toml` — declares the dependency
- `crates/cw-server/Cargo.toml` — declares the dependency
- `crates/cw-update/Cargo.toml` — declares the dependency
- `crates/cw-vault/Cargo.toml` — declares the dependency
- `crates/cw-wiki/Cargo.toml` — declares the dependency

## Wire-up

- `crates/codewiki-mcp/src/main.rs` — imports `serde`
- `crates/cw-core/src/model.rs` — imports `serde`
- `crates/cw-index/src/search.rs` — imports `serde`
- `crates/cw-index/src/store.rs` — imports `serde`
- `crates/cw-models/src/catalog.rs` — imports `serde`
- `crates/cw-models/src/hardware.rs` — imports `serde`
- `crates/cw-models/src/recommend.rs` — imports `serde`
- `crates/cw-models/src/worker.rs` — imports `serde`

## See also

- [[_index]]
