---
tags: [core, cw-server, module]
---

# Settings Module (cw-server)

> What the settings routes do: describe the settings in effect, and change the saved ones.

## What it does

Backs the settings routes. It describes the settings in effect, changes the saved ones, tests the chosen embedder and lists the models its server offers. A description never holds a secret. A key shows as unset, saved, given by the environment, or unreadable, with the last characters of a saved one. A setting whose environment variable is set cannot be changed here, because the variable wins.

## Source

- `crates/cw-server/src/settings.rs` — module `cw_server::settings`

## How it works

`resolved` merges the saved settings file with the environment into a `Resolved`. `describe` walks every known key and reports its value, its source and its default. It also adds the provider options and the name of the embedder the settings select, or the problem that stops one.

`change` takes the settings lock and reads the body as sections that hold fields. For each field it checks the key exists and the environment does not set it, then `apply` does the work. `null` or empty text clears a setting. Plain text goes through `checked`: a provider must be on the known list, and an address must start with http:// or https://, with the end slash removed. A secret takes `{"set": ...}` or `{"remove": true}`. All changes happen inside `settings::update`, so one bad field leaves everything unchanged. If an embedder key changed, semantic search is configured again.

`test_embedder` embeds one word with `probe`. On success it calls `resume`, so documents that wait for vectors get embedded. `embed_models` lists the provider's models and never fails: an unreachable server shows as `reachable: false` with a problem.

## Items

- `const EMBEDDER_KEYS: [Key; 12]` — settings that decide which embedder semantic search uses.
- `const ADDRESS_KEYS: [Key; 6]` — settings that hold the address of a server.
- `pub(crate) fn resolved(state: &AppState) -> Resolved` — the settings in effect for this server.
- `fn message(error: &Error) -> String` — an error's text for a person: without the prefix that tells kinds of error apart.
- `selected` — the name of what the settings select, or why they select nothing usable.
- `pub(crate) fn describe(state: &AppState) -> Value` — every setting with its value in effect and where that comes from.
- `fn test_result(name: Option<String>, outcome: Result<Value>, started: Instant) -> Value` — what a test of a provider found: `ok`, the provider's name, and either how long it took (with `extra`, what the answer showed) or its error.
- `pub(crate) fn test_embedder(state: &AppState) -> Value` — have the chosen embedder embed one word. On success `dimensions` is the length of its vectors, and documents that still wait for a vector are embedded again: a job that failed stays failed until something tries, and a Test that succeeds is that try. Called in a blocking task of a handler.
- `pub(crate) fn embed_models(state: &AppState) -> Value` — the models the chosen provider offers, for a page that lets a person pick one. `models` is null when the provider lists none; `reachable` is false when the server did not answer, and `problem` says why.
- `fn checked(key: Key, value: &str) -> Result<String>` — A value that passed the checks of its setting, as it is saved.
- `fn apply(saved: &mut Settings, key: Key, value: &Value) -> Result<()>` — apply one field of a change. `null` clears a setting. A plain setting takes text; a secret takes `{"set": "<secret>"}` or `{"remove": true}`.
- `pub(crate) fn change(state: &AppState, body: &Value) -> Result<Value>` — change the saved settings named in `body`, an object of sections holding fields, and return the new description. Fields not named stay as they are. One field that cannot be applied leaves everything unchanged.

## Depends on

- [[Settings Module (cw-core)]] — uses `ALL`, `EMBED_MODEL`, `EMBED_PROVIDER`, `LEMONADE_KEY`
- [[Resolved]] — uses `Resolved`, `from_parts`, `secret_state`, `source`
- [[cw-embed]] — uses `PROVIDERS`, `from_settings`, `list_models_in`
- [[Error]] — uses `Error`, `invalid`
- [[Error Module]] — uses `Result`
- [[Secret Module]] — uses `available`
- [[Embed Job Module]] — uses `resume`
- [[Semantic Module]] — uses `configure`

## Used by

- [[Api Module]] — from `embed_models`, `read_settings`, `test_embedder`, `write_settings`
- [[cw-server]] — from `serve`

## Gotchas

- Changing the provider without naming a model clears the saved model, since a model belongs to one provider. A model set by the environment stays.
- If the system cannot store secrets, saving one fails with a hint to use the environment variable.
- `settings_lock` is separate from the write lock, so a settings change never waits for a long update.
- A successful Test is the only retry for a failed embedding job, besides an update or a restart.

## See also

- [[_index]]
- [[cw-server]]
