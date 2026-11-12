//! Test map and impacted tests on a fixture workspace.

use std::fs;
use std::path::Path;

use cw_index::Index;
use cw_wiki::service;
use cw_wiki::testmap::{self, TestKind};

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// `util.rs` (unit tests) <- `area.rs` <- `tests/area.rs` and `app/main.rs`
/// (unit test); `lonely.rs` is used and tested by nothing.
fn fixture() -> (tempfile::TempDir, Index) {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
    write(r, "crates/lib/Cargo.toml", "[package]\nname = \"shapes\"\n");
    write(
        r,
        "crates/lib/src/lib.rs",
        "pub mod area;\npub mod lonely;\npub mod util;\n",
    );
    write(
        r,
        "crates/lib/src/util.rs",
        "pub fn square(x: f64) -> f64 { x * x }\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn squares() {\n        assert_eq!(square(3.0), 9.0);\n    }\n}\n",
    );
    write(
        r,
        "crates/lib/src/area.rs",
        "use crate::util::square;\n\npub fn of_square(side: f64) -> f64 { square(side) }\n",
    );
    write(r, "crates/lib/src/lonely.rs", "pub fn nobody() {}\n");
    write(
        r,
        "crates/lib/tests/area.rs",
        "use shapes::area::of_square;\n\n#[test]\nfn square_area() {\n    assert_eq!(of_square(2.0), 4.0);\n}\n\n#[test]\nfn zero_side() {\n    assert_eq!(of_square(0.0), 0.0);\n}\n\nfn helper() {}\n",
    );
    write(
        r,
        "crates/app/Cargo.toml",
        "[package]\nname = \"app\"\n[dependencies]\nshapes = { path = \"../lib\" }\n",
    );
    write(
        r,
        "crates/app/src/main.rs",
        "use shapes::area::of_square;\n\nfn main() {\n    let _ = of_square(1.0);\n}\n\n#[cfg(test)]\nmod tests {\n    #[tokio::test]\n    async fn runs() {}\n}\n",
    );
    let mut index = Index::open_in_memory().unwrap();
    service::reindex(r, &mut index).unwrap();
    (dir, index)
}

#[test]
fn maps_tests_to_the_files_they_reach() {
    let (dir, index) = fixture();
    let map = testmap::test_map(dir.path(), &index).unwrap();

    assert_eq!(map.tests_total, 4);
    let found: Vec<(&str, TestKind, Vec<&str>)> = map
        .test_files
        .iter()
        .map(|t| {
            (
                t.path.as_str(),
                t.kind,
                t.tests.iter().map(|c| c.name.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            ("crates/app/src/main.rs", TestKind::Unit, vec!["runs"]),
            ("crates/lib/src/util.rs", TestKind::Unit, vec!["squares"]),
            (
                "crates/lib/tests/area.rs",
                TestKind::Integration,
                vec!["square_area", "zero_side"]
            ),
        ]
    );
    assert_eq!(map.test_files[1].tests[0].line, 8);
    assert_eq!(
        map.test_files[2].command,
        "cargo test -p shapes --test area"
    );

    let file = |path: &str| map.files.iter().find(|f| f.path == path).unwrap();
    let area = file("crates/lib/src/area.rs");
    assert_eq!(area.unit_tests, 0);
    assert_eq!(
        area.direct,
        ["crates/app/src/main.rs", "crates/lib/tests/area.rs"]
    );
    let util = file("crates/lib/src/util.rs");
    assert_eq!(util.unit_tests, 1);
    assert!(util.direct.is_empty(), "{util:?}");
    assert_eq!(
        util.indirect,
        ["crates/app/src/main.rs", "crates/lib/tests/area.rs"]
    );
    // Test files are not listed as code to be tested.
    assert!(map.files.iter().all(|f| !f.path.contains("/tests/")));
    // `lib.rs` only declares modules, so it is not reported.
    assert_eq!(map.untested, ["crates/lib/src/lonely.rs"]);
}

#[test]
fn lists_the_tests_a_change_can_break() {
    let (dir, index) = fixture();
    let changed = ["crates\\lib\\src\\util.rs".to_string()];
    let impact = testmap::impacted_tests(dir.path(), &index, &changed).unwrap();

    assert_eq!(impact.changed, ["crates/lib/src/util.rs"]);
    let rows: Vec<(&str, u32)> = impact
        .test_files
        .iter()
        .map(|t| (t.path.as_str(), t.distance))
        .collect();
    assert_eq!(
        rows,
        [
            ("crates/lib/src/util.rs", 0),
            ("crates/app/src/main.rs", 2),
            ("crates/lib/tests/area.rs", 2),
        ],
        "nearest first"
    );
    assert_eq!(impact.tests_total, 4);
    assert_eq!(
        impact.commands,
        [
            "cargo test -p shapes --lib util::",
            "cargo test -p app --bins",
            "cargo test -p shapes --test area",
        ]
    );
    assert_eq!(impact.test_files[2].tests, ["square_area", "zero_side"]);
    assert_eq!(impact.test_files[2].because, ["crates/lib/src/util.rs"]);
    assert!(impact.untested_changes.is_empty());

    // A change nothing depends on, a changed test file, and a non-source file.
    let changed = [
        "crates/lib/src/lonely.rs".to_string(),
        "crates/lib/tests/area.rs".to_string(),
        "README.md".to_string(),
    ];
    let impact = testmap::impacted_tests(dir.path(), &index, &changed).unwrap();
    assert_eq!(impact.test_files.len(), 1);
    assert_eq!(impact.test_files[0].path, "crates/lib/tests/area.rs");
    assert_eq!(impact.test_files[0].distance, 0);
    assert_eq!(impact.untested_changes, ["crates/lib/src/lonely.rs"]);

    let none = testmap::impacted_tests(dir.path(), &index, &[]).unwrap();
    assert!(none.test_files.is_empty() && none.commands.is_empty());
}
