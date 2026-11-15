//! Self-update of an installed CodeWiki.
//!
//! The desktop app asks GitHub for the latest release ([`check`]). When the
//! user accepts, [`install`] downloads the package into the installation's
//! `.update` folder, verifies it against the release's `.sha256` file, unpacks
//! it, checks that the new `codewiki.exe` starts, stops the backend and swaps
//! the files ([`swap`]). Windows lets a running program be renamed but not
//! replaced, so every file that is replaced is first renamed to `<name>.old`;
//! programs still running (the app, an agent's MCP bridge) keep running from
//! that file. [`recover_and_clean`], run when the app starts, finishes the job:
//! it undoes a swap that was interrupted and deletes the `.old` files.
//!
//! Only a copy installed by `install.ps1` updates itself ([`installed_dir`]).

mod download;
mod release;
mod stage;
mod swap;
#[cfg(test)]
mod wire_tests;

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub use download::download;
pub use release::{Check, Release, check};
pub use semver::Version;
pub use stage::{program_version, stage};
pub use swap::{leftovers, recover_and_clean, swap};

/// GitHub repository whose releases are offered.
pub const REPOSITORY: &str = "Fly-onlyone/CodeWiki";

/// Overrides the GitHub API address, for tests against a stand-in server.
pub const URL_ENV: &str = "CODEWIKI_UPDATE_URL";

pub(crate) const GITHUB_API: &str = "https://api.github.com";

/// Folder inside the installation that holds downloads, the unpacked package
/// and the swap journal. Inside the installation, so every move is a rename on
/// one volume.
pub const UPDATE_DIR: &str = ".update";

/// The programs a package must hold.
pub const PROGRAMS: [&str; 3] = ["codewiki.exe", "codewiki-mcp.exe", "codewiki-app.exe"];

/// Address of the GitHub API: [`URL_ENV`] when set, otherwise GitHub's.
pub fn api_base() -> String {
    std::env::var(URL_ENV)
        .ok()
        .filter(|url| !url.is_empty())
        .map(|url| url.trim_end_matches('/').to_string())
        .unwrap_or_else(|| GITHUB_API.to_string())
}

/// Whether `dir` is an installation made by `install.ps1`. It copies
/// `uninstall.ps1` but not `install.ps1`; an unpacked package holds both and
/// a development build neither.
pub fn is_installation(dir: &Path) -> bool {
    dir.join("uninstall.ps1").is_file() && !dir.join("install.ps1").exists()
}

/// The installation this program runs from, if it was installed.
pub fn installed_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.to_path_buf();
    is_installation(&dir).then_some(dir)
}

/// What [`install`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Installed {
    /// The files were replaced.
    Swapped,
    /// The installation already had this version or a newer one, put there
    /// by another running copy of the app. Nothing was replaced.
    AlreadyCurrent,
}

/// Download, verify and unpack `release`, then stop the backend with
/// `stop_backend` and swap the files of the installation at `install`. On an
/// error before the swap, or in it, the installation is left as it was and
/// the message says so; the one exception, a swap that could not be undone,
/// says that instead.
pub fn install(
    install: &Path,
    release: &Release,
    stop_backend: impl FnOnce() -> Result<(), String>,
) -> Result<Installed, String> {
    let unchanged = |e: String| format!("{e}; nothing was changed");
    let dir = install.join(UPDATE_DIR).join(release.version.to_string());
    log::info!("update: downloading {}", release.zip_url);
    let zip = download(release, &dir).map_err(unchanged)?;
    let staged = dir.join("stage");
    log::info!("update: unpacking {}", zip.display());
    stage(&zip, &staged, &release.version).map_err(unchanged)?;
    if let Ok(current) = program_version(&install.join("codewiki.exe"))
        && current >= release.version
    {
        log::info!("update: {current} is already installed");
        return Ok(Installed::AlreadyCurrent);
    }
    log::info!("update: stopping the backend");
    stop_backend().map_err(unchanged)?;
    log::info!("update: replacing the files in {}", install.display());
    swap(install, &staged)?;
    log::info!("update: installed {}", release.version);
    Ok(Installed::Swapped)
}

/// Start the installed app with `args`, the way the Start Menu shortcut does.
pub fn relaunch(install: &Path, args: &[OsString]) -> Result<(), String> {
    let app = install.join("codewiki-app.exe");
    Command::new(&app)
        .args(args)
        // Not the old app's folder: the app takes a working folder that is
        // a repository as the one to open.
        .current_dir(install)
        .stdin(Stdio::null())
        .spawn()
        .map(drop)
        .map_err(|e| format!("could not start {}: {e}", app.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_installed_copy_counts_as_an_installation() {
        let dir = tempfile::tempdir().unwrap();
        // A development build: no scripts at all.
        assert!(!is_installation(dir.path()));
        // An unpacked package: both scripts.
        std::fs::write(dir.path().join("uninstall.ps1"), "").unwrap();
        std::fs::write(dir.path().join("install.ps1"), "").unwrap();
        assert!(!is_installation(dir.path()));
        // What install.ps1 copies: uninstall.ps1 only.
        std::fs::remove_file(dir.path().join("install.ps1")).unwrap();
        assert!(is_installation(dir.path()));
    }
}
