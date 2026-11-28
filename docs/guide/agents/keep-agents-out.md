---
description: Keep CodeWiki away from coding agents in chosen folders.
icon: ban
---

# Folders kept from coding agents

You can keep coding agents from using CodeWiki in a folder. Use it for a project whose code or wiki must not reach an agent.

The list covers Claude Code and every other agent that calls CodeWiki through its MCP bridge. The app and the command line still work in a kept folder: the list keeps agents out, not you.

## What the list covers

An entry covers its folder and everything below it. It also keeps agents out of every folder that holds it. With `D:\work\clients\acme` listed, an agent working in `D:\work`, or naming `D:\` as a tool's `repo`, gets no CodeWiki, because indexing that folder would index acme's code.

Folders are compared by whole names. `D:\work\client` neither holds `D:\work\clients\acme` nor lies in it.

The list is the file `agents-off.json` in the data folder (`%LOCALAPPDATA%\CodeWiki`, or the folder that `CODEWIKI_DATA_DIR` names).

## Change the list

{% tabs %}
{% tab title="In the app" %}
Settings → Repositories → **Kept from coding agents** adds and removes folders. **Add folder…** picks one. **Remove** takes one off.

On [Home](../the-app/home.md), a project's row has **Keep agents out** and **Let agents in**. The command palette has **Keep agents out of this project** and **Let agents into this project**.

A project kept by a folder above it says so (**Kept from agents by \<folder>**), and so does a project that holds a kept folder (**Kept from agents (contains \<folder>)**). Such a row has a **Settings…** button in place of a **Keep agents out** button. A project that is on the list itself while a folder above it is listed too shows the folder above, since removing its own entry would not let agents in.
{% endtab %}

{% tab title="On the command line" %}
```
codewiki agents off [<folder>]
codewiki agents on [<folder>]
codewiki agents list
codewiki agents status [<folder>]
```

Without a folder, the current one is used.

* `off` adds the folder to the list.
* `on` removes it. If a folder above it is still listed, it says `still kept from coding agents by <entry>` and exits with status 1.
* `list` prints the folders. `--json` prints them as JSON.
* `status` says `kept from coding agents by <entry>` for a covered folder, `kept from coding agents: contains <entry>` for one that holds an entry, and `open to coding agents` otherwise.
{% endtab %}

{% tab title="In the file" %}
Edit `agents-off.json` by hand. It holds a version number and a list of folders:

```json
{"version": 1, "folders": ["D:\\work\\secret"]}
```

Each entry must be an absolute folder. A list with an empty or relative entry cannot be read.
{% endtab %}
{% endtabs %}

## What an agent sees

In a kept folder, or in one that holds a kept folder:

* The session-start briefing prints nothing.
* Every MCP tool refuses, with an answer that tells the agent that CodeWiki is turned off for this project. The skills tell the agent to stop using CodeWiki's tools and the `codewiki` command for the rest of the session.

Elsewhere:

* A tool whose `repo` names a kept repository, or one that holds a kept folder, refuses for that repository alone: **CodeWiki is turned off for that repository**. The skills tell the agent not to use CodeWiki for that repository again.
* `list_repos`, `get_workspace` and `get_contracts` with `workspace: true` leave out kept repositories and those that hold a kept folder.

A project that holds a kept folder is kept from agents as a whole, its root included. To let agents into the rest of it, move the kept code out of the project or take the entry off the list.

{% hint style="warning" %}
A list that cannot be read keeps every agent out, until you fix or delete the file. Settings offers to **Start a new list**.
{% endhint %}

## With a backend on another machine

The bridge reads that backend's repository paths as paths on this machine. A repository whose path names a kept folder here, or holds one, is refused and hidden. On Windows a Linux path such as `/srv/code/app` reads as `\srv\code\app` on the current drive. Other repositories of that backend are not hidden, because the list cannot name the other machine's folders.

## Another way, for Claude Code only

Claude Code has its own per-project switch. In the file `.claude/settings.local.json` of the project, set the plugin `codewiki@codewiki` to `false` under `enabledPlugins`. Claude Code then does not load the plugin there at all. It covers Claude Code only. The list above covers every agent that calls CodeWiki through the MCP bridge.
