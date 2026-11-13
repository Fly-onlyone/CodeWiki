//! Which repository the app opens when it starts.
//!
//! A folder given as the first argument is opened as given. Without one, the
//! working folder counts only when it looks like a repository: started from
//! the Start menu, the app's working folder is wherever the shortcut says (it
//! once said the user's profile), and opening that folder would index
//! whatever is in it. Otherwise the repository opened last, while its folder
//! is still there. With none of these, the app asks for a folder.

use std::path::{Path, PathBuf};

use crate::prefs;

/// The setting that remembers the repository opened last.
const LAST_REPO: &str = "last_repo";

/// Whether `dir` looks like a repository: it holds `.git` (a folder, or a
/// file in a git worktree or submodule) or a CodeWiki vault.
pub fn looks_like_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
        || dir
            .join(cw_core::VAULT_DIR)
            .join(cw_core::MANIFEST_DIR)
            .is_dir()
}

/// The folder to open, from the first argument, the working folder and the
/// repository opened last, in that order.
fn choose(arg: Option<String>, cwd: Option<PathBuf>, last: Option<String>) -> Option<String> {
    if let Some(arg) = arg.filter(|arg| !arg.is_empty()) {
        return Some(arg);
    }
    if let Some(cwd) = cwd.filter(|cwd| looks_like_repo(cwd)) {
        return Some(cwd.to_string_lossy().into_owned());
    }
    last.filter(|last| Path::new(last).is_dir())
}

/// The folder to open at startup, if any.
pub fn repo() -> Option<String> {
    choose(
        std::env::args().nth(1),
        std::env::current_dir().ok(),
        prefs::read(LAST_REPO),
    )
}

/// Remember `path` as the repository to open next time.
pub fn remember(path: &str) {
    prefs::write(LAST_REPO, path);
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// A fresh folder under the system's temporary folder, removed when the
    /// test ends.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(test: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("codewiki-startup-{test}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn text(&self) -> String {
            self.0.to_string_lossy().into_owned()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_git_folder_a_git_file_or_a_vault_makes_a_repository() {
        let plain = Scratch::new("plain");
        assert!(!looks_like_repo(&plain.0));

        let git = Scratch::new("git");
        fs::create_dir(git.0.join(".git")).unwrap();
        assert!(looks_like_repo(&git.0));

        // A git worktree or submodule has a .git file, not a folder.
        let worktree = Scratch::new("worktree");
        fs::write(worktree.0.join(".git"), "gitdir: elsewhere").unwrap();
        assert!(looks_like_repo(&worktree.0));

        let vault = Scratch::new("vault");
        fs::create_dir_all(vault.0.join("obsidian").join(".codewiki")).unwrap();
        assert!(looks_like_repo(&vault.0));

        // An obsidian folder alone is not CodeWiki's vault.
        let notes = Scratch::new("notes");
        fs::create_dir(notes.0.join("obsidian")).unwrap();
        assert!(!looks_like_repo(&notes.0));
    }

    #[test]
    fn an_argument_is_taken_as_given() {
        let repo = Scratch::new("arg-repo");
        fs::create_dir(repo.0.join(".git")).unwrap();
        assert_eq!(
            choose(Some(r"D:\code\app".into()), Some(repo.0.clone()), None),
            Some(r"D:\code\app".into())
        );
        // An empty argument is no argument.
        assert_eq!(
            choose(Some(String::new()), Some(repo.0.clone()), None),
            Some(repo.text())
        );
    }

    #[test]
    fn the_working_folder_counts_only_when_it_is_a_repository() {
        let repo = Scratch::new("cwd-repo");
        fs::create_dir(repo.0.join(".git")).unwrap();
        let last = Scratch::new("cwd-last");
        assert_eq!(
            choose(None, Some(repo.0.clone()), Some(last.text())),
            Some(repo.text())
        );

        // The profile or the install folder: not a repository.
        let profile = Scratch::new("cwd-profile");
        assert_eq!(
            choose(None, Some(profile.0.clone()), Some(last.text())),
            Some(last.text())
        );
        assert_eq!(choose(None, Some(profile.0.clone()), None), None);
    }

    #[test]
    fn the_last_repository_counts_only_while_its_folder_is_there() {
        let gone = Scratch::new("gone");
        let path = gone.text();
        drop(gone);
        assert_eq!(choose(None, None, Some(path)), None);
        assert_eq!(choose(None, None, None), None);
    }
}
