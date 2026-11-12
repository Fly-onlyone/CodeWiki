//! Drives the router in-process: auth, repository lifecycle, queries, review.

use std::fs;
use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use cw_server::{ServerConfig, router};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const TOKEN: &str = "test-token";
const SECRET: &str = "hook-secret";

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

struct Harness {
    app: Router,
    _data: tempfile::TempDir,
    repo: tempfile::TempDir,
}

impl Harness {
    fn new() -> Self {
        Self::with_webhook_secret(Some(SECRET))
    }

    fn with_webhook_secret(secret: Option<&str>) -> Self {
        let data = tempfile::tempdir().unwrap();
        let repo = tempfile::tempdir().unwrap();
        let r = repo.path();
        write(
            r,
            "Cargo.toml",
            "[package]\nname = \"demo\"\ndescription = \"Demo crate\"\n",
        );
        write(
            r,
            "src/lib.rs",
            "//! Demo crate.\npub mod math;\n\n/// Entry point.\npub fn run() -> u64 { math::double(21) }\n",
        );
        write(
            r,
            "src/math.rs",
            "//! Arithmetic helpers.\n\n/// Doubles a number.\npub fn double(x: u64) -> u64 { x * 2 }\n",
        );
        let app = router(ServerConfig {
            data_dir: data.path().to_path_buf(),
            token: TOKEN.into(),
            webhook_secret: secret.map(str::to_string),
        });
        Self {
            app,
            _data: data,
            repo,
        }
    }

