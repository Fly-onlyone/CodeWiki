---
tags: [core, codewiki]
---

# codewiki

> CodeWiki command line and backend server.

## Source

- `crates/codewiki/Cargo.toml` — package manifest
- `crates/codewiki/src/main.rs` — crate root

## How it works

Builds a binary.

Modules:

- [[Model Module (codewiki)]] — `codewiki model`: the built-in embedding models from the command line.
- [[Serve Module]] — `codewiki serve`: run the backend HTTP server.

Defined at the crate root:

- `Command` — enum
- `HookEvent` — enum
- `embed_worker` — function
- `run` — function
- `session_start_hook` — print the session-start briefing for the repository at `repo`.
- `print_json` — print a result encoded with `serde_json::to_string_pretty`.
- `print_report` — function
- `Cli` — struct

## Depends on

- [[Ci Module]] — uses `check`, `fails`, `to_junit`, `to_sarif`
- [[Gitintel Module]] — uses `change_risk`, `file_risk`, `hotspots`, `load`
- [[Coverage Module]] — uses `import`, `load`, `report`, `save`
- [[Service Module]] — uses `is_stale`, `open_index`, `reindex`, `update`
- [[Error]] — uses `external`, `invalid`, `io`, `not_found`
- [[Proposal Module]] — uses `approve`, `list`, `reject`
- [[Decisions Module]] — uses `create`, `list`, `why`
- [[Health Module]] — uses `dead_code`, `doc_drift`, `file_health`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
