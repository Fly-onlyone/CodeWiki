---
tags: [core, cw-server, module]
---

# Models Module

> The built-in embedding models, as the model routes see them: what there is, what is installed, and the one long job that may run.

## What it does

A job installs a model, switches the embedder to it, and embeds the documents of every repository again; or does only the later steps. It runs in the background and reports how far it is in `GET /v1/models`. Nothing is kept on disk about it: a backend that starts looks at the indexes instead, and goes on where documents still wait for a vector.

Locks: embedding takes minutes and holds none. The write lock is taken for each batch's write alone, so a note can be saved and a wiki updated while a job runs.

## Source

- `crates/cw-server/src/models.rs` — module `cw_server::models`

## How it works

1. `Jobs::begin` and `Jobs::change` in [[Jobs]], and 3 more, call `lock`.
2. `read_models` in [[Api Module]] calls `describe`. `describe` calls `hardware`, then `cuda_usable` in [[Worker Module]], then `store`, then `chosen`, then `all` in [[Catalog Module]], then `resolved` in [[Settings Module (cw-server)]], and 1 more. `hardware` calls `lock`. `store` calls `Store::new` in [[Store]].
3. `start_model_job` in [[Api Module]] calls `start`. `start` calls `unavailable`, then `store`, then `Error::invalid` in [[Error]], then `run`. `unavailable` calls `Error::invalid` in [[Error]]. `run` calls `work`.
4. `remove_model` in [[Api Module]] calls `remove`. `remove` calls `unavailable`, then `entry`, then `Error::invalid` in [[Error]], then `chosen`, then `change` in [[Settings Module (cw-server)]], then `global` in [[Worker Module]], and 2 more. `entry` calls `find` in [[Catalog Module]], then `Error::not_found` in [[Error]]. `chosen` calls `resolved` in [[Settings Module (cw-server)]].
5. `run_update` in [[Api Module]] calls `after_update`. `after_update` calls `open_existing_index_in` in [[Service Module]], then `behind` in [[Semantic Module]], then `unavailable`, then `start`.

1 more entry point: `resume`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub(crate) enum Kind` — what a job does.
- `pub(crate) enum Phase` — internal enum; used by `Job`.
- `pub(crate) struct Job` — A job as a page shows it.
- [[Jobs]] — the job that runs, or the last one that ran.
- `fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T>` — private function, returns `MutexGuard<'_, T>`; called by `Jobs::begin`, `Jobs::change`, `Jobs::running`, `Jobs::sleep` and 3 more.
- `pub(crate) struct HardwareCache` — the machine as last read, with the time of the reading.
- `fn hardware(state: &AppState) -> Hardware` — private function, returns `Hardware`; called by `describe`.
- `fn store(state: &AppState) -> Store` — private function, returns `Store`; called by `describe`, `remove`, `start`, `work`.
- `fn chosen(state: &AppState) -> Option<String>` — the id of the built-in model the settings name, whether it works or not.
- `pub(crate) fn describe(state: &AppState) -> Value` — the body of `GET /v1/models`.
- `pub(crate) fn unavailable() -> Option<Error>` — why a model route does nothing on this backend; `None` when it works.
- `fn entry(id: &str) -> Result<&'static Entry>` — private function, returns `Result<&'static Entry>`; called by `describe`, `remove`, `run`, `start` and 2 more.
- `pub(crate) fn start(state: &AppState, kind: Kind, model: Option<&str>) -> Result<Value>` — start a job in the background and answer with it as it starts. Must be called where the async runtime is known: a handler, or a blocking task of it.
- `fn run(state: &AppState, kind: Kind, entry: Option<&'static Entry>)` — private function; called by `embed_repo`, `start`.
- `fn work(state: &AppState, kind: Kind, entry: Option<&'static Entry>) -> Result<End>` — private function, returns `Result<End>`; called by `run`.
- `enum Failure` — why a repository's embedding failed. An index that cannot be read or written is that repository's problem, and the job goes on with the next; an embedder that fails would fail for the next one too.
- `embed_repo` — embed one repository's documents. `Ok(false)`: the job was stopped.
- `pub(crate) fn remove(state: &AppState, id: &str) -> Result<Value>` — delete a model. One that is in use first hands semantic search back to the embedder that needs no model.
- `pub(crate) fn after_update(state: &AppState, root: &std::path::Path)` — what an update of a wiki leaves to do: count it, so that a run under way starts over, and embed in the background what the update left waiting.
- `pub(crate) fn resume(state: &AppState)` — at the backend's start: go on with what a job cut short left waiting. Only for a built-in model, whose embedding is what takes long.

And 6 more items.

## Depends on

- [[Jobs]] — uses `begin`, `change`, `running`, `sleep`
- [[Store]] — uses `Store`, `install`, `is_installed`, `new`
- [[Error]] — uses `Error`, `invalid`, `not_found`
- [[Run]] — uses `is_done`, `plan`, `progress`
- [[Settings Module (cw-core)]] — uses `EMBED_MODEL`, `EMBED_PROVIDER`
- [[Catalog Module]] — uses `all`, `find`
- [[Worker Module]] — uses `cuda_usable`, `global`
- [[Repos Module]] — uses `RepoEntry`, `load`

## Used by

- [[Jobs]] — from `Jobs`, `begin`, `change`, `running`
- [[Api Module]] — from `read_models`, `remove_model`, `run_update`, `start_model_job`
- [[cw-server]] — from `serve`
- [[Inner]] — from `Inner`

## See also

- [[_index]]
- [[cw-server]]
