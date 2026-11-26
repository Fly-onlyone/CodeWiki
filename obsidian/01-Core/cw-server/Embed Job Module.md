---
tags: [core, cw-server, module]
---

# Embed Job Module

> The background embedding job: the one long job that may run.

## What it does

Semantic search needs a vector for each document. This module builds them in the background, so a wiki update does not wait for a slow model. One job at a time embeds every repository that still lacks vectors for the embedder in use. Three things start a job: the end of a wiki update, the start of the backend, and `POST /v1/embed/reembed`. Progress shows in `GET /v1/embed/job`.

## Source

- `crates/cw-server/src/embed_job.rs` — module `cw_server::embed_job`

## How it works

`start` asks the host for the embedder, takes the one place with `Jobs::begin` and spawns `run` on a blocking task. `run` calls `embed_all`, which makes up to `PASSES` passes over the registered repositories whose folder exists. `embed_repo` skips a repository that is not indexed or not behind. Otherwise it plans a `Run`, then loops: embed a batch with no lock held, take the write lock, and store the batch.

Each `after_update` raises the `updates` counter. If the counter moved while a batch was embedded, the batch is dropped and the run starts over from the index as it is now. A new pass starts when updates ran meanwhile.

Failures split in two. An index that cannot be read puts that repository in `skipped` and the job goes on. A failing embedder ends the job as failed, because it would fail for the next repository too.

`after_update` and `resume` start a job only when documents wait and none runs. `resume` runs at backend start and after a successful settings Test.

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

## Gotchas

- Nothing is saved about a job. A restart looks at the indexes and goes on from there.
- Nothing retries a failed job. The next update, reembed or restart does.
- When the backend is closing, `run` returns without setting a final phase.
- With semantic search off, the job finishes at once with a note.
- Only the write step takes the write lock, so notes can be saved while a job runs.

## See also

- [[_index]]
- [[cw-server]]
