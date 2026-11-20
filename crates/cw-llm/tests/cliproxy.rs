//! The CLIProxyAPI provider and its management client, over real HTTP
//! against a stand-in server that answers as CLIProxyAPI 8 does.
//!
//! The shapes come from a live CLIProxyAPI 8.0.79 and its source
//! (`sdk/cliproxy/auth/errors.go`, `internal/api/handlers/management`).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use cw_core::Error;
use cw_llm::LlmProvider;
use cw_llm::cliproxy::{Account, CliProxy, LoginStatus, Management, models};
use serde_json::{Value, json};

/// One request the stand-in received.
struct Received {
    method: String,
    /// Path with its query string.
    target: String,
    headers: Vec<(String, String)>,
    body: Value,
}

impl Received {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// One answer of the stand-in: status, extra headers, body text.
struct Answer {
    status: u16,
    headers: Vec<(&'static str, String)>,
    body: String,
}

fn json_answer(status: u16, body: Value) -> Answer {
    Answer {
        status,
        headers: vec![("Content-Type", "application/json".into())],
        body: body.to_string(),
    }
}

/// Serve `answers` in order, one per connection, and hand back what was
/// received. Returns the base URL.
fn serve(answers: Vec<Answer>) -> (String, JoinHandle<Vec<Received>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let mut received = Vec::new();
        for answer in answers {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            let mut parts = request_line.split_whitespace();
            let method = parts.next().unwrap_or("").to_string();
            let target = parts.next().unwrap_or("").to_string();
            let mut headers = Vec::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                let line = line.trim_end();
                if line.is_empty() {
                    break;
                }
                if let Some((key, value)) = line.split_once(':') {
                    headers.push((key.trim().to_string(), value.trim().to_string()));
                }
            }
            let length: usize = headers
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                .and_then(|(_, value)| value.parse().ok())
                .unwrap_or(0);
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            let mut stream = reader.into_inner();
            let mut head = format!("HTTP/1.1 {} X\r\n", answer.status);
            for (key, value) in &answer.headers {
                head.push_str(&format!("{key}: {value}\r\n"));
            }
            head.push_str(&format!(
                "Content-Length: {}\r\nConnection: close\r\n\r\n",
                answer.body.len()
            ));
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(answer.body.as_bytes()).unwrap();
            stream.flush().unwrap();
            received.push(Received {
                method,
                target,
                headers,
                body: serde_json::from_slice(&body).unwrap_or(Value::Null),
            });
        }
        received
    });
    (url, handle)
}

fn proxy(url: &str) -> CliProxy {
    CliProxy {
        base_url: url.to_string(),
        // Made-up keys: nothing here is a real one.
        api_key: "client-key".into(),
        model: "claude-sonnet-x".into(),
    }
}

fn management(url: &str) -> Management {
    Management {
        base_url: url.to_string(),
        key: "management-key".into(),
    }
}

/// A URL where nothing listens.
fn closed_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    url
}

#[test]
fn chat_goes_to_the_proxy_with_the_client_key() {
    let (url, server) = serve(vec![json_answer(
        200,
        json!({
            "choices": [{ "message": { "role": "assistant", "content": "Hello" } }],
            "usage": { "prompt_tokens": 9, "completion_tokens": 2 },
        }),
    )]);
    let provider = proxy(&format!("{url}/"));
    assert_eq!(provider.name(), "cliproxy:claude-sonnet-x");
    let completion = provider.complete("be brief", "say hello").unwrap();
    assert_eq!(completion.text, "Hello");
    assert_eq!((completion.input_tokens, completion.output_tokens), (9, 2));

    let got = server.join().unwrap().remove(0);
    assert_eq!(
        (got.method.as_str(), got.target.as_str()),
        ("POST", "/v1/chat/completions")
    );
    assert_eq!(got.header("authorization"), Some("Bearer client-key"));
    assert_eq!(got.body["model"], "claude-sonnet-x");
    assert_eq!(got.body["messages"][0]["role"], "system");
    assert_eq!(got.body["messages"][1]["content"], "say hello");
}

