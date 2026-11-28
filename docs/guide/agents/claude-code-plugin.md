---
description: >-
  Give Claude Code the wiki, so that it reads what CodeWiki knows before it
  explores by hand, and lets it write the notes' explanations.
icon: plug
---

# Claude Code plugin

The plugin connects Claude Code to the wiki and the index. It adds four things:

* **Tools.** Claude gets 27 tools that read the wiki, look up symbols, search, weigh the risk of a change and write notes. See [MCP tools](mcp-tools.md).
* **A briefing when a session starts.** Four or five lines tell Claude whether the repository has a wiki, whether the wiki is behind the latest commit, how many proposals wait for review, how many notes wait for an explanation, and which tools to call first. See [The session-start briefing](claude-code-plugin.md#the-session-start-briefing).
* **The skill `write-notes`**, also the command `/codewiki:write-notes`. With it Claude explains the notes that wait. See [Agent-written notes](writing-notes.md).
* **The agent `note-writer`.** With [auto explaining](writing-notes.md#auto-explaining) on, it writes notes in the background when a session starts.

## Install the plugin

{% stepper %}
{% step %}
### Install CodeWiki

Follow [Install](../getting-started/install.md). Then open a new terminal, so that `codewiki` and `codewiki-mcp` are on your `PATH`.
{% endstep %}

{% step %}
### Add the marketplace

Add the `plugins` folder of your installation as a plugin marketplace:

```
claude plugin marketplace add "%LOCALAPPDATA%\Programs\CodeWiki\plugins"
```

In PowerShell the folder is `"$env:LOCALAPPDATA\Programs\CodeWiki\plugins"`. The folder holds the agent plugins and nothing else. Claude Code reads it where it is: nothing is downloaded, and git is not needed.

Without the Windows package (programs built from source, or a backend on another machine), add the repository instead, or the folder of a clone:

```
claude plugin marketplace add Fly-onlyone/CodeWiki --sparse .claude-plugin plugins/claude-code
```

With `--sparse`, Claude Code checks out only the marketplace file and the plugin's folder, not the whole repository. It needs git. A marketplace added from GitHub without `--sparse` holds the whole repository. Remove it with `claude plugin marketplace remove codewiki`, then add it again with this command.
{% endstep %}

{% step %}
### Install the plugin

```
claude plugin install codewiki@codewiki
```

The plugin is installed for your user. To record it in the repository's `.claude/settings.json` instead, which you can commit to share it with your team, add `--scope project`.
{% endstep %}

{% step %}
### Write the wiki

In a repository that has no wiki yet:

```
codewiki init
```
{% endstep %}

{% step %}
### Start Claude Code

Start Claude Code in the repository. In a session that is already open, run `/reload-plugins`. `/mcp` lists the server as `plugin:codewiki:codewiki`.
{% endstep %}
{% endstepper %}

## How it reaches the wiki

The plugin runs `codewiki-mcp`, a small program that passes Claude's calls to the CodeWiki backend. It starts a backend on your machine at the first call, when none is running.

{% hint style="info" %}
The desktop app keeps the backend running while its window is hidden in the tray. When you quit the app, it stops the backend. If Claude Code was using the same backend, its next call starts a new one. You do not have to do anything.
{% endhint %}

To use a backend in Docker or on another machine, set `CODEWIKI_URL` and `CODEWIKI_TOKEN` in the environment Claude Code runs in, or save it in Settings → Backend.

## Let the note-writer work without prompts

The `note-writer` agent runs in the background, but each CodeWiki tool call it makes asks you in the main session. To let it work without prompts, allow `mcp__plugin_codewiki_codewiki__*` in your permission rules.

## The session-start briefing

Each time a session starts or resumes, and after `/clear` or compaction, the plugin runs `codewiki hook session-start`. Claude Code adds what the command prints to Claude's context. You do not see it in the chat.

The briefing is four or five lines. For example:

```
CodeWiki: this repository has a maintained wiki of 123 notes in `obsidian/`. Its home note is `obsidian/_HOME.md`.
The wiki is stale: it was generated at commit 3f2a9c1b7e, and HEAD is 9d04e6a1c2. The MCP tool `sync_wiki` brings it up to date.
Pending proposals: 2 (changes to human-edited notes that wait for review; those notes may lag behind the code).
Notes waiting for an explanation: 14 (3 whose explanation may be behind the code, 11 never explained). CodeWiki writes each note's facts from the code; what the thing is for and how it works is written by a coding agent, with the MCP tools `list_waiting_notes`, `get_note_brief` and `submit_note`. This line says what waits; write notes when the user asks, or for files you changed.
The CodeWiki MCP tools `get_overview` (architecture and index status) and `get_context` (what to know before touching a file or symbol) answer from this wiki. Call them before exploring the code by hand.
```

### What each line tells Claude

* **The wiki.** How many notes it has, and where its home note is. The home note maps the architecture and links every area.
* **Up to date or stale.** The wiki is stale when files outside `obsidian/` changed since the commit the wiki was written from. Committing the wiki itself does not make it stale. If git no longer knows that commit, after a rebase or in a shallow clone, the wiki counts as stale. A stale line names both commits and points Claude to `sync_wiki`. In a folder without git, the line says that the wiki cannot be compared with HEAD.
* **Pending proposals.** How many changes to notes you edited wait for your review. The line warns Claude that those notes may lag behind the code. See [Review](../the-app/review.md).
* **Notes that wait.** Shown only when some do. The line states how many are behind the code and how many were never explained, and says to write notes when you ask, or for files Claude changed. With [auto explaining](writing-notes.md#auto-explaining) on and a fresh session, the end of the line is an order to start the `note-writer` agent instead.
* **Where to start.** `get_overview` and `get_context` answer from the wiki, and Claude is told to call them before it explores the code by hand. See [MCP tools](mcp-tools.md).

### When there is no wiki yet

The briefing is two lines. They tell Claude that `sync_wiki` writes the wiki:

```
CodeWiki: no wiki has been generated for this repository yet (its home note would be `obsidian/_HOME.md`).
The CodeWiki MCP tool `sync_wiki` generates it. After that, `get_overview` and `get_context` answer questions about architecture and about single files or symbols.
```

### What it reads, and what it does not do

The command reads the wiki's manifest and its proposals, and asks git for the latest commit. It does not start the backend, does not open the index and writes nothing. It does not update the wiki either: Claude decides whether to call `sync_wiki`.

It starts in the project folder and goes up through the folders above it until it finds a wiki. A session started in a subfolder of the repository still gets the briefing.

A folder [kept from coding agents](keep-agents-out.md) gets no briefing at all.

### It cannot break a session

On any error the command prints nothing and exits with status 0. Claude Code then starts the session without a briefing. The hook has a 10 second timeout; a run takes well under a second.

### See what Claude sees

Run the command yourself in the repository:

```
codewiki hook session-start
```

To check another repository, name its folder:

```
codewiki --repo <folder> hook session-start
```

If it prints nothing, see [Troubleshooting](../troubleshooting.md#claude-does-not-mention-the-wiki-when-a-session-starts).

## Update or remove the plugin

```
claude plugin update codewiki@codewiki
claude plugin uninstall codewiki@codewiki
claude plugin marketplace remove codewiki
```

Restart Claude Code after an update.
