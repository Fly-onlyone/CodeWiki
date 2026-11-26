---
tags: [core, cw-server]
---

# Jobs (cw-server)

> The job that runs, or the last one that ran.

## What it does

Holds the state of the one background embedding job: the one that runs, or the last that ran. A job computes search vectors for documents that still wait for one. The server allows only one job at a time. `Jobs` tracks its phase and progress so `GET /v1/embed/job` and `GET /v1/status` can report them, and it carries the flags that stop the job.

## Source

- `crates/cw-server/src/embed_job.rs` — definition of `Jobs`

## How it works

`Jobs` is created by `state` in [[cw-server]] through `default`. It holds the current `Job` behind a mutex, plus two flags, `cancel` and `closing`.

`start` calls `begin`, which refuses with an error when a job is still running, then clears the cancel flag and stores a fresh job in the `Embedding` phase. The work runs on a blocking task through `run` and `embed_all`.

`embed_all` goes over every repository that exists on disk, for up to three passes. A pass repeats only if a wiki update finished meanwhile. `embed_repo` skips a repository with no index, or one that is not behind. Otherwise it plans a run, embeds batch by batch, and stores each batch.

The write lock is held only while a batch is stored, not while the embedder works. If an update finished in between, the run starts over from the index.

When the work ends, `run` sets the phase to `Done`, `Cancelled` or `Failed`. A failure keeps the error text. `status` gives a short value for the tray only while a job is not over; otherwise null. `cancel` and `close` set the flags, and the job stops at its next batch.

## Items

Declared as `pub(crate) struct Jobs`.

Members:

- `fn default() -> Self` — public method, returns `Self`; called by `state`.
- `fn begin(&self, model: Option<String>) -> Result<Job>` — take the place of the running job, or say that one runs.
- `pub(crate) fn snapshot(&self) -> Option<Job>` — internal method, returns `Option<Job>`; called by `Jobs::status`.
- `pub(crate) fn cancel(&self)` — ask the running job to stop. What it stored stays.
- `pub(crate) fn close(&self)` — the backend stops: end the job at its next step.
- `pub(crate) fn status(&self) -> Value` — one line for `GET /v1/status`, where the tray reads it.

And 3 more private items with no comment.

## Depends on

- [[Embed Job Module]] — uses `Job`, `lock`, `over`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `invalid`

## Used by

- [[cw-server]] — from `call`, `call_as`, `hook`, `serve`
- [[Api Module]] — from `backend_status`, `cancel_embed_job`, `log_request`, `read_embed_job`
- [[Embed Job Module]] — from `after_update`, `embed_all`, `embed_repo`, `run`
- [[Inner]] — from `Inner`

## Gotchas

- An index that cannot be read or written skips that repository and goes on. The reason lands in `skipped`. A failing embedder ends the whole job, since it would fail for the next repository too.
- Nothing retries by itself. A failed job stays failed until the next update or a manual reembed.
- Nothing is kept on disk. On start, `resume` looks at the indexes and starts a job if documents wait.
- When the backend is closing, `run` returns without setting a final phase.
- With semantic search off, the job ends as `Done` with a note, and does no work.
- `cancel` does nothing when no job runs. What the job already stored stays.

## See also

- [[_index]]
- [[Embed Job Module]]
