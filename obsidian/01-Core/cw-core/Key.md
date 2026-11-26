---
tags: [core, cw-core]
---

# Key

> One setting: the environment variable that names it everywhere, and where it sits in the file.

## What it does

`Key` describes one setting the backend keeps: semantic search provider and model, and the keys and addresses of the embedding servers. It holds the environment variable that names the setting, the section and field where it sits in `settings.json`, whether it is a secret, and a default. One `Key` serves the file, the HTTP settings API and the code that reads the value, so all three use the same name.

## Source

- `crates/cw-core/src/settings.rs` — definition of `Key`

## How it works

Each setting is a constant of type `Key`, such as `EMBED_PROVIDER` or `OPENAI_API_KEY`. `ALL` lists the twelve of them in the order a page shows them. `key` finds a setting by its environment variable, and `key_at` finds it by section and field.

`path` gives the name used in the file and over HTTP, written `section.field`, for example `openai.api_key`. The `secret` flag marks a setting that is stored encrypted and never shown. The `default` is used when neither the variable nor the file gives a value. It is `None` where the default depends on another setting, such as a model that depends on its provider.

## Items

Declared as `pub struct Key`.

Members:

- `pub fn path(&self) -> String` — the setting's name in the file and over HTTP, as `section.field`.
- `pub fn section(&self) -> &'static str` — public method, returns `&'static str`.
- `pub fn field(&self) -> &'static str` — public method, returns `&'static str`.

## Used by

- [[Settings Module (cw-core)]] — from `ALL`, `EMBED_MODEL`, `EMBED_PROVIDER`, `LEMONADE_KEY`
- [[Settings]] — from `clear`, `field`, `put`, `secret`
- [[Settings Module (cw-server)]] — from `ADDRESS_KEYS`, `EMBEDDER_KEYS`, `apply`, `checked`
- [[Resolved]] — from `saved_secret`, `secret_state`, `source`

## Gotchas

- The environment variable wins over the file. A setting made with a variable behaves the same whatever the file says.
- A setting not listed in `ALL` cannot be found by `key` or `key_at`. Add every new `Key` there, and update the array length.
- `section` and `field` are private. Use the `section()` and `field()` methods.
- The mistral.rs server has a URL key and no key for a secret: its server takes none by default.

## See also

- [[_index]]
- [[Settings Module (cw-core)]]
