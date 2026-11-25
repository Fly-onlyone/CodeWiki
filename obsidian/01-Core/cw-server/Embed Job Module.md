---
tags: [core, cw-server, module]
---

# Embed Job Module

> The background embedding job: the one long job that may run.

## What it does

A job embeds the documents of every repository that still wait for a vector of the embedder in use, whichever embedder that is. An update of a wiki leaves it what does not fit the inline budget, a backend starts one for what waits, and `POST /v1/embed/reembed` starts one by hand. It runs in the background and reports how far it is in `GET /v1/embed/job`. Nothing is kept on disk about it: a backend that starts looks at the indexes instead, and goes on where documents still wait for a vector.

A job whose embedder fails ends with the reason, and the documents keep waiting. Nothing tries again by itself: the next update or reembed does.

Locks: embedding takes minutes and holds none. The write lock is taken for each batch's write alone, so a note can be saved and a wiki updated while a job runs.

## Source

- `crates/cw-server/src/embed_job.rs` — module `cw_server::embed_job`

## How it works

1. `serve` in [[cw-server]] and `test_embedder` in [[Settings Module (cw-server)]] call `resume`. `resume` calls `load` in [[Repos Module]], then `open_existing_index_in` in [[Service Module]], then `behind` in [[Semantic Module]], then `start`.
2. `Jobs::begin` and `Jobs::change` in [[Jobs (cw-server)]], and 2 more, call `lock`.
3. `reembed` in [[Api Module]] calls `start`. `start` calls `run`. `run` calls `embed_all`.
4. `run_update` in [[Api Module]] calls `after_update`. `after_update` calls `open_existing_index_in` in [[Service Module]], then `behind` in [[Semantic Module]], then `start`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const PASSES: usize` — how often a job looks over the repositories again when updates ran while it worked.
- `pub(crate) enum Phase` — internal enum; used by `Job`.
- `pub(crate) struct Job` — A job as a page shows it.
- [[Jobs (cw-server)]] — the job that runs, or the last one that ran.
- `fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T>` — private function, returns `MutexGuard<'_, T>`; called by `Jobs::begin`, `Jobs::change`, `Jobs::running`, `Jobs::snapshot` and 1 more.
- `pub(crate) fn start(state: &AppState) -> Result<Value>` — start a job in the background and answer with it as it starts. Must be called where the async runtime is known: a handler, or a blocking task of it.
- `enum End` — how a job's work ended when it did not fail.
- `fn run(state: &AppState, embedder: Option<Arc<dyn Embedder>>)` — private function; called by `embed_repo`, `start`.
- `enum Failure` — why a repository's embedding failed. An index that cannot be read or written is that repository's problem, and the job goes on with the next; an embedder that fails would fail for the next one too.
- `embed_repo` — embed one repository's documents. `Ok(false)`: the job was stopped.
- `fn embed_all(state: &AppState, embedder: Option<Arc<dyn Embedder>>) -> Result<End>` — embed the documents of every repository that waits, one after the other.
- `pub(crate) fn after_update(state: &AppState, root: &std::path::Path)` — what an update of a wiki leaves to do: count it, so that a run under way starts over, and embed in the background what the update left waiting.
- `pub(crate) fn resume(state: &AppState)` — at the backend's start, and after a Test that succeeds: go on with what waits, for any embedder. It tries once; a job that fails stays failed.

## Depends on

- [[Jobs (cw-server)]] — uses `begin`, `change`, `running`, `stopped`
- [[Repos Module]] — uses `RepoEntry`, `load`
- [[Service Module]] — uses `DOCUMENT_KINDS`, `open_existing_index_in`
- [[Run]] — uses `is_done`, `plan`
- [[cw-index]] — uses `documents`
- [[Error Module]] — uses `Result`
- [[Semantic Module]] — uses `behind`
- [[Error]] — uses `Error`

## Used by

- [[Jobs (cw-server)]] — from `Jobs`, `begin`, `change`, `running`
- [[Api Module]] — from `reembed`, `run_update`
- [[cw-server]] — from `serve`
- [[Settings Module (cw-server)]] — from `test_embedder`

## See also

- [[_index]]
- [[cw-server]]
