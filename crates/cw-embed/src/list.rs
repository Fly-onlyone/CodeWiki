//! The models a server offers, for the page that lets someone pick one.
//!
//! Each server lists its models in its own shape. The parsers below read the
//! shapes the servers' documentation gives; [`arrange`] puts the models that
//! embed first and drops the ones known not to.

use std::time::Duration;

use cw_core::{Error, Result};
use serde_json::{Value, json};

use crate::{
    EMBED_ENV, OLLAMA_DEFAULT_URL, Posted, Server, get_json, post_json, refused, server_root,
};

/// How long a model list may take. Someone is watching a page.
const LIST_TIMEOUT: Duration = Duration::from_secs(10);

/// One model a server offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub id: String,
    /// Whether the model embeds text; `None` when the server does not say.
    pub embedding: Option<bool>,
    /// Whether the server holds the model in memory; `None` when unknown.
    pub loaded: Option<bool>,
}

fn unknown(id: &str, embedding: Option<bool>) -> Listed {
    Listed {
        id: id.to_string(),
        embedding,
        loaded: None,
    }
}

fn rows(value: &Value) -> impl Iterator<Item = &Value> {
    value["data"].as_array().into_iter().flatten()
}

/// LM Studio's `GET /api/v0/models`: `type` is `llm`, `vlm` or `embeddings`,
/// `state` is `loaded` or `not-loaded`.
/// https://lmstudio.ai/docs/developer/rest/endpoints
fn parse_lm_studio(value: &Value) -> Vec<Listed> {
    rows(value)
        .filter_map(|row| {
            let id = row["id"].as_str()?;
            let kind = row["type"].as_str()?;
            Some(Listed {
                id: id.to_string(),
                embedding: Some(matches!(kind, "embeddings" | "embedding")),
                loaded: row["state"].as_str().map(|state| state == "loaded"),
            })
        })
        .collect()
}

/// The ids of an OpenAI-style `GET /v1/models`, with nothing known about them.
/// llama.cpp serves `/models` and `/v1/models` from one handler, in router
/// mode too (verified: both routes call `routes.get_models`), and a router
/// entry carries a `status` of `loaded`, `unloaded` and so on, so `/v1/models`
/// is the right path and `/models` is not needed. mistral.rs lists what it
/// serves.
/// https://github.com/ggml-org/llama.cpp/blob/master/tools/server/server.cpp
/// https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md
fn parse_ids(value: &Value) -> Vec<Listed> {
    rows(value)
        .filter_map(|row| Some(unknown(row["id"].as_str()?, None)))
        .collect()
}

/// Lemonade's `GET /v1/models`: `labels` holds what a model does, such as
/// `chat`, `embeddings` or `reranking`. A model without `labels` is unknown.
/// https://github.com/lemonade-sdk/lemonade/blob/main/docs/api/openai.md
fn parse_lemonade(value: &Value) -> Vec<Listed> {
    rows(value)
        .filter_map(|row| {
            let embedding = row["labels"]
                .as_array()
                .map(|labels| labels.iter().any(|label| label == "embeddings"));
            Some(unknown(row["id"].as_str()?, embedding))
        })
        .collect()
}

/// OpenAI's `GET /models`. The list holds every model, and only the id says
/// what one does.
fn parse_openai_models(value: &Value) -> Vec<Listed> {
    rows(value)
        .filter_map(|row| {
            let id = row["id"].as_str()?;
            Some(unknown(id, Some(id.contains("embed"))))
        })
        .collect()
}

/// The names in Ollama's `GET /api/tags`.
fn parse_ollama_tags(value: &Value) -> Vec<String> {
    value["models"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| row["name"].as_str().map(str::to_string))
        .collect()
}

/// Whether Ollama's `POST /api/show` answer says the model embeds. Its
/// `capabilities` is a list such as `["completion", "vision"]`; a model that
/// embeds has `embedding` in it. An answer without the list is unknown (an
/// older Ollama).
/// https://docs.ollama.com/api/show
/// https://github.com/ollama/ollama/blob/main/docs/api.md
fn ollama_embeds(value: &Value) -> Option<bool> {
    let capabilities = value["capabilities"].as_array()?;
    Some(capabilities.iter().any(|c| c == "embedding"))
}

/// Known embedding models first, then unknown ones, each by id. Models known
/// not to embed are left out.
fn arrange(mut models: Vec<Listed>) -> Vec<Listed> {
    models.retain(|model| model.embedding != Some(false));
    models.sort_by(|a, b| (a.embedding.is_none(), &a.id).cmp(&(b.embedding.is_none(), &b.id)));
    models
}

/// The JSON a GET answers, or the error for a refusal.
fn fetch(label: &str, url: &str, key: Option<&str>) -> Result<Value> {
    match get_json(label, url, key, LIST_TIMEOUT)? {
        Posted::Done(value) => Ok(value),
        Posted::Refused { status, detail } => Err(refused(label, status, &detail)),
    }
}

