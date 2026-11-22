---
tags: [core, cw-update]
---

# cw-update

> Self-update of an installed CodeWiki: release check, download, verification and file swap.

## What it does

The desktop app asks GitHub for the latest release (`check`). When the user accepts, `install` downloads the package into the installation's `.update` folder, verifies it against the release's `.sha256` file, unpacks it, checks that the new `codewiki.exe` starts, stops the backend and swaps the files (`swap`). Windows lets a running program be renamed but not replaced, so every file that is replaced is first renamed to `<name>.old`; programs still running (the app, an agent's MCP bridge) keep running from that file. `recover_and_clean`, run when the app starts, finishes the job: it undoes a swap that was interrupted and deletes the `.old` files.

Only a copy installed by `install.ps1` updates itself (`installed_dir`).

## Source

- `crates/cw-update/Cargo.toml` — package manifest
- `crates/cw-update/src/lib.rs` — crate root

## How it works

Builds a library.

1. `AppView::install_update` and `AppView::start_updates` in [[codewiki-app]], and 2 more, call `installed_dir`. `installed_dir` calls `is_installation`.
2. `check` in [[Update Module]] calls `api_base`.
3. `AppView::install_update` in [[codewiki-app]] calls `install`. `install` calls `download` in [[Download Module]], then `stage`, then `program_version` (both in [[Stage Module]]), then `swap` in [[Swap Module]], then `record_version` in [[Apps Entry Module]].
4. `AppView::install_update` in [[codewiki-app]] calls `relaunch`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Modules:

- [[Apps Entry Module]] — the setup program's entry in Windows Settings > Apps.
- [[Download Module]] — downloading a package and checking it against its `.sha256` file.
- [[Release Module]] — asking GitHub for the latest release.
- [[Stage Module]] — unpacking a verified package and checking that its programs start.
- [[Swap Module]] — replacing the files of an installation, and undoing it.
- [[Wire Tests Module]] — drives the check and the download against a stand-in for GitHub, so no network is needed.

Defined at the crate root:

- `pub const REPOSITORY: &str` — GitHub repository whose releases are offered.
- `pub const URL_ENV: &str` — overrides the GitHub API address, for tests against a stand-in server.
- `pub(crate) const GITHUB_API: &str` — internal constant; used by `api_base`, `from_repository`.
- `pub const UPDATE_DIR: &str` — folder inside the installation that holds downloads, the unpacked package and the swap journal. Inside the installation, so every move is a rename on one volume.
- `pub const PROGRAMS: [&str; 3]` — the programs a package must hold.
- `pub fn api_base() -> String` — address of the GitHub API: `URL_ENV` when set, otherwise GitHub's.
- `pub fn is_installation(dir: &Path) -> bool` — whether `dir` is an installation made by `install.ps1`. It copies `uninstall.ps1` but not `install.ps1`; an unpacked package holds both and a development build neither.
- `pub fn installed_dir() -> Option<PathBuf>` — the installation this program runs from, if it was installed.
- `pub enum Installed` — what `install` did.
- `install` — download, verify and unpack `release`, then stop the backend with `stop_backend` and swap the files of the installation at `install`. On an error before the swap, or in it, the installation is left as it was and the message says so; the one exception, a swap that could not be undone, says that instead.
- `pub fn relaunch(install: &Path, args: &[OsString]) -> Result<(), String>` — start the installed app with `args`, the way the Start Menu shortcut does.

## Depends on

- [[Apps Entry Module]] — uses `APPS_ENTRY`, `record_version`
- [[Stage Module]] — uses `program_version`, `stage`
- [[Download Module]] — uses `download`
- [[Swap Module]] — uses `swap`
- [[Release]] — uses `Release`

## Used by

- [[Swap Module]] — from `journal_path`, `leftovers`, `recover_and_clean`
- [[codewiki-app]] — from `install_update`, `start_updates`
- [[Autostart Module]] — from `enabled`, `set`
- [[Release Module]] — from `check`, `from_repository`
- [[Update Module]] — from `check`
- [[Stage Module]] — from `stage`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
