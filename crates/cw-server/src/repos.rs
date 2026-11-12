//! The list of repositories this backend manages, stored as `repos.json`.

use std::path::{Path, PathBuf};

use cw_core::paths::repo_id;
use cw_core::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoEntry {
    pub id: String,
    pub name: String,
    /// Absolute path of the repository root on the backend's machine.
    pub path: String,
}

fn file(data_dir: &Path) -> PathBuf {
    data_dir.join("repos.json")
}

pub fn load(data_dir: &Path) -> Result<Vec<RepoEntry>> {
    let path = file(data_dir);
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            serde_json::from_str(&text).map_err(|e| Error::external("parsing repos.json", e))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(Error::io(path, e)),
    }
}

fn save(data_dir: &Path, repos: &[RepoEntry]) -> Result<()> {
    std::fs::create_dir_all(data_dir).map_err(|e| Error::io(data_dir, e))?;
    let text = serde_json::to_string_pretty(repos)
        .map_err(|e| Error::external("encoding repos.json", e))?;
    let path = file(data_dir);
    std::fs::write(&path, text + "\n").map_err(|e| Error::io(path, e))
}

/// Add a repository by path. Adding the same path twice returns the existing
/// entry.
pub fn add(data_dir: &Path, path: &str) -> Result<RepoEntry> {
    let root = Path::new(path)
        .canonicalize()
        .map_err(|_| Error::invalid(format!("'{path}' is not an existing folder")))?;
    if !root.is_dir() {
        return Err(Error::invalid(format!("'{path}' is not a folder")));
    }
    let mut repos = load(data_dir)?;
    let id = repo_id(&root);
    if let Some(existing) = repos.iter().find(|r| r.id == id) {
        return Ok(existing.clone());
    }
    let display = root.to_string_lossy();
    let entry = RepoEntry {
        id,
        name: root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| display.to_string()),
        // Windows canonical paths carry a `\\?\` prefix that tools reject.
        path: display.trim_start_matches(r"\\?\").to_string(),
    };
    repos.push(entry.clone());
    repos.sort_by_key(|a| a.name.to_lowercase());
    save(data_dir, &repos)?;
    Ok(entry)
}

/// Forget a repository. Its vault and source are not touched.
pub fn remove(data_dir: &Path, id: &str) -> Result<()> {
    let mut repos = load(data_dir)?;
    let before = repos.len();
    repos.retain(|r| r.id != id);
    if repos.len() == before {
        return Err(Error::not_found(format!("repository '{id}'")));
    }
    save(data_dir, &repos)
}

pub fn get(data_dir: &Path, id: &str) -> Result<RepoEntry> {
    load(data_dir)?
        .into_iter()
        .find(|r| r.id == id)
        .ok_or_else(|| Error::not_found(format!("repository '{id}'")))
}
