//! Dead code, documentation drift and file health on a fixture repository.

mod support;

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
        "//! Shapes.\n//!\n//! A user may list more in `crates/lib/shapes.toml`.\n\n/// Something with an area.\npub trait Shape {\n    /// Area in square units.\n    fn area(&self) -> f64;\n}\n\n/// A square.\npub struct Square(pub f64);\n\nimpl Shape for Square {\n    fn area(&self) -> f64 { side_squared(self.0) }\n}\n\nfn side_squared(side: f64) -> f64 { side * side }\n\nfn forgotten_helper() -> u8 { 7 }\n\n/// Public API nobody in the repo calls yet.\npub fn exported_but_unused() {}\n\n#[no_mangle]\nfn registered_elsewhere() {}\n\nconst UNUSED_LIMIT: u8 = 3;\n\n#[cfg(test)]\nmod tests {\n    fn test_only_helper() {}\n}\n",
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
    support::no_model_server();
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

/// Private items used in ways a name-based resolver can miss: methods on a
/// receiver of unknown type whose name other methods share, a method called
/// inside a macro, a constant read only by another constant.
fn receivers_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
    write(r, "crates/kit/Cargo.toml", "[package]\nname = \"kit\"\n");
    write(
        r,
        "crates/kit/src/lib.rs",
        "//! Kit.\npub mod consts;\npub mod logs;\npub mod page;\npub mod tray;\npub mod units;\n",
    );
    write(
        r,
        "crates/kit/src/page.rs",
        "//! Pages.\nstruct Page(u8);\n\nimpl Page {\n    fn id(&self) -> u8 { self.0 }\n    fn really_unused(&self) -> u8 { 0 }\n}\n\n/// The shown page.\npub fn shown() -> u8 {\n    let page = Page(1);\n    page.id()\n}\n",
    );
    write(
        r,
        "crates/kit/src/tray.rs",
        "//! Tray.\n/// A command.\npub struct Command(u8);\n\nimpl Command {\n    /// Its id.\n    pub fn id(&self) -> u8 { self.0 }\n}\n",
    );
    write(
        r,
        "crates/kit/src/logs.rs",
        "//! Logs.\nuse std::sync::atomic::{AtomicU8, Ordering};\n\nstruct Level(AtomicU8);\n\nimpl Level {\n    fn get(&self) -> u8 { self.0.load(Ordering::Relaxed) }\n}\n\n/// A logger.\npub struct Logger {\n    level: Level,\n}\n\nimpl Logger {\n    /// Whether it writes anything.\n    pub fn enabled(&self) -> bool { self.level.get() > 0 }\n}\n",
    );
    write(
        r,
        "crates/kit/src/units.rs",
        "//! Units.\nstruct Family;\n\nimpl Family {\n    fn suffix(&self) -> &'static str { \"s\" }\n}\n\n/// A unit's name.\npub fn name(family: Family) -> String {\n    format!(\"unit{}\", family.suffix())\n}\n",
    );
    write(
        r,
        "crates/kit/src/consts.rs",
        "//! Sizes.\nconst THUMB_PADDING: f32 = 2.0;\n\n/// The halo around a thumb.\npub const HALO: f32 = 20.0 + 2.0 * THUMB_PADDING;\n\nconst STRENGTH: f32 = 0.2;\nconst _: () = assert!(STRENGTH <= 0.3);\n",
    );
    dir
}

#[test]
fn private_items_used_through_receivers_and_constants_are_alive() {
    let dir = receivers_fixture();
    let index = indexed(dir.path());
    let dead = health::dead_code(&index).unwrap();
    let find = |name: &str| dead.iter().find(|d| d.qualified.ends_with(name));

    for alive in [
        "::page::Page::id",        // `page.id()`, though `tray::Command::id` exists
        "::logs::Level::get",      // `self.level.get()`, a common method name
        "::units::Family::suffix", // `family.suffix()` inside `format!`
        "::consts::THUMB_PADDING", // read by `HALO`
        "::consts::STRENGTH",      // read by the anonymous assertion
    ] {
        assert!(find(alive).is_none(), "{alive} was reported dead: {dead:?}");
    }
    assert!(
        !dead.iter().any(|d| d.qualified.ends_with("::_")),
        "the anonymous constant is not an item: {dead:?}"
    );
    assert_eq!(
        find("::page::Page::really_unused").map(|d| d.certainty),
        Some(Certainty::Likely),
        "{dead:?}"
    );
}