#[test]
fn no_account_left_is_exhausted_with_the_wait() {
    let mut answer = json_answer(
        503,
        json!({ "error": {
            "message": "no auth available (providers=claude, model=claude-sonnet-x)",
            "type": "server_error",
            "code": "auth_unavailable",
        }}),
    );
    answer.headers.push(("Retry-After", "120".into()));
    let (url, server) = serve(vec![answer]);
    match proxy(&url).complete("s", "u") {
        Err(Error::Exhausted {
            message,
            retry_after,
        }) => {
            assert!(message.contains("no auth available"), "{message}");
            assert_eq!(retry_after, Some(120));
        }
        other => panic!("expected Exhausted, got {other:?}"),
    }
    server.join().unwrap();
}

#[test]
fn a_limit_or_an_empty_pool_is_exhausted_too() {
    let (url, server) = serve(vec![
        json_answer(
            429,
            json!({ "error": { "message": "model cooling down", "code": "model_cooldown" } }),
        ),
        json_answer(
            503,
            json!({ "error": { "message": "no auth available", "code": "auth_not_found" } }),
        ),
    ]);
    for _ in 0..2 {
        let error = proxy(&url).complete("s", "u").unwrap_err();
        assert!(
            matches!(
                error,
                Error::Exhausted {
                    retry_after: None,
                    ..
                }
            ),
            "{error:?}"
        );
    }
    server.join().unwrap();
}

#[test]
fn other_failures_stay_ordinary_errors() {
    let (url, server) = serve(vec![
        json_answer(500, json!({ "error": { "message": "upstream broke" } })),
        json_answer(
            503,
            json!({ "error": { "message": "binding busy", "code": "thread_auth_unavailable" } }),
        ),
        json_answer(
            400,
            json!({ "error": { "message": "messages: too long", "code": "invalid_request_error" } }),
        ),
    ]);
    for expected in ["HTTP 500", "HTTP 503", "messages: too long"] {
        let error = proxy(&url).complete("s", "u").unwrap_err();
        assert!(!matches!(error, Error::Exhausted { .. }), "{error:?}");
        assert!(error.to_string().contains(expected), "{error}");
    }
    server.join().unwrap();
}

#[test]
fn a_model_no_account_serves_is_exhausted() {
    // What CLIProxyAPI 8.0.79 answers once the account behind a model is
    // removed or disabled.
    let (url, server) = serve(vec![json_answer(
        400,
        json!({ "error": {
            "message": "unknown provider for model claude-sonnet-x",
            "type": "invalid_request_error",
            "code": "model_not_found",
            "param": "model",
        }}),
    )]);
    match proxy(&url).complete("s", "u") {
        Err(Error::Exhausted {
            message,
            retry_after: None,
        }) => assert!(
            message.contains("unknown provider for model claude-sonnet-x")
                && message.contains("no account in the proxy serves this model"),
            "{message}"
        ),
        other => panic!("expected Exhausted, got {other:?}"),
    }
    server.join().unwrap();
}

#[test]
fn a_proxy_that_does_not_answer_is_named() {
    let url = closed_url();
    let error = proxy(&url).complete("s", "u").unwrap_err().to_string();
    assert!(
        error.contains(&format!("CLIProxyAPI not reachable at {url}")),
        "{error}"
    );
    let error = management(&url).accounts().unwrap_err().to_string();
    assert!(error.contains("CLIProxyAPI not reachable"), "{error}");
}

#[test]
fn models_are_listed_with_the_client_key() {
    let (url, server) = serve(vec![json_answer(
        200,
        json!({ "object": "list", "data": [
            { "id": "gpt-5-codex", "object": "model" },
            { "id": "claude-sonnet-x", "object": "model" },
        ]}),
    )]);
    let listed = models(&url, "client-key").unwrap();
    assert_eq!(listed, ["claude-sonnet-x", "gpt-5-codex"]);
    let got = server.join().unwrap().remove(0);
    assert_eq!(
        (got.method.as_str(), got.target.as_str()),
        ("GET", "/v1/models")
    );
    assert_eq!(got.header("authorization"), Some("Bearer client-key"));
}

