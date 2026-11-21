---
tags: [core, cw-server]
---

# Jobs (cw-server)

> The job that runs, or the last one that ran.

## Source

- `crates/cw-server/src/embed_job.rs` — definition of `Jobs`

## How it works

1. `state` in [[cw-server]] calls `default`.

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

## See also

- [[_index]]
- [[Embed Job Module]]
