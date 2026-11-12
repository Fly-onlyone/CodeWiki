# CodeWiki plugin for Claude Code

Registers the `codewiki` MCP server so Claude Code can read the repository's wiki and structural index, and a `SessionStart` hook that tells Claude the wiki is there.

## Requirements

`codewiki` and `codewiki-mcp` must be on `PATH`:

```
cargo install --path crates/codewiki
cargo install --path crates/codewiki-mcp
```

The bridge starts a local backend on first use. To use a backend running in Docker or on another machine instead, set `CODEWIKI_URL` and `CODEWIKI_TOKEN` in the environment Claude Code runs in.

## Install

```
claude plugin marketplace add <path-or-url-of-this-repository>
claude plugin install codewiki
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
| `sync_wiki` | Re-index and update the wiki. |
| `list_repos` | Repositories the backend manages. |
| `get_workspace` | Every registered repository at a glance, and the external packages two or more of them use. |
| `get_contracts` | HTTP endpoints a repository provides and the URL paths it calls. With `workspace: true`, matched across all registered repositories. |

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
