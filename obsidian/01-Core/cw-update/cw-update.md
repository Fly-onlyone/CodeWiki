---
tags: [core, cw-update]
---

# cw-update

> Self-update of an installed CodeWiki: release check, download, verification and file swap.

## What it does

Lets an installed CodeWiki update itself. The desktop app asks GitHub for the latest release. If the user accepts, this crate downloads the package, verifies it, unpacks it, checks that the new program starts, stops the backend and swaps the files in. Only a copy set up by `install.ps1` updates itself; a development build or an unpacked package does not.

## Source

- `crates/cw-update/Cargo.toml` — package manifest
- `crates/cw-update/src/lib.rs` — crate root

## How it works

`installed_dir` finds the installation: the folder of the running program, if `is_installation` says it holds `uninstall.ps1` and no `install.ps1`. `api_base` gives the API address, which an environment variable can override for tests.

`install` runs the whole update inside the `.update` folder:

1. `download` in [[Download Module]] fetches and verifies the zip.
2. `stage` in [[Stage Module]] unpacks it and checks its version.
3. If the installed `codewiki.exe` already has this version or a newer one (another app copy updated first), it returns `AlreadyCurrent`.
4. It calls the `stop_backend` function the caller passed in.
5. `swap` in [[Swap Module]] replaces the files.
6. `record_version` in [[Apps Entry Module]] updates the version shown in Windows Settings.

A failure up to and including the swap leaves the installation as it was, and the message says "nothing was changed". The one exception is a swap that could not be undone. `relaunch` then starts the new app. Checking for a release is done by `check` in [[Release Module]].

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

## Gotchas

- Folder `.update` lives inside the installation on purpose, so every move is a rename on one volume.
- A failed Apps-entry update is only logged. It does not undo a working update.
- `relaunch` starts the new app with the installation folder as working directory. The app treats a working folder that is a repository as the one to open, so the old app's folder must not be passed on.
- `install` takes the backend stop as a closure, so this crate does not know how the backend runs.
- `PROGRAMS` lists the three programs a package must hold. A package missing one is rejected before any file is replaced.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
