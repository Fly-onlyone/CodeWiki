---
tags: [core, cw-embed]
---

# OpenAiEmbedder

> Any endpoint that speaks the OpenAI embeddings API.

## What it does

`OpenAiEmbedder` gets vectors from any server that takes the OpenAI embeddings request. That covers OpenAI itself, LM Studio, llama.cpp, Lemonade and mistral.rs. Its [[Server]] value says which one, and so which address, token setting and quirks apply. Because many models need a special prefix before queries or documents, it looks up a rule in [[Prefix Module]] by model name.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `OpenAiEmbedder`

## How it works

`embed_within` splits the texts into batches of 64. For each batch:

1. `check_listed` runs, but only for llama.cpp and mistral.rs, and only until the first success.
2. Each text gets the prefix, then is clipped to 8,000 bytes.
3. The batch is posted to `/embeddings` (OpenAI) or `/v1/embeddings` (the others).
4. If the server says the input is too long, the batch is sent again cut to 2,000 characters.
5. If that is refused too, `one_by_one` sends each text alone, and a text refused again is cut to 400 characters.
6. `vectors` checks that the count and lengths match, or turns a refusal into an error.

A successful batch sets the `warm` flag. Before that the timeout is 300 seconds, because a local server loads its model on the first request. After it, the timeout is 120 seconds. `embed_query` uses the query prefix; `embed` and `probe` use the document prefix.

## Items

Declared as `pub struct OpenAiEmbedder`.

Members:

- `pub fn new(server: Server, base_url: &str, api_key: Option<&str>, model: &str) -> Self` — public method, returns `Self`; called by `from_config`, `local_server`.
- `fn timeout(&self) -> Duration` — how long a batch may take: until the first success, long enough for the server to load the model.
- `fn refusal(&self, status: u16, detail: &str) -> Error` — the error for a refused batch. A local server that asks for a token says where to set it.
- `fn check_listed(&self) -> Result<()>` — llama.cpp and mistral.rs embed with the model they hold, whatever model a request names: a model swapped in the server would make vectors under the name of the one chosen here, and two models' vectors would be compared. Until a call succeeds, the server's own list must hold the chosen model. A list that cannot be read is an error too.
- `fn vectors(&self, posted: Posted, count: usize) -> Result<Vec<Vec<f32>>>` — the `count` vectors of an answer, or the error of a refusal.
- `one_by_one` — embed `texts` one per request, each cut to `SHORT_CHARS`, and a text refused alone as too long once more at `TINY_CHARS`. The vectors keep the order of the texts.
- `embed_within` — embed `texts`, each with `prefix` in front.
- `fn name(&self) -> String` — public method, returns `String`; called by `OpenAiEmbedder::embed_query`.
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>` — public method, returns `Result<Vec<Vec<f32>>>`.
- `fn embed_query(&self, text: &str) -> Result<Vec<f32>>` — public method, returns `Result<Vec<f32>>`.
- `fn probe(&self) -> Result<usize>` — public method, returns `Result<usize>`.

And 2 more private items with no comment.

## Depends on

- [[cw-embed]] — uses `BATCH`, `FIRST_LOAD_TIMEOUT`, `PROBE_TEXT`, `PROBE_TIMEOUT`
- [[Server]] — uses `Server`, `id`, `key_env`, `label`
- [[Error]] — uses `Error`, `external`, `invalid`
- [[Prefix Module]] — uses `Rule`, `rule`
- [[Error Module]] — uses `Result`
- [[List Module (cw-embed)]] — uses `listed_by`
- [[Embedder]] — uses `Embedder`

## Used by

- [[cw-embed]] — from `a_batch_refused_as_too_long_is_sent_again_with_shorter_texts`, `a_batch_refused_twice_is_sent_text_by_text`, `a_failed_probe_carries_the_providers_explanation`, `a_known_model_gets_its_prefixes_and_a_name_that_says_so`

## Gotchas

- llama.cpp and mistral.rs embed with whatever model they hold, even if the request names another. `check_listed` therefore compares the server's list with the chosen model first; a server that does not list it, or cannot be listed, gives an error.
- mistral.rs gets `truncate_sequence` set in each request.
- A 401 or 403 from a local server says which setting holds its token. For OpenAI the plain refusal is used.
- A "too long" refusal is recognised only for status 400, 413, 422 or 500 with words like "context" or "exceed" in the message. A rate limit or bad token is never retried as too long.
- The vector name is the provider, the model, and `+id` when a prefix rule applies.

## See also

- [[_index]]
- [[cw-embed]]
