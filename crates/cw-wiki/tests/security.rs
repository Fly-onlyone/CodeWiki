//! Security findings across a fixture repository, and their place in `check`.

use std::fs;
use std::path::Path;

use cw_index::Index;
use cw_wiki::ci::{self, Level};
use cw_wiki::{security, service};

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn fixture() -> (tempfile::TempDir, Index) {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Cargo.toml", "[package]\nname = \"demo\"\n");
    write(r, "src/lib.rs", "pub mod clean;\npub mod net;\n");
    write(
        r,
        "src/clean.rs",
        "//! Nothing to see: unsafe { } and http://api.partner.io in a comment.\npub fn add(a: u8, b: u8) -> u8 { a + b }\n",
    );
    write(
        r,
        "src/net.rs",
        "use std::process::Command;\n\nconst API_TOKEN: &str = \"9f8a7b6c5d4e3f2a1b0c9d8e7f6a5b4c\";\nconst FEED: &str = \"http://feed.partner.io/v2\";\n\npub fn list(dir: &str) {\n    let _ = Command::new(\"sh\").arg(\"-c\").arg(format!(\"ls {dir}\")).status();\n}\n\npub fn find(name: &str) -> String {\n    format!(\"SELECT id FROM users WHERE name = '{name}'\")\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn uses_a_fake_token() {\n        let api_token = \"0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d\";\n        std::env::set_var(\"TOKEN\", api_token);\n    }\n}\n",
    );
    write(
        r,
        "tests/it.rs",
        "#[test]\nfn t() {\n    std::env::set_var(\"A\", \"http://feed.partner.io\");\n}\n",
    );
    // Not Rust: the rules do not apply.
    write(r, "notes.md", "std::env::set_var(\"A\", \"b\")\n");
    let mut index = Index::open_in_memory().unwrap();
    service::reindex(r, &mut index).unwrap();
    (dir, index)
}

#[test]
fn scans_source_files_and_skips_tests() {
    let (dir, index) = fixture();
    let found = security::scan(dir.path(), &index).unwrap();
    let rows: Vec<(&str, u32, &str, Level)> = found
        .iter()
        .map(|f| (f.file.as_str(), f.line, f.rule.as_str(), f.level))
        .collect();
    assert_eq!(
        rows,
        [
            ("src/net.rs", 3, security::HARDCODED_SECRET, Level::Error),
            ("src/net.rs", 4, security::INSECURE_HTTP, Level::Warning),
            ("src/net.rs", 7, security::COMMAND_INJECTION, Level::Warning),
            ("src/net.rs", 11, security::SQL_FORMAT, Level::Warning),
        ]
    );
    assert!(
        found[0].message.contains("`API_TOKEN`"),
        "{}",
        found[0].message
    );
    assert!(found[1].message.contains("feed.partner.io"));

    // A file deleted after indexing is skipped, not an error.
    fs::remove_file(dir.path().join("src/net.rs")).unwrap();
    assert!(security::scan(dir.path(), &index).unwrap().is_empty());
}

#[test]
fn check_reports_security_findings_as_rules() {
    let (dir, index) = fixture();
    let findings = ci::check(dir.path(), &index).unwrap();
    let security: Vec<&ci::CheckFinding> = findings
        .iter()
        .filter(|f| f.rule.starts_with("security/"))
        .collect();
    assert_eq!(security.len(), 4, "{findings:?}");
    assert!(
        security
            .iter()
            .all(|f| f.file == "src/net.rs" && f.line > 0)
    );
    // The hard-coded token is an error, so the default gate fails.
    assert!(ci::fails(&findings, Level::Error));

    let text = ci::to_text(&findings);
    assert!(
        text.contains("error: src/net.rs:3: [security/hardcoded-secret]"),
        "{text}"
    );
    let sarif = ci::to_sarif(&findings);
    let declared: Vec<&str> = sarif["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    for finding in &findings {
        assert!(
            declared.contains(&finding.rule.as_str()),
            "{}",
            finding.rule
        );
    }
    let junit = ci::to_junit(&findings, Level::Error);
    assert!(junit.contains("name=\"security/hardcoded-secret\">\n    <failure"));
    assert!(junit.contains("name=\"security/unsafe-block\"></testcase>"));
}
