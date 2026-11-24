//! The session-start briefing against a real repository and vault.

mod support;

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
    support::no_model_server();
    let facts = service::reindex(root, &mut index).unwrap();
    let metrics = index.metrics().unwrap();
    cw_wiki::generate(root, &facts, &metrics, cw_wiki::git::head_commit(root)).unwrap();
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
    // Rules wrote the notes; none has an explanation yet.
    let waiting = cw_wiki::explanations::waiting_counts(r).unwrap();
    assert!(
        waiting.unexplained >= 2 && waiting.behind == 0,
        "{waiting:?}"
    );
    assert!(
        fresh.contains(&format!(
            "Notes waiting for an explanation: {} (0 whose explanation may be behind the code, {} never explained).",
            waiting.unexplained, waiting.unexplained
        )),
        "{fresh}"
    );
    assert!(
        fresh.contains("`list_waiting_notes`, `get_note_brief` and `submit_note`")
            && fresh.contains("write notes when the user asks"),
        "{fresh}"
    );
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
fn the_line_about_waiting_notes_goes_when_none_waits() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Cargo.toml", "[package]\nname = \"demo\"\n");
    write(r, "src/lib.rs", "//! Demo crate.\npub fn run() {}\n");
    update(r);
    assert!(
        briefing::session_start(r)
            .unwrap()
            .contains("Notes waiting for an explanation:")
    );
    // Every note is explained, as after an agent wrote them all.
    let vault = r.join("obsidian");
    let mut meta = cw_vault::VaultMeta::load(&vault).unwrap();
    for entry in meta.notes.values_mut().filter(|e| e.kind.explainable()) {
        entry.explained = Some(cw_vault::Explained {
            sections: vec![],
            sources: vec![],
            commit: None,
        });
    }
    meta.save(&vault).unwrap();
    let text = briefing::session_start(r).unwrap();
    assert!(!text.contains("waiting for an explanation"), "{text}");
    assert!(text.contains("Pending proposals: 0.\n"), "{text}");
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

#[test]
fn a_linked_worktree_is_told_from_the_main_checkout() {
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join("main");
    fs::create_dir(&main).unwrap();
    git(&main, &["init", "-q", "-b", "main"]);
    write(&main, "a.txt", "one\n");
    git(&main, &["add", "-A"]);
    git(&main, &["commit", "-q", "-m", "first"]);
    assert!(!cw_wiki::git::is_linked_worktree(&main));

    let linked = dir.path().join("linked");
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            linked.to_str().unwrap(),
            "-b",
            "side",
        ],
    );
    assert!(cw_wiki::git::is_linked_worktree(&linked));
    assert!(!cw_wiki::git::is_linked_worktree(&main));

    let bare = tempfile::tempdir().unwrap();
    assert!(!cw_wiki::git::is_linked_worktree(bare.path()));
}
