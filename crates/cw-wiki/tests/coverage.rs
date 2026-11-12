//! Coverage import and gap ranking on a fixture repository.

use std::fs;
use std::path::Path;

use cw_core::Error;
use cw_index::Index;
use cw_wiki::coverage::{self, Format};
use cw_wiki::service;

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// `core.rs` is used by `api.rs` and `cli.rs`; `leaf.rs` by nothing.
fn fixture() -> (tempfile::TempDir, Index) {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Cargo.toml", "[package]\nname = \"demo\"\n");
    write(
        r,
        "src/lib.rs",
        "pub mod api;\npub mod cli;\npub mod core;\npub mod leaf;\npub mod unseen;\n",
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
        "use crate::core::compute;\npub fn run() -> u64 { compute(2) }\n",
    );
    write(r, "src/leaf.rs", "pub fn alone() -> u64 { 7 }\n");
    write(r, "src/unseen.rs", "pub fn never_built() {}\n");
    let mut index = Index::open_in_memory().unwrap();
    service::reindex(r, &mut index).unwrap();
    (dir, index)
}

/// `core.rs` and `leaf.rs` both have 2 of 10 lines covered; `api.rs` is full.
fn lcov(root: &Path) -> String {
    let abs = |rel: &str| root.join(rel).to_string_lossy().to_string();
    let mut text = String::new();
    for file in ["src/core.rs", "src/leaf.rs"] {
        text.push_str(&format!("TN:\nSF:{}\n", abs(file)));
        for line in 1..=10 {
            text.push_str(&format!("DA:{line},{}\n", if line <= 2 { 5 } else { 0 }));
        }
        text.push_str("LF:10\nLH:2\nend_of_record\n");
    }
    text.push_str("SF:src/api.rs\nDA:1,1\nDA:2,4\nend_of_record\n");
    // A dependency outside the repository.
    text.push_str("SF:/home/u/.cargo/registry/src/serde/de.rs\nDA:1,0\nend_of_record\n");
    text
}

#[test]
fn imports_lcov_and_ranks_gaps_by_what_depends_on_them() {
    let (dir, index) = fixture();
    write(dir.path(), "coverage/lcov.info", &lcov(dir.path()));

    // No path given: the report is found in a conventional place.
    let data = coverage::import(dir.path(), &index, None).unwrap();
    assert_eq!(data.format, Format::Lcov);
    assert_eq!(data.source, "coverage/lcov.info");
    assert_eq!(data.unmatched, 1);
    let paths: Vec<&str> = data.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["src/api.rs", "src/core.rs", "src/leaf.rs"]);
    let core = &data.files[1];
    assert_eq!(
        (core.lines_found, core.lines_hit, core.percent),
        (10, 2, 20.0)
    );
    assert_eq!(core.uncovered, [[3, 10]]);

    coverage::save(&index, &data).unwrap();
    assert_eq!(coverage::load(&index).unwrap().as_ref(), Some(&data));

    let report = coverage::report(&index, &data, 10).unwrap();
    assert_eq!((report.lines_found, report.lines_hit), (22, 6));
    assert_eq!(report.percent, 27.3);
    assert_eq!(report.files[0].percent, 20.0, "least covered first");
    assert_eq!(report.files[2].path, "src/api.rs");
    assert_eq!(report.not_in_report, ["src/cli.rs", "src/unseen.rs"]);

    // Equal coverage: the file more code depends on comes first.
    let gaps: Vec<&str> = report
        .least_covered
        .iter()
        .map(|g| g.path.as_str())
        .collect();
    assert_eq!(gaps, ["src/core.rs", "src/leaf.rs"], "{report:?}");
    let (core, leaf) = (&report.least_covered[0], &report.least_covered[1]);
    assert!(core.rank > leaf.rank && core.score > leaf.score);
    assert_eq!(core.uncovered_lines, 8);
    assert!(report.least_covered.iter().all(|g| g.score <= 100.0));
    assert_eq!(
        coverage::report(&index, &data, 1)
            .unwrap()
            .least_covered
            .len(),
        1
    );
}

#[test]
fn imports_cobertura_with_the_same_numbers() {
    let (dir, index) = fixture();
    let lines = |hit: u32| -> String {
        (1..=10)
            .map(|n| {
                format!(
                    "<line number=\"{n}\" hits=\"{}\"/>",
                    if n <= hit { 3 } else { 0 }
                )
            })
            .collect()
    };
    let xml = format!(
        "<?xml version=\"1.0\"?>\n<coverage line-rate=\"0.2\">\n<sources><source>{}</source></sources>\n<packages><package name=\"demo\"><classes>\n<class name=\"core\" filename=\"src/core.rs\"><methods/><lines>{}</lines></class>\n<class name=\"leaf\" filename=\"src\\leaf.rs\"><lines>{}</lines></class>\n</classes></package></packages>\n</coverage>\n",
        dir.path().display(),
        lines(2),
        lines(10)
    );
    write(dir.path(), "out/cov.xml", &xml);

    let data = coverage::import(dir.path(), &index, Some(Path::new("out/cov.xml"))).unwrap();
    assert_eq!(data.format, Format::Cobertura);
    assert_eq!(data.unmatched, 0);
    let rows: Vec<(&str, u32, u32)> = data
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.lines_found, f.lines_hit))
        .collect();
    assert_eq!(rows, [("src/core.rs", 10, 2), ("src/leaf.rs", 10, 10)]);

    let report = coverage::report(&index, &data, 10).unwrap();
    assert_eq!(report.percent, 60.0);
    // Fully covered files are not gaps.
    assert_eq!(report.least_covered.len(), 1);
    assert_eq!(report.least_covered[0].path, "src/core.rs");

    // Posted text works the same as a file.
    let inline = coverage::import_text(&index, &xml, "inline").unwrap();
    assert_eq!(inline.files, data.files);
    assert_eq!(inline.source, "inline");
}

#[test]
fn missing_and_malformed_reports_are_errors() {
    let (dir, index) = fixture();
    assert!(coverage::load(&index).unwrap().is_none());
    let missing = coverage::import(dir.path(), &index, None).unwrap_err();
    assert!(matches!(missing, Error::NotFound(_)), "{missing}");
    assert!(missing.to_string().contains("lcov.info"));
    let gone = coverage::import(dir.path(), &index, Some(Path::new("nope.info"))).unwrap_err();
    assert!(matches!(gone, Error::NotFound(_)), "{gone}");
    let bad = coverage::import_text(&index, "hello\n", "inline").unwrap_err();
    assert!(matches!(bad, Error::Invalid(_)), "{bad}");
}
