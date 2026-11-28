---
tags: [operations]
---

# Build and Test

> Commands and automation that build, check and test the repository.

## Source

- `.github/workflows/action-test.yml` — CI workflow
- `.github/workflows/ci.yml` — CI workflow
- `.github/workflows/docker.yml` — CI workflow
- `.github/workflows/release.yml` — CI workflow
- `.github/workflows/security.yml` — CI workflow
- `Cargo.toml` — workspace manifest
- `Dockerfile` — container image
- `deny.toml` — license and advisory policy
- `rust-toolchain.toml` — pinned toolchain

## How it works

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
```

Four GitHub workflows run the checks.

- CI runs on pushes to main and on pull requests. The `check` job runs on Windows. It runs `cargo fmt --all -- --check`, then clippy with warnings as errors, then the tests, all with `--locked`. The `deny` job runs on Linux in two parts, `advisories` and `bans sources`. The `wiki-check` job runs CodeWiki's own `check` on this repository and uploads the result as SARIF to code scanning. It never fails the build. It also proves that the backend still compiles on Linux.
- Security runs gitleaks, which scans for committed secrets. It runs on pushes and pull requests, and weekly on Mondays with the RustSec advisory check.
- Docker builds the backend image from the Dockerfile. A version tag pushes it to the GitHub Container Registry. A pull request that touches the image inputs builds it without pushing.
- Release runs on a version tag. It builds the Windows package with the packaging script, writes checksums and release notes, and publishes the release. It also uploads debug files to Sentry when a token is set. A pull request that changes packaging does the same as a dry run and publishes nothing.

The toolchain is `stable` with rustfmt and clippy. The workspace needs Rust 1.95 and edition 2024. A release build uses thin LTO and one codegen unit.

The workspace manifest also patches one dependency: `gpui-base` comes from the `vendor` folder, not from crates.io. It is the published 0.9.1 copy with a link hover look and a text view that reports its headings and where each block starts, which the note outline needs. The `vendor` folder is outside the workspace members.

## Depends on

- [[codewiki-app]] — builds a binary
- [[codewiki-mcp]] — builds a binary
- [[codewiki]] — builds a binary

## Gotchas

- A bare `cargo deny check` also runs the license check. `deny.toml` lists no allowed licenses, so it refuses every crate. Run `cargo deny check advisories bans sources`.
- A new RustSec advisory can fail CI with no change in the repository. Fix the dependency, or add the ID to `ignore` in `deny.toml` with a reason.
- A release tag must match the workspace version in `Cargo.toml`, or the Release job stops. A tag with a pre-release part, such as `-rc.1`, is published as a pre-release.
- The Release job fails if the setup program was not built. It needs the Inno Setup compiler.
- Dev builds keep little debug information for dependencies and no incremental cache, to keep the build folder small.
- The vendored `gpui-base` is a patch of one release. When `gpui-kit` is bumped, drop the patch or copy the new release and patch it again; otherwise the outline's heading calls stop compiling.
- The Dockerfile copies `vendor`, because Cargo reads every `[patch]` of the workspace. The server in the image refuses to start on a public address without `CODEWIKI_TOKEN`.
- Runs on a pull request cancel older runs of the same branch. Runs on main are never cancelled.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
