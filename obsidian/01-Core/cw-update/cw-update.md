---
tags: [core, cw-update]
---

# cw-update

> Self-update of an installed CodeWiki: release check, download, verification and file swap.

## Source

- `crates/cw-update/Cargo.toml` — package manifest
- `crates/cw-update/src/lib.rs` — crate root

## How it works

Builds a library.

Modules:

- [[Download Module]] — downloading a package and checking it against its `.sha256` file.
- [[Release Module]] — asking GitHub for the latest release.
- [[Stage Module]] — unpacking a verified package and checking that its programs start.
- [[Swap Module]] — replacing the files of an installation, and undoing it.
- [[Wire Tests Module]] — drives the check and the download against a stand-in for GitHub, so no network is needed.

Defined at the crate root:

- `UPDATE_DIR` — folder inside the installation that holds downloads, the unpacked package and the swap journal.
- `install` — download, verify and unpack `release`, then stop the backend with `stop_backend` and swap the files of the installation at `install`.
- `is_installation` — whether `dir` is an installation made by `install.ps1`.
- `REPOSITORY` — GitHub repository whose releases are offered.
- `URL_ENV` — overrides the GitHub API address, for tests against a stand-in server.
- `PROGRAMS` — the programs a package must hold.
- `api_base` — address of the GitHub API: `URL_ENV` when set, otherwise GitHub's.
- `installed_dir` — the installation this program runs from, if it was installed.

## Depends on

- [[Stage Module]] — uses `program_version`, `stage`
- [[Download Module]] — uses `download`
- [[Swap Module]] — uses `swap`
- [[Release]] — uses `Release`

## Used by

- [[codewiki-app]] — from `install_update`, `start_updates`
- [[Release Module]] — from `check`, `from_repository`
- [[Swap Module]] — from `journal_path`, `recover_and_clean`
- [[Update Module]] — from `check`
- [[Stage Module]] — from `stage`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
