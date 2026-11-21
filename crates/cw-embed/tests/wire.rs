//! HTTP embedders exercised over real HTTP against a stand-in server.
//!
//! These check the request each embedder sends and that it reads a response
//! in its service's documented shape. They do not replace a run against the
//! real service.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use cw_embed::{Embedder, OllamaEmbedder, OpenAiEmbedder, Server, server_root};
use serde_json::{Value, json};

struct Received {
    path: String,
    authorization: Option<String>,
    body: Value,
}

/// Answer one request on `listener` with `status` and `reply`.
fn answer(listener: &TcpListener, status: u16, reply: &Value) -> Received {
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
}

/// Serve exactly one request with `status` and `reply`.
fn serve_once(status: u16, reply: Value) -> (String, JoinHandle<Received>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || answer(&listener, status, &reply));
    (url, handle)
}

/// Serve the `replies` in order, one request each, and return every request
/// received.
fn serve_many(replies: Vec<(u16, Value)>) -> (String, JoinHandle<Vec<Received>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        replies
            .iter()
            .map(|(status, reply)| answer(&listener, *status, reply))
            .collect()
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
    let embedder = OllamaEmbedder::new(&url, "embeddinggemma");
    let vectors = embedder.embed(&texts()).unwrap();
    assert_eq!(vectors, [vec![0.6, 0.8], vec![0.0, 1.0]]);

    let got = server.join().unwrap();
    assert_eq!(got.path, "/api/embed");
    assert_eq!(got.authorization, None);
    assert_eq!(got.body["model"], "embeddinggemma");
    assert_eq!(
        got.body["input"],
        json!([
            "title: none | text: first text",
            "title: none | text: second text"
        ])
    );
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
    let embedder = OpenAiEmbedder::new(
        Server::OpenAi,
        &format!("{url}/v1/"),
        Some("sk-test"),
        "text-embedding-3-small",
    );
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
    let embedder = OllamaEmbedder::new(&url, "nope");
    let error = embedder.embed(&texts()).unwrap_err().to_string();
    assert!(
        error.contains("404") && error.contains("not found"),
        "{error}"
    );
    server.join().unwrap();

    // One vector for two texts must not be accepted.
    let (url, server) = serve_once(200, json!({ "embeddings": [[1.0, 0.0]] }));
    let embedder = OllamaEmbedder::new(&url, "m");
    assert!(embedder.embed(&texts()).is_err());
    server.join().unwrap();
}

#[test]
fn a_probe_embeds_one_word_and_reports_the_vector_length() {
    let (url, server) = serve_once(200, json!({ "embeddings": [[3.0, 4.0, 0.0]] }));
    let embedder = OllamaEmbedder::new(&url, "embeddinggemma");
    assert_eq!(embedder.probe().unwrap(), 3);
    let got = server.join().unwrap();
    assert_eq!(got.path, "/api/embed");
    assert_eq!(got.body["input"], json!(["title: none | text: ping"]));

    let (url, server) = serve_once(
        200,
        json!({ "data": [{ "index": 0, "embedding": [1.0, 0.0] }] }),
    );
    let embedder = OpenAiEmbedder::new(
        Server::OpenAi,
        &url,
        Some("sk-test"),
        "text-embedding-3-small",
    );
    assert_eq!(embedder.probe().unwrap(), 2);
    let got = server.join().unwrap();
    assert_eq!(got.authorization.as_deref(), Some("Bearer sk-test"));
    assert_eq!(got.body["input"], json!(["ping"]));

    // The local embedder needs no server.
    let local = cw_embed::LocalEmbedder::default();
    assert_eq!(local.probe().unwrap(), local.embed_one("ping").len());
}

#[test]
fn a_failed_probe_carries_the_providers_explanation() {
    let (url, server) = serve_once(
        401,
        json!({ "error": { "message": "Incorrect API key provided" } }),
    );
    let embedder = OpenAiEmbedder::new(
        Server::OpenAi,
        &url,
        Some("sk-wrong"),
        "text-embedding-3-small",
    );
    let error = embedder.probe().unwrap_err().to_string();
    assert!(
        error.contains("401") && error.contains("Incorrect API key"),
        "{error}"
    );
    server.join().unwrap();
}

#[test]
fn each_local_server_gets_its_path_and_a_token_only_when_one_is_set() {
    for (server, key) in [
        (Server::LmStudio, Some("t")),
        (Server::LlamaCpp, None),
        (Server::Lemonade, Some("t")),
        (Server::MistralRs, None),
    ] {
        let (url, srv) = serve_once(
            200,
            json!({ "data": [{ "index": 0, "embedding": [3.0, 4.0] }] }),
        );
        let e = OpenAiEmbedder::new(server, &url, key, "m");
        assert_eq!(e.embed(&["a".into()]).unwrap(), [vec![0.6, 0.8]]);
        let got = srv.join().unwrap();
        assert_eq!(got.path, "/v1/embeddings");
        assert_eq!(got.authorization.is_some(), key.is_some());
        assert_eq!(got.body["model"], "m");
        assert_eq!(
            got.body.get("truncate_sequence").is_some(),
            server == Server::MistralRs
        );
        assert_eq!(e.name(), format!("{}:m", server.id()));
    }
}

