---
tags: [operations]
---

# Build and Test

> Commands and automation that build, check and test the repository.

## Source

- `.github/workflows/ci.yml` — CI workflow
- `Cargo.toml` — workspace manifest
- `Dockerfile` — container image
- `deny.toml` — license and advisory policy
- `rust-toolchain.toml` — pinned toolchain

## How it works

The repository is a Cargo workspace of 16 packages. Run every command from the repository root.

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
```

Continuous integration runs the same checks from `.github/workflows/ci.yml`.

## Depends on

- [[codewiki-app]] — builds a binary
- [[codewiki-mcp]] — builds a binary
- [[codewiki]] — builds a binary

## See also

- [[_index]]
- [[Crate Dependency Flow]]
