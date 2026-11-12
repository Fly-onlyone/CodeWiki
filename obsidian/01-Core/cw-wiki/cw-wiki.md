---
tags: [core, cw-wiki]
---

# cw-wiki

> Concept selection and wiki generation pipeline for CodeWiki.

## Source

- `crates/cw-wiki/Cargo.toml` — package manifest
- `crates/cw-wiki/src/lib.rs` — crate root

## How it works

Builds a library.

Modules:

- [[Agentdocs Module]] — keeps a compact project briefing inside `CLAUDE.md` and `AGENTS.md`.
- [[Analyze Module]] — graph metrics for a scanned repository.
- [[Briefing Module]] — the briefing a coding agent is given when it starts a session.
- [[Ci Module]] — findings for continuous integration, as text, SARIF or JUnit.
- [[Contracts Module]] — HTTP contracts: the endpoints a repository provides and the ones it calls.
- [[Coverage Module]] — test coverage imported from LCOV (`lcov.info`) and Cobertura XML reports.
- [[Decisions Module]] — decision records and the "why" query.
- [[Generate Module]] — the generation pipeline: scan, plan, freeze names, render, lint, write.
- [[Git Module]] — minimal git access through the `git` command line.
- [[Gitintel Module]] — Git intelligence: what history says about each file.
- [[Health Module]] — code health: dead code, documentation drift and per-file health scores.
- [[Level Module]] — severity of a finding, shared by the analyses and the CI report.
- [[Plan Module]] — concept selection: decide which notes a repository gets.
- [[Query Module]] — Read-side queries served to the app and to agents.
- [[Refactor Module]] — refactoring plan: suggestions derived from facts the index already holds.
- [[Render Module]] — rendering of planned notes into Markdown in the vault's house style.
- [[Rustscan Module]] — A light lexical scan of Rust source, shared by the test map and the security rules.
- [[Security Module]] — security pattern findings for Rust source.
- [[Semantic Module]] — semantic search: embeds search documents and blends vector matches with full-text matches.
- [[Service Module]] — High-level operations shared by the CLI, the server and the MCP tools.
- [[Testmap Module]] — test map: which tests exist, what they exercise, and which to run after a change.
- [[Units Module]] — units: the top-level parts of a repository, each with its own folder under `01-Core`.

## Depends on

- [[Ci Module]] — uses `CheckFinding`, `check`, `fails`, `to_junit`
- [[Gitintel Module]] — uses `blast_radius`, `change_risk`, `file_risk`, `hotspots`
- [[Coverage Module]] — uses `import`, `import_text`, `load`, `report`
- [[Security Module]] — uses `COMMAND_INJECTION`, `HARDCODED_SECRET`, `INSECURE_HTTP`, `SQL_FORMAT`
- [[Health Module]] — uses `DriftKind`, `dead_code`, `doc_drift`, `file_health`
- [[VaultMeta]] — uses `by_path`, `load`, `registry`, `save`
- [[Proposal Module]] — uses `list`, `patch_text`, `reject`
- [[Contracts Module]] — uses `RepoContracts`, `match_workspace`, `scan`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
