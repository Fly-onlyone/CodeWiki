---
name: using-codewiki
description: Use when working in a repository that has a CodeWiki wiki (an `obsidian/` folder with `_HOME.md`, or a session briefing that starts with "CodeWiki:"), before exploring unfamiliar code, before editing a file or symbol, after a change, or when asked to write or refresh the wiki. Says which `codewiki` MCP tool to call at each step.
---

# Using CodeWiki

CodeWiki keeps a wiki of the repository in `obsidian/` and a structural index of the code. The `codewiki` MCP tools answer from both. They are faster and more complete than reading files one by one, so call them before you explore by hand.

Every tool except `list_repos` and `get_workspace` takes an optional `repo` path. It defaults to the folder Codex runs in.

## When to call which tool

| Moment | Call |
|---|---|
| Start of work in a repository you do not know | `get_overview`: the home note, the architecture map and whether the index is current. `get_architecture` gives the flow notes in full. |
| Looking for something | `search_codebase` for notes, symbols and files; `get_answer` for the notes that best answer a question; `read_note` for one note. It leaves out `How it works`, the walk-through of calls, which you read in the code; pass `full: true` when you want it too. |
| Before you edit a file or symbol | `get_context` on it. For a symbol, `get_symbol` gives its definition, members, dependencies and users. |
| The code looks deliberate and you want to change it | `get_why`: decision records that cite it, recorded rationale and recent commits. `list_decisions` lists the records. |
| Before you change a file others depend on | `get_risk` for one file, `get_blast_radius` for what a change in behaviour reaches, `get_dependency_path` for how two symbols connect. |
| After a change, before you finish | `get_change_risk` on the uncommitted changes, then `get_impacted_tests` for the tests to run. |
| Code health questions | `get_hotspots`, `get_health`, `get_dead_code`, `get_coverage`, `get_security_findings`, `get_refactoring_plan`, `get_contracts`. |
| Several repositories | `list_repos`, `get_workspace`. |

## Writing and refreshing the wiki

- **No wiki yet.** `get_overview` says so, and the session briefing says "no wiki has been generated". Call `sync_wiki`. It indexes the repository and writes the wiki into `obsidian/`.
- **Wiki behind the code.** The briefing says "The wiki is stale", or `get_overview` reports it. Call `sync_wiki` after substantial code changes, and before you rely on notes about code that just changed.
- `sync_wiki` never overwrites a note a person edited. It turns the change it wanted to make into a proposal that waits for review in the CodeWiki app, or through `codewiki proposals`, `codewiki approve <id>` and `codewiki reject <id>`. Do not approve proposals yourself unless the user asks.

## Rules

- Never edit `obsidian/.codewiki/`. It holds CodeWiki's manifest and proposals.
- To change a note by hand, follow the `wiki-note-style` skill. A note you edit counts as edited by a person from then on: CodeWiki proposes changes to it instead of rewriting it.
- To record a decision, run `codewiki decisions --new "<title>"` and fill in the note it creates.
- A tool error that says the backend is not reachable usually means `codewiki-mcp` could not start one. Tell the user, and do not work around it by editing the wiki's files.
