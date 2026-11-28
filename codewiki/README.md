---
description: >-
  A wiki for your codebase that stays current and serves both people and coding
  agents.
icon: house
---

# CodeWiki

CodeWiki reads a repository and writes a wiki about it: what the packages and modules are, how the program starts, what depends on what, and where the risks are. It keeps the wiki current as the code changes, and it gives the same knowledge to coding agents such as Claude Code and Codex.

![The wiki map of a project in the CodeWiki app, with the sidebar, the toolbar and the details pane](<.gitbook/assets/wiki-map (1).png>)

## What you get

* **A wiki in your repository.** Plain Markdown notes in an `obsidian` folder, with diagrams. They are versioned with your code and readable without CodeWiki, on GitHub or in Obsidian.
* **A wiki that keeps up.** After new commits, CodeWiki rewrites only the notes whose source changed.
* **Your edits stay yours.** A note you edited is never overwritten. A change CodeWiki wants to make waits as a proposal until you approve or reject it.
* **Explanations from your coding agent.** CodeWiki writes the facts of each note from your code. Your coding agent writes what a thing is for and how it works, through the plugin. CodeWiki calls no language model and needs no API key. See [Agent-written notes](agents/writing-notes.md).
* **Answers for agents.** A coding agent gets the wiki, symbol lookups, search, and risk and impact analysis through 27 tools.
* **Search by meaning, if you want it.** Full-text search always works. You can add vectors from a model server that you run on your machine. See [Set up semantic search](getting-started/semantic-search.md).
* **Analysis of the code.** Hotspots, ownership, change coupling, blast radius, dead code, documentation drift, file health, test impact, coverage gaps, security patterns and refactoring suggestions.

CodeWiki reads the symbols of Rust, Python, TypeScript, JavaScript, Java, C#, Go, Kotlin, C and C++. A repository in any other language still gets a wiki of its folders and files. See [Languages](reference/languages.md).

## Start here

<table data-view="cards"><thead><tr><th></th><th></th><th data-hidden data-card-target data-type="content-ref"></th></tr></thead><tbody><tr><td><strong>Install</strong></td><td>Run the setup program, or unpack the zip.</td><td><a href="getting-started/install.md">install.md</a></td></tr><tr><td><strong>Your first wiki</strong></td><td>Open a project and let CodeWiki write its wiki.</td><td><a href="getting-started/your-first-wiki.md">your-first-wiki.md</a></td></tr><tr><td><strong>Connect Claude Code</strong></td><td>Give your coding agent the wiki.</td><td><a href="agents/claude-code-plugin.md">claude-code-plugin.md</a></td></tr><tr><td><strong>Connect Codex</strong></td><td>The same for Codex, with its own plugin.</td><td><a href="agents/codex.md">codex.md</a></td></tr></tbody></table>

## The three programs

| Program                | What it is for                                                  |
| ---------------------- | --------------------------------------------------------------- |
| **CodeWiki** (the app) | Read the wiki, explore the graph, review proposals. Windows 11. |
| `codewiki`             | The same from a terminal, and in CI.                            |
| `codewiki-mcp`         | The bridge that coding agents use.                              |

All three talk to one backend, which runs on your machine and starts by itself. CodeWiki sends nothing about you or your code unless you turn on [error reports](reference/error-reports.md).
