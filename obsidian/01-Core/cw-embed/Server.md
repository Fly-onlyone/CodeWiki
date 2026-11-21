---
tags: [core, cw-embed]
---

# Server

> A server that speaks the OpenAI embeddings API.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `Server`

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

- [[OpenAiEmbedder]] — from `OpenAiEmbedder`, `embed_within`, `name`, `new`
- [[cw-embed]] — from `each_local_server_gets_its_path_and_a_token_only_when_one_is_set`, `from_config`, `local_server`
- [[List Module (cw-embed)]] — from `list_models`

## See also

- [[_index]]
- [[cw-embed]]