#[test]
fn accounts_are_read_from_the_credentials_list() {
    let (url, server) = serve(vec![json_answer(
        200,
        json!({ "observed_at": "2026-11-20T01:30:37Z", "files": [
            {
                "name": "claude-a@example.com.json",
                "provider": "claude",
                "email": "a@example.com",
                "status": "active",
                "status_message": "",
                "disabled": false,
                "unavailable": false,
            },
            {
                "name": "codex-b.json",
                "provider": "codex",
                "status": "error",
                "status_message": "quota exhausted",
                "unavailable": true,
                "next_retry_after": "2026-11-20T03:00:00Z",
            },
        ]}),
    )]);
    let accounts = management(&url).accounts().unwrap();
    assert_eq!(
        accounts,
        [
            Account {
                name: "claude-a@example.com.json".into(),
                provider: "claude".into(),
                email: Some("a@example.com".into()),
                status: "active".into(),
                message: None,
                unavailable: false,
                disabled: false,
                next_retry_after: None,
            },
            Account {
                name: "codex-b.json".into(),
                provider: "codex".into(),
                email: None,
                status: "error".into(),
                message: Some("quota exhausted".into()),
                unavailable: true,
                disabled: false,
                next_retry_after: Some("2026-11-20T03:00:00Z".into()),
            },
        ]
    );
    let got = server.join().unwrap().remove(0);
    assert_eq!(got.target, "/v8/management/credentials");
    assert_eq!(got.header("authorization"), Some("Bearer management-key"));
}

#[test]
fn a_login_is_started_watched_and_cancelled() {
    let (url, server) = serve(vec![
        json_answer(
            200,
            json!({ "status": "ok", "state": "s-1", "url": "https://auth.example/authorize?x=1" }),
        ),
        json_answer(200, json!({ "status": "wait" })),
        json_answer(200, json!({ "status": "ok" })),
        json_answer(
            200,
            json!({ "status": "error", "error": "unknown or expired state" }),
        ),
        json_answer(200, json!({ "status": "ok", "cancelled": true })),
    ]);
    let manage = management(&url);
    let login = manage.start_login("codex").unwrap();
    assert_eq!(login.state, "s-1");
    assert_eq!(login.url, "https://auth.example/authorize?x=1");
    assert_eq!(manage.login_status("s-1").unwrap(), LoginStatus::Waiting);
    assert_eq!(manage.login_status("s-1").unwrap(), LoginStatus::Done);
    assert_eq!(
        manage.login_status("s-1").unwrap(),
        LoginStatus::Failed("unknown or expired state".into())
    );
    manage.cancel_login("s-1").unwrap();

    let got = server.join().unwrap();
    assert_eq!(
        got[0].target,
        "/v8/management/oauth/auth-url?provider=codex&is_webui=true"
    );
    assert_eq!(got[1].target, "/v8/management/oauth/status?state=s-1");
    assert_eq!(
        (got[4].method.as_str(), got[4].target.as_str()),
        ("DELETE", "/v8/management/oauth/session?state=s-1")
    );
}

#[test]
fn an_account_is_removed_by_its_file_name() {
    let (url, server) = serve(vec![json_answer(200, json!({ "status": "ok" }))]);
    management(&url)
        .remove("claude-a@example.com.json")
        .unwrap();
    let got = server.join().unwrap().remove(0);
    assert_eq!(got.method, "DELETE");
    assert_eq!(
        got.target,
        "/v8/management/credentials?name=claude-a%40example.com.json"
    );
}

#[test]
fn management_errors_say_what_to_fix() {
    let (url, server) = serve(vec![
        json_answer(401, json!({ "error": "invalid management key" })),
        Answer {
            status: 404,
            headers: vec![("Content-Type", "text/plain".into())],
            body: "404 page not found".into(),
        },
        json_answer(404, json!({ "error": "provider_not_found" })),
    ]);
    let manage = management(&url);
    let error = manage.accounts().unwrap_err().to_string();
    assert!(error.contains("invalid management key"), "{error}");
    let error = manage.accounts().unwrap_err().to_string();
    assert!(
        error.contains("CLIProxyAPI 8") && error.contains("management.secret-key"),
        "{error}"
    );
    let error = manage.start_login("gemini").unwrap_err().to_string();
    assert!(error.contains("provider_not_found"), "{error}");
    server.join().unwrap();
}
