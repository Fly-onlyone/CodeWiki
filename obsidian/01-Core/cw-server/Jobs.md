---
tags: [core, cw-server]
---

# Jobs

> The job that runs, or the last one that ran.

## Source

- `crates/cw-server/src/models.rs` — definition of `Jobs`

## How it works

Declared as `pub(crate) struct Jobs`.

Public API:

- `pub(crate) fn status(&self) -> Value` — one line for `GET /v1/status`, where the tray reads it.
- `pub(crate) fn cancel(&self)` — ask the running job to stop.
- `pub(crate) fn close(&self)` — the backend stops: end the job at its next step, also a paused one.
- `pub(crate) fn snapshot(&self) -> Option<Job>`
- `fn default() -> Self`

## Depends on

- [[Models Module]] — uses `Job`, `Kind`, `lock`, `over`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `invalid`

## Used by

- [[Models Module]] — from `after_update`, `describe`, `embed_all`, `embed_repo`
- [[cw-server]] — from `call`, `call_as`, `hook`, `serve`
- [[Api Module]] — from `backend_status`, `cancel_model_job`, `log_request`
- [[Inner]] — from `Inner`

## See also

- [[_index]]
- [[Models Module]]
