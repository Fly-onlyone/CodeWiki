# CodeWiki for Codex

Codex reaches CodeWiki through the same MCP bridge as Claude Code.

## Requirements

`codewiki` and `codewiki-mcp` must be on `PATH`:

```
cargo install --path crates/codewiki
cargo install --path crates/codewiki-mcp
```

## Install

Either run:

```
codex mcp add codewiki -- codewiki-mcp
```

or add this to `~/.codex/config.toml`:

```toml
[mcp_servers.codewiki]
command = "codewiki-mcp"
args = []
```

To use a backend running in Docker or on another machine, pass its address:

```toml
[mcp_servers.codewiki]
command = "codewiki-mcp"
args = []
env = { CODEWIKI_URL = "http://127.0.0.1:7411", CODEWIKI_TOKEN = "<token>" }
```

## Tools

The tool list is the same as in `plugins/claude-code/README.md`.

## Session-start briefing

Codex has a `SessionStart` hook, and it adds the plain text a command hook prints to the session as developer context. To get the same briefing Claude Code gets (whether the repository has a wiki, whether it is stale, how many proposals wait, and which tools read it), add this to `~/.codex/hooks.json`:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup|resume|clear|compact",
        "hooks": [
          {
            "type": "command",
            "command": "codewiki hook session-start",
            "timeout": 10
          }
        ]
      }
    ]
  }
}
```

or the equivalent table to `~/.codex/config.toml`:

```toml
[[hooks.SessionStart]]
matcher = "startup|resume|clear|compact"

[[hooks.SessionStart.hooks]]
type = "command"
command = "codewiki hook session-start"
timeout = 10
```

For one repository only, put the file at `<repo>/.codex/hooks.json` instead; Codex loads project hooks only when the project's `.codex/` folder is trusted. Codex skips a new or changed hook until you approve it: run `/hooks` in the Codex CLI once after adding it.

The command looks for the wiki in the working directory and its parent folders. It reads the vault's manifest and asks git for `HEAD`; it does not start the backend. On any error it prints nothing and exits with status 0, so it cannot fail a session.
