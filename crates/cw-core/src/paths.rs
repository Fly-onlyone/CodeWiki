//! Locations of per-user data.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::DEV_BUILD;
use crate::hash::sha256_hex;

/// Environment variable that overrides the data directory (used in Docker).
pub const DATA_DIR_ENV: &str = "CODEWIKI_DATA_DIR";

/// The name of CodeWiki's folders under `%LOCALAPPDATA%` and `%APPDATA%`. A
/// development build has folders of its own ([`crate::DEV_BUILD`]): a run
/// from the sources would otherwise use, and on Quit stop, the backend of an
/// installed CodeWiki, and read and write its index and settings.
pub const APP_FOLDER: &str = if DEV_BUILD {
    "CodeWiki-Dev"
} else {
    "CodeWiki"
};

/// [`APP_FOLDER`] as it is spelled under `$XDG_DATA_HOME` and `~/.local/share`.
const UNIX_FOLDER: &str = if DEV_BUILD {
    "codewiki-dev"
} else {
    "codewiki"
};

/// Directory for indexes and backend state.
///
/// `CODEWIKI_DATA_DIR` when set, otherwise `%LOCALAPPDATA%\CodeWiki` on
/// Windows and `$XDG_DATA_HOME/codewiki` or `~/.local/share/codewiki` elsewhere.
/// A development build uses `CodeWiki-Dev` and `codewiki-dev` instead
/// ([`APP_FOLDER`]).
pub fn data_dir() -> PathBuf {
    data_dir_from(|name| std::env::var_os(name))
}

/// [`data_dir`], with the environment read through `env`.
fn data_dir_from(env: impl Fn(&str) -> Option<OsString>) -> PathBuf {
    let var = |name: &str| env(name).filter(|value| !value.is_empty());
    if let Some(dir) = var(DATA_DIR_ENV) {
        return PathBuf::from(dir);
    }
    if let Some(local) = var("LOCALAPPDATA") {
        return PathBuf::from(local).join(APP_FOLDER);
    }
    if let Some(xdg) = var("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join(UNIX_FOLDER);
    }
    match var("HOME") {
        Some(home) => PathBuf::from(home)
            .join(".local")
            .join("share")
            .join(UNIX_FOLDER),
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

    fn env_of<'a>(vars: &'a [(&str, &str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[test]
    fn data_dir_takes_the_variable_first_then_the_app_folder() {
        let named = [
            (DATA_DIR_ENV, r"D:\data"),
            ("LOCALAPPDATA", r"C:\Users\me\AppData\Local"),
        ];
        assert_eq!(data_dir_from(env_of(&named)), PathBuf::from(r"D:\data"));

        // An empty variable counts as unset.
        let local = [
            (DATA_DIR_ENV, ""),
            ("LOCALAPPDATA", r"C:\Users\me\AppData\Local"),
        ];
        assert_eq!(
            data_dir_from(env_of(&local)),
            PathBuf::from(r"C:\Users\me\AppData\Local").join(APP_FOLDER)
        );

        let xdg = [("XDG_DATA_HOME", "/home/me/.data")];
        assert_eq!(
            data_dir_from(env_of(&xdg)),
            PathBuf::from("/home/me/.data").join(UNIX_FOLDER)
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn a_development_build_keeps_folders_and_a_name_of_its_own() {
        assert_eq!(APP_FOLDER, "CodeWiki-Dev");
        assert_eq!(UNIX_FOLDER, "codewiki-dev");
        assert_eq!(crate::APP_NAME, "CodeWiki Dev");
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn a_release_build_uses_the_installed_folders_and_name() {
        assert_eq!(APP_FOLDER, "CodeWiki");
        assert_eq!(UNIX_FOLDER, "codewiki");
        assert_eq!(crate::APP_NAME, "CodeWiki");
    }
}
