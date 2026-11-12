//! The briefing written into CLAUDE.md and AGENTS.md.

use std::fs;
use std::path::Path;

use cw_index::Index;
use cw_wiki::{agentdocs, analyze, generate, service};

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn fixture() -> (tempfile::TempDir, Index) {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
    write(
        r,
        "crates/engine/Cargo.toml",
        "[package]\nname = \"engine\"\ndescription = \"Order matching | engine\"\n",
    );
    write(
        r,
        "crates/engine/src/lib.rs",
        "//! Engine.\npub fn run() {}\n",
    );
    write(
        r,
        "crates/cli/Cargo.toml",
        "[package]\nname = \"cli\"\n[dependencies]\nengine = { path = \"../engine\" }\n",
    );
    write(r, "crates/cli/src/main.rs", "fn main() { engine::run() }\n");
    let mut index = Index::open_in_memory().unwrap();
    let facts = service::reindex(r, &mut index).unwrap();
    generate(r, &facts, &analyze(&facts), None, None).unwrap();
    (dir, index)
}

#[test]
fn briefing_routes_agents_to_the_wiki() {
    let (dir, index) = fixture();
    let briefing = agentdocs::render(dir.path(), &index).unwrap();
    assert!(briefing.starts_with("## Codebase wiki\n"));
    assert!(briefing.contains("Read `obsidian/_HOME.md` before exploring code"));
    assert!(briefing.contains("| `obsidian/00-Architecture/Crate Dependency Flow.md` |"));
    // Crate notes are routed; a pipe in a summary cannot break the table.
    assert!(
        briefing.contains("| `obsidian/01-Core/engine/engine.md` | Order matching / engine. |"),
        "{briefing}"
    );
    assert!(!briefing.contains("_index.md"), "map notes are not routes");
    assert!(briefing.lines().count() < 40, "the briefing stays short");
}

#[test]
fn apply_writes_both_files_and_preserves_the_authors_text() {
    let (dir, index) = fixture();
    write(
        dir.path(),
        "CLAUDE.md",
        "# My rules\n\nNever push to main.\n",
    );

    let changed = agentdocs::apply(dir.path(), &index).unwrap();
    assert_eq!(changed, ["CLAUDE.md", "AGENTS.md"]);
    let claude = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert!(claude.starts_with("# My rules\n\nNever push to main.\n\n<!-- codewiki:start -->"));
    assert!(claude.trim_end().ends_with("<!-- codewiki:end -->"));
    let agents = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(agents.starts_with("<!-- codewiki:start -->\n## Codebase wiki"));

    // A second run with unchanged facts touches nothing.
    assert!(agentdocs::apply(dir.path(), &index).unwrap().is_empty());

    // Text added by hand around the block survives the next refresh.
    write(
        dir.path(),
        "CLAUDE.md",
        &format!("{claude}\n## Later notes\n\nMine.\n"),
    );
    write(
        dir.path(),
        "crates/engine/Cargo.toml",
        "[package]\nname = \"engine\"\ndescription = \"Matching engine, second edition\"\n",
    );
    let mut index = Index::open_in_memory().unwrap();
    let facts = service::reindex(dir.path(), &mut index).unwrap();
    generate(dir.path(), &facts, &analyze(&facts), None, None).unwrap();
    assert_eq!(
        agentdocs::apply(dir.path(), &index).unwrap(),
        ["CLAUDE.md", "AGENTS.md"]
    );
    let refreshed = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert!(refreshed.starts_with("# My rules\n\nNever push to main.\n"));
    assert!(refreshed.contains("second edition"));
    assert!(refreshed.trim_end().ends_with("## Later notes\n\nMine."));
}
