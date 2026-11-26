---
tags: [core, cw-embed]
---

# Server

> A server that speaks the OpenAI embeddings API.

## What it does

`Server` names which OpenAI-compatible server an [[OpenAiEmbedder]] talks to: OpenAI, LM Studio, llama.cpp, Lemonade or mistral.rs. Each value carries the small facts that differ between them: its id, its label in error messages, the setting names for its address and token, and its default address.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `Server`

## How it works

The methods are plain lookups. `id` is the text of the `CODEWIKI_EMBED` choice and the start of a vector's name. `label` is how errors name it. `url_env` and `key_env` give the setting names (mistral.rs has no token setting). `address` reads the address from those settings and falls back to `default_url`, and also reads the token. Defaults are the OpenAI API for OpenAI, port 1234 for LM Studio and mistral.rs, 9931 for llama.cpp and 13305 for Lemonade. [[List Module (cw-embed)]] also uses `Server` to pick which models endpoint and parser to use.

## Items

Declared as `pub enum Server`.

Members:

- `pub fn id(self) -> &'static str` — the provider id: what `CODEWIKI_EMBED` says and a vector's name starts with.
- `pub fn label(self) -> &'static str` — how an error names the server.
- `pub fn url_env(self) -> &'static str` — the setting that holds the server's address.
- `pub fn key_env(self) -> Option<&'static str>` — the setting that holds the server's API token, when it has one.
- `fn address(self, lookup: &impl Fn(&str) -> Option<String>) -> (String, Option<String>)` — the address and API token set for this server, the address falling back to the default.
- `fn default_url(self) -> &'static str` — the address a server listens on when nothing else is set.

## Used by

- [[OpenAiEmbedder]] — from `OpenAiEmbedder`, `check_listed`, `name`, `new`
- [[cw-embed]] — from `a_server_that_does_not_list_the_model_is_not_asked_for_vectors`, `each_local_server_gets_its_path_and_a_token_only_when_one_is_set`, `from_config`, `local_server`
- [[List Module (cw-embed)]] — from `list_models`, `listed_by`

## Gotchas

- LM Studio and mistral.rs share the same default address, so only one can run on it at a time.
- Only OpenAI has a default model in `from_config`. The others have none, because it depends on what the user loaded.
- The `OpenAi` value also stands for any service whose address ends in `/v1`. Its URLs are built differently from the local servers, which use a root plus `/v1`.

## See also

- [[_index]]
- [[cw-embed]]
