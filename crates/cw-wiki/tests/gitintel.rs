//! Git intelligence against a real repository built with the `git` CLI.

use std::fs;
use std::path::Path;
use std::process::Command;

use cw_index::Index;
use cw_wiki::gitintel::{self, RiskLevel};
use cw_wiki::service;

fn git(root: &Path, author: &str, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            &format!("user.name={author}"),
            "-c",
            "user.email=dev@example.com",
        ])
        .args(["-c", "commit.gpgsign=false", "-c", "core.autocrlf=false"])
        .args(args)
        .output()
        .expect("git is installed");
    assert!(
        status.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&status.stderr)
    );
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn commit(root: &Path, author: &str, message: &str) {
    git(root, author, &["add", "-A"]);
    git(root, author, &["commit", "-q", "-m", message]);
}

/// `core.rs` is used by `api.rs` and `cli.rs`; `core.rs` and `api.rs` change
/// together; Ann wrote nearly all of `core.rs`.
fn fixture() -> (tempfile::TempDir, Index) {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    git(r, "Ann", &["init", "-q", "-b", "main"]);
    write(r, "Cargo.toml", "[package]\nname = \"demo\"\n");
    write(
        r,
        "src/lib.rs",
        "pub mod api;\npub mod cli;\npub mod core;\n",
    );
    write(r, "src/core.rs", "pub fn compute(x: u64) -> u64 { x }\n");
    write(
        r,
        "src/api.rs",
        "use crate::core::compute;\npub fn handle() -> u64 { compute(1) }\n",
    );
    write(
        r,
        "src/cli.rs",
        "use crate::api::handle;\npub fn run() -> u64 { handle() }\n",
    );
    write(r, "README.md", "# Demo\n");
    commit(r, "Ann", "initial");
    for round in 2..=6u64 {
        write(
            r,
            "src/core.rs",
            &format!("pub fn compute(x: u64) -> u64 {{ x + {round} }}\n"),
        );
        write(
            r,
            "src/api.rs",
            &format!("use crate::core::compute;\npub fn handle() -> u64 {{ compute({round}) }}\n"),
        );
        commit(r, "Ann", &format!("round {round}"));
    }
    write(r, "README.md", "# Demo\n\nMore.\n");
    commit(r, "Bo", "docs");

    let mut index = Index::open_in_memory().unwrap();
    service::reindex(r, &mut index).unwrap();
    (dir, index)
}

#[test]
fn mines_history_and_ranks_hotspots() {
    let (_dir, index) = fixture();
    let intel = gitintel::load(&index).unwrap();
    assert_eq!(intel.commits_scanned, 7);

    let core = &intel.files["src/core.rs"];
    assert_eq!(core.commits, 6);
    assert_eq!(core.bus_factor, 1);
    assert_eq!(core.owners[0].name, "Ann");
    assert_eq!(core.partners[0].path, "src/api.rs");
    assert_eq!(core.partners[0].together, 6);

    let hot = gitintel::hotspots(&index, &intel, 10).unwrap();
    let top: Vec<&str> = hot.iter().take(2).map(|h| h.path.as_str()).collect();
    assert!(
        top.contains(&"src/core.rs") && top.contains(&"src/api.rs"),
        "{top:?}"
    );
    assert_eq!(hot[0].score, 100.0);
    assert!(hot.iter().all(|h| h.score <= 100.0));
}

#[test]
fn blast_radius_follows_dependents() {
    let (_dir, index) = fixture();
    let radius = gitintel::blast_radius(&index, "src/core.rs").unwrap();
    assert_eq!(radius.direct, ["src/api.rs"]);
    assert_eq!(radius.transitive, ["src/cli.rs"]);
    let leaf = gitintel::blast_radius(&index, "src/cli.rs").unwrap();
    assert!(leaf.direct.is_empty() && leaf.transitive.is_empty());
}

#[test]
fn file_risk_explains_itself() {
    let (_dir, index) = fixture();
    let intel = gitintel::load(&index).unwrap();
    let core = gitintel::file_risk(&index, &intel, "src/core.rs").unwrap();
    let readme = gitintel::file_risk(&index, &intel, "README.md").unwrap();
    assert!(
        core.score > readme.score,
        "{} vs {}",
        core.score,
        readme.score
    );
    assert!(core.level >= RiskLevel::Medium, "{core:?}");
    assert_eq!(core.dependents, 2);
    let text = core.reasons.join(" | ");
    assert!(text.contains("2 files depend on it"), "{text}");
    assert!(text.contains("one main author (Ann"), "{text}");
    assert!(
        text.contains("usually changes together with `src/api.rs`"),
        "{text}"
    );

    let unknown = gitintel::file_risk(&index, &intel, "src/new.rs").unwrap();
    assert_eq!(unknown.reasons, ["no commit history yet"]);
    assert_eq!(unknown.level, RiskLevel::Low);
}

#[test]
fn change_risk_flags_forgotten_partners_and_reads_the_working_tree() {
    let (dir, index) = fixture();
    let intel = gitintel::load(&index).unwrap();

    let alone = gitintel::change_risk(&index, &intel, &["src/core.rs".to_string()]).unwrap();
    assert_eq!(alone.missing_partners.len(), 1);
    assert_eq!(alone.missing_partners[0].path, "src/api.rs");
    assert_eq!(alone.affected, ["src/api.rs", "src/cli.rs"]);

    let both = gitintel::change_risk(
        &index,
        &intel,
        &["src/core.rs".to_string(), "src/api.rs".to_string()],
    )
    .unwrap();
    assert!(both.missing_partners.is_empty());
    assert_eq!(both.affected, ["src/cli.rs"]);
    assert_eq!(both.files[0].path, "src/core.rs", "riskiest file first");

    assert!(gitintel::working_tree_changes(dir.path()).is_empty());
    write(
        dir.path(),
        "src/core.rs",
        "pub fn compute(x: u64) -> u64 { x * 2 }\n",
    );
    write(dir.path(), "src/brand new.rs", "pub fn fresh() {}\n");
    let mut changed = gitintel::working_tree_changes(dir.path());
    changed.sort();
    assert_eq!(changed, ["src/brand new.rs", "src/core.rs"]);
}

#[test]
fn works_without_git_history() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
    write(dir.path(), "src/lib.rs", "pub fn f() {}\n");
    let mut index = Index::open_in_memory().unwrap();
    service::reindex(dir.path(), &mut index).unwrap();
    let intel = gitintel::load(&index).unwrap();
    assert_eq!(intel.commits_scanned, 0);
    assert!(gitintel::hotspots(&index, &intel, 5).unwrap().is_empty());
    assert_eq!(
        gitintel::file_risk(&index, &intel, "src/lib.rs")
            .unwrap()
            .level,
        RiskLevel::Low
    );
}
