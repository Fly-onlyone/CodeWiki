//! The session-start briefing against a real repository and vault.

use std::fs;
use std::path::Path;
use std::process::Command;

use cw_index::Index;
use cw_vault::proposal;
use cw_wiki::{briefing, service};

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

/// Scan and generate the vault the way `codewiki update` does, with the
/// index kept in memory.
fn update(root: &Path) {
    let mut index = Index::open_in_memory().unwrap();
    let facts = service::reindex(root, &mut index).unwrap();
    let metrics = index.metrics().unwrap();
    cw_wiki::generate(
        root,
        &facts,
        &metrics,
        cw_wiki::git::head_commit(root),
        None,
    )
    .unwrap();
}

#[test]
fn reports_a_missing_wiki_then_its_state() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    git(r, &["init", "-q", "-b", "main"]);
    write(
        r,
        "Cargo.toml",
        "[package]\nname = \"demo\"\ndescription = \"Demo crate\"\n",
    );
    write(
        r,
        "src/lib.rs",
        "//! Demo crate.\npub mod math;\n\n/// Entry point.\npub fn run() -> u64 { math::double(21) }\n",
    );
    write(
        r,
        "src/math.rs",
        "//! Arithmetic helpers.\n\n/// Doubles a number.\npub fn double(x: u64) -> u64 { x * 2 }\n",
    );
    commit(r, "first");

    let none = briefing::session_start(r).unwrap();
    assert!(none.contains("no wiki has been generated"), "{none}");
    assert!(none.contains("`sync_wiki`") && none.contains("`get_overview`"));
    assert_eq!(briefing::vault_root(r), None);

    update(r);
    let fresh = briefing::session_start(r).unwrap();
    assert!(
        fresh.contains("Its home note is `obsidian/_HOME.md`."),
        "{fresh}"
    );
    assert!(
        fresh.contains("The wiki is up to date with the code at HEAD."),
        "{fresh}"
    );
    // Committing the wiki itself moves HEAD but changes no code, so the wiki
    // is still current.
    commit(r, "add the wiki");
    let committed = briefing::session_start(r).unwrap();
    assert!(
        committed.contains("The wiki is up to date with the code at HEAD."),
        "{committed}"
    );
    assert!(!service::is_stale(r).unwrap());
    assert!(fresh.contains("Pending proposals: 0.\n"), "{fresh}");
    assert!(fresh.contains("`get_overview`") && fresh.contains("`get_context`"));
    assert!(
        fresh.lines().count() <= 5,
        "the briefing stays short:\n{fresh}"
    );
    // A session started in a subfolder finds the repository's vault.
    assert_eq!(briefing::vault_root(&r.join("src")).as_deref(), Some(r));

    // A human edit followed by a source change queues a proposal, and a new
    // commit leaves the wiki behind HEAD.
    let note = r.join("obsidian/01-Core/demo/Math Module.md");
    let text = fs::read_to_string(&note).unwrap();
    fs::write(&note, text + "\n## Gotchas\n\n- Mine.\n").unwrap();
    write(
        r,
        "src/math.rs",
        "//! Arithmetic helpers for the demo.\n\n/// Doubles a number.\npub fn double(x: u64) -> u64 { x * 2 }\n",
    );
    update(r);
    assert_eq!(proposal::list(&r.join("obsidian")).unwrap().len(), 1);
    commit(r, "second");
    let stale = briefing::session_start(r).unwrap();
    assert!(stale.contains("The wiki is stale"), "{stale}");
    assert!(stale.contains("Pending proposals: 1 ("), "{stale}");
}

#[test]
fn a_folder_without_git_is_not_called_stale() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Cargo.toml", "[package]\nname = \"demo\"\n");
    write(r, "src/lib.rs", "//! Demo crate.\npub fn run() {}\n");
    update(r);
    let text = briefing::session_start(r).unwrap();
    assert!(text.contains("no git HEAD"), "{text}");
    assert!(!text.contains("stale"), "{text}");
}
