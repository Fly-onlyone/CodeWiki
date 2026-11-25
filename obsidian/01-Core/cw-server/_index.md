---
tags: [moc, core, cw-server]
---

# Core / cw-server — Map of Content

> Notes for `cw-server`: 13 concepts.

## Notes

- [[Activity]] — `Activity` struct defined in `cw_server::activity` with 4 methods.
- [[Activity Module]] — what the backend is doing right now, for `GET /v1/status`.
- [[Api Module]] — routes and handlers.
- [[AppState]] — `AppState` struct defined in `cw_server`.
- [[Claims]] — `Claims` struct defined in `cw_server::claims` with 5 methods.
- [[Claims Module]] — which waiting notes an agent holds.
- [[cw-server]] — HTTP API of the CodeWiki backend.
- [[Embed Job Module]] — the background embedding job: the one long job that may run.
- [[Inner]] — `Inner` struct defined in `cw_server`.
- [[Jobs (cw-server)]] — the job that runs, or the last one that ran.
- [[Repos Module]] — the list of repositories this backend manages, stored as `repos.json`.
- [[Settings Module (cw-server)]] — what the settings routes do: describe the settings in effect, and change the saved ones.
- [[Workspace Module (cw-server)]] — views across every repository the backend manages.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
