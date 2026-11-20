---
tags: [core, codewiki, module]
---

# Model Module (codewiki)

> `codewiki model`: the built-in embedding models from the command line.

## What it does

These commands work on the models folder of the data directory, as the backend does. They do not choose the embedder: the app does that, or the environment (`CODEWIKI_EMBED=builtin` with `CODEWIKI_EMBED_MODEL`), since only the backend writes the saved settings.

## Source

- `crates/codewiki/src/model.rs` — module `codewiki::model`

## How it works

1. `run` in [[codewiki]] calls `run`. `run` calls `Error::invalid` in [[Error]], then `list`, then `recommend`, then `install`, then `entry`, then `remove`. `list` calls `store`, then `all` in [[Catalog Module]], then `megabytes`. `recommend` calls `probe` in [[Hardware Module]], then `data_dir` in [[Paths Module]], then `cuda_program` in [[Worker Module]], then `megabytes`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- [[ModelAction]] — enum
- `fn megabytes(bytes: u64) -> String` — private function, returns `String`; called by `install`, `list`, `recommend`.
- `fn store() -> Store` — private function, returns `Store`; called by `install`, `list`, `remove`.
- `fn entry(id: &str) -> Result<&'static Entry>` — private function, returns `Result<&'static Entry>`; called by `install`, `list`, `recommend`, `remove` and 1 more.
- `pub fn run(action: &ModelAction, as_json: bool) -> Result<()>` — public function, returns `Result<()>`; called by `run`.
- `fn list(as_json: bool) -> Result<()>` — private function, returns `Result<()>`; called by `run`.
- `fn recommend(as_json: bool) -> Result<()>` — private function, returns `Result<()>`; called by `run`.
- `fn install(entry: &Entry) -> Result<()>` — private function, returns `Result<()>`; called by `run`.
- `fn remove(entry: &Entry) -> Result<()>` — private function, returns `Result<()>`; called by `run`.

And 1 more private item with no comment.

## Depends on

- [[Store]] — uses `Store`, `install`, `is_installed`, `new`
- [[Settings Module (cw-core)]] — uses `EMBED_MODEL`, `EMBED_PROVIDER`, `process_env`
- [[Catalog Module]] — uses `all`, `find`
- [[Entry]] — uses `Entry`, `size`
- [[cw-models]] — uses `ENGINE`
- [[Error Module]] — uses `Result`
- [[Logs Module]] — uses `flush`
- [[Paths Module]] — uses `data_dir`

## Used by

- [[codewiki]] — from `run`

## See also

- [[_index]]
- [[codewiki]]
