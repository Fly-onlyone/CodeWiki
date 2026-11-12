---
tags: [moc, core, cw-wiki]
---

# Core / cw-wiki — Map of Content

> Notes for `cw-wiki`: 35 concepts.

## Notes

- [[Agentdocs Module]] — keeps a compact project briefing inside `CLAUDE.md` and `AGENTS.md`.
- [[Analyze Module]] — graph metrics for a scanned repository.
- [[Briefing Module]] — the briefing a coding agent is given when it starts a session.
- [[Ci Module]] — findings for continuous integration, as text, SARIF or JUnit.
- [[Contracts Module]] — HTTP contracts: the endpoints a repository provides and the ones it calls.
- [[Coverage Module]] — test coverage imported from LCOV (`lcov.info`) and Cobertura XML reports.
- [[cw-wiki]] — concept selection and wiki generation pipeline for CodeWiki.
- [[Decisions Module]] — decision records and the "why" query.
- [[FileHistory]] — `FileHistory` struct defined in `cw_wiki::gitintel`.
- [[Generate Module]] — the generation pipeline: scan, plan, freeze names, render, lint, write.
- [[Git Module]] — minimal git access through the `git` command line.
- [[Gitintel Module]] — Git intelligence: what history says about each file.
- [[Health Module]] — code health: dead code, documentation drift and per-file health scores.
- [[Level]] — `Level` enum defined in `cw_wiki::level` with 2 methods.
- [[Level Module]] — severity of a finding, shared by the analyses and the CI report.
- [[Literal]] — one string literal.
- [[Plan Module]] — concept selection: decide which notes a repository gets.
- [[Planned]] — `Planned` struct defined in `cw_wiki::plan`.
- [[ProseWriter]] — replaces the structural prose of a note, typically with an LLM.
- [[Query Module]] — Read-side queries served to the app and to agents.
- [[Refactor Module]] — refactoring plan: suggestions derived from facts the index already holds.
- [[Render Module]] — rendering of planned notes into Markdown in the vault's house style.
- [[RenderCtx]] — `RenderCtx` struct defined in `cw_wiki::render` with 22 methods.
- [[Repo]] — lookup tables over the scanned facts.
- [[Rustscan Module]] — A light lexical scan of Rust source, shared by the test map and the security rules.
- [[Scanned]] — `Scanned` struct defined in `cw_wiki::rustscan` with 7 methods.
- [[Security Module]] — security pattern findings for Rust source.
- [[Semantic Module]] — semantic search: embeds search documents and blends vector matches with full-text matches.
- [[Service Module]] — High-level operations shared by the CLI, the server and the MCP tools.
- [[TestFn]] — A `test`-like function found in a file.
- [[Testmap Module]] — test map: which tests exist, what they exercise, and which to run after a change.
- [[Unit]] — one top-level part of the repository.
- [[Units]] — every unit of a repository and the lookup from code to unit.
- [[Units Module]] — units: the top-level parts of a repository, each with its own folder under `01-Core`.
- [[WriterUsage]] — what a writer spent during one run.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
