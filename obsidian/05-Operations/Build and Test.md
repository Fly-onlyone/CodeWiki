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

Five GitHub workflows run the checks.

- CI runs on pushes to main and on pull requests. The `check` job runs on Windows: `cargo fmt --all -- --check`, then clippy with warnings as errors, then the tests, the last two with `--locked`. The `deny` job runs on Linux in two parts, `advisories` and `bans sources`, and neither cancels the other. The `wiki-check` job runs on Linux with the full git history. It runs CodeWiki's own `check` and uploads the SARIF result to code scanning, which never fails the build. Then it runs `update` and fails when the committed `obsidian/` folder differs from what the code generates, or when the sync would add notes. Only the commit named in the manifest may differ. Dependabot pull requests skip that step.
- Action tests `action.yml` as another repository would use it. It makes a small repository in three languages, runs the action to write its wiki, then checks that a synced wiki passes and that code changed without its wiki fails. It runs on Ubuntu and Windows, only when the action or its workflow changes.
- Security runs gitleaks, which scans for committed secrets, on pushes and pull requests, and weekly on Mondays with the RustSec advisory check.
- Docker builds the backend image. A version tag pushes it to the GitHub Container Registry. A pull request that touches the image inputs builds it without pushing.
- Release runs on a version tag. It builds the Windows package, writes checksums and release notes, and publishes. It uploads debug files to Sentry when a token is set. A pull request that changes packaging runs it as a dry run.

The toolchain is `stable`. The workspace needs Rust 1.95 and edition 2024. A release build uses thin LTO and one codegen unit. The `spikes` and `vendor` folders are outside the workspace.

## Depends on

- [[codewiki-app]] — builds a binary
- [[codewiki-mcp]] — builds a binary
- [[codewiki]] — builds a binary

## Gotchas

- A bare `cargo deny check` also runs the license check. `deny.toml` lists no allowed licenses, so it refuses every crate. Run `cargo deny check advisories bans sources`.
- A new RustSec advisory can fail CI with no change in the repository. Fix the dependency, or add the ID to `ignore` in `deny.toml` with a reason.
- Code changed without its wiki fails CI. After a code change, run `cargo run -p codewiki -- update` in a tree with no stray files and commit `obsidian/` as a `docs(wiki)` commit.
- Source files are hashed with CRLF read as LF, so a Windows checkout does not mark notes behind.
- A release tag must match the workspace version in `Cargo.toml` (now 0.7.0), or the Release job stops. A tag with a pre-release part, such as `-rc.1`, is published as a pre-release.
- The Release job fails if the setup program was not built. It needs the Inno Setup compiler.
- Dev builds keep only line tables for our crates, no debug information for dependencies, and no incremental cache. Debug builds once grew past 40 GB.
- The vendored `gpui-base` is a patch of one release (`[patch.crates-io]` in the root manifest). It adds the link hover look and the heading and block position calls the outline uses. When `gpui-kit` is bumped, drop the patch or copy the new release and patch it again; otherwise those calls stop compiling.
- The Dockerfile copies `vendor`, because Cargo reads every `[patch]` of the workspace. The server in the image refuses to start on a public address without `CODEWIKI_TOKEN`.
- Runs on a pull request cancel older runs of the same branch. Runs on main are never cancelled.
- Clippy warns on an `unsafe` block with no `// SAFETY:` comment, and with `-D warnings` that fails CI.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
