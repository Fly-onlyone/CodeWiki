# CodeWiki plugin for Codex

Gives Codex the repository's wiki and structural index. The plugin adds:

- **The `codewiki` MCP server.** It runs `codewiki-mcp`, the same bridge Claude Code uses, and gives Codex the 27 tools listed in `plugins/claude-code/README.md`.
- **A session-start briefing.** On `startup`, `resume`, `clear` and `compact`, a hook runs `codewiki hook session-start`. Codex adds what it prints to the session as developer context: whether the repository has a wiki, whether the wiki is stale against `HEAD`, how many proposals wait for review, and which tools to call first.
- **Two skills.** `using-codewiki` says which tool to call at each step of a change, and when to write or refresh the wiki with `sync_wiki`. `wiki-note-style` holds the house style of CodeWiki's notes, for when Codex edits a note in `obsidian/` by hand.

## Requirements

`codewiki` and `codewiki-mcp` must be on `PATH`. The release zip's `install.cmd` puts them there (see Install in the root `README.md`); open a new terminal afterwards. To build them from source instead:

```
cargo install --path crates/codewiki
cargo install --path crates/codewiki-mcp
```

## Install

```
codex plugin marketplace add "%LOCALAPPDATA%\Programs\CodeWiki\plugins"
codex plugin add codewiki@codewiki
```

The first line adds the `plugins` folder of an installation made from the release zip (in PowerShell: `"$env:LOCALAPPDATA\Programs\CodeWiki\plugins"`). Codex reads its marketplace from `.agents/plugins/marketplace.json` in that folder.

Without that installation, add the repository in its place, or the folder of a clone:

```
codex plugin marketplace add Fly-onlyone/CodeWiki --sparse .agents/plugins --sparse plugins/codex
```

With `--sparse`, Codex downloads only the marketplace file and this plugin's folder, not the whole repository. A marketplace added from GitHub without `--sparse` holds the whole repository with its history: run `codex plugin marketplace remove codewiki`, then add it again with this command.

Then start a new Codex session in the repository.

**Trust the hook once.** Codex skips a hook that comes with a plugin until you review and trust it. Run `/hooks` in Codex, and trust the `SessionStart` hook that runs `codewiki hook session-start`. Do it again after an update changes the hook.

## Update or remove

Codex installs a copy of the plugin. After you install a new CodeWiki version, replace the copy:

```
codex plugin remove codewiki@codewiki
codex plugin add codewiki@codewiki
```

When the marketplace came from GitHub, run `codex plugin marketplace upgrade codewiki` first, so that Codex fetches the new version.

To remove the plugin and the marketplace:

```
codex plugin remove codewiki@codewiki
codex plugin marketplace remove codewiki
```

## How it finds the repository

- The MCP server has no `cwd` in `.mcp.json`, so Codex starts it in the folder the session runs in. Every tool except `list_repos` and `get_workspace` uses that folder unless it is given a `repo` path.
- The hook looks for the wiki in the same folder and in the folders above it, so a session started in a subfolder still gets the briefing. It reads the vault's manifest and asks git for `HEAD`. It does not start the backend. On any error it prints nothing and exits with status 0, so it cannot fail a session.

The bridge starts a local backend on first use. To use a backend in Docker or on another machine, set `CODEWIKI_URL` and `CODEWIKI_TOKEN` in the environment Codex runs in.

## Without the plugin

The same setup by hand, in `~/.codex/config.toml`:

```toml
[mcp_servers.codewiki]
command = "codewiki-mcp"
args = []

[[hooks.SessionStart]]
matcher = "startup|resume|clear|compact"

[[hooks.SessionStart.hooks]]
type = "command"
command = "codewiki hook session-start"
timeout = 10
```

`codex mcp add codewiki -- codewiki-mcp` adds the server alone. A hook added this way also waits for `/hooks` before it runs. Do not use the plugin and this setup together: Codex would start two `codewiki` servers.

## Status

The plugin's files are checked against Codex's source and by `crates/codewiki/tests/plugins.rs`. It has not yet been run in a Codex session.
