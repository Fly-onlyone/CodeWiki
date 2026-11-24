//! Minimal git access through the `git` command line.

use std::path::Path;
use std::process::Command;

fn run(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Commit id of `HEAD`, or `None` outside a repository or before the first
/// commit.
pub fn head_commit(root: &Path) -> Option<String> {
    run(root, &["rev-parse", "--verify", "HEAD"]).filter(|s| !s.is_empty())
}

/// True when the code moved on since the vault was generated at commit
/// `generated`. Commits that touch only the vault do not count: committing
/// the wiki must not make the wiki look out of date.
pub fn vault_is_behind(root: &Path, generated: Option<&str>) -> bool {
    let head = head_commit(root);
    match (generated, head.as_deref()) {
        (Some(at), Some(head)) if at == head => false,
        (Some(at), Some(_)) => match changed_files(root, at) {
            Some(files) => {
                let vault = format!("{}/", cw_core::VAULT_DIR);
                files.iter().any(|file| !file.starts_with(&vault))
            }
            // The commit is unknown here (rebased away, shallow clone).
            None => true,
        },
        (generated, head) => generated != head,
    }
}

/// Repo-relative paths changed between `base` and `HEAD`. `None` when `base`
/// is unknown to the repository (rebased away, shallow clone).
pub fn changed_files(root: &Path, base: &str) -> Option<Vec<String>> {
    let out = run(root, &["diff", "--name-only", &format!("{base}..HEAD")])?;
    Some(
        out.lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
    )
}

/// True when `root` is a linked worktree: its git directory is not the
/// repository's common one. Any failure counts as false.
///
/// Git 2.31 and later print both directories as absolute paths when asked
/// with `--path-format=absolute`. An older git echoes the flag it does not
/// know as a line of its own and prints paths relative to `root`, so that
/// line is skipped and a relative path is read against `root`.
pub fn is_linked_worktree(root: &Path) -> bool {
    let Some(out) = run(
        root,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-dir",
            "--git-common-dir",
        ],
    ) else {
        return false;
    };
    let mut dirs = out
        .lines()
        .filter(|line| !line.starts_with("--"))
        .map(|line| {
            let path = Path::new(line.trim());
            let full = if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            };
            full.canonicalize().ok()
        });
    match (dirs.next().flatten(), dirs.next().flatten()) {
        (Some(git_dir), Some(common)) => git_dir != common,
        _ => false,
    }
}
