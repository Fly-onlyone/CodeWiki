---
description: Every codewiki command, with the options you are most likely to need.
icon: terminal
---

# Command line

`codewiki` does everything the app does, from a terminal. Run it inside a repository, or name one with `--repo`.

```
codewiki [--repo <folder>] [--json] <command>
```

| Option            | Meaning                                                       |
| ----------------- | ------------------------------------------------------------- |
| `--repo <folder>` | The repository to work on. Default: the current folder.       |
| `--json`          | Print JSON instead of text, for scripts.                      |
| `--version`       | Print the version.                                            |
| `--help`          | List the commands. `codewiki <command> --help` describes one. |

## Write and update the wiki

| Command               | What it does                                                                                                                          |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `codewiki init`       | Indexes the repository and writes the wiki for the first time, into `obsidian\` inside it.                                            |
| `codewiki update`     | Indexes again and rewrites only the notes whose source changed. Run it after new commits.                                             |
| `codewiki reindex`    | Rebuilds the index and leaves the notes alone.                                                                                        |
| `codewiki status`     | Shows the state of the index and the wiki.                                                                                            |
| `codewiki agent-docs` | Writes a short project briefing into `CLAUDE.md` and `AGENTS.md`, between marker comments. Text outside the markers is never changed. |

{% hint style="info" %}
A note you edited is never overwritten. `update` turns the change it wanted to make into a proposal, which waits until you approve or reject it.
{% endhint %}

## Review proposals

| Command                 | What it does                                                |
| ----------------------- | ----------------------------------------------------------- |
| `codewiki proposals`    | Lists the proposals that wait for review.                   |
| `codewiki approve <id>` | Applies a proposal to its note.                             |
| `codewiki reject <id>`  | Discards a proposal. The same change is not proposed again. |

The command line decides one proposal at a time. To decide several at once, use the [Review](../the-app/review.md) page in the app.

## Look things up

| Command                         | What it does                                                                                                                                   |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `codewiki search <words>`       | Searches notes, symbols and files. `--kind note`, `symbol` or `file` keeps one kind. `--limit <n>` sets how many hits to show (10 by default). |
| `codewiki symbol <name>`        | Looks up symbols by name.                                                                                                                      |
| `codewiki why <file or symbol>` | Says why something is the way it is: decision records, recorded rationale and recent commits.                                                  |
| `codewiki decisions`            | Lists decision records. `--new <title>` creates one.                                                                                           |
| `codewiki workspace`            | Shows every registered repository, and the external packages two or more of them use.                                                          |

## Analyse the code

| Command                           | What it reports                                                                                                                                                                                                                                                                                   |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `codewiki hotspots`               | Files that are both large and often changed.                                                                                                                                                                                                                                                      |
| `codewiki health`                 | A health score for each source file, worst first.                                                                                                                                                                                                                                                 |
| `codewiki risk [file]`            | The risk of changing a file, or of your uncommitted changes when you name none.                                                                                                                                                                                                                   |
| `codewiki tests`                  | Tests for each file, and source files no test reaches.                                                                                                                                                                                                                                            |
| `codewiki impacted-tests [files]` | The tests to run after changing those files, or after your uncommitted changes, with the commands that run them. It covers every language CodeWiki reads in depth.                                                                                                                                |
| `codewiki coverage`               | Imports an LCOV or Cobertura report and ranks the least covered files by how often they change and how much depends on them. It finds `lcov.info`, `coverage.xml` and similar names in the repository; `--report <file>` names another. `--limit <n>` sets how many gaps to list (20 by default). |
| `codewiki dead-code`              | Symbols nothing refers to, most certain first. Check before you delete.                                                                                                                                                                                                                           |
| `codewiki doc-drift`              | Notes that no longer match the code, and how much of the code is documented.                                                                                                                                                                                                                      |
| `codewiki security`               | Security patterns in Rust source, test code left out. `--level note`, `warning` or `error` sets the lowest level to show.                                                                                                                                                                         |
| `codewiki refactor`               | Refactoring suggestions, each with its evidence and ordered steps. `--limit <n>` sets how many to show (10 by default).                                                                                                                                                                           |
| `codewiki contracts`              | The HTTP endpoints the repository provides and the URL paths it calls, read from Rust source. `--workspace` matches the contracts of every registered repository against each other.                                                                                                              |

## Use it in CI

`codewiki check` runs every analysis and reports the findings.

```
codewiki check --format sarif --output codewiki.sarif --fail-on error
```

| Option            | Values                              | Default         |
| ----------------- | ----------------------------------- | --------------- |
| `--format`        | `text`, `sarif`, `junit`            | `text`          |
| `--fail-on`       | `note`, `warning`, `error`, `never` | `error`         |
| `--output <file>` | Write the report to a file          | Standard output |

The command exits with status 1 when a finding is at or above the `--fail-on` level.

## Keep agents out of a folder

`codewiki agents` keeps coding agents from using CodeWiki in a folder. Without a folder, the current one is used.

| Command                             | What it does                                                                                  |
| ----------------------------------- | --------------------------------------------------------------------------------------------- |
| `codewiki agents off [<folder>]`    | Adds the folder to the list of folders kept from coding agents.                               |
| `codewiki agents on [<folder>]`     | Takes the folder off the list. It exits with status 1 when a folder above it is still listed. |
| `codewiki agents list`              | Lists the folders kept from coding agents.                                                    |
| `codewiki agents status [<folder>]` | Says whether the folder is kept, and by which entry.                                          |

See [Folders kept from coding agents](../agents/keep-agents-out.md).

## Agent hooks

Coding agents run this command themselves. You run it only to see what an agent sees.

| Command                                         | What it does                                                                                                                                                                                                                                                                                                                                                                                                 |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `codewiki hook session-start [--agent <agent>]` | Prints the briefing an agent gets when a session starts: whether the repository has a wiki, whether the wiki is stale, how many proposals and notes wait, and which tools read it. `--agent` is `claude-code` (the default) or `codex`. It only reads. On any error it prints nothing and exits with status 0. See [The session-start briefing](../agents/claude-code-plugin.md#the-session-start-briefing). |

The Claude Code plugin runs it at the start of each session. The Codex plugin runs it with `--agent codex`; see [Codex](../agents/codex.md).

## Run the backend yourself

`codewiki serve` runs the HTTP backend that the app and coding agents use. You rarely need it: the app and the agent bridge start one when none is running.

| Option          | Meaning                                                                                   |
| --------------- | ----------------------------------------------------------------------------------------- |
| `--host <ip>`   | The address to listen on. Default: `127.0.0.1`. Any other address needs `CODEWIKI_TOKEN`. |
| `--port <port>` | The port. Default: `0`, which picks a free port.                                          |

Use it to run the backend on another machine or in Docker, then point the clients at it with `CODEWIKI_URL` and `CODEWIKI_TOKEN`, or save it in Settings → Backend.
