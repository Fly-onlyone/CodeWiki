# CodeWiki

A native Windows app and a Rust backend that write and maintain a wiki for a codebase, for both people and coding agents.

The wiki is an Obsidian vault of Markdown notes at `<repo>/obsidian/`. It lives in the repository, so it is versioned with the code and readable without CodeWiki.

> Status: pre-release, version 0.1.0. See `ROADMAP.md` for what is verified and what is not.

## What it does

- **Writes the wiki.** Parses the repository, builds a dependency graph, and writes notes for packages, modules, key types, startup flows, external integrations, patterns and operations, with Mermaid diagrams and illustrations.
- **Keeps it current.** After new commits it rewrites only the notes whose source changed.
- **Protects your edits.** A note you edited is never overwritten. Proposed changes wait in a review queue until you approve or reject them.
- **Answers agents.** An MCP server gives Claude Code and other agents the wiki, symbol detail, search, risk and impact analysis.
- **Analyses the code.** Hotspots, ownership, change coupling, blast radius, dead code, documentation drift, file health, test impact, coverage gaps, security patterns, refactoring suggestions, decision records and API contracts.

Symbols are extracted for Rust, Python, TypeScript, JavaScript and Java. Other files are indexed by path.

No LLM is required. Without one, notes are built from the code's structure. With one, the prose, the answers and the illustrations are written by the model and checked against the house style before they are saved.

## Parts

| Part | What it is |
|---|---|
| `codewiki` | Command line tool and backend server (HTTP API, no web pages) |
| `codewiki-app` | Desktop app for Windows 11, native Rust on GPUI |
| `codewiki-mcp` | Small stdio program that connects coding agents to the backend |

The app and the MCP program start a local backend when none is running, or connect to one in Docker or on another machine.

## Install

Requires Windows 11, Rust 1.95 or newer and the MSVC toolchain.

```
pwsh scripts\package.ps1
```

This builds the three programs and writes `dist\CodeWiki-<version>-windows-x64.zip`. Unpack it and run `install.ps1`; it copies the programs to `%LOCALAPPDATA%\Programs\CodeWiki`, adds that folder to your `PATH` and creates a Start Menu shortcut. `uninstall.ps1` reverses it.

## Use

In a repository:

```
codewiki init          # index the code and write the wiki
codewiki update        # after new commits
codewiki status
codewiki check         # findings for CI; --format sarif or junit
```

`codewiki --help` lists every command. Start `codewiki-app` from a repository folder, or pass the folder as its first argument. Ctrl+K opens the command palette. The Settings page changes the theme and turns animations down; by default the app follows the Windows animation setting.

### With an LLM

| Variable | Values |
|---|---|
| `CODEWIKI_LLM` | `anthropic`, `openai`, `ollama`, `claude-cli`, `codex-cli`, `none` |
| `CODEWIKI_MODEL` | Model name for the chosen provider |
| `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `OPENAI_BASE_URL` | Credentials and endpoint for the API providers |

### Search

Search combines full-text matching with vectors. `CODEWIKI_EMBED` selects where vectors come from: `local` (default, built in, no model needed), `ollama`, `openai` or `none`. `CODEWIKI_EMBED_MODEL` names the model for the last two.

### Coding agents

`plugins/claude-code/` is a Claude Code plugin: it registers the MCP server and a session-start briefing. `plugins/codex/README.md` has the equivalent configuration for Codex.

### Docker

```
docker build -t codewiki .
docker run --rm -p 127.0.0.1:7411:7411 -e CODEWIKI_TOKEN=<secret> -v codewiki-data:/data -v C:\path\to\repos:/repos codewiki
```

Point the app or the MCP program at it with `CODEWIKI_URL=http://127.0.0.1:7411` and the same `CODEWIKI_TOKEN`. Set `CODEWIKI_WEBHOOK_SECRET` to let a git host trigger updates through `POST /v1/hooks/git`.

## Develop

```
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

`pwsh scripts\clean.ps1` frees the disk space builds leave behind. `AGENTS.md` holds the project rules.

## License

Apache-2.0. See `LICENSE` and `NOTICE`.