/// The models of Ollama at `base`. A model whose details cannot be read is
/// listed with embedding unknown.
fn list_ollama(base: &str) -> Result<Vec<Listed>> {
    let base = base.trim_end_matches('/');
    let tags = fetch("Ollama", &format!("{base}/api/tags"), None)?;
    Ok(parse_ollama_tags(&tags)
        .into_iter()
        .map(|name| {
            let shown = post_json(
                "Ollama",
                &format!("{base}/api/show"),
                None,
                &json!({ "model": name }),
                LIST_TIMEOUT,
            );
            let embedding = match shown {
                Ok(Posted::Done(value)) => ollama_embeds(&value),
                _ => None,
            };
            unknown(&name, embedding)
        })
        .collect())
}

/// The models the provider chosen by `lookup` offers. `None` for a provider
/// with no list (`local`, `none`). An error names the server that did not
/// answer.
pub fn list_models(lookup: impl Fn(&str) -> Option<String>) -> Result<Option<Vec<Listed>>> {
    let chosen = lookup(EMBED_ENV).map(|v| v.to_lowercase());
    let server = match chosen.as_deref() {
        None | Some("local" | "none" | "builtin") => return Ok(None),
        Some("ollama") => {
            let base = lookup("OLLAMA_HOST").unwrap_or_else(|| OLLAMA_DEFAULT_URL.into());
            return list_ollama(&base).map(|models| Some(arrange(models)));
        }
        Some("openai") => Server::OpenAi,
        Some("lmstudio") => Server::LmStudio,
        Some("llamacpp") => Server::LlamaCpp,
        Some("lemonade") => Server::Lemonade,
        Some("mistralrs") => Server::MistralRs,
        Some(other) => {
            return Err(Error::invalid(format!(
                "unknown embedder '{other}'; use {}",
                crate::PROVIDERS.join(", ")
            )));
        }
    };
    let (base, key) = server.address(&lookup);
    let root = server_root(&base);
    let (url, parse): (String, fn(&Value) -> Vec<Listed>) = match server {
        Server::OpenAi => (
            format!("{}/models", base.trim_end_matches('/')),
            parse_openai_models,
        ),
        Server::LmStudio => (format!("{root}/api/v0/models"), parse_lm_studio),
        Server::Lemonade => (format!("{root}/v1/models"), parse_lemonade),
        Server::LlamaCpp | Server::MistralRs => (format!("{root}/v1/models"), parse_ids),
    };
    let value = fetch(server.label(), &url, key.as_deref())?;
    Ok(Some(arrange(parse(&value))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Fixtures are copied from each server's documentation (URLs beside the
    // parsers above). The Lemonade one is trimmed to the `id` and `labels`
    // fields its docs show.
    #[test]
    fn lm_studio_marks_its_embedding_models() {
        let listing = json!({ "object": "list", "data": [
            { "id": "qwen2-vl-7b-instruct", "object": "model", "type": "vlm",
              "state": "not-loaded", "max_context_length": 32768 },
            { "id": "meta-llama-3.1-8b-instruct", "object": "model", "type": "llm",
              "state": "not-loaded", "max_context_length": 131072 },
            { "id": "text-embedding-nomic-embed-text-v1.5", "object": "model",
              "type": "embeddings", "state": "not-loaded", "max_context_length": 2048 },
        ]});
        assert_eq!(
            arrange(parse_lm_studio(&listing)),
            [Listed {
                id: "text-embedding-nomic-embed-text-v1.5".into(),
                embedding: Some(true),
                loaded: Some(false),
            }]
        );
        let loaded = json!({ "data": [{ "id": "e", "type": "embedding", "state": "loaded" }] });
        assert_eq!(parse_lm_studio(&loaded)[0].loaded, Some(true));
    }

    #[test]
    fn an_openai_list_keeps_the_embedding_ids() {
        let listing = json!({ "object": "list", "data": [
            { "id": "gpt-4o", "object": "model" },
            { "id": "text-embedding-3-small", "object": "model" },
            { "id": "text-embedding-ada-002", "object": "model" },
        ]});
        let ids: Vec<String> = arrange(parse_openai_models(&listing))
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert_eq!(ids, ["text-embedding-3-small", "text-embedding-ada-002"]);
    }

    #[test]
    fn lemonade_reads_the_labels() {
        let listing = json!({ "object": "list", "data": [
            { "id": "Qwen3-4B-GGUF", "labels": ["chat", "reasoning"] },
            { "id": "nomic-embed-text-v1-GGUF", "labels": ["embeddings"] },
            { "id": "mystery" },
        ]});
        let listed = arrange(parse_lemonade(&listing));
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].id, "nomic-embed-text-v1-GGUF");
        assert_eq!(listed[0].embedding, Some(true));
        assert_eq!(listed[1].id, "mystery");
        assert_eq!(listed[1].embedding, None);
    }

    #[test]
    fn the_order_puts_known_embedding_models_first() {
        let model = |id: &str, embedding| Listed {
            id: id.into(),
            embedding,
            loaded: None,
        };
        let ids: Vec<String> = arrange(vec![
            model("b", None),
            model("z", Some(true)),
            model("chat", Some(false)),
            model("a", None),
            model("m", Some(true)),
        ])
        .into_iter()
        .map(|m| m.id)
        .collect();
        assert_eq!(ids, ["m", "z", "a", "b"]);
    }
}
