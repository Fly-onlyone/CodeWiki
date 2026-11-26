---
tags: [core, cw-embed, module]
---

# List Module (cw-embed)

> The models a server offers, for the page that lets someone pick one.

## What it does

This module asks an embedding server which models it offers, so the Settings page can show a pick list instead of making the user type a name. Each server answers in its own shape, so there is one parser for each. A model that is known not to embed text (a chat model) is dropped, and the ones that embed come first. The same list also lets [[OpenAiEmbedder]] check that a server really serves the chosen model.

## Source

- `crates/cw-embed/src/list.rs` — module `cw_embed::list`

## How it works

`list_models` reads the chosen provider from the settings lookup. `local` and `none` have no list and give `None`. Ollama has its own path: `list_ollama` fetches the tag list, then asks `/api/show` for each model to learn whether it embeds. Every other server goes through `listed_by`, which picks the URL and parser for its [[Server]]: OpenAI at `/models`, LM Studio at `/api/v0/models`, and Lemonade, llama.cpp and mistral.rs at `/v1/models`. `fetch` makes the GET with a 10 second timeout and turns an HTTP refusal into an error that names the server. `arrange` then removes models marked as not embedding and sorts known embedders before unknown ones, each group by id.

## Items

- `const LIST_TIMEOUT: Duration` — how long a model list may take. Someone is watching a page.
- `pub struct Listed` — one model a server offers.
- `fn unknown(id: &str, embedding: Option<bool>) -> Listed` — private function, returns `Listed`; called by `list_ollama`, `parse_ids`, `parse_lemonade`, `parse_openai_models`.
- `fn parse_lm_studio(value: &Value) -> Vec<Listed>` — LM Studio's `GET /api/v0/models`: `type` is `llm`, `vlm` or `embeddings`, `state` is `loaded` or `not-loaded`. https://lmstudio.ai/docs/developer/rest/endpoints.
- `fn parse_ids(value: &Value) -> Vec<Listed>` — the ids of an OpenAI-style `GET /v1/models`, with nothing known about them. llama.cpp serves `/models` and `/v1/models` from one handler, in router mode too (verified: both routes call `routes.get_models`), and a router entry carries a `status` of `loaded`, `unloaded` and so on, so `/v1/models` is the right path and `/models` is not needed. mistral.rs lists what it serves. https://github.com/ggml-org/llama.cpp/blob/master/tools/server/server.cpp https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md.
- `fn parse_lemonade(value: &Value) -> Vec<Listed>` — Lemonade's `GET /v1/models`: `labels` holds what a model does, such as `chat`, `embeddings` or `reranking`. A model without `labels` is unknown. https://github.com/lemonade-sdk/lemonade/blob/main/docs/api/openai.md.
- `fn parse_openai_models(value: &Value) -> Vec<Listed>` — OpenAI's `GET /models`. The list holds every model, and only the id says what one does.
- `fn parse_ollama_tags(value: &Value) -> Vec<String>` — the names in Ollama's `GET /api/tags`.
- `fn ollama_embeds(value: &Value) -> Option<bool>` — whether Ollama's `POST /api/show` answer says the model embeds. Its `capabilities` is a list such as `["completion", "vision"]`; a model that embeds has `embedding` in it. An answer without the list is unknown (an older Ollama). https://docs.ollama.com/api/show https://github.com/ollama/ollama/blob/main/docs/api.md.
- `fn arrange(mut models: Vec<Listed>) -> Vec<Listed>` — known embedding models first, then unknown ones, each by id. Models known not to embed are left out.
- `fn fetch(label: &str, url: &str, key: Option<&str>) -> Result<Value>` — the JSON a GET answers, or the error for a refusal.
- `fn list_ollama(base: &str) -> Result<Vec<Listed>>` — the models of Ollama at `base`. A model whose details cannot be read is listed with embedding unknown.
- `pub fn list_models(lookup: impl Fn(&str) -> Option<String>) -> Result<Option<Vec<Listed>>>` — the models the provider chosen by `lookup` offers. `None` for a provider with no list (`local`, `none`). An error names the server that did not answer.
- `pub(crate) fn listed_by(server: Server, base: &str, key: Option<&str>) -> Result<Vec<Listed>>` — the models `server` at `base` lists, embedding ones first. An error names the server.

And 1 more private item with no comment.

## Depends on

- [[cw-embed]] — uses `EMBED_ENV`, `OLLAMA_DEFAULT_URL`, `PROVIDERS`, `get_json`
- [[Server]] — uses `Server`, `address`, `label`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `invalid`

## Used by

- [[cw-embed]] — from `a_server_lists_with_its_token_and_path`, `a_server_that_does_not_answer_is_an_error_naming_it`, `an_ollama_model_whose_details_fail_is_still_listed`, `list_models_in`
- [[OpenAiEmbedder]] — from `check_listed`

## Gotchas

- `embedding` is a three-way value. `Some(false)` is removed; `None` (the server does not say) is kept but sorted last. llama.cpp and mistral.rs say nothing, so all their models are unknown.
- OpenAI is judged by the id containing "embed". A model with another name would be hidden.
- Ollama is asked once per model, so a server with many models makes many calls. One that fails to answer only leaves that model unknown.
- Parsers follow each server's documentation. A server that changes its format gives an empty list, not an error.
- The 10 second limit exists because a person is watching the page.

## See also

- [[_index]]
- [[cw-embed]]