    async fn call(
        &self,
        method: &str,
        uri: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        let request = match body {
            Some(json) => request
                .header("content-type", "application/json")
                .body(Body::from(json.to_string()))
                .unwrap(),
            None => request.body(Body::empty()).unwrap(),
        };
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// Deliver a webhook: a raw body with the given headers, no bearer token.
    async fn hook(&self, headers: &[(&str, &str)], body: &str) -> (StatusCode, Value) {
        let mut request = Request::builder().method("POST").uri("/v1/hooks/git");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let request = request.body(Body::from(body.to_string())).unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// Register a folder and return its id and name.
    async fn register(&self, root: &Path) -> (String, String) {
        let path = root.to_string_lossy().to_string();
        let repo = self.post("/v1/repos", Some(json!({ "path": path }))).await;
        (
            repo["id"].as_str().unwrap().to_string(),
            repo["name"].as_str().unwrap().to_string(),
        )
    }

    /// Wait for a background update to give the repository notes.
    async fn wait_for_notes(&self, id: &str) -> u64 {
        for _ in 0..600 {
            let notes = self.get(&format!("/v1/repos/{id}/status")).await["notes"]
                .as_u64()
                .unwrap();
            if notes > 0 {
                return notes;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        panic!("the update started by the webhook never finished");
    }

    async fn get(&self, uri: &str) -> Value {
        let (status, body) = self.call("GET", uri, Some(TOKEN), None).await;
        assert_eq!(status, StatusCode::OK, "GET {uri}: {body}");
        body
    }

    async fn post(&self, uri: &str, body: Option<Value>) -> Value {
        let (status, body) = self.call("POST", uri, Some(TOKEN), body).await;
        assert_eq!(status, StatusCode::OK, "POST {uri}: {body}");
        body
    }
}

#[tokio::test]
async fn health_is_open_and_everything_else_needs_the_token() {
    let h = Harness::new();
    let (status, body) = h.call("GET", "/v1/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");

    for token in [None, Some("wrong"), Some("")] {
        let (status, _) = h.call("GET", "/v1/repos", token, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{token:?}");
    }
    let (status, body) = h.call("GET", "/v1/repos", Some(TOKEN), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!([]));
}

#[tokio::test]
async fn full_repository_flow() {
    let h = Harness::new();
    let path = h.repo.path().to_string_lossy().to_string();

    let (status, _) = h
        .call(
            "POST",
            "/v1/repos",
            Some(TOKEN),
            Some(json!({ "path": "Z:/does/not/exist" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let repo = h.post("/v1/repos", Some(json!({ "path": path }))).await;
    let id = repo["id"].as_str().unwrap().to_string();
    // Adding twice is idempotent.
    assert_eq!(
        h.post("/v1/repos", Some(json!({ "path": path }))).await["id"],
        id
    );
    assert_eq!(h.get("/v1/repos").await.as_array().unwrap().len(), 1);

    let report = h.post(&format!("/v1/repos/{id}/update"), None).await;
    assert!(report["written"].as_array().unwrap().len() >= 4, "{report}");
    assert_eq!(report["link_rate"], 1.0);

    let status = h.get(&format!("/v1/repos/{id}/status")).await;
    assert_eq!(status["files"], 3);
    assert!(status["symbols"].as_u64().unwrap() >= 4);

    let notes = h.get(&format!("/v1/repos/{id}/notes")).await;
    let paths: Vec<&str> = notes
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["path"].as_str().unwrap())
        .collect();
    assert!(
        paths.contains(&"_HOME.md") && paths.contains(&"01-Core/demo/Math Module.md"),
        "{paths:?}"
    );

    let note = h
        .get(&format!(
            "/v1/repos/{id}/note?path=01-Core/demo/Math%20Module.md"
        ))
        .await;
    assert!(
        note["text"]
            .as_str()
            .unwrap()
            .contains("> Arithmetic helpers.")
    );
    assert_eq!(note["info"]["state"], "generated");
    // Notes can also be fetched by name.
    assert_eq!(
        h.get(&format!("/v1/repos/{id}/note?path=math%20module"))
            .await["info"]["path"],
        "01-Core/demo/Math Module.md"
    );

    let (status, _) = h
        .call(
            "GET",
            &format!("/v1/repos/{id}/note?path=../../Cargo.toml"),
            Some(TOKEN),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let hits = h.get(&format!("/v1/repos/{id}/search?q=doubles")).await;
    assert!(
        hits.as_array()
            .unwrap()
            .iter()
            .any(|x| x["key"] == "demo::math::double"),
        "{hits}"
    );

    let symbol = h.get(&format!("/v1/repos/{id}/symbol?name=double")).await;
    assert_eq!(symbol["symbol"]["qualified"], "demo::math::double");
    assert_eq!(symbol["used_by"][0]["qualified"], "demo::run");

    let context = h
        .get(&format!("/v1/repos/{id}/context?target=src/math.rs"))
        .await;
    assert_eq!(context["type"], "file");
    assert!(
        context["notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n == "01-Core/demo/Math Module.md")
    );

    let chain = h
        .get(&format!("/v1/repos/{id}/path?from=run&to=double"))
        .await;
    assert_eq!(chain, json!(["demo::run", "demo::math::double"]));

    let graph = h.get(&format!("/v1/repos/{id}/graph")).await;
    assert!(graph["nodes"].as_array().unwrap().len() >= 4);
    assert_eq!(graph["edges"].as_array().unwrap().len(), 1);

    let answer = h
        .post(
            &format!("/v1/repos/{id}/ask"),
            Some(json!({ "question": "arithmetic helpers" })),
        )
        .await;
    assert!(
        answer["answer"].as_str().unwrap().contains("Math Module"),
        "{answer}"
    );

    // Edit a note through the API, change its source, and review the proposal.
    let edited = note["text"].as_str().unwrap().to_string() + "\n## Gotchas\n\n- Mine.\n";
    let (status, _) = h
        .call(
            "PUT",
            &format!("/v1/repos/{id}/note?path=01-Core/demo/Math%20Module.md"),
            Some(TOKEN),
            Some(json!({ "text": edited })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    write(
        h.repo.path(),
        "src/math.rs",
        "//! Arithmetic helpers for the demo.\n\n/// Doubles a number.\npub fn double(x: u64) -> u64 { x * 2 }\n",
    );
    let report = h.post(&format!("/v1/repos/{id}/update"), None).await;
    assert_eq!(report["proposed"], json!(["01-Core/demo/Math Module.md"]));

    let proposals = h.get(&format!("/v1/repos/{id}/proposals")).await;
    let pid = proposals[0]["id"].as_str().unwrap().to_string();
    let detail = h.get(&format!("/v1/repos/{id}/proposals/{pid}")).await;
    assert!(detail["patch"].as_str().unwrap().contains("for the demo"));

    let approved = h
        .post(&format!("/v1/repos/{id}/proposals/{pid}/approve"), None)
        .await;
    assert_eq!(approved["status"], "approved");
    let after = h
        .get(&format!(
            "/v1/repos/{id}/note?path=01-Core/demo/Math%20Module.md"
        ))
        .await;
    assert!(after["text"].as_str().unwrap().contains("for the demo"));
    // Approval settles the note: the next update proposes nothing.
    let quiet = h.post(&format!("/v1/repos/{id}/update"), None).await;
    assert_eq!(quiet["proposed"], json!([]));

    let (status, _) = h
        .call("DELETE", &format!("/v1/repos/{id}"), Some(TOKEN), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = h
        .call("GET", &format!("/v1/repos/{id}/status"), Some(TOKEN), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(
        h.repo.path().join("obsidian/_HOME.md").exists(),
        "removing a repo keeps its vault"
    );
}

#[tokio::test]
async fn analysis_routes() {
    let h = Harness::new();
    let root = h.repo.path();
    write(
        root,
        "tests/math.rs",
        "use demo::math::double;\n\n#[test]\nfn doubles() {\n    assert_eq!(double(2), 4);\n}\n",
    );
    write(
        root,
        "src/net.rs",
        "pub fn configure() {\n    std::env::set_var(\"MODE\", \"fast\");\n    let _ = unsafe { std::mem::zeroed::<u8>() };\n}\n",
    );
    write(
        root,
        "src/lib.rs",
        "//! Demo crate.\npub mod math;\npub mod net;\n\n/// Entry point.\npub fn run() -> u64 { math::double(21) }\n",
    );
    write(
        root,
        "lcov.info",
        "SF:src/math.rs\nDA:4,3\nend_of_record\nSF:src/net.rs\nDA:1,0\nDA:2,0\nDA:3,0\nend_of_record\n",
    );
    let path = root.to_string_lossy().to_string();
    let id = h.post("/v1/repos", Some(json!({ "path": path }))).await["id"]
        .as_str()
        .unwrap()
        .to_string();
    h.post(&format!("/v1/repos/{id}/reindex"), None).await;

    let map = h.get(&format!("/v1/repos/{id}/tests")).await;
    assert_eq!(map["tests_total"], 1);
    assert_eq!(map["test_files"][0]["path"], "tests/math.rs");
    assert_eq!(map["test_files"][0]["kind"], "integration");
    // `run` in `lib.rs` uses `math.rs`, but no test calls `run`.
    assert_eq!(map["untested"], json!(["src/lib.rs", "src/net.rs"]));

    let impact = h
        .post(
            &format!("/v1/repos/{id}/impacted-tests"),
            Some(json!({ "files": ["src\\math.rs", "src/net.rs"] })),
        )
        .await;
    assert_eq!(impact["test_files"][0]["path"], "tests/math.rs");
    assert_eq!(impact["test_files"][0]["tests"], json!(["doubles"]));
    assert_eq!(
        impact["commands"],
        json!(["cargo test -p demo --test math"])
    );
    assert_eq!(impact["untested_changes"], json!(["src/net.rs"]));
    // No body: the working tree's changes are used; outside git there are none.
    let idle = h
        .post(&format!("/v1/repos/{id}/impacted-tests"), None)
        .await;
    assert_eq!(idle["test_files"], json!([]));

    // Nothing imported yet: the report in the repository root is read.
    let found = h.get(&format!("/v1/repos/{id}/coverage")).await;
    assert_eq!(found["source"], "lcov.info");
    let coverage = h
        .post(
            &format!("/v1/repos/{id}/coverage"),
            Some(json!({ "report": "lcov.info", "limit": 5 })),
        )
        .await;
    assert_eq!(coverage["format"], "lcov");
    assert_eq!(coverage["percent"], 25.0);
    assert_eq!(coverage["least_covered"][0]["path"], "src/net.rs");
    assert_eq!(coverage["least_covered"][0]["uncovered_lines"], 3);
    // The import is stored: it survives the report file.
    fs::remove_file(root.join("lcov.info")).unwrap();
    let stored = h.get(&format!("/v1/repos/{id}/coverage?limit=1")).await;
    assert_eq!(stored["lines_found"], 4);
    let inline = h
        .post(
            &format!("/v1/repos/{id}/coverage"),
            Some(json!({ "text": "<coverage><packages><package><classes><class filename=\"src/math.rs\"><lines><line number=\"4\" hits=\"1\"/></lines></class></classes></package></packages></coverage>" })),
        )
        .await;
    assert_eq!(inline["format"], "cobertura");
    assert_eq!(inline["percent"], 100.0);
    for (body, expected) in [
        (
            json!({ "report": "../outside.info" }),
            StatusCode::BAD_REQUEST,
        ),
        (json!({ "report": "missing.info" }), StatusCode::NOT_FOUND),
        (json!({ "text": "not a report" }), StatusCode::BAD_REQUEST),
    ] {
        let (status, _) = h
            .call(
                "POST",
                &format!("/v1/repos/{id}/coverage"),
                Some(TOKEN),
                Some(body.clone()),
            )
            .await;
        assert_eq!(status, expected, "{body}");
    }

    let findings = h.get(&format!("/v1/repos/{id}/security")).await;
    let rules: Vec<(&str, u64)> = findings
        .as_array()
        .unwrap()
        .iter()
        .map(|f| (f["rule"].as_str().unwrap(), f["line"].as_u64().unwrap()))
        .collect();
    assert_eq!(
        rules,
        [("security/env-set-var", 2), ("security/unsafe-block", 3)]
    );
    assert_eq!(findings[0]["file"], "src/net.rs");
    assert_eq!(findings[0]["level"], "warning");
    let warnings = h
        .get(&format!("/v1/repos/{id}/security?level=warning"))
        .await;
    assert_eq!(warnings.as_array().unwrap().len(), 1);
    let (status, _) = h
        .call(
            "GET",
            &format!("/v1/repos/{id}/security?level=loud"),
            Some(TOKEN),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // A small healthy repository has nothing to refactor.
    assert_eq!(h.get(&format!("/v1/repos/{id}/refactor")).await, json!([]));
    let mut big = String::from("//! Big.\n");
    for i in 0..650 {
        big.push_str(&format!("pub const C{i}: u32 = {i};\n"));
    }
    write(root, "src/big.rs", &big);
    write(
        root,
        "src/lib.rs",
        "//! Demo crate.\npub mod big;\npub mod math;\npub mod net;\n\n/// Entry point.\npub fn run() -> u64 { math::double(21) }\n",
    );
    h.post(&format!("/v1/repos/{id}/reindex"), None).await;
    let plan = h.get(&format!("/v1/repos/{id}/refactor?limit=5")).await;
    assert_eq!(plan[0]["kind"], "split_file");
    assert_eq!(plan[0]["target"], "src/big.rs");
    assert!(plan[0]["steps"].as_array().unwrap().len() >= 3, "{plan}");
    assert!(plan[0]["evidence"][0].as_str().unwrap().contains("lines"));

    for route in ["tests", "coverage", "security", "refactor"] {
        let (status, _) = h
            .call("GET", &format!("/v1/repos/{id}/{route}"), None, None)
            .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{route}");
    }
}

/// `X-Hub-Signature-256` value for `body` under `secret`.
fn signature(secret: &str, body: &str) -> String {
    let mac = cw_core::hash::hmac_sha256(secret.as_bytes(), body.as_bytes());
    let hex: String = mac.iter().map(|b| format!("{b:02x}")).collect();
    format!("sha256={hex}")
}

#[tokio::test]
async fn git_webhook_is_off_without_a_secret() {
    let h = Harness::with_webhook_secret(None);
    let body = json!({ "repository": { "name": "demo" } }).to_string();
    let (status, _) = h
        .hook(&[("x-hub-signature-256", &signature(SECRET, &body))], &body)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn git_webhook_updates_the_matching_repository() {
    let h = Harness::new();
    let (id, name) = h.register(h.repo.path()).await;
    let body = json!({
        "ref": "refs/heads/main",
        "repository": { "name": name, "full_name": format!("acme/{name}") },
    })
    .to_string();
    let signed = signature(SECRET, &body);

    // The bearer token is not what opens this route.
    let bearer = format!("Bearer {TOKEN}");
    for headers in [
        vec![],
        vec![("authorization", bearer.as_str())],
        vec![("x-hub-signature-256", "sha256=00")],
        vec![("x-hub-signature-256", signed.trim_start_matches("sha256="))],
        vec![("x-gitlab-token", "wrong")],
    ] {
        let (status, _) = h.hook(&headers, &body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{headers:?}");
    }
    // A signature covers the exact bytes sent.
    let (status, _) = h
        .hook(&[("x-hub-signature-256", &signed)], &format!("{body} "))
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let other = json!({ "repository": { "name": "someone-elses-repo" } }).to_string();
    let (status, _) = h
        .hook(
            &[("x-hub-signature-256", &signature(SECRET, &other))],
            &other,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = h
        .hook(&[("x-hub-signature-256", &signature(SECRET, "{"))], "{")
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(h.get(&format!("/v1/repos/{id}/status")).await["notes"], 0);

    let (status, reply) = h.hook(&[("x-hub-signature-256", &signed)], &body).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{reply}");
    assert_eq!(reply["repo"], id);
    // The update runs after the response; it is the one `/update` runs.
    assert!(h.wait_for_notes(&id).await >= 4);
    assert!(h.repo.path().join("obsidian/_HOME.md").exists());

    // GitLab sends the secret itself and names the project differently.
    let gitlab = json!({
        "object_kind": "push",
        "project": { "name": "Demo", "path_with_namespace": format!("group/sub/{name}") },
    })
    .to_string();
    let (status, reply) = h.hook(&[("x-gitlab-token", SECRET)], &gitlab).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{reply}");
    assert_eq!(reply["repo"], id);
}

#[tokio::test]
async fn workspace_view_and_contracts() {
    let h = Harness::new();
    let server = tempfile::tempdir().unwrap();
    let client = tempfile::tempdir().unwrap();
    let gone = tempfile::tempdir().unwrap();
    write(
        server.path(),
        "Cargo.toml",
        "[package]\nname = \"server\"\n\n[dependencies]\nserde = \"1\"\naxum = \"0.8\"\n",
    );
    write(server.path(), "src/lib.rs", "pub mod api;\n");
    write(
        server.path(),
        "src/api.rs",
        "pub fn routes() -> Router {\n    let api = Router::new()\n        .route(\"/items\", get(list).post(add))\n        .route(\"/items/{id}\", get(read))\n        .route(\"/export\", get(export));\n    Router::new().nest(\"/v1\", api)\n}\n",
    );
    write(
        client.path(),
        "Cargo.toml",
        "[package]\nname = \"client\"\n\n[dependencies]\nserde = \"1\"\nureq = \"3\"\n",
    );
    write(
        client.path(),
        "src/main.rs",
        "fn main() {\n    let id = 7;\n    get(\"/v1/items\");\n    get(&format!(\"/v1/items/{id}?full=1\"));\n    get(\"/v1/reports/daily\");\n}\n",
    );
    let (server_id, server_name) = h.register(server.path()).await;
    let (client_id, client_name) = h.register(client.path()).await;
    let (gone_id, gone_name) = h.register(gone.path()).await;
    drop(gone);

    let view = h.get("/v1/workspace").await;
    let repos = view["repos"].as_array().unwrap();
    assert_eq!(repos.len(), 3, "{view}");
    let entry = |id: &str| repos.iter().find(|r| r["id"] == id).unwrap();
    let s = entry(&server_id);
    assert_eq!(s["name"], server_name.as_str());
    assert_eq!(s["files"], 3);
    assert!(s["symbols"].as_u64().unwrap() >= 1, "{s}");
    assert_eq!(s["notes"], 0);
    assert_eq!(s["pending_proposals"], 0);
    assert_eq!(s["stale"], false);
    assert!(s["hotspots"].as_array().unwrap().len() <= 3);
    assert!(s.get("error").is_none(), "{s}");
    // A repository whose folder is gone is listed, not fatal.
    let g = entry(&gone_id);
    assert!(g["error"].as_str().unwrap().contains("not found"), "{g}");
    assert_eq!(g["files"], 0);

    let shared = view["shared_dependencies"].as_array().unwrap();
    assert_eq!(shared.len(), 1, "{view}");
    assert_eq!(shared[0]["name"], "serde");
    let mut users: Vec<&str> = shared[0]["repos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_str().unwrap())
        .collect();
    users.sort_unstable();
    let mut expected = [server_name.as_str(), client_name.as_str()];
    expected.sort_unstable();
    assert_eq!(users, expected);

    let own = h.get(&format!("/v1/repos/{server_id}/contracts")).await;
    let provided: Vec<(&str, &str, u64)> = own["provided"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["method"].as_str().unwrap(),
                p["path"].as_str().unwrap(),
                p["line"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        provided,
        [
            ("GET", "/v1/items", 3),
            ("POST", "/v1/items", 3),
            ("GET", "/v1/items/{id}", 4),
            ("GET", "/v1/export", 5),
        ]
    );
    assert_eq!(own["provided"][0]["file"], "src/api.rs");
    assert_eq!(own["consumed"], json!([]));
    let calls = h.get(&format!("/v1/repos/{client_id}/contracts")).await;
    assert_eq!(calls["consumed"].as_array().unwrap().len(), 3, "{calls}");
    assert_eq!(calls["consumed"][1]["path"], "/v1/items/{id}");
    assert_eq!(calls["consumed"][1]["line"], 4);

    let all = h.get("/v1/workspace/contracts").await;
    let matched: Vec<(&str, usize)> = all["matched"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["path"].as_str().unwrap(),
                l["providers"].as_array().unwrap().len(),
            )
        })
        .collect();
    assert_eq!(matched, [("/v1/items", 2), ("/v1/items/{id}", 1)], "{all}");
    assert_eq!(all["matched"][0]["repo"], client_name.as_str());
    assert_eq!(
        all["matched"][0]["providers"][0]["repo"],
        server_name.as_str()
    );
    assert_eq!(all["unprovided"].as_array().unwrap().len(), 1, "{all}");
    assert_eq!(all["unprovided"][0]["path"], "/v1/reports/daily");
    assert_eq!(all["unconsumed"].as_array().unwrap().len(), 1, "{all}");
    assert_eq!(all["unconsumed"][0]["path"], "/v1/export");
    assert_eq!(all["errors"].as_array().unwrap().len(), 1, "{all}");
    assert_eq!(all["errors"][0]["repo"], gone_name.as_str());

    for uri in [
        "/v1/workspace".to_string(),
        "/v1/workspace/contracts".to_string(),
        format!("/v1/repos/{server_id}/contracts"),
    ] {
        let (status, _) = h.call("GET", &uri, None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{uri}");
    }
    let (status, _) = h
        .call("GET", "/v1/repos/unknown/contracts", Some(TOKEN), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
