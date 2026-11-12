//! Decision records, staleness and the "why" query on a real git repository.

use std::fs;
use std::path::Path;
use std::process::Command;

use cw_index::Index;
use cw_wiki::{analyze, decisions, generate, service};

fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["-c", "user.name=Ann", "-c", "user.email=dev@example.com"])
        .args(["-c", "commit.gpgsign=false", "-c", "core.autocrlf=false"])
        .args(args)
        .output()
        .expect("git is installed");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Commit with an explicit timestamp so ordering does not depend on the clock.
fn commit(root: &Path, message: &str, day: u32) {
    git(root, &["add", "-A"]);
    let date = format!("2026-01-{day:02}T12:00:00");
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "user.name=Ann",
            "-c",
            "user.email=dev@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(["commit", "-q", "-m", message])
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn sync(root: &Path) -> Index {
    let mut index = Index::open_in_memory().unwrap();
    let facts = service::reindex(root, &mut index).unwrap();
    generate(root, &facts, &analyze(&facts), None, None).unwrap();
    index
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    git(r, &["init", "-q", "-b", "main"]);
    write(r, "Cargo.toml", "[package]\nname = \"demo\"\n");
    write(r, "src/lib.rs", "pub mod cache;\npub mod store;\n");
    write(
        r,
        "src/cache.rs",
        "//! In-memory cache.\n\n/// Look up a key.\npub fn lookup(key: u64) -> u64 { key }\n",
    );
    write(
        r,
        "src/store.rs",
        "//! Durable store.\n\n/// Persist a value.\npub fn persist(value: u64) -> u64 { value }\n",
    );
    write(
        r,
        "docs/adr/0001-cache-in-memory.md",
        "# 1. Keep the cache in memory\n\n## Status\n\nAccepted\n\n## Context\n\nLookups in `src/cache.rs` must not touch disk.\n\n## Decision\n\nThe cache holds everything in memory.\n",
    );
    write(
        r,
        "docs/adr/0002-old-queue.md",
        "# 2. Use a queue\n\nStatus: Superseded by 3\n\nThe queue lived in `src/queue.rs`.\n",
    );
    write(r, "docs/adr/README.md", "# Decisions\n");
    commit(r, "initial with decisions", 1);
    dir
}

#[test]
fn lists_decisions_from_adr_files_and_vault_notes() {
    let dir = fixture();
    let index = sync(dir.path());
    let created = decisions::create(dir.path(), "Store values durably").unwrap();
    assert_eq!(created, "07-Decisions/ADR-0001 Store values durably.md");
    assert_eq!(
        decisions::create(dir.path(), "Second one").unwrap(),
        "07-Decisions/ADR-0002 Second one.md"
    );

    let all = decisions::list(dir.path(), &index).unwrap();
    let paths: Vec<&str> = all.iter().map(|d| d.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "docs/adr/0001-cache-in-memory.md",
            "docs/adr/0002-old-queue.md",
            "obsidian/07-Decisions/ADR-0001 Store values durably.md",
            "obsidian/07-Decisions/ADR-0002 Second one.md",
        ]
    );
    let cache = &all[0];
    assert_eq!(cache.title, "1. Keep the cache in memory");
    assert_eq!(cache.status, "accepted");
    assert_eq!(cache.cites, ["src/cache.rs"]);
    assert!(!cache.stale, "{cache:?}");

    let queue = &all[1];
    assert_eq!(queue.status, "superseded");
    assert_eq!(queue.missing, ["src/queue.rs"]);
    assert!(!queue.stale, "a superseded decision is history, not drift");
    assert_eq!(all[2].status, "proposed");
}

#[test]
fn a_decision_goes_stale_when_its_code_changes_later() {
    let dir = fixture();
    write(
        dir.path(),
        "src/cache.rs",
        "//! Cache, now backed by disk.\n\n/// Look up a key.\npub fn lookup(key: u64) -> u64 { key + 1 }\n",
    );
    commit(dir.path(), "cache: read through to disk", 5);
    let index = sync(dir.path());
    let all = decisions::list(dir.path(), &index).unwrap();
    let cache = &all[0];
    assert!(cache.stale);
    assert_eq!(
        cache.stale_reasons,
        ["`src/cache.rs` changed after this decision was last updated"]
    );

    // Updating the record after the code settles the question.
    let adr = fs::read_to_string(dir.path().join("docs/adr/0001-cache-in-memory.md")).unwrap();
    write(
        dir.path(),
        "docs/adr/0001-cache-in-memory.md",
        &format!("{adr}\nReviewed after the disk change.\n"),
    );
    commit(dir.path(), "adr: review", 9);
    let index = sync(dir.path());
    assert!(!decisions::list(dir.path(), &index).unwrap()[0].stale);
}

#[test]
fn why_gathers_decisions_rationale_and_commits() {
    let dir = fixture();
    write(
        dir.path(),
        "src/cache.rs",
        "//! In-memory cache.\n\n/// Look up a key.\npub fn lookup(key: u64) -> u64 { key * 2 }\n",
    );
    commit(dir.path(), "cache: double the key", 3);
    let index = sync(dir.path());
    // A hand-written gotcha on the generated note.
    let note = dir.path().join("obsidian/01-Core/demo/Cache Module.md");
    let text = fs::read_to_string(&note).unwrap();
    fs::write(
        &note,
        format!("{text}\n## Gotchas\n\n- **Never** block inside `lookup`.\n"),
    )
    .unwrap();

    let by_file = decisions::why(dir.path(), &index, "src/cache.rs").unwrap();
    assert_eq!(by_file.file, "src/cache.rs");
    assert_eq!(by_file.decisions.len(), 1);
    assert_eq!(
        by_file.decisions[0].path,
        "docs/adr/0001-cache-in-memory.md"
    );
    assert!(
        by_file
            .rationale
            .iter()
            .any(|r| r.section == "Gotchas" && r.text.contains("**Never** block inside `lookup`"))
    );
    let subjects: Vec<&str> = by_file.commits.iter().map(|c| c.subject.as_str()).collect();
    assert_eq!(
        subjects,
        ["cache: double the key", "initial with decisions"]
    );
    assert_eq!(by_file.commits[0].date, "2026-01-03");
    assert_eq!(by_file.commits[0].author, "Ann");

    // A symbol name resolves to its file.
    let by_symbol = decisions::why(dir.path(), &index, "lookup").unwrap();
    assert_eq!(by_symbol.file, "src/cache.rs");
    assert_eq!(by_symbol.decisions.len(), 1);

    let store = decisions::why(dir.path(), &index, "src/store.rs").unwrap();
    assert!(store.decisions.is_empty() && store.rationale.is_empty());
    assert!(decisions::why(dir.path(), &index, "no_such_thing").is_err());
}