#[test]
fn an_address_ending_in_v1_is_the_server_root() {
    let (url, srv) = serve_once(
        200,
        json!({ "data": [{ "index": 0, "embedding": [1.0, 0.0] }] }),
    );
    let e = OpenAiEmbedder::new(Server::LmStudio, &format!("{url}/v1/"), None, "m");
    e.embed(&["a".into()]).unwrap();
    assert_eq!(srv.join().unwrap().path, "/v1/embeddings");
    assert_eq!(server_root("http://h:1/v1/"), "http://h:1");
    assert_eq!(server_root("http://h:1/v1"), "http://h:1");
    assert_eq!(server_root("http://h:1/"), "http://h:1");
    assert_eq!(server_root("http://h:1"), "http://h:1");
}

#[test]
fn a_refused_token_says_which_setting_holds_it() {
    let (url, srv) = serve_once(401, json!({ "error": { "message": "Unauthorized" } }));
    let e = OpenAiEmbedder::new(Server::LmStudio, &url, None, "m");
    let error = e.embed(&["a".into()]).unwrap_err().to_string();
    assert!(
        error.contains("CODEWIKI_LMSTUDIO_KEY") && error.contains("token"),
        "{error}"
    );
    srv.join().unwrap();
}

#[test]
fn a_rate_limit_is_not_retried_even_when_it_says_exceeded() {
    let (url, srv) = serve_many(vec![(
        429,
        json!({ "error": { "message": "You exceeded your current quota" } }),
    )]);
    let e = OpenAiEmbedder::new(Server::OpenAi, &url, Some("k"), "m");
    let error = e.embed(&["a".into()]).unwrap_err().to_string();
    assert!(error.contains("429"), "{error}");
    assert_eq!(srv.join().unwrap().len(), 1);
}

#[test]
fn a_batch_refused_as_too_long_is_sent_again_with_shorter_texts() {
    let too_long = json!({ "error": { "message": "input is too large to process. increase the physical batch size" } });
    let two = json!({ "data": [
        { "index": 0, "embedding": [1.0, 0.0] },
        { "index": 1, "embedding": [0.0, 1.0] },
    ]});
    let (url, srv) = serve_many(vec![(400, too_long.clone()), (200, two)]);
    let e = OpenAiEmbedder::new(Server::LlamaCpp, &url, None, "m");
    let texts = vec!["x".repeat(9_000), "short".to_string()];
    assert_eq!(e.embed(&texts).unwrap().len(), 2);
    let got = srv.join().unwrap();
    assert_eq!(got.len(), 2);
    let second = got[1].body["input"].as_array().unwrap();
    assert!(second[0].as_str().unwrap().len() <= 2_000);
    assert_eq!(second[1], "short");

    // A 400 that is not about length is not retried.
    let (url, srv) = serve_many(vec![(
        400,
        json!({ "error": { "message": "model not loaded" } }),
    )]);
    let e = OpenAiEmbedder::new(Server::LlamaCpp, &url, None, "m");
    assert!(e.embed(&texts).is_err());
    assert_eq!(srv.join().unwrap().len(), 1);
}

#[test]
fn a_known_model_gets_its_prefixes_and_a_name_that_says_so() {
    let (url, srv) = serve_many(vec![
        (200, json!({ "embeddings": [[1.0, 0.0]] })),
        (200, json!({ "embeddings": [[1.0, 0.0]] })),
    ]);
    let e = OllamaEmbedder::new(&url, "embeddinggemma");
    assert_eq!(e.name(), "ollama:embeddinggemma+gemma");
    e.embed(&["a".into()]).unwrap();
    e.embed_query("q").unwrap();
    let got = srv.join().unwrap();
    assert_eq!(got[0].body["input"], json!(["title: none | text: a"]));
    assert_eq!(
        got[1].body["input"],
        json!(["task: search result | query: q"])
    );

    // A model nobody has a rule for keeps its name and its text.
    let one = json!({ "data": [{ "index": 0, "embedding": [1.0, 0.0] }] });
    let (url, srv) = serve_many(vec![(200, one.clone()), (200, one)]);
    let e = OpenAiEmbedder::new(Server::OpenAi, &url, Some("k"), "text-embedding-3-small");
    assert_eq!(e.name(), "openai:text-embedding-3-small");
    e.embed(&["a".into()]).unwrap();
    e.embed_query("q").unwrap();
    let got = srv.join().unwrap();
    assert_eq!(got[0].body["input"], json!(["a"]));
    assert_eq!(got[1].body["input"], json!(["q"]));
}

#[test]
fn the_retry_for_a_too_long_batch_cuts_the_prefixed_text() {
    let too_long = json!({ "error": { "message": "input is too large to process" } });
    let one = json!({ "data": [{ "index": 0, "embedding": [1.0, 0.0] }] });
    let (url, srv) = serve_many(vec![(400, too_long), (200, one)]);
    let e = OpenAiEmbedder::new(Server::LlamaCpp, &url, None, "nomic-embed-text-v1.5");
    assert_eq!(e.name(), "llamacpp:nomic-embed-text-v1.5+nomic");
    e.embed(&["x".repeat(9_000)]).unwrap();
    let got = srv.join().unwrap();
    let first = got[0].body["input"][0].as_str().unwrap();
    assert!(first.starts_with("search_document: x") && first.len() == 8_000);
    let second = got[1].body["input"][0].as_str().unwrap();
    assert!(second.starts_with("search_document: x") && second.len() == 2_000);
}