#[test]
fn a_cycle_is_two_sibling_files_not_a_module_and_its_parent() {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
    write(r, "crates/srv/Cargo.toml", "[package]\nname = \"srv\"\n");
    // `lib.rs` calls into `api.rs`, which uses `lib.rs`'s `State`.
    write(
        r,
        "crates/srv/src/lib.rs",
        "//! Server.\npub mod a;\npub mod api;\npub mod b;\npub mod net;\n\n/// Shared state.\npub struct State;\n\n/// Serve.\npub fn serve() -> State { api::route() }\n",
    );
    write(
        r,
        "crates/srv/src/api.rs",
        "//! Routes.\nuse crate::State;\n\n/// The one route.\npub fn route() -> State { State }\n",
    );
    // Two siblings that call each other.
    write(
        r,
        "crates/srv/src/a.rs",
        "//! A.\n/// Ping.\npub fn ping(n: u8) -> u8 { if n == 0 { 0 } else { crate::b::pong(n - 1) } }\n",
    );
    write(
        r,
        "crates/srv/src/b.rs",
        "//! B.\n/// Pong.\npub fn pong(n: u8) -> u8 { if n == 0 { 0 } else { crate::a::ping(n - 1) } }\n",
    );
    // The library calls `.header(..)` on a type from elsewhere; the test's
    // private `Received::header` is not that method.
    write(
        r,
        "crates/srv/src/net.rs",
        "//! Network.\n/// Send a request.\npub fn send(request: http::Request<()>) -> bool {\n    request.headers().contains_key(\"a\")\n}\n\n/// Build a request.\npub fn build() -> http::request::Builder {\n    http::Request::builder().header(\"a\", \"b\")\n}\n",
    );
    write(
        r,
        "crates/srv/tests/wire.rs",
        "struct Received;\n\nimpl Received {\n    fn header(&self) -> u8 { 0 }\n}\n\n#[test]\nfn sends() {\n    let _ = srv::net::build();\n    assert_eq!(Received.header(), 0);\n}\n",
    );

    let health = health::file_health(&indexed(r)).unwrap();
    let mut cyclic: Vec<&str> = health
        .iter()
        .filter(|h| h.findings.iter().any(|f| f.detector == "mutual-dependency"))
        .map(|h| h.path.as_str())
        .collect();
    cyclic.sort_unstable();
    assert_eq!(
        cyclic,
        ["crates/srv/src/a.rs", "crates/srv/src/b.rs"],
        "{health:?}"
    );
}

#[test]
fn reports_documentation_drift_and_coverage() {
    let dir = fixture();
    let facts = cw_parse::scan_repo(dir.path()).unwrap();
    generate(dir.path(), &facts, &analyze(&facts), None).unwrap();
    let vault = dir.path().join("obsidian");

    let clean = health::doc_drift(dir.path(), &indexed(dir.path())).unwrap();
    assert!(clean.drift.is_empty(), "{:?}", clean.drift);
    assert_eq!(clean.coverage, 1.0);
    assert_eq!(clean.source_files, 2);
    // The crate's comment names a file a user may add, and its note says
    // what the comment says. Nobody edited the note: that is no drift.
    let crate_note = fs::read_to_string(vault.join("01-Core/shapes/shapes.md")).unwrap();
    assert!(
        crate_note.contains("`crates/lib/shapes.toml`"),
        "{crate_note}"
    );

    // A hand-written note with a dead path and a broken link, then the source moves on.
    write(
        &vault,
        "04-Patterns/Area Pattern.md",
        "---\ntags: [pattern]\n---\n\n# Area Pattern\n\n> Mine.\n\n## Source\n\n- `crates/lib/src/lib.rs` — the trait\n- `crates/lib/src/gone.rs` — removed long ago\n- `styles/createTransitions.js` — another project's file\n\n## See also\n\n- [[_index]]\n- [[No Such Note]]\n",
    );
    let facts = cw_parse::scan_repo(dir.path()).unwrap();
    generate(dir.path(), &facts, &analyze(&facts), None).unwrap();
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
    // A path that starts at no folder of the repository is not one of ours.
    assert!(
        !kinds.iter().any(|(_, d)| d.contains("createTransitions")),
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
