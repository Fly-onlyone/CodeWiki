//! Refactoring plan on a fixture repository with one of each problem.

use std::fs;
use std::path::Path;

use cw_index::Index;
use cw_wiki::refactor::{self, Kind, Suggestion};
use cw_wiki::service;

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A long file holding a type with many methods and a second sizeable type.
fn big_file() -> String {
    let mut text = String::from("//! Big.\n\npub struct Engine;\n\nimpl Engine {\n");
    let mut method = |name: &str| {
        text.push_str(&format!("    pub fn {name}(&self) -> u32 {{\n"));
        for i in 0..18 {
            text.push_str(&format!("        let _v{i} = {i};\n"));
        }
        text.push_str("        0\n    }\n\n");
    };
    for name in [
        "render_home",
        "render_note",
        "render_index",
        "render_crate",
        "render_flow",
    ] {
        method(name);
    }
    for name in ["parse_a", "parse_b", "parse_c", "parse_d"] {
        method(name);
    }
    for i in 0..17 {
        method(&format!("step{i}"));
    }
    text.push_str("}\n\npub struct Ledger;\n\nimpl Ledger {\n");
    let mut method = |name: &str| {
        text.push_str(&format!("    pub fn {name}(&self) -> u32 {{\n"));
        for i in 0..40 {
            text.push_str(&format!("        let _v{i} = {i};\n"));
        }
        text.push_str("        0\n    }\n\n");
    };
    method("post");
    method("settle");
    text.push_str("}\n");
    text
}

fn fixture() -> (tempfile::TempDir, Index) {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Cargo.toml", "[package]\nname = \"demo\"\n");
    write(
        r,
        "src/lib.rs",
        "pub mod big;\npub mod geo;\npub mod misc;\npub mod orders;\npub mod stock;\npub mod user;\n",
    );
    write(r, "src/big.rs", &big_file());
    write(
        r,
        "src/user.rs",
        "use crate::big::Engine;\n\npub fn drive(e: &Engine) -> u32 { e.render_home() }\n",
    );
    // orders -> stock once, stock -> orders twice.
    write(
        r,
        "src/orders.rs",
        "use crate::stock::reserve;\n\npub fn place() -> u32 { reserve() }\n\npub fn total() -> u32 { 1 }\n\npub fn count() -> u32 { total() }\n",
    );
    write(
        r,
        "src/stock.rs",
        "use crate::orders::{count, total};\n\npub fn reserve() -> u32 { level() }\n\npub fn level() -> u32 { total() + count() }\n",
    );
    write(
        r,
        "src/geo.rs",
        "pub fn north() -> u32 { 1 }\n\npub fn south() -> u32 { 2 }\n\npub fn east() -> u32 { north() + south() }\n",
    );
    // `span` works only with `geo.rs`; `local` and `uses_local` belong here.
    write(
        r,
        "src/misc.rs",
        "use crate::geo::{east, north, south};\n\npub fn span() -> u32 { north() + south() + east() }\n\npub fn local() -> u32 { 3 }\n\npub fn uses_local() -> u32 { local() + north() }\n",
    );
    let mut index = Index::open_in_memory().unwrap();
    service::reindex(r, &mut index).unwrap();
    (dir, index)
}

fn of(plan: &[Suggestion], kind: Kind) -> Vec<&Suggestion> {
    plan.iter().filter(|s| s.kind == kind).collect()
}

#[test]
fn suggests_one_refactoring_per_problem_with_evidence_and_steps() {
    let (_dir, index) = fixture();
    let plan = refactor::plan(&index).unwrap();
    let all = |needle: &str, lines: &[String]| lines.iter().any(|l| l.contains(needle));

    let split = of(&plan, Kind::SplitFile);
    assert_eq!(split.len(), 1, "{plan:#?}");
    assert_eq!(split[0].target, "src/big.rs");
    assert!(all("lines", &split[0].evidence), "{:?}", split[0].evidence);
    assert!(all("1 other files depend on it", &split[0].evidence));
    // The largest type stays; the next one moves.
    assert!(
        all("Move `Ledger` with its 2 methods", &split[0].steps)
            && all("`src/ledger.rs`", &split[0].steps),
        "{:?}",
        split[0].steps
    );
    assert!(all("pub use", &split[0].steps));
    assert!(split[0].steps[0].contains("get_impacted_tests"));
    assert!(
        split[0]
            .steps
            .last()
            .unwrap()
            .contains("cargo test -p demo")
    );

    let extract = of(&plan, Kind::ExtractType);
    assert_eq!(extract.len(), 1, "{plan:#?}");
    assert_eq!(extract[0].target, "demo::big::Engine");
    assert!(all("`Engine` has 26 methods", &extract[0].evidence));
    assert!(all("5 methods start with `render_`", &extract[0].evidence));
    assert!(all("4 methods start with `parse_`", &extract[0].evidence));
    assert!(all("Create `EngineRender`", &extract[0].steps));
    assert!(all("`parse_*` group", &extract[0].steps));

    let cycle = of(&plan, Kind::BreakCycle);
    assert_eq!(cycle.len(), 1, "{plan:#?}");
    assert_eq!(cycle[0].target, "src/orders.rs, src/stock.rs");
    assert_eq!(cycle[0].file, "src/orders.rs");
    assert!(
        all(
            "weakest link: `src/orders.rs` uses `src/stock.rs` through 1 references to `demo::stock::reserve`",
            &cycle[0].evidence
        ),
        "{:?}",
        cycle[0].evidence
    );
    assert!(cycle[0].steps[0].contains("from `demo::orders::place`"));

    let moves = of(&plan, Kind::MoveFunction);
    assert_eq!(moves.len(), 1, "{plan:#?}");
    assert_eq!(moves[0].target, "demo::misc::span");
    assert_eq!(moves[0].title, "Move `span` to `src/geo.rs`");
    assert!(all("3 links to `src/geo.rs`", &moves[0].evidence));
    assert!(all("make the function private", &moves[0].steps));

    assert!(plan.windows(2).all(|w| w[0].priority >= w[1].priority));
    assert!(
        plan.iter()
            .all(|s| s.priority <= 100 && !s.evidence.is_empty() && s.steps.len() >= 3)
    );
    assert_eq!(plan, refactor::plan(&index).unwrap(), "deterministic");
}

#[test]
fn a_healthy_repository_gets_no_suggestions() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "Cargo.toml", "[package]\nname = \"demo\"\n");
    write(
        dir.path(),
        "src/lib.rs",
        "pub mod math;\n\npub fn run() -> u64 { math::double(21) }\n",
    );
    write(
        dir.path(),
        "src/math.rs",
        "pub fn double(x: u64) -> u64 { x * 2 }\n",
    );
    let mut index = Index::open_in_memory().unwrap();
    service::reindex(dir.path(), &mut index).unwrap();
    assert!(refactor::plan(&index).unwrap().is_empty());
}
