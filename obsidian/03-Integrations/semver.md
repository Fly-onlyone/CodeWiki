---
tags: [integration]
---

# semver

> External crate `semver`, imported by 3 source files.

## What it does

The `semver` crate parses and compares version numbers such as `0.5.0`. The self-update code uses it to decide whether a GitHub release is newer than the running program, and to check that an unpacked package holds the version it should.

## Used for

Imported by 3 source files. Main users:

- [[Release Module]] — imports `Version`
- [[Stage Module]] — imports `Version`
- [[cw-update]] — imports `Version`

## Configuration

- `crates/cw-client/Cargo.toml`, `crates/cw-update/Cargo.toml` — declare the dependency: version `1.0.28`, locked `1.0.28`

## Wire-up

`cw-update` re-exports `Version` from its root, so other crates use `cw_update::Version` and need no direct dependency of their own.

Two places use it.

- `check` in the release module reads the running version with `Version::parse`. `parse` reads the latest release's tag, with a leading `v` removed, and compares the two. The package name is built from the new version: `CodeWiki-<version>-windows-x64.zip`, plus a `.sha256` file.
- `stage` unpacks a package and calls `program_version`. That runs `codewiki.exe --version`, reads the second word of the output with `parse_version_line`, and checks it equals the expected `Version`.

## Auth mode

No sign-in. The crate only parses text and compares values, and it needs no secret.

## Gotchas

- A release counts as newer only when its version is higher and has no pre-release part. A tag such as `1.0.0-rc.1` gives `UpToDate`.
- A tag that is not a valid version gives `Unavailable` with the reason, not a crash. So does a running version that cannot be read.
- The text `--version` prints must hold the version as its second word, as clap prints it. A different format fails the stage step.
- If the package's own version differs from the release's, `stage` refuses it.

## See also

- [[_index]]
