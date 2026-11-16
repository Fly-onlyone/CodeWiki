# CodeWiki plugin for Claude Code

Registers the `codewiki` MCP server so Claude Code can read the repository's wiki and structural index, and a `SessionStart` hook that tells Claude the wiki is there.

## Requirements

`codewiki` and `codewiki-mcp` must be on `PATH`. The release zip's `install.cmd` puts them there (see Install in the root `README.md`); open a new terminal afterwards. To build them from source instead:

```
cargo install --path crates/codewiki
cargo install --path crates/codewiki-mcp
```

The bridge starts a local backend on first use. To use a backend running in Docker or on another machine instead, set `CODEWIKI_URL` and `CODEWIKI_TOKEN` in the environment Claude Code runs in.

## Install

```
claude plugin marketplace add "%LOCALAPPDATA%\Programs\CodeWiki\plugins"
claude plugin install codewiki@codewiki
```

The first line adds the `plugins` folder of an installation made from the release zip (in PowerShell: `"$env:LOCALAPPDATA\Programs\CodeWiki\plugins"`). The folder holds the plugin and nothing else. Claude Code reads it where it is, so nothing is downloaded and git is not needed.

Without that installation, add the repository in its place: `claude plugin marketplace add Fly-onlyone/CodeWiki`, or the folder of a clone. Claude Code then clones the whole repository, which needs git.

The plugin is installed for your user; `--scope project` on the install records it in the repository's `.claude/settings.json` instead.

Then start Claude Code, or run `/reload-plugins` in a session that is already open. `/mcp` lists the server as `plugin:codewiki:codewiki`.

To update or remove it:

```
claude plugin update codewiki@codewiki        # then restart Claude Code
claude plugin uninstall codewiki@codewiki
claude plugin marketplace remove codewiki
```

## Tools

| Tool | Use |
|---|---|
| `get_overview` | Home note and index status. Start here. |
| `get_context` | What to know before touching a file or symbol. |
| `get_symbol` | Definition, members, dependencies and users of a symbol. |
| `search_codebase` | Full-text search over notes, symbols and files. |
| `get_answer` | Best-matching note summaries for a question. |
| `get_dependency_path` | Shortest dependency chain between two symbols. |
| `get_architecture` | Architecture flow notes in full. |
| `read_note` | One note in full. |
| `get_why` | Why a file or symbol is the way it is: decision records that cite it, rationale recorded in notes, and the latest commits that touched it. |
| `list_decisions` | Decision records with their status, and which are stale because the code they cite changed. |
| `get_risk` | Risk of changing one file, 0 to 100, with the reasons. |
| `get_change_risk` | Risk of a set of changed files taken together (by default the uncommitted changes), files that usually change with them but are missing, and files outside the change that depend on it. |
| `get_blast_radius` | Files affected if a file changes its behavior: direct dependents, then further hops. |
| `get_impacted_tests` | Tests to run after a change (by default the uncommitted changes), nearest first, with the `cargo test` commands, plus changed files no test reaches. |
| `get_hotspots` | Files that are both large and often changed, with main author and bus factor. |
| `get_dead_code` | Symbols nothing refers to, marked `likely` or `possible`. Verify before deleting. |
| `get_health` | Files scoring below 10 with the findings behind the score, plus documentation coverage and drift. |
| `get_coverage` | Line coverage from an LCOV or Cobertura report, with the least covered files ranked by how often they change and how much depends on them. `report` imports a new report. |
| `get_security_findings` | Security pattern findings in Rust source, test code excluded: hard-coded secrets, shell commands and SQL built with `format!`, disabled TLS verification, `unsafe` blocks and more. |
| `get_refactoring_plan` | Refactoring suggestions, highest priority first, each with its evidence and ordered steps. No model is involved. |
| `get_contracts` | HTTP endpoints a repository provides and the URL paths it calls. With `workspace: true`, matched across all registered repositories. |
| `sync_wiki` | Re-index and update the wiki. Notes a person edited are never overwritten; changes to them wait as proposals. |
| `list_repos` | Repositories the backend manages. |
| `get_workspace` | Every registered repository at a glance, and the external packages two or more of them use. |

Every tool except `list_repos` and `get_workspace` takes an optional `repo` path and defaults to the directory Claude Code runs in.

## Session-start hook

`hooks/hooks.json` runs `codewiki hook session-start` on the `SessionStart` event: when a session starts or resumes, and after `/clear` and compaction. Claude Code adds what the command prints to Claude's context. The briefing is at most five lines:

- whether the repository has a wiki, and where its home note is (`obsidian/_HOME.md`);
- whether the wiki is up to date with `HEAD` or stale;
- how many proposals wait for review;
- that the `get_overview` and `get_context` tools answer from the wiki.

For example:

```
CodeWiki: this repository has a maintained wiki of 123 notes in `obsidian/`. Its home note is `obsidian/_HOME.md`.
The wiki is stale: it was generated at commit 3f2a9c1b7e, and HEAD is 9d04e6a1c2. The MCP tool `sync_wiki` brings it up to date.
Pending proposals: 2 (changes to human-edited notes that wait for review; those notes may lag behind the code).
The CodeWiki MCP tools `get_overview` (architecture and index status) and `get_context` (what to know before touching a file or symbol) answer from this wiki. Call them before exploring the code by hand.
```

The command reads the vault's manifest and asks git for `HEAD`. It does not start the backend, does not open the index, and writes nothing. It looks for the vault in the project folder (`CLAUDE_PROJECT_DIR`, otherwise the working directory) and its parent folders. It cannot fail a session: on any error it prints nothing and exits with status 0. The hook has a 10 second timeout; a run takes well under a second.

To try it outside Claude Code, run `codewiki hook session-start` in a repository.
