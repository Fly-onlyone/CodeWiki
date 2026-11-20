//! The CLIProxyAPI provider and its model list, over real HTTP against a
//! stand-in server that answers as CLIProxyAPI 8 does.
//!
//! The shapes come from a live CLIProxyAPI 8.0.79 and its source
//! (`sdk/cliproxy/auth/errors.go`, `sdk/api/handlers/handlers_errors.go`).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use cw_core::Error;
use cw_llm::LlmProvider;
use cw_llm::cliproxy::{CliProxy, models, reachable};
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
    let error = models(&url, "client-key").unwrap_err().to_string();
    assert!(error.contains("CLIProxyAPI not reachable"), "{error}");
    assert!(!reachable(&url));
}

#[test]
fn any_answer_means_the_proxy_is_there() {
    // A refused key still proves something listens.
    let (url, server) = serve(vec![json_answer(
        401,
        json!({ "error": "missing api key" }),
    )]);
    assert!(reachable(&url));
    let got = server.join().unwrap().remove(0);
    assert_eq!(got.target, "/v1/models");
    assert_eq!(got.header("authorization"), None);
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
