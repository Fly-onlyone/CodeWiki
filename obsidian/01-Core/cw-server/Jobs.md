---
tags: [core, cw-server]
---

# Jobs

> The job that runs, or the last one that ran.

## Source

- `crates/cw-server/src/models.rs` — definition of `Jobs`

## How it works

1. `state` in [[cw-server]] calls `default`.

## Items

Declared as `pub(crate) struct Jobs`.

Members:

- `fn default() -> Self` — public method, returns `Self`; called by `state`.
- `fn begin(&self, kind: Kind, model: Option<String>) -> Result<Job>` — take the place of the running job, or say that one runs.
- `pub(crate) fn snapshot(&self) -> Option<Job>` — internal method, returns `Option<Job>`; called by `Jobs::status`.
- `pub(crate) fn cancel(&self)` — ask the running job to stop. What it stored stays.
- `pub(crate) fn close(&self)` — the backend stops: end the job at its next step, also a paused one.
- `fn sleep(&self, time: Duration)` — sleep, unless a cancel or the backend's stop comes first.
- `pub(crate) fn status(&self) -> Value` — one line for `GET /v1/status`, where the tray reads it.

And 3 more private items with no comment.

## Depends on

- [[Models Module]] — uses `Job`, `Kind`, `lock`, `over`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `invalid`

## Used by

- [[Models Module]] — from `after_update`, `describe`, `embed_all`, `embed_repo`
- [[cw-server]] — from `call`, `call_as`, `hook`, `serve`
- [[Api Module]] — from `backend_status`, `cancel_model_job`, `log_request`, `trace_request`
- [[Inner]] — from `Inner`

## See also

- [[_index]]
- [[Models Module]]
