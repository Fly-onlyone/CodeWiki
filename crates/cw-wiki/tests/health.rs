//! Dead code, documentation drift and file health on a fixture repository.

use std::fs;
use std::path::Path;

use cw_index::Index;
use cw_wiki::health::{self, Certainty, DriftKind};
use cw_wiki::{analyze, generate, service};

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
    write(r, "crates/lib/Cargo.toml", "[package]\nname = \"shapes\"\n");
    write(
        r,
        "crates/lib/src/lib.rs",
        "//! Shapes.\n\n/// Something with an area.\npub trait Shape {\n    /// Area in square units.\n    fn area(&self) -> f64;\n}\n\n/// A square.\npub struct Square(pub f64);\n\nimpl Shape for Square {\n    fn area(&self) -> f64 { side_squared(self.0) }\n}\n\nfn side_squared(side: f64) -> f64 { side * side }\n\nfn forgotten_helper() -> u8 { 7 }\n\n/// Public API nobody in the repo calls yet.\npub fn exported_but_unused() {}\n\n#[no_mangle]\nfn registered_elsewhere() {}\n\nconst UNUSED_LIMIT: u8 = 3;\n\n#[cfg(test)]\nmod tests {\n    fn test_only_helper() {}\n}\n",
    );
    write(
        r,
        "crates/app/Cargo.toml",
        "[package]\nname = \"app\"\n[dependencies]\nshapes = { path = \"../lib\" }\n",
    );
    write(
        r,
        "crates/app/src/main.rs",
        "use shapes::{Shape, Square};\n\nfn main() {\n    let s = Square(2.0);\n    let _ = s.area();\n}\n\npub fn orphan_in_binary() {}\n",
    );
    dir
}

fn indexed(root: &Path) -> Index {
    let mut index = Index::open_in_memory().unwrap();
    service::reindex(root, &mut index).unwrap();
    index
}

#[test]
fn finds_dead_code_without_false_alarms() {
    let dir = fixture();
    let index = indexed(dir.path());
    let dead = health::dead_code(&index).unwrap();
    let find = |name: &str| dead.iter().find(|d| d.qualified.ends_with(name));

    let helper = find("::forgotten_helper").expect("private unreferenced function");
    assert_eq!(helper.certainty, Certainty::Likely);
    assert_eq!(helper.file, "crates/lib/src/lib.rs");
    assert_eq!(find("::UNUSED_LIMIT").unwrap().certainty, Certainty::Likely);

    // An attribute may register the item somewhere the index cannot see.
    let registered = find("::registered_elsewhere").unwrap();
    assert_eq!(registered.certainty, Certainty::Possible);
    assert!(
        registered.reason.contains("#[no_mangle]"),
        "{}",
        registered.reason
    );
    // Public in a binary crate: nothing outside can call it.
    assert_eq!(
        find("::orphan_in_binary").unwrap().certainty,
        Certainty::Possible
    );

    for alive in [
        "::side_squared",        // called
        "::Square",              // referenced
        "::Shape",               // implemented
        "::area",                // trait item and trait-impl method
        "::main",                // entry point
        "::exported_but_unused", // public API of a library
        "::test_only_helper",    // test code is not indexed
    ] {
        assert!(find(alive).is_none(), "{alive} was reported dead: {dead:?}");
    }
    assert_eq!(
        dead[0].certainty,
        Certainty::Likely,
        "likely findings come first"
    );
}

#[test]
fn reports_documentation_drift_and_coverage() {
    let dir = fixture();
    let facts = cw_parse::scan_repo(dir.path()).unwrap();
    generate(dir.path(), &facts, &analyze(&facts), None, None).unwrap();
    let vault = dir.path().join("obsidian");

    let clean = health::doc_drift(dir.path(), &indexed(dir.path())).unwrap();
    assert!(clean.drift.is_empty(), "{:?}", clean.drift);
    assert_eq!(clean.coverage, 1.0);
    assert_eq!(clean.source_files, 2);

    // A hand-written note with a dead path and a broken link, then the source moves on.
    write(
        &vault,
        "04-Patterns/Area Pattern.md",
        "---\ntags: [pattern]\n---\n\n# Area Pattern\n\n> Mine.\n\n## Source\n\n- `crates/lib/src/lib.rs` — the trait\n- `crates/lib/src/gone.rs` — removed long ago\n\n## See also\n\n- [[_index]]\n- [[No Such Note]]\n",
    );
    let facts = cw_parse::scan_repo(dir.path()).unwrap();
    generate(dir.path(), &facts, &analyze(&facts), None, None).unwrap();
    let lib = fs::read_to_string(dir.path().join("crates/lib/src/lib.rs")).unwrap();
    write(
        dir.path(),
        "crates/lib/src/lib.rs",
        &lib.replace("A square.", "A square with a side."),
    );
    write(dir.path(), "crates/lib/src/extra.rs", "pub fn extra() {}\n");

    let report = health::doc_drift(dir.path(), &indexed(dir.path())).unwrap();
    let kinds: Vec<(DriftKind, &str)> = report
        .drift
        .iter()
        .filter(|d| d.note == "04-Patterns/Area Pattern.md")
        .map(|d| (d.kind, d.detail.as_str()))
        .collect();
    assert!(
        kinds
            .iter()
            .any(|(k, d)| *k == DriftKind::DeadPath && d.contains("gone.rs")),
        "{kinds:?}"
    );
    assert!(
        kinds
            .iter()
            .any(|(k, d)| *k == DriftKind::BrokenLink && d.contains("No Such Note")),
        "{kinds:?}"
    );
    assert!(
        kinds.iter().any(|(k, _)| *k == DriftKind::SourceChanged),
        "{kinds:?}"
    );
    assert_eq!(report.undocumented, ["crates/lib/src/extra.rs"]);
    assert!(report.coverage < 1.0);
}

#[test]
fn scores_file_health_with_reasons() {
    let dir = fixture();
    let mut big = String::from("//! Big.\n");
    big.push_str("pub fn sprawling() {\n");
    for i in 0..200 {
        big.push_str(&format!("    let _v{i} = {i};\n"));
    }
    big.push_str("}\n");
    for i in 0..6 {
        big.push_str(&format!("pub fn undocumented_{i}() {{}}\n"));
    }
    write(dir.path(), "crates/lib/src/big.rs", &big);
    let lib = fs::read_to_string(dir.path().join("crates/lib/src/lib.rs")).unwrap();
    write(
        dir.path(),
        "crates/lib/src/lib.rs",
        &format!("pub mod big;\n{lib}"),
    );

    let health = health::file_health(&indexed(dir.path())).unwrap();
    assert_eq!(health[0].path, "crates/lib/src/big.rs", "worst file first");
    let detectors: Vec<&str> = health[0]
        .findings
        .iter()
        .map(|f| f.detector.as_str())
        .collect();
    assert!(
        detectors.contains(&"long-function") && detectors.contains(&"undocumented-api"),
        "{detectors:?}"
    );
    assert!(
        health[0]
            .findings
            .iter()
            .any(|f| f.message.contains("`sprawling` is 202 lines"))
    );
    assert_eq!(health[0].score, 7);

    let lib = health
        .iter()
        .find(|h| h.path == "crates/lib/src/lib.rs")
        .unwrap();
    assert!(
        lib.findings.iter().any(|f| f.detector == "dead-code"),
        "{lib:?}"
    );
    let app = health
        .iter()
        .find(|h| h.path == "crates/app/src/main.rs")
        .unwrap();
    assert_eq!((app.score, app.findings.len()), (10, 0));
    assert!(health.iter().all(|h| (1..=10).contains(&h.score)));
}
