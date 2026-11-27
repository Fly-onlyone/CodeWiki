//! What a sync asks git before it removes, moves or keeps a note.

use std::fs;
use std::path::Path;
use std::process::Command;

use cw_wiki::git as wiki_git;

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["-c", "user.name=Ann", "-c", "user.email=dev@example.com"])
        .args(["-c", "commit.gpgsign=false", "-c", "core.autocrlf=false"])
        .args(args)
        .output()
        .expect("git is installed");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn commit(root: &Path, message: &str) {
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", message]);
}

/// Long enough that git still sees a rename after the file moved.
const BODY: &str = "pub fn run() -> u8 {\n    let a = 1;\n    let b = 2;\n    a + b\n}\n";

#[test]
fn history_keeps_deleted_paths_and_follows_moves_to_their_end() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    git(r, &["init", "-q", "-b", "main"]);
    write(r, "gone.txt", "soon deleted\n");
    write(r, "src/old name.rs", BODY);
    commit(r, "first");
    git(r, &["mv", "src/old name.rs", "src/middle.rs"]);
    commit(r, "move once");
    fs::create_dir_all(r.join("lib")).unwrap();
    git(r, &["mv", "src/middle.rs", "lib/new.rs"]);
    fs::remove_file(r.join("gone.txt")).unwrap();
    commit(r, "move again, delete one");

    let history = wiki_git::history_paths(r);
    for path in ["gone.txt", "src/old name.rs", "src/middle.rs", "lib/new.rs"] {
        assert!(history.contains(path), "{path} in {history:?}");
    }

    let moves = wiki_git::moves(r);
    assert_eq!(
        moves.get("src/old name.rs").map(String::as_str),
        Some("lib/new.rs")
    );
    assert_eq!(
        moves.get("src/middle.rs").map(String::as_str),
        Some("lib/new.rs")
    );
    assert!(!moves.contains_key("gone.txt"), "a deletion is not a move");

    let head = wiki_git::at_head(r).expect("a repository with a commit");
    assert!(head.contains("lib/new.rs"));
    assert!(!head.contains("gone.txt") && !head.contains("src/old name.rs"));
}

#[test]
fn a_deletion_that_is_not_committed_is_still_at_head() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    git(r, &["init", "-q", "-b", "main"]);
    write(r, "src/a.rs", BODY);
    commit(r, "first");
    fs::remove_file(r.join("src/a.rs")).unwrap();

    assert!(wiki_git::at_head(r).unwrap().contains("src/a.rs"));
}

#[test]
fn only_a_tracked_and_unchanged_note_is_recoverable() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    git(r, &["init", "-q", "-b", "main"]);
    write(r, "obsidian/01-Core/Kept Note.md", "# Kept Note\n");
    write(r, "obsidian/01-Core/Edited.md", "# Edited\n");
    write(r, "obsidian/01-Core/Staged.md", "# Staged\n");
    write(r, "src/a.rs", BODY);
    commit(r, "first");
    write(r, "obsidian/01-Core/Edited.md", "# Edited\n\nMore.\n");
    write(r, "obsidian/01-Core/Staged.md", "# Staged\n\nMore.\n");
    git(r, &["add", "obsidian/01-Core/Staged.md"]);
    write(r, "obsidian/01-Core/New.md", "# New\n");

    let safe = wiki_git::recoverable(r);
    assert!(safe.contains("01-Core/Kept Note.md"), "{safe:?}");
    for lost in ["01-Core/Edited.md", "01-Core/Staged.md", "01-Core/New.md"] {
        assert!(!safe.contains(lost), "{lost} in {safe:?}");
    }
}

#[test]
fn a_repository_inside_a_bigger_checkout_reads_its_own_paths() {
    let dir = tempfile::tempdir().unwrap();
    let top = dir.path();
    git(top, &["init", "-q", "-b", "main"]);
    let r = top.join("apps/one");
    write(&r, "obsidian/A.md", "# A\n");
    write(&r, "obsidian/B.md", "# B\n");
    write(&r, "src/a.rs", BODY);
    write(top, "other/z.rs", BODY);
    commit(top, "first");
    write(&r, "obsidian/B.md", "# B\n\nMore.\n");

    let safe = wiki_git::recoverable(&r);
    assert!(safe.contains("A.md") && !safe.contains("B.md"), "{safe:?}");
    let head = wiki_git::at_head(&r).unwrap();
    assert!(head.contains("src/a.rs"), "{head:?}");
    assert!(wiki_git::history_paths(&r).contains("src/a.rs"));
}

#[test]
fn without_git_nothing_is_known_and_nothing_is_recoverable() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "obsidian/A.md", "# A\n");
    assert!(wiki_git::history_paths(r).is_empty());
    assert!(wiki_git::moves(r).is_empty());
    assert!(wiki_git::at_head(r).is_none());
    assert!(wiki_git::recoverable(r).is_empty());
}
