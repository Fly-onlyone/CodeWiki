---
tags: [moc, core, cw-server]
---

# Core / cw-server — Map of Content

> Notes for `cw-server`: 9 concepts.

## Notes

- [[Activity]] — `Activity` struct defined in `cw_server::activity` with 3 methods.
- [[Activity Module]] — what the backend is doing right now, for `GET /v1/status`.
- [[Api Module]] — routes and handlers.
- [[AppState]] — `AppState` struct defined in `cw_server`.
- [[cw-server]] — HTTP API of the CodeWiki backend.
- [[Inner]] — `Inner` struct defined in `cw_server`.
- [[Repos Module]] — the list of repositories this backend manages, stored as `repos.json`.
- [[Settings Module (cw-server)]] — what the settings routes do: describe the settings in effect, and change the saved ones.
- [[Workspace Module (cw-server)]] — views across every repository the backend manages.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
