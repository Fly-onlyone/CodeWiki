---
tags: [core, cw-server, module]
---

# Models Module

> The built-in embedding models, as the model routes see them: what there is, what is installed, and the one long job that may run.

## Source

- `crates/cw-server/src/models.rs` — module `cw_server::models`

## How it works

Main items:

- [[Jobs]] — the job that runs, or the last one that ran.
- `lock` — function
- `Job` — A job as a page shows it.
- `Phase` — enum
- `Kind` — what a job does.
- `HardwareCache` — the machine as last read, with the time of the reading.
- `Failure` — why a repository's embedding failed.
- `start` — start a job in the background and answer with it as it starts.

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
