---
tags: [moc, core, cw-core]
---

# Core / cw-core — Map of Content

> Notes for `cw-core`: 20 concepts.

## Notes

- [[Agents Off Module]] — the list of folders closed to coding agents.
- [[Auto Explain Module]] — the switch for auto explaining.
- [[Confidence]] — how sure the resolver is that an edge is real.
- [[Count]] — how many waiting notes a session explains.
- [[cw-core]] — shared identifiers, hashing, configuration and error types for CodeWiki.
- [[Error]] — `Error` enum defined in `cw_core::error` with 4 methods.
- [[Error Module]] — one error type shared by the engine crates.
- [[Hash Module]] — content hashing used for change detection.
- [[Key]] — one setting: the environment variable that names it everywhere, and where it sits in the file.
- [[Language]] — source language of a file.
- [[Logs Module]] — log files of the programs, under `<data dir>/logs`.
- [[Model Module]] — data model shared between the parser, index, graph and API layers.
- [[Paths Module]] — locations of per-user data.
- [[Resolved]] — the settings in effect: for each one, the environment variable when it is set, otherwise the saved value, otherwise the default.
- [[Secret Module]] — secrets kept in a settings file: API keys and the token of a remote backend.
- [[Settings]] — what `settings.json` holds.
- [[Settings Module (cw-core)]] — settings the backend keeps for its user: which embedder serves semantic search, and the keys and addresses it needs.
- [[Symbol]] — A named code entity.
- [[SymbolKind]] — `SymbolKind` enum defined in `cw_core::model` with 3 methods.
- [[Visibility]] — `Visibility` enum defined in `cw_core::model` with 2 methods.

## Folders

- [[01-Core/cw-core/secret/_index|secret]] — 1 notes

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
