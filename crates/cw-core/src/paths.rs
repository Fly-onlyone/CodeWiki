//! Locations of per-user data.

use std::path::{Path, PathBuf};

use crate::hash::sha256_hex;

/// Environment variable that overrides the data directory (used in Docker).
pub const DATA_DIR_ENV: &str = "CODEWIKI_DATA_DIR";

/// Directory for indexes and backend state.
///
/// `CODEWIKI_DATA_DIR` when set, otherwise `%LOCALAPPDATA%\CodeWiki` on
/// Windows and `$XDG_DATA_HOME/codewiki` or `~/.local/share/codewiki` elsewhere.
pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os(DATA_DIR_ENV).filter(|d| !d.is_empty()) {
        return PathBuf::from(dir);
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA").filter(|d| !d.is_empty()) {
        return PathBuf::from(local).join("CodeWiki");
    }
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|d| !d.is_empty()) {
        return PathBuf::from(xdg).join("codewiki");
    }
    match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("codewiki"),
        None => PathBuf::from(".codewiki-data"),
    }
}

/// Stable identifier for a repository: its folder name plus a short hash of
/// the full path, so two checkouts with the same name do not collide.
pub fn repo_id(root: &Path) -> String {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let text = canonical
        .to_string_lossy()
        .replace('\\', "/")
        .to_lowercase();
    let text = text.trim_start_matches("//?/").trim_end_matches('/');
    let name: String = text
        .rsplit('/')
        .next()
        .unwrap_or("repo")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("{name}-{}", &sha256_hex(text)[..8])
}

/// Path of the index database for a repository.
pub fn index_path(root: &Path) -> PathBuf {
    index_path_in(&data_dir(), root)
}

/// Path of the index database for a repository under a given data directory.
pub fn index_path_in(data_dir: &Path, root: &Path) -> PathBuf {
    data_dir.join("index").join(format!("{}.db", repo_id(root)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_id_is_stable_and_file_name_safe() {
        let a = repo_id(Path::new("C:/Code/My Repo"));
        let b = repo_id(Path::new("c:\\code\\my repo\\"));
        assert_eq!(a, b);
        assert!(a.starts_with("my-repo-"));
        assert!(
            a.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
        assert_ne!(a, repo_id(Path::new("D:/Other/My Repo")));
    }
}
