---
tags: [integration]
---

# semver

> External crate `semver`, imported by 3 source files.

## Used for

Imported by 3 source files. Main users:

- [[Release Module]]
- [[Stage Module]]
- [[cw-update]]

## Configuration

- `crates/cw-client/Cargo.toml` — declares the dependency
- `crates/cw-update/Cargo.toml` — declares the dependency

## Wire-up

- `crates/cw-update/src/lib.rs` — imports `semver`
- `crates/cw-update/src/release.rs` — imports `semver`
- `crates/cw-update/src/stage.rs` — imports `semver`

## See also

- [[_index]]
