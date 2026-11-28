---
description: The 27 tools a coding agent gets, by what they are for.
icon: wrench
---

# MCP tools

`codewiki-mcp` gives a coding agent 27 tools. Every tool except `list_repos` and `get_workspace` takes an optional `repo` path, and works on the folder the agent runs in when you give none.

In a folder [kept from coding agents](keep-agents-out.md), every tool refuses and says that CodeWiki is turned off for this project.

## Find your way

| Tool               | What it returns                                                                 |
| ------------------ | ------------------------------------------------------------------------------- |
| `get_overview`     | The wiki's map note and the state of the index. Start here.                     |
| `get_architecture` | The architecture flow notes in full.                                            |
| `get_workspace`    | Every registered repository, and the external packages two or more of them use. |
| `list_repos`       | The repositories the backend manages.                                           |

## Read the code

| Tool                  | What it returns                                                                                                                                                                   |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_context`         | What to know before touching a file or symbol.                                                                                                                                    |
| `get_symbol`          | A symbol's definition, members, dependencies and users.                                                                                                                           |
| `search_codebase`     | Full-text search over notes, symbols and files. `kind` keeps one of `note`, `symbol` or `file`.                                                                                   |
| `get_answer`          | The note summaries that best match a question.                                                                                                                                    |
| `read_note`           | One note, by name or vault path. It leaves out the pictures, and the walk-through of calls under `How it works` while CodeWiki alone wrote it. `full: true` gives the whole text. |
| `get_dependency_path` | The shortest dependency chain between two symbols.                                                                                                                                |
| `get_why`             | Why a file or symbol is the way it is: decision records that cite it, rationale recorded in notes, and the latest commits that touched it.                                        |
| `list_decisions`      | Decision records with their status, and which are stale because the code they cite changed.                                                                                       |

## Weigh a change

| Tool                 | What it returns                                                                                                                                                                                            |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_risk`           | The risk of changing one file, from 0 to 100, with the reasons.                                                                                                                                            |
| `get_change_risk`    | The risk of a set of changed files taken together (the uncommitted changes when you name none), files that usually change with them but are missing, and files outside the change that depend on it.       |
| `get_blast_radius`   | The files affected if a file changes its behavior: direct dependents first, then further hops.                                                                                                             |
| `get_impacted_tests` | The tests to run after a change (by default the uncommitted changes), nearest first, with the commands that run them, and changed files no test reaches. It covers every language CodeWiki reads in depth. |

The commands by language: `cargo test` for Rust, `dotnet test` for C#, `go test` for Go, `gradle` or `mvn` for Java and Kotlin, `ctest` or the test executable for C and C++, `pytest` or unittest for Python, and the project's own runner (`vitest`, `jest`, `mocha`, `playwright` or `ava`) through its package manager for TypeScript and JavaScript.

## Check code health

| Tool                    | What it returns                                                                                                                                                                                                 |
| ----------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `get_hotspots`          | Files that are both large and often changed, with the main author and the bus factor.                                                                                                                           |
| `get_dead_code`         | Symbols nothing refers to, marked `likely` or `possible`.                                                                                                                                                       |
| `get_health`            | Files that score below 10, with the findings behind the score, plus documentation coverage and drift.                                                                                                           |
| `get_coverage`          | Line coverage from an LCOV or Cobertura report, with the least covered files ranked by how often they change and how much depends on them. `report` imports a new report.                                       |
| `get_security_findings` | Security patterns in Rust source, test code left out: hard-coded secrets, shell commands and SQL built from strings, switched-off TLS checks, `unsafe` blocks and more. `level` sets the lowest level returned. |
| `get_refactoring_plan`  | Refactoring suggestions, most important first, each with its evidence and ordered steps.                                                                                                                        |
| `get_contracts`         | The HTTP endpoints a repository provides and the URL paths it calls, read from Rust source. With `workspace: true`, matched across all registered repositories.                                                 |

## Keep the wiki current

| Tool        | What it does                                                                                                                                                  |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `sync_wiki` | Indexes again and updates the wiki. A note a person edited is never overwritten; the change waits as a proposal. Says how many notes wait for an explanation. |

## Write notes

CodeWiki writes each note's facts. These three tools let an agent write the explanation. See [Agent-written notes](writing-notes.md).

| Tool                 | What it does                                                                                                                                                                                                                                                                                              |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `list_waiting_notes` | Lists the notes that wait for an explanation: those whose explanation may be behind the code first, then those never explained. `folder` and `files` narrow the list, and `limit` caps it. With `claim: true`, the listed notes are kept for this agent for 30 minutes, and other agents get other notes. |
| `get_note_brief`     | Gives what an agent needs to explain one note: the note as it is, which sections are the agent's, the source files to read first, and the limits of text and pictures.                                                                                                                                    |
| `submit_note`        | Sends the sections the agent wrote, with pictures where one explains more than text. CodeWiki checks them and puts them into the note. The answer is `saved`, or a list of faults to fix, or a request to read the brief again. A note a person edited is not written: the text waits as a proposal.      |
