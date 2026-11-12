//! Providers exercised over real HTTP against a stand-in server.
//!
//! These check what goes on the wire (path, headers, body) and that each
//! provider reads a response in its service's documented shape. They do not
//! replace a run against the real service.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

use cw_llm::{Anthropic, LlmProvider, Ollama, OpenAiCompatible};
use serde_json::{Value, json};

/// What the stand-in server received.
struct Received {
    path: String,
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

/// Serve exactly one request with `status` and `reply`, and hand back what
/// was received. Returns the base URL to point a provider at.
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
            headers,
            body: serde_json::from_slice(&body).unwrap_or(Value::Null),
        }
    });
    (url, handle)
}

#[test]
fn anthropic_sends_the_messages_request_and_reads_text_blocks() {
    let (url, server) = serve_once(
        200,
        json!({
            "stop_reason": "end_turn",
            "content": [
                { "type": "thinking", "thinking": "" },
                { "type": "text", "text": "Hello " },
                { "type": "text", "text": "world" },
            ],
            "usage": { "input_tokens": 12, "output_tokens": 3 },
        }),
    );
    let provider = Anthropic {
        api_key: "key-123".into(),
        model: "claude-opus-5-5".into(),
        base_url: url,
    };
    let completion = provider.complete("be brief", "say hello").unwrap();
    assert_eq!(completion.text, "Hello world");
    assert_eq!((completion.input_tokens, completion.output_tokens), (12, 3));

    let got = server.join().unwrap();
    assert_eq!(got.path, "/v1/messages");
    assert_eq!(got.header("x-api-key"), Some("key-123"));
    assert!(got.header("anthropic-version").is_some());
    assert_eq!(got.body["model"], "claude-opus-5-5");
    assert_eq!(got.body["system"], "be brief");
    assert_eq!(got.body["messages"][0]["role"], "user");
    assert_eq!(got.body["messages"][0]["content"], "say hello");
    assert!(got.body["max_tokens"].as_u64().unwrap() > 0);
}

#[test]
fn anthropic_reports_refusals_and_http_errors() {
    let (url, server) = serve_once(
        200,
        json!({ "stop_reason": "refusal", "stop_details": { "category": "cyber" }, "content": [] }),
    );
    let provider = Anthropic {
        api_key: "k".into(),
        model: "m".into(),
        base_url: url,
    };
    let error = provider.complete("s", "u").unwrap_err().to_string();
    assert!(
        error.contains("declined") && error.contains("cyber"),
        "{error}"
    );
    server.join().unwrap();

    let (url, server) = serve_once(
        401,
        json!({ "type": "error", "error": { "type": "authentication_error", "message": "invalid x-api-key" } }),
    );
    let provider = Anthropic {
        api_key: "bad".into(),
        model: "m".into(),
        base_url: url,
    };
    let error = provider.complete("s", "u").unwrap_err().to_string();
    assert!(
        error.contains("401") && error.contains("invalid x-api-key"),
        "{error}"
    );
    server.join().unwrap();
}

#[test]
fn openai_compatible_sends_chat_completions_with_a_bearer_token() {
    let (url, server) = serve_once(
        200,
        json!({
            "choices": [{ "message": { "role": "assistant", "content": "four" } }],
            "usage": { "prompt_tokens": 9, "completion_tokens": 1 },
        }),
    );
    let provider = OpenAiCompatible {
        api_key: "sk-test".into(),
        model: "some-model".into(),
        base_url: format!("{url}/v1"),
    };
    let completion = provider.complete("sys", "2+2?").unwrap();
    assert_eq!(completion.text, "four");
    assert_eq!((completion.input_tokens, completion.output_tokens), (9, 1));

    let got = server.join().unwrap();
    assert_eq!(got.path, "/v1/chat/completions");
    assert_eq!(got.header("authorization"), Some("Bearer sk-test"));
    assert_eq!(got.body["model"], "some-model");
    assert_eq!(got.body["messages"][0]["role"], "system");
    assert_eq!(got.body["messages"][0]["content"], "sys");
    assert_eq!(got.body["messages"][1]["role"], "user");
    assert_eq!(got.body["messages"][1]["content"], "2+2?");
}

#[test]
fn ollama_sends_a_non_streaming_chat_request() {
    let (url, server) = serve_once(
        200,
        json!({
            "message": { "role": "assistant", "content": "ok" },
            "prompt_eval_count": 7,
            "eval_count": 2,
            "done": true,
        }),
    );
    let provider = Ollama {
        model: "llama".into(),
        base_url: format!("{url}/"),
    };
    let completion = provider.complete("sys", "hi").unwrap();
    assert_eq!(completion.text, "ok");
    assert_eq!((completion.input_tokens, completion.output_tokens), (7, 2));

    let got = server.join().unwrap();
    assert_eq!(got.path, "/api/chat");
    assert_eq!(got.body["model"], "llama");
    assert_eq!(got.body["stream"], false);
    assert_eq!(got.body["messages"][1]["content"], "hi");
}
