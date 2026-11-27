# CodeWiki plugin for Claude Code

Registers the `codewiki` MCP server so Claude Code can read the repository's wiki and structural index, a `SessionStart` hook that tells Claude the wiki is there, the skill `write-notes`, with which Claude writes the explanations of the notes, and the agent `note-writer`, which writes them in the background when auto explaining is on.

## Requirements

`codewiki` and `codewiki-mcp` must be on `PATH`. The release zip's `install.cmd` puts them there (see Install in the root `README.md`); open a new terminal afterwards. To build them from source instead:

```
cargo install --path crates/codewiki
cargo install --path crates/codewiki-mcp
```

The bridge starts a local backend on first use. To use a backend running in Docker or on another machine instead, set `CODEWIKI_URL` and `CODEWIKI_TOKEN` in the environment Claude Code runs in.

## Install

```
claude plugin marketplace add "%LOCALAPPDATA%\Programs\CodeWiki\plugins"
claude plugin install codewiki@codewiki
```

The first line adds the `plugins` folder of an installation made from the release zip (in PowerShell: `"$env:LOCALAPPDATA\Programs\CodeWiki\plugins"`). The folder holds the plugin and nothing else. Claude Code reads it where it is, so nothing is downloaded and git is not needed.

Without that installation, add the repository in its place, or the folder of a clone:

```
claude plugin marketplace add Fly-onlyone/CodeWiki --sparse .claude-plugin plugins/claude-code
```

With `--sparse`, Claude Code checks out only the marketplace file and this plugin's folder, not the whole repository. It needs git. A marketplace added from GitHub without `--sparse` holds the whole repository: run `claude plugin marketplace remove codewiki`, then add it again with this command.

The plugin is installed for your user; `--scope project` on the install records it in the repository's `.claude/settings.json` instead.

Then start Claude Code, or run `/reload-plugins` in a session that is already open. `/mcp` lists the server as `plugin:codewiki:codewiki`.

To update or remove it:

```
claude plugin update codewiki@codewiki        # then restart Claude Code
claude plugin uninstall codewiki@codewiki
claude plugin marketplace remove codewiki
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
| `read_note` | One note: what the thing is for, its items, its gotchas and what it depends on. `full: true` adds the walk-through of calls under `How it works`. |
| `get_why` | Why a file or symbol is the way it is: decision records that cite it, rationale recorded in notes, and the latest commits that touched it. |
| `list_decisions` | Decision records with their status, and which are stale because the code they cite changed. |
| `get_risk` | Risk of changing one file, 0 to 100, with the reasons. |
| `get_change_risk` | Risk of a set of changed files taken together (by default the uncommitted changes), files that usually change with them but are missing, and files outside the change that depend on it. |
| `get_blast_radius` | Files affected if a file changes its behavior: direct dependents, then further hops. |
| `get_impacted_tests` | Tests to run after a change (by default the uncommitted changes), nearest first, with the commands that run them (`cargo test` for Rust, `dotnet test` for C#, `go test` for Go), plus changed files no test reaches. |
| `get_hotspots` | Files that are both large and often changed, with main author and bus factor. |
| `get_dead_code` | Symbols nothing refers to, marked `likely` or `possible`. Verify before deleting. |
| `get_health` | Files scoring below 10 with the findings behind the score, plus documentation coverage and drift. |
| `get_coverage` | Line coverage from an LCOV or Cobertura report, with the least covered files ranked by how often they change and how much depends on them. `report` imports a new report. |
| `get_security_findings` | Security pattern findings in Rust source, test code excluded: hard-coded secrets, shell commands and SQL built with `format!`, disabled TLS verification, `unsafe` blocks with no `// SAFETY:` comment and more. |
| `get_refactoring_plan` | Refactoring suggestions, highest priority first, each with its evidence and ordered steps. No model is involved. |
| `get_contracts` | HTTP endpoints a repository provides and the URL paths it calls. With `workspace: true`, matched across all registered repositories. |
| `sync_wiki` | Re-index and update the wiki. Notes a person edited are never overwritten; changes to them wait as proposals. Says how many notes wait for an explanation. |
| `list_waiting_notes` | The notes that wait for an agent to explain them: those whose explanation may be behind the code first, then those never explained. `folder` and `files` narrow the list. |
| `get_note_brief` | What an agent needs to explain one note: the note, which sections are the agent's, the source files to read, and the limits of text and pictures. |
| `submit_note` | Send the sections an agent wrote for a note, with pictures. CodeWiki checks them and puts them into the note; it answers `saved` or the faults to fix. A note a person edited is not written: the text waits as a proposal. |
| `list_repos` | Repositories the backend manages. |
| `get_workspace` | Every registered repository at a glance, and the external packages two or more of them use. |

Every tool except `list_repos` and `get_workspace` takes an optional `repo` path and defaults to the directory Claude Code runs in.

## Writing notes

CodeWiki writes each note's facts from the code: where the thing is, its items, what it depends on. What the thing is for, how it works and what can go wrong is written by Claude, through the skill `write-notes`. CodeWiki itself calls no AI model.

```
/codewiki:write-notes                      the first 5 notes that wait
/codewiki:write-notes waiting 12           the first 12
/codewiki:write-notes waiting all          every note that waits, 20 at a time
/codewiki:write-notes 01-Core/cw-parse/    the waiting notes of one folder
/codewiki:write-notes "Scan Module"        one note
/codewiki:write-notes changed              the notes about the files you changed
```

For each note Claude reads the note's source files, writes the sections that are its own, and sends them with `submit_note`. CodeWiki checks what it gets and puts it into the note. Claude cannot change a fact: it sends sections, never a whole note.

