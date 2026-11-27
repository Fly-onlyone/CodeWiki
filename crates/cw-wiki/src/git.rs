//! Minimal git access through the `git` command line.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;

/// Standard output of a git command as it was printed. `None` when git is
/// missing, `root` is no repository or the command fails.
fn raw(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        // Print a path with other letters than ASCII as it is, not escaped.
        .args(["-c", "core.quotepath=off"])
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The entries of an output asked for with `-z`.
fn entries(out: &str) -> impl Iterator<Item = &str> {
    out.split('\0').filter(|entry| !entry.is_empty())
}

/// Every path under `root` that a commit of any branch ever held, deleted
/// ones included. Empty outside a repository.
pub fn history_paths(root: &Path) -> HashSet<String> {
    raw(
        root,
        &[
            "log",
            "--all",
            "--relative",
            "--pretty=format:",
            "--name-only",
        ],
    )
    .map(|out| {
        out.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

/// Where each path that git saw renamed is now: the old path, and the last
/// path of its chain of renames. Empty outside a repository.
pub fn moves(root: &Path) -> HashMap<String, String> {
    let Some(out) = raw(
        root,
        &[
            "log",
            "--all",
            "--relative",
            "-M",
            "--diff-filter=R",
            "--pretty=format:",
            "--name-status",
        ],
    ) else {
        return HashMap::new();
    };
    // The log is newest first, so the first rename seen of a path is its
    // latest one.
    let mut step: HashMap<&str, &str> = HashMap::new();
    for line in out.lines() {
        let mut fields = line.split('\t');
        if let (Some(status), Some(old), Some(new)) = (fields.next(), fields.next(), fields.next())
            && status.starts_with('R')
        {
            step.entry(old).or_insert(new);
        }
    }
    step.keys()
        .map(|old| {
            let mut end = step[old];
            // A path renamed back and forth must not loop.
            for _ in 0..step.len() {
                match step.get(end) {
                    Some(next) if next != old => end = next,
                    _ => break,
                }
            }
            (old.to_string(), end.to_string())
        })
        .collect()
}

/// Paths under `root` in the commit at `HEAD`. `None` outside a repository
/// or before the first commit.
pub fn at_head(root: &Path) -> Option<HashSet<String>> {
    let out = raw(root, &["ls-tree", "-r", "-z", "--name-only", "HEAD"])?;
    Some(entries(&out).map(str::to_string).collect())
}

/// Paths under `root` with a change that is not committed: edited, added
/// or deleted, staged or not. `None` outside a repository.
pub fn changed(root: &Path) -> Option<HashSet<String>> {
    let status = raw(root, &["status", "--porcelain", "-z", "--no-renames"])?;
    let prefix = run(root, &["rev-parse", "--show-prefix"]).unwrap_or_default();
    Some(
        entries(&status)
            .filter_map(|entry| entry.get(3..))
            .filter_map(|path| path.strip_prefix(prefix.as_str()))
            .map(str::to_string)
            .collect(),
    )
}

/// Vault-relative paths of the notes git can bring back as they are now:
/// tracked, and without a change since their last commit, staged or not.
/// Empty outside a repository.
pub fn recoverable(root: &Path) -> HashSet<String> {
    let vault = cw_core::VAULT_DIR;
    let inside = format!("{vault}/");
    let Some(tracked) = raw(root, &["ls-files", "-z", "--", vault]) else {
        return HashSet::new();
    };
    let Some(status) = raw(
        root,
        &["status", "--porcelain", "-z", "--no-renames", "--", vault],
    ) else {
        return HashSet::new();
    };
    // `status` names paths from the top of the checkout, `ls-files` from
    // `root`; they differ when `root` is a folder inside a checkout.
    let prefix = run(root, &["rev-parse", "--show-prefix"]).unwrap_or_default();
    let changed: HashSet<&str> = entries(&status)
        .filter_map(|entry| entry.get(3..))
        .map(|path| path.strip_prefix(prefix.as_str()).unwrap_or(path))
        .collect();
    entries(&tracked)
        .filter(|path| !changed.contains(path))
        .filter_map(|path| path.strip_prefix(inside.as_str()))
        .map(str::to_string)
        .collect()
}

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

/// The main checkout of the repository that the linked worktree `root`
/// belongs to: the folder that holds the repository's common git directory.
/// `None` when `root` is no linked worktree.
pub fn main_checkout(root: &Path) -> Option<std::path::PathBuf> {
    if !is_linked_worktree(root) {
        return None;
    }
    let out = run(
        root,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )?;
    // An older git echoes the flag it does not know; see above.
    let line = out.lines().find(|line| !line.starts_with("--"))?;
    let path = Path::new(line.trim());
    let common = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    Some(common.canonicalize().ok()?.parent()?.to_path_buf())
}
