//! HTTP embedders exercised over real HTTP against a stand-in server.
//!
//! These check the request each embedder sends and that it reads a response
//! in its service's documented shape. They do not replace a run against the
//! real service.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use cw_embed::{Embedder, OllamaEmbedder, OpenAiEmbedder};
use serde_json::{Value, json};

struct Received {
    path: String,
    authorization: Option<String>,
    body: Value,
}

/// Serve exactly one request with `status` and `reply`.
fn serve_once(status: u16, reply: Value) -> (String, JoinHandle<Received>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line).unwrap();
        let path = request_line
            .split_whitespace()
            .nth(1)
            .unwrap_or("")
            .to_string();
        let (mut length, mut authorization) = (0usize, None);
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some((key, value)) = line.split_once(':') {
                if key.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse().unwrap_or(0);
                } else if key.eq_ignore_ascii_case("authorization") {
                    authorization = Some(value.trim().to_string());
                }
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        let text = reply.to_string();
        let mut stream = reader.into_inner();
        write!(
            stream,
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
            text.len()
        )
        .unwrap();
        stream.flush().unwrap();
        Received {
            path,
            authorization,
            body: serde_json::from_slice(&body).unwrap_or(Value::Null),
        }
    });
    (url, handle)
}

fn texts() -> Vec<String> {
    vec!["first text".to_string(), "second text".to_string()]
}

#[test]
fn ollama_embedder_posts_a_batch_and_normalizes_vectors() {
    let (url, server) = serve_once(
        200,
        json!({ "model": "m", "embeddings": [[3.0, 4.0], [0.0, 5.0]] }),
    );
    let embedder = OllamaEmbedder {
        model: "embeddinggemma".into(),
        base_url: url,
    };
    let vectors = embedder.embed(&texts()).unwrap();
    assert_eq!(vectors, [vec![0.6, 0.8], vec![0.0, 1.0]]);

    let got = server.join().unwrap();
    assert_eq!(got.path, "/api/embed");
    assert_eq!(got.authorization, None);
    assert_eq!(got.body["model"], "embeddinggemma");
    assert_eq!(got.body["input"], json!(["first text", "second text"]));
}

#[test]
fn openai_embedder_sends_a_bearer_token_and_orders_by_index() {
    let (url, server) = serve_once(
        200,
        json!({ "data": [
            { "index": 1, "embedding": [0.0, 2.0] },
            { "index": 0, "embedding": [2.0, 0.0] },
        ]}),
    );
    let embedder = OpenAiEmbedder {
        api_key: "sk-test".into(),
        model: "text-embedding-3-small".into(),
        base_url: format!("{url}/v1/"),
    };
    let vectors = embedder.embed(&texts()).unwrap();
    assert_eq!(vectors, [vec![1.0, 0.0], vec![0.0, 1.0]]);

    let got = server.join().unwrap();
    assert_eq!(got.path, "/v1/embeddings");
    assert_eq!(got.authorization.as_deref(), Some("Bearer sk-test"));
    assert_eq!(got.body["model"], "text-embedding-3-small");
    assert_eq!(got.body["input"], json!(["first text", "second text"]));
}

#[test]
fn embedders_report_http_errors_and_short_responses() {
    let (url, server) = serve_once(404, json!({ "error": "model \"nope\" not found" }));
    let embedder = OllamaEmbedder {
        model: "nope".into(),
        base_url: url,
    };
    let error = embedder.embed(&texts()).unwrap_err().to_string();
    assert!(
        error.contains("404") && error.contains("not found"),
        "{error}"
    );
    server.join().unwrap();

    // One vector for two texts must not be accepted.
    let (url, server) = serve_once(200, json!({ "embeddings": [[1.0, 0.0]] }));
    let embedder = OllamaEmbedder {
        model: "m".into(),
        base_url: url,
    };
    assert!(embedder.embed(&texts()).is_err());
    server.join().unwrap();
}
