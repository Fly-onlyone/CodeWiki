---
tags: [moc, core, cw-server]
---

# Core / cw-server — Map of Content

> Notes for `cw-server`: 13 concepts.

## Notes

- [[Activity]] — `Activity` struct defined in `cw_server::activity` with 3 methods.
- [[Activity Module]] — what the backend is doing right now, for `GET /v1/status`.
- [[Api Module]] — routes and handlers.
- [[AppState]] — `AppState` struct defined in `cw_server`.
- [[Cliproxy Module (cw-server)]] — what the CLIProxyAPI route does: describe the proxy the settings name.
- [[cw-server]] — HTTP API of the CodeWiki backend.
- [[Host]] — what the work on built-in models asks of the machine.
- [[Inner]] — `Inner` struct defined in `cw_server`.
- [[Jobs]] — the job that runs, or the last one that ran.
- [[Models Module]] — the built-in embedding models, as the model routes see them: what there is, what is installed, and the one long job that may run.
- [[Repos Module]] — the list of repositories this backend manages, stored as `repos.json`.
- [[Settings Module (cw-server)]] — what the settings routes do: describe the settings in effect, and change the saved ones.
- [[Workspace Module (cw-server)]] — views across every repository the backend manages.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
