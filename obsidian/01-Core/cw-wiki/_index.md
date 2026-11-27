---
tags: [moc, core, cw-wiki]
---

# Core / cw-wiki — Map of Content

> Notes for `cw-wiki`: 42 concepts.

## Notes

- [[Agentdocs Module]] — keeps a compact project briefing inside `CLAUDE.md` and `AGENTS.md`.
- [[Analyze Module]] — graph metrics for a scanned repository.
- [[Briefing Module]] — the briefing a coding agent is given when it starts a session.
- [[Ci Module]] — findings for continuous integration, as text, SARIF or JUnit.
- [[Contracts Module]] — HTTP contracts: the endpoints a repository provides and the ones it calls.
- [[Counts]] — how many notes wait for an agent.
- [[Coverage Module]] — test coverage imported from LCOV (`lcov.info`) and Cobertura XML reports.
- [[cw-wiki]] — concept selection and wiki generation pipeline for CodeWiki.
- [[Decisions Module]] — decision records and the "why" query.
- [[Doctext Module]] — from a doc comment to text that may stand in a note.
- [[Explain Module]] — what a note says beyond its subject's own comment: the walk-through of what happens when the code is used, and the list of its items.
- [[Explanations Module]] — explanations: what a coding agent writes into a note, and how.
- [[Generate Module]] — the generation pipeline: scan, plan, freeze names, render, lint, write.
- [[Git]] — what git knows about a repository, each answer asked for once and only when a note needs it: most syncs remove nothing.
- [[Git Module]] — minimal git access through the `git` command line.
- [[Gitintel Module]] — Git intelligence: what history says about each file.
- [[Gone Module]] — notes whose subject is gone from the repository: which of them leave the vault in a sync, which stay, and why.
- [[Health Module]] — code health: dead code, documentation drift and per-file health scores.
- [[Level]] — `Level` enum defined in `cw_wiki::level` with 2 methods.
- [[Level Module]] — severity of a finding, shared by the analyses and the CI report.
- [[Literal]] — one string literal.
- [[Machine]] — reads the files a build on this machine left behind: the sources cargo unpacked into its registry cache, and `node_modules`.
- [[Manifests Module]] — what a repository's build files say about a package from outside it: the version asked for, the version locked, the features turned on, and what the package is.
- [[Plan Module]] — concept selection: decide which notes a repository gets.
- [[Query Module]] — Read-side queries served to the app and to agents.
- [[Refactor Module]] — refactoring plan: suggestions derived from facts the index already holds.
- [[Render Module]] — rendering of planned notes into Markdown in the vault's house style.
- [[RenderCtx]] — `RenderCtx` struct defined in `cw_wiki::render` with 20 methods.
- [[Repo]] — lookup tables over the scanned facts.
- [[Report]] — `Report` struct defined in `cw_wiki::generate` with 1 methods.
- [[Run]] — one pass that brings the stored vectors of some kinds in line with their documents, a batch at a time.
- [[Rustscan Module]] — A light lexical scan of Rust source, shared by the test map and the security rules.
- [[Scanned]] — `Scanned` struct defined in `cw_wiki::rustscan` with 7 methods.
- [[Security Module]] — security pattern findings for Rust source.
- [[Semantic Module]] — semantic search: embeds search documents and blends vector matches with full-text matches.
- [[Service Module]] — High-level operations shared by the CLI, the server and the MCP tools.
- [[Slot]] — the place that holds the embedder in use.
- [[TestFn]] — A `#[test]`-like function found in a file.
- [[Testmap Module]] — test map: which tests exist, what they exercise, and which to run after a change.
- [[Unit]] — one top-level part of the repository.
- [[Units]] — every unit of a repository and the lookup from code to unit.
- [[Units Module]] — units: the top-level parts of a repository, each with its own folder under `01-Core`.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
