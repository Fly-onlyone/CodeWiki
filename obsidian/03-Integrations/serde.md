---
tags: [integration]
---

# serde

> External crate `serde`, imported by 26 source files.

## Used for

Imported by 26 source files. Main users:

- [[Api Module]]
- [[Ci Module]]
- [[Contracts Module]]
- [[Coverage Module]]
- [[Decisions Module]]
- [[Generate Module]]
- [[Gitintel Module]]
- [[Health Module]]

## Configuration

- `crates/codewiki-mcp/Cargo.toml` — declares the dependency
- `crates/cw-core/Cargo.toml` — declares the dependency
- `crates/cw-index/Cargo.toml` — declares the dependency
- `crates/cw-parse/Cargo.toml` — declares the dependency
- `crates/cw-server/Cargo.toml` — declares the dependency
- `crates/cw-vault/Cargo.toml` — declares the dependency
- `crates/cw-wiki/Cargo.toml` — declares the dependency

## Wire-up

- `crates/codewiki-mcp/src/main.rs` — imports `serde`
- `crates/cw-core/src/model.rs` — imports `serde`
- `crates/cw-index/src/search.rs` — imports `serde`
- `crates/cw-index/src/store.rs` — imports `serde`
- `crates/cw-parse/src/workspace.rs` — imports `serde`
- `crates/cw-server/src/api.rs` — imports `serde`
- `crates/cw-server/src/repos.rs` — imports `serde`
- `crates/cw-server/src/workspace.rs` — imports `serde`

## See also

- [[_index]]