- **Pictures.** Claude may add a picture to a section: boxes joined by arrows that show what happens from a cause to its result, and marked key points. The app draws them. `skills/write-notes/pictures.md` lists the components and their limits.
- **By itself.** After a code change, Claude writes the notes about the files it changed, at most three. The count of waiting notes in the session briefing does not start it on its own; auto explaining does (below).
- **Auto explaining** is on by default and is switched in the app (Settings → Repositories → "Auto explaining"; the file `auto-explain.txt` in the data directory, where `off` is off and a missing file is on). Under the switch, "Notes per session" sets how many notes a session explains: 5, 10, 20 (the default), 50, 100 or Unlimited (the file `auto-explain-count.txt`, a number or `all`; a missing or unreadable file means 20). When it is on, a session that starts fresh (`startup`, `resume` or `clear`) and is not in a linked git worktree gets an order in the briefing. The hook leaves the order out when the host reports plan mode; the order itself tells the agent to skip it in plan mode. The order says: when the user sends the first message, launch the `codewiki:note-writer` agent in the background (`agents/note-writer.md`: Sonnet, tools `Read`, `Grep`, `Glob`, `Skill` and the CodeWiki MCP tools, the `write-notes` skill preloaded) to explain that many waiting notes, stale first, and tell the user in one line. An agent writes at most 20 notes, so a larger count launches one agent for each 20, all at once: 50 notes take three agents (20, 20 and 10), and Unlimited with 203 notes waiting takes eleven. Each agent lists its notes once, with `claim: true`, so that the other agents, and another session's, get other notes; a claim lasts 30 minutes. It edits no file and ends with one line: how many notes it saved, proposed or failed. A folder kept from coding agents gets no briefing and no agent. Each note uses your Claude usage allowance, and several agents at once use it faster.
- **Permissions.** The agent runs in the background, yet each CodeWiki tool call it makes asks you in the main session unless `mcp__plugin_codewiki_codewiki__*` is allowed in your permission rules. Allow it to let the agent work without prompts.
- **A note you edited** is never written. Claude's text waits in the review queue as a proposal.
- **After a code change** with no agent running, `sync_wiki` and `codewiki update` refresh the facts of an explained note and keep its explanation. The note is marked as possibly behind and comes first in the list of waiting notes.

Writing many notes uses your Claude usage allowance: each note is a few file reads and one or two tool calls.

## Session-start hook

`hooks/hooks.json` runs `codewiki hook session-start` on the `SessionStart` event: when a session starts or resumes, and after `/clear` and compaction. Claude Code adds what the command prints to Claude's context. The briefing is four or five lines, or two when the repository has no wiki yet:

- whether the repository has a wiki, and where its home note is (`obsidian/_HOME.md`);
- whether the wiki is up to date with `HEAD` or stale. It is stale only when files outside `obsidian/` changed since the commit the wiki was generated at; committing the wiki itself does not make it stale. If git no longer knows that commit (after a rebase, or in a shallow clone), the wiki counts as stale;
- how many proposals wait for review;
- how many notes wait for an explanation, when any does. The line states the count. It tells Claude to write notes when you ask or for files it changed; with auto explaining on and a fresh session, it orders the note-writer instead;
- that the `get_overview` and `get_context` tools answer from the wiki.

For example:

```
CodeWiki: this repository has a maintained wiki of 123 notes in `obsidian/`. Its home note is `obsidian/_HOME.md`.
The wiki is stale: it was generated at commit 3f2a9c1b7e, and HEAD is 9d04e6a1c2. The MCP tool `sync_wiki` brings it up to date.
Pending proposals: 2 (changes to human-edited notes that wait for review; those notes may lag behind the code).
Notes waiting for an explanation: 14 (3 whose explanation may be behind the code, 11 never explained). CodeWiki writes each note's facts from the code; what the thing is for and how it works is written by a coding agent, with the MCP tools `list_waiting_notes`, `get_note_brief` and `submit_note`. This line says what waits; write notes when the user asks, or for files you changed.
The CodeWiki MCP tools `get_overview` (architecture and index status) and `get_context` (what to know before touching a file or symbol) answer from this wiki. Call them before exploring the code by hand.
```

The command reads the vault's manifest and asks git for `HEAD`. It does not start the backend, does not open the index, and writes nothing. It looks for the vault in the project folder (`CLAUDE_PROJECT_DIR`, otherwise the working directory) and its parent folders. It cannot fail a session: on any error it prints nothing and exits with status 0. The hook has a 10 second timeout; a run takes well under a second.

With auto explaining on and a fresh session, the end of the notes line is the order instead. With the default of 20 notes:

```
The user turned on auto explaining: when the user sends their first message, unless you are in plan mode, launch the `codewiki:note-writer` agent in the background to explain up to 20 waiting notes, stale first, tell the user in one line that it is writing wiki notes, then go on with their request. Do not write these notes yourself.
```

With 50 notes:

```
The user turned on auto explaining: when the user sends their first message, unless you are in plan mode, launch 3 `codewiki:note-writer` agents in the background at once, in one message, and tell 2 of them to explain 20 waiting notes each and the last to explain 10, stale first; tell the user in one line that 3 agents are writing wiki notes, then go on with their request. Do not write these notes yourself.
```

The hook reads the data directory's `auto-explain.txt` and `auto-explain-count.txt` and the hook's input (`source`, `permission_mode`). In the app, "To explain (N)" in the toolbar shows the notes that wait and opens the Waiting notes page.

To try it outside Claude Code, run `codewiki hook session-start` in a repository.
