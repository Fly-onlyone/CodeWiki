---
description: >-
  Give Codex the wiki and the session-start briefing with the CodeWiki plugin
  for Codex.
icon: plug
---

# Codex

The CodeWiki plugin for Codex gives Codex the same tools and the same session-start briefing as the Claude Code plugin. It adds three things:

* **The `codewiki` MCP server.** It runs `codewiki-mcp`, the bridge Claude Code uses too, and gives Codex the 27 tools. See [MCP tools](mcp-tools.md).
* **A session-start briefing.** On `startup`, `resume`, `clear` and `compact`, a hook runs `codewiki hook session-start --agent codex`. Codex adds what it prints as developer context. See [The session-start briefing](claude-code-plugin.md#the-session-start-briefing).
* **Three skills.** `write-notes` writes the explanations of waiting notes. `using-codewiki` says which tool to call at each step of a change. `wiki-note-style` holds the house style of CodeWiki's notes, for when Codex edits a note in `obsidian/` by hand.

{% hint style="info" %}
The plugin's files are checked against Codex's source and by a test. It has not yet been run in a Codex session.
{% endhint %}

## Install the plugin

{% stepper %}
{% step %}
### Install CodeWiki

Follow [Install](../getting-started/install.md). Then open a new terminal, so that `codewiki` and `codewiki-mcp` are on your `PATH`.
{% endstep %}

{% step %}
### Add the marketplace and the plugin

```
codex plugin marketplace add "%LOCALAPPDATA%\Programs\CodeWiki\plugins"
codex plugin add codewiki@codewiki
```

In PowerShell the folder is `"$env:LOCALAPPDATA\Programs\CodeWiki\plugins"`.

Without the Windows package, add the repository instead:

```
codex plugin marketplace add Fly-onlyone/CodeWiki --sparse .agents/plugins --sparse plugins/codex
```

With `--sparse`, Codex downloads only the marketplace file and the plugin's folder, not the whole repository. A marketplace added from GitHub without `--sparse` holds the whole repository with its history. Run `codex plugin marketplace remove codewiki`, then add it again with this command.
{% endstep %}

{% step %}
### Trust the hook once

Codex skips a hook that comes with a plugin until you trust it. Run `/hooks` in Codex and trust the `SessionStart` hook that runs `codewiki hook session-start --agent codex`. Do it again after an update changes the hook.
{% endstep %}

{% step %}
### Start a new Codex session

Start a new session in the repository.
{% endstep %}
{% endstepper %}

{% hint style="warning" %}
The hook needs `--agent codex`. Without it, the hook gives the order that Claude Code follows, and Codex cannot follow that. The plugin sets it for you.
{% endhint %}

## Writing notes with Codex

With [auto explaining](writing-notes.md#auto-explaining) on, the briefing tells Codex to use the `write-notes` skill after your first request, for as many notes as **Notes per session** says (20 by default, or Unlimited). Codex lists 20 notes at a time. Codex has no background agent: it writes the notes itself, after it finishes your request.

## Update or remove the plugin

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

* Codex starts the MCP server in the folder the session runs in. Every tool except `list_repos` and `get_workspace` uses that folder unless it is given a `repo` path.
* The hook looks for the wiki in the same folder and in the folders above it, so a session started in a subfolder still gets the briefing. It reads the wiki's manifest and asks git for `HEAD`. It does not start the backend. On any error it prints nothing and exits with status 0, so it cannot fail a session.

The bridge starts a local backend on first use. To use a backend in Docker or on another machine, set `CODEWIKI_URL` and `CODEWIKI_TOKEN` in the environment Codex runs in, or save it in Settings → Backend.

## Without the plugin

You can set up the same by hand, in `~/.codex/config.toml`:

```toml
[mcp_servers.codewiki]
command = "codewiki-mcp"
args = []

[[hooks.SessionStart]]
matcher = "startup|resume|clear|compact"

[[hooks.SessionStart.hooks]]
type = "command"
command = "codewiki hook session-start --agent codex"
timeout = 10
```

`codex mcp add codewiki -- codewiki-mcp` adds the server alone. A hook added this way also waits for `/hooks` before it runs.

To use a backend on another machine, add `env = { CODEWIKI_URL = "http://127.0.0.1:7411", CODEWIKI_TOKEN = "<token>" }` to the `[mcp_servers.codewiki]` table.

{% hint style="warning" %}
Do not use the plugin and this setup together. Codex would start two `codewiki` servers.
{% endhint %}
