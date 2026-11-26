---
tags: [core, codewiki-app]
---

# FieldId

> A setting the user types: each has one text input, which lives as long as the app does.

## What it does

`FieldId` names one setting that the user types. There are 13: the OpenAI key and base URL, the Ollama host, the URL and key of LM Studio, llama.cpp and Lemonade, the Mistral.rs URL, the embedding model, and the address and token of a remote backend. Each has one text input, kept as long as the app lives. The id ties a `Field` or `Secret` [[Block]] on the page to that input.

## Source

- `crates/codewiki-app/src/settings.rs` — definition of `FieldId`

## How it works

`place` says where the setting is kept: a section and field name in the backend's `GET /v1/settings` answer, such as `openai` and `api_key`. For the remote address and token it returns `None`, because the app keeps those two itself.

`secret` marks the five fields whose text is hidden and never shown again once saved: the four keys and the remote token. `placeholder` is the hint an empty input shows, such as a local server's default address. `ALL` lists every id so the app can build all inputs at start and commit them.

## Items

Declared as `pub enum FieldId`.

Members:

- `pub const ALL: [Self; 13]` — public constant; used by `AppView::commit_settings_fields`, `AppView::take_settings`, `State::new`.
- `pub fn place(self) -> Option<(&'static str, &'static str)>` — where the backend keeps the setting: its section and field in `GET /v1/settings`. `None` for the two the app keeps itself, the address and token of a backend on another machine.
- `pub fn secret(self) -> bool` — whether what is typed is hidden, and never shown again once saved.
- `pub fn placeholder(self) -> &'static str` — what an empty input shows.

## Used by

- [[codewiki-app]] — from `commit_settings_field`, `commit_settings_fields`, `connect_remote`, `forget_saved_note`
- [[Settings Pages Module]] — from `Model`, `entry`, `field`, `secret`
- [[Doc View Module]] — from `Controls`, `secret_control`
- [[Settings View Module]] — from `State`, `new`
- [[Action]] — from `Action`
- [[Block]] — from `Block`
- [[Doc]] — from `fields`

## Gotchas

- Every field kept by the backend must have a place of its own. A test checks that no two share one.
- Every secret kept by the backend must have the field name `api_key`. A test checks that too.
- A new field needs an entry in `ALL`, `place`, `placeholder` and, for a secret, `secret`. `ALL` has a fixed length of 13, so the compiler flags a missing one.
- A remote token is a secret and is kept by the app, not by the backend.

## See also

- [[_index]]
- [[Settings Module]]
