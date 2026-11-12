//! HTTP contracts across fixture repositories: a server and its client.

use std::fs;
use std::path::Path;

use cw_index::Index;
use cw_wiki::contracts::{self, RepoContracts};
use cw_wiki::service;

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn scan(files: &[(&str, &str)]) -> RepoContracts {
    let dir = tempfile::tempdir().unwrap();
    for (rel, text) in files {
        write(dir.path(), rel, text);
    }
    let mut index = Index::open_in_memory().unwrap();
    service::reindex(dir.path(), &mut index).unwrap();
    contracts::scan(dir.path(), &index).unwrap()
}

fn server() -> RepoContracts {
    scan(&[
        ("Cargo.toml", "[package]\nname = \"server\"\n"),
        ("src/lib.rs", "pub mod api;\n"),
        (
            "src/api.rs",
            "pub fn routes() -> Router {\n    let protected = Router::new()\n        .route(\"/repos\", get(list).post(add))\n        .route(\"/repos/{id}/notes\", get(notes))\n        .route(\"/repos/{id}/export\", get(export));\n    Router::new()\n        .route(\"/v1/health\", get(health))\n        .nest(\"/v1\", protected)\n}\n",
        ),
        // Test routers and requests made by tests are not contracts.
        (
            "tests/api.rs",
            "fn app() { Router::new().route(\"/v1/test-only\", get(h)); call(\"/v1/repos\"); }\n",
        ),
        // Not Rust: out of scope.
        ("web/app.ts", "fetch(\"/v1/from-typescript\");\n"),
    ])
}

fn client() -> RepoContracts {
    scan(&[
        ("Cargo.toml", "[package]\nname = \"client\"\n"),
        (
            "src/main.rs",
            "fn main() {\n    let url = base();\n    get(&format!(\"{url}/v1/health\"));\n    post(\"/v1/repos\", body());\n    let id = \"demo\";\n    get(&format!(\"/v1/repos/{id}/notes?kind={}\", 1));\n    get(\"/v1/repos/demo/graph\");\n    std::fs::read(\"/etc/hosts\").ok();\n    std::fs::read(format!(\"{id}/Cargo.toml\")).ok();\n}\n",
        ),
    ])
}

#[test]
fn finds_what_a_server_provides() {
    let found = server();
    let rows: Vec<(&str, &str, &str, u32)> = found
        .provided
        .iter()
        .map(|p| (p.method.as_str(), p.path.as_str(), p.file.as_str(), p.line))
        .collect();
    assert_eq!(
        rows,
        [
            ("GET", "/v1/repos", "src/api.rs", 3),
            ("POST", "/v1/repos", "src/api.rs", 3),
            ("GET", "/v1/repos/{id}/notes", "src/api.rs", 4),
            ("GET", "/v1/repos/{id}/export", "src/api.rs", 5),
            ("GET", "/v1/health", "src/api.rs", 7),
        ]
    );
    assert!(found.consumed.is_empty(), "{:?}", found.consumed);
}

#[test]
fn finds_what_a_client_consumes() {
    let found = client();
    assert!(found.provided.is_empty());
    let rows: Vec<(&str, u32)> = found
        .consumed
        .iter()
        .map(|c| (c.path.as_str(), c.line))
        .collect();
    assert_eq!(
        rows,
        [
            ("/v1/health", 3),
            ("/v1/repos", 4),
            ("/v1/repos/{id}/notes", 6),
            ("/v1/repos/demo/graph", 7),
        ]
    );
    assert!(found.consumed.iter().all(|c| c.file == "src/main.rs"));
}

#[test]
fn matches_the_client_to_the_server() {
    let all = contracts::match_workspace(&[
        ("server".to_string(), server()),
        ("client".to_string(), client()),
    ]);
    assert_eq!(
        (all.repos[0].provided, all.repos[0].consumed),
        (5, 0),
        "{:?}",
        all.repos
    );
    assert_eq!((all.repos[1].provided, all.repos[1].consumed), (0, 4));

    let matched: Vec<(&str, Vec<&str>)> = all
        .matched
        .iter()
        .map(|l| {
            (
                l.path.as_str(),
                l.providers.iter().map(|p| p.method.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        matched,
        [
            ("/v1/health", vec!["GET"]),
            ("/v1/repos", vec!["GET", "POST"]),
            ("/v1/repos/{id}/notes", vec!["GET"]),
        ]
    );
    assert!(all.matched.iter().all(|l| l.repo == "client"));
    assert!(
        all.matched
            .iter()
            .all(|l| l.providers.iter().all(|p| p.repo == "server"))
    );

    let unprovided: Vec<&str> = all.unprovided.iter().map(|c| c.path.as_str()).collect();
    assert_eq!(unprovided, ["/v1/repos/demo/graph"]);
    let unconsumed: Vec<(&str, &str)> = all
        .unconsumed
        .iter()
        .map(|r| (r.method.as_str(), r.path.as_str()))
        .collect();
    assert_eq!(unconsumed, [("GET", "/v1/repos/{id}/export")]);
}
