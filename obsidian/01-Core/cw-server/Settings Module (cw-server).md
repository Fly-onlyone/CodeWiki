---
tags: [core, cw-server, module]
---

# Settings Module (cw-server)

> What the settings routes do: describe the settings in effect, and change the saved ones.

## Source

- `crates/cw-server/src/settings.rs` — module `cw_server::settings`

## How it works

Main items:

- `message` — an error's text for a person: without the prefix that tells kinds of error apart.
- `resolved` — the settings in effect for this server.
- `test_result` — what a test of a provider found: `ok`, the provider's name, and either how long it took (with `extra`, what the answer showed) or its error.
- `describe` — every setting with its value in effect and where that comes from.
- `change` — change the saved settings named in `body`, an object of sections holding fields, and return the new description.
- `test_embedder` — have the chosen embedder embed one word.
- `test_llm` — send the chosen LLM one very small request.
- `checked` — A value that passed the checks of its setting, as it is saved.

## Depends on

- [[Settings Module (cw-core)]] — uses `ALL`, `EMBED_PROVIDER`, `LLM_PROVIDER`, `OLLAMA_HOST`
- [[Resolved]] — uses `Resolved`, `from_parts`, `secret_state`, `source`
- [[cw-embed]] — uses `PROVIDERS`, `from_settings`
- [[Provider Module]] — uses `PROVIDERS`, `from_settings`
- [[Error]] — uses `Error`, `invalid`
- [[Error Module]] — uses `Result`
- [[Secret Module]] — uses `available`
- [[Semantic Module]] — uses `configure`

## Used by

- [[Api Module]] — from `ask`, `read_settings`, `run_update`, `test_embedder`
- [[cw-server]] — from `serve`

## See also

- [[_index]]
- [[cw-server]]
