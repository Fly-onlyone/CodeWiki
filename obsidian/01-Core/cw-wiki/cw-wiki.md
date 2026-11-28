---
tags: [core, cw-wiki]
---

# cw-wiki

> Concept selection and wiki generation pipeline for CodeWiki.

## What it does

The crate that turns a scanned repository into a wiki, and answers questions about it afterwards. It decides which notes a repository gets, renders them in the vault's house style, and holds the read-side queries that the app, the server and the MCP tools share. It also holds the analyses that sit on top of the index: health, hotspots, security findings, test map, coverage, contracts and decisions.

## Source

- `crates/cw-wiki/Cargo.toml` — package manifest
- `crates/cw-wiki/src/lib.rs` — crate root

## How it works

The crate root only lists modules and re-exports a few names: `analyze`, `dependency_graph`, `generate`, `generate_with`, `Report`, `KeptGone`, `Moved`, `TEMPLATE_VERSION`, `Plan`, `Planned`, `Repo` and `Subject`. The work falls into groups.

- Generation: [[Plan Module]] picks the notes, [[Units Module]] names the top-level parts, [[Render Module]] writes Markdown, and [[Generate Module]] runs the whole pipeline: scan, plan, freeze names, render, lint, write. `Gone Module` decides what happens to notes whose subject is gone.
- Source facts: [[Filemap Module]] maps files of languages with no parser, [[Manifests Module]] reads build files, [[Pm Module]] writes package commands for the right JavaScript package manager, and [[Git Module]] runs the `git` command line.
- Agent explanations: [[Explanations Module]] is the only way a coding agent writes into a note. [[Doctext Module]] cleans doc comments so they fit the note rules. [[Explain Module]] adds the walk-through and item lists.
- Questions: [[Query Module]], [[Semantic Module]] and [[Service Module]] answer from the index. Service holds the operations the CLI, server and MCP tools share.
- Analyses: [[Analyze Module]], [[Gitintel Module]], [[Health Module]], [[Security Module]], [[Testmap Module]], [[Coverage Module]], [[Contracts Module]], [[Refactor Module]] and [[Decisions Module]].
- Output for tools: [[Briefing Module]], [[Agentdocs Module]] and [[Ci Module]].

The crate builds on the embedding, illustration, core, graph, index, parse and vault crates, plus `serde`, `serde_json` and `log`.

## Items

Modules:

- [[Agentdocs Module]] — keeps a compact project briefing inside `CLAUDE.md` and `AGENTS.md`.
- [[Analyze Module]] — graph metrics for a scanned repository.
- [[Briefing Module]] — the briefing a coding agent is given when it starts a session.
- [[Ci Module]] — findings for continuous integration, as text, SARIF or JUnit.
- [[Contracts Module]] — HTTP contracts: the endpoints a repository provides and the ones it calls.
- [[Coverage Module]] — test coverage imported from LCOV (`lcov.info`) and Cobertura XML reports.
- [[Decisions Module]] — decision records and the "why" query.
- [[Doctext Module]] — from a doc comment to text that may stand in a note.
- [[Explain Module]] — what a note says beyond its subject's own comment: the walk-through of what happens when the code is used, and the list of its items.
- [[Explanations Module]] — explanations: what a coding agent writes into a note, and how.
- [[Filemap Module]] — the file-level map: units and folders for source files of a language CodeWiki has no parser for.
- [[Generate Module]] — the generation pipeline: scan, plan, freeze names, render, lint, write.
- [[Git Module]] — minimal git access through the `git` command line.
- [[Gitintel Module]] — Git intelligence: what history says about each file.
- [[Gone Module]] — notes whose subject is gone from the repository: which of them leave the vault in a sync, which stay, and why.
- [[Health Module]] — code health: dead code, documentation drift and per-file health scores.
- [[Level Module]] — severity of a finding, shared by the analyses and the CI report.
- [[Manifests Module]] — what a repository's build files say about a package from outside it: the version asked for, the version locked, the features turned on, and what the package is.
- [[Plan Module]] — concept selection: decide which notes a repository gets.
- [[Pm Module]] — the commands of a JavaScript or TypeScript package, written for its package manager (`npm`, `pnpm`, `yarn` or `bun`; see `cw_parse::jsbuild` for how the manager is found).
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

- [[Explanations Module]] — uses `Brief`, `DEFAULT_LIMIT`, `MAX_LIMIT`, `MAX_SECTION_WORDS`
- [[Git Module]] — uses `at_head`, `head_commit`, `history_paths`, `is_linked_worktree`
- [[Proposal Module]] — uses `Proposal`, `get`, `list`, `patch_text`
- [[Ci Module]] — uses `CheckFinding`, `check`, `fails`, `to_junit`
- [[Gitintel Module]] — uses `blast_radius`, `change_risk`, `file_risk`, `hotspots`
- [[Coverage Module]] — uses `import`, `import_text`, `load`, `report`
- [[Security Module]] — uses `COMMAND_INJECTION`, `HARDCODED_SECRET`, `INSECURE_HTTP`, `SQL_FORMAT`
- [[Service Module]] — uses `index_notes`, `is_stale`, `open_existing_index_in`, `open_index_in`

## Gotchas

- `explain`, `gone`, `level` and `rustscan` are private modules. Use them through the public modules that wrap them.
- `TEMPLATE_VERSION` is exported from `generate`. A change to how notes are laid out is tied to it, so check it when you change a template.
- `KeptGone` and `Moved` are re-exported so callers can read what a sync did with notes whose code is gone or moved. They are part of `Report`.
- The manifest gives the two sibling crates `cw-embed` and `cw-illustrate` an explicit version next to their path, while the other workspace crates have a path only. When the workspace version changes, change those two numbers too, or a release build that resolves them by version breaks.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
