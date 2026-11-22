# Keep coding agents out of chosen folders

Date: 2026-11-22. Status: design, for approval.

## Intent

The owner wants to "blacklist a project and not generate a wiki for it". The unattended path that makes wikis today is the coding agents: with the CodeWiki plugin installed in Claude Code (or Codex), any CodeWiki tool an agent calls registers the folder it works in (`codewiki-mcp/src/main.rs` `Bridge::call` → `cw-client` `Backend::repo_id` → `POST /v1/repos`), and the session-start briefing tells the agent to call `sync_wiki` when no wiki exists (`cw-wiki/src/briefing.rs:43-46`; the Codex skill `using-codewiki` says the same), which writes an `obsidian/` folder into the project. The owner wants a list of folders where agents get no CodeWiki at all.

Owner's decisions (2026-11-22):

- **Scope: the agents only.** The desktop app and the command line keep working on a listed folder exactly as now: the owner can still open it, sync it and browse its wiki by hand.
- **An entry is a folder and everything below it**, so `D:\work\clients` covers every project under it, including ones cloned later, and an agent started in any subfolder.
- **In a listed folder every CodeWiki tool refuses**, with a plain answer that CodeWiki is turned off for this project, and the session-start hook prints nothing. Nothing is registered, indexed or written.
- **Managed from three places:** a card in Settings → Repositories, a button on each project's row on Home, and a command-line command.
- **Approach: a local list file** that the MCP bridge and the session-start hook read themselves (below).
- **Listed projects are hidden from agents everywhere:** an agent working in a folder that is not listed does not see listed projects in the cross-project tools either (the list of repositories, the workspace view, API contracts across projects). The reason is privacy as much as generation.

## Goals

- An agent working anywhere under a listed folder cannot register it, build its index or generate its wiki through CodeWiki, by any tool, with Claude Code or Codex.
- The agent is told once, clearly, that CodeWiki is off here, so it stops trying and does not retry or work around it.
- Adding and removing entries is easy from the app and from a terminal, and takes effect for the next tool call without restarting anything.

## Non-goals

- Blocking the desktop app or the command line (`codewiki init`, `update`, `serve` …). The owner may still choose to make a wiki for a listed folder by hand.
- Removing anything that exists: an already registered repository, its index or its `obsidian/` folder stay as they are. Removing them stays the job of Home's Remove and of the user.
- A per-project marker file committed into a repository (it would travel to other people; the owner wants a list of their own).
- Other MCP clients talking to the backend's HTTP API directly. The agents reach CodeWiki through the bridge and the hook only.

## Anti-goals

- An agent in a listed folder still ends up with an `obsidian/` folder, a registration in `repos.json` or an index in `index/` from any path.
- A listed folder that blocks the app or the command line.
- A list that only works while a backend runs, or that differs between the app and the agents on the same machine.
- Matching that misses because of letter case, a trailing separator, `/` versus `\`, or a relative path; or that over-matches a sibling (`D:\work\client` must not cover `D:\work\clients`).

## Approaches

1. **A local list file read by the bridge and the hook — chosen.** One file in CodeWiki's data folder. The MCP bridge checks it before every tool call, the hook before printing; the app and the command line edit it. No backend change, so the app and the command line keep working on listed folders as the owner wants; works with no backend running and with a backend in Docker or on another machine, since the agents run on this machine.
2. **The list in the backend's settings, enforced by the backend** for requests from the bridge (the client header `cw_core::CLIENT_HEADER`). One source of truth with the other settings, but the hook runs without a backend and would have to read the backend's file itself, and a remote backend's list would not be this machine's. Rejected.
3. **A marker file in each project, or Claude Code's per-project plugin switch** (`.claude/settings.json`). Per project, not a list the owner manages; the plugin switch is Claude Code's only, Codex needs its own. Rejected; the plugin switch stays a documented alternative in README.

## Design

### The list

- A file `agents-off.json` in the data folder (`cw_core::paths::data_dir()`, `%LOCALAPPDATA%\CodeWiki` by default; `CODEWIKI_DATA_DIR` moves it, as for everything else). Shape: `{ "version": 1, "folders": ["D:\\work\\clients", "C:\\src\\secret"] }`. A missing file is an empty list; one that does not parse fails closed (below). Never crash a tool call because of it.
- **What is stored:** the absolute, normalized form of the folder the user chose (the folder picker and the command line both give one); that is what Settings shows and what `remove` matches, case-insensitively. Symlinks and junctions are resolved at check time on both sides when the folders exist, so a folder created or linked later still matches. *My call.*
- **The app reads and writes this file itself,** through `cw-core`, which it already links. AGENTS.md says the app gets its data from the backend; like its own preference files, this list is local to the machine and is the agents' setting, not the backend's, so it is a named exception, added to AGENTS.md. *My call.*
- A small module in `cw-core` (for example `cw_core::agents_off`) owns it: `load() -> Vec<PathBuf>`, `add(path)`, `remove(path)`, `covering(path) -> Option<PathBuf>` (the entry that covers `path`, if any). Writes are atomic (write a temporary file, rename), as other files in the data folder.
- **Matching:** both sides are made absolute (relative to the current folder), normalized (`.` and `..` resolved, separators unified, no trailing separator), and compared component by component, case-insensitively on Windows. A path is covered when an entry equals it or is one of its ancestors. **A path that contains an entry is blocked too** (owner's decision after the review, 2026-11-22: "Refuse the bigger folder"): with `D:\work\clients\acme` listed, `D:\work` and `D:\` get no CodeWiki, because indexing them would index acme's code. `agents_off::blocking(entries, path)` gives `Block::Covered(entry)` or `Block::Contains(entry)`, and every reader uses it; `D:\work\client` neither contains `D:\work\clients\x` nor lies in it. An entry that is empty or not absolute makes the list unreadable. Canonicalizing through symlinks and junctions is done when the path exists (`dunce`-style, without the `\\?\` prefix; check what the workspace already uses), falling back to the normalized form.
- Adding an entry that an existing entry already covers is a no-op that says which one covers it; adding a parent of existing entries keeps them (they are harmless) — **builder's call** whether to fold them.

### The agents' side

- **MCP bridge** (`codewiki-mcp`): before any tool runs, global ones included (`list_repos`, `get_workspace`, `get_contracts` with `workspace: true`), the bridge decides which folders the call is about and refuses when any of them is covered or contains an entry:
  - the session's folders: its current folder **and** `CLAUDE_PROJECT_DIR` when set (the hook uses the latter first; checking both closes the case where an agent starts the bridge elsewhere). *My call.*
  - a `repo` argument that is a path (absolute, or relative to the current folder).
  - a `repo` argument that is not a path but an id or a folder name, which `cw-client`'s `Backend::repo_id` also accepts: the bridge looks it up in `GET /v1/repos` and checks the path the backend gives. That lookup is the one HTTP request a refusal may cost; a backend on another machine gives its own paths, which this machine's list cannot match, so there the check can only use the session's folders. *My call.*

  When a session folder is blocked, the bridge answers the tool call with a normal text result (not a protocol error): "CodeWiki is turned off for this project (`<entry>` is kept from coding agents in CodeWiki's settings). Do not call CodeWiki tools here; continue without them." It makes no HTTP request and does not start a backend for that call. The list is read on each call (it is tiny), so a change takes effect at once.
  When the session folders are open and the refusal comes from the `repo` argument (covered, containing an entry, or resolved by id or name to such a repository), the answer names no path and stops only that repository (owner's decision after the review, 2026-11-22: "No path, stop for that repo"): "CodeWiki is turned off for that repository; it is kept from coding agents in CodeWiki's settings. Do not call CodeWiki tools for it." With entries on the list, the bridge also checks the repository a call reaches once it has its id, through one listing, so a lookup that failed before the call or the backend's fallback to a repository's name cannot reach a kept repository.
  - The bridge's server instructions, sent once at connection, stay as they are; when the session's folder is covered, its `instructions` field instead says the same sentence, so an agent that reads instructions learns it before calling anything. **Builder's call** whether this is worth doing given that the first tool call says it anyway.
- **Cross-project tools in a session that is not covered** (`list_repos`, `get_workspace`, `get_contracts` with `workspace: true`): the bridge leaves out every repository whose backend path is covered or contains an entry, from the list, from the workspace's per-repository rows and shared dependencies, and from contract matches, so an agent elsewhere learns nothing about listed projects. With a remote backend its paths are read as paths on this machine: only one that names a kept folder here, or contains one, is left out; README says so.
- **Session-start hook** (`codewiki hook session-start`, `codewiki/src/main.rs` and `cw-wiki/src/briefing.rs`): when the project folder (`CLAUDE_PROJECT_DIR` or the current folder) **or** the vault root it finds above it is covered or contains an entry, it prints nothing and exits 0, as it does today on any problem. *My call: either one blocked is enough.*
- **A list file that cannot be read fails closed.** The bridge then refuses every tool with "CodeWiki's list of folders kept from coding agents (`<path>`) cannot be read; fix or delete it", the hook prints nothing, and the app's card shows the error with a button that starts a new, empty list. Keeping agents out is what the owner asked for, so a broken list must not let them back in silently. *My call.*
- The Claude Code skill `write-notes` and the Codex skills `using-codewiki` and `wiki-note-style` get two lines: when a CodeWiki tool says CodeWiki is turned off for this project, stop using CodeWiki's tools and the `codewiki` command in this session; when it says CodeWiki is turned off for that repository, do not use them for that repository again.

### Managing the list

- **Settings → Repositories**: a new card "Kept from coding agents", under the list of registered repositories, with one row per entry (the folder, a Remove button) and an "Add folder…" button that opens Windows' folder picker. A line under the title: "Claude Code and Codex get no CodeWiki tools in these folders and everything below them. The app and the command line still work there." An entry whose folder no longer exists is shown as such and can still be removed.
- **Home**: each project row gets a toggle-like action, judged by the project's registered folder alone: "Keep agents out" when nothing covers it, "Let agents in" when an entry for exactly that folder covers it. When a parent entry covers it, the row says "Kept from agents by `<entry>`" and points to Settings → Repositories instead of offering a button (removing the parent would release other projects too); so it does when the project's own entry and a parent entry both cover it, since removing its own would not let agents in. A project whose folder contains an entry (a subfolder is listed) is kept from agents as a whole: its row says "Kept from agents (contains `<entry's last folder name>`)", points to Settings → Repositories and offers no toggle, and the palette offers neither command. *My call, changed by the owner's decision above.* Covered projects carry a small marker on Home so the state is visible at a glance; **builder's call** on the exact look, within the existing MUI components.
- **Palette**: "Keep agents out of this project" / "Let agents into this project" for the open project, as for other per-project actions.
- **Command line**: `codewiki agents off <folder>`, `codewiki agents on <folder>`, `codewiki agents list`, and `codewiki agents status [<folder>]` (says whether a folder is covered and by which entry, or contains an entry: `kept from coding agents: contains <entry>`). `<folder>` defaults to the current folder. Exit 0 on success; `on` for a folder that is only covered by a parent says so and exits 1, and so does `on` that removes the folder's own entry while a parent still covers it (`still kept from coding agents by <entry>`).
- All of these write the same file through `cw_core::agents_off`. The app does not need the backend for this.

### What stays as it is

- The backend, `repos.json`, the indexes, existing wikis, the app's own opening and syncing, the command line's `init`/`update`, the webhook.
- The app keeps registering the folder it starts in; only agents are kept out.

## Testing

- Unit tests for matching: case, separators, trailing separators, relative paths, `..`, siblings with a common prefix (`client` vs `clients`), a subfolder, a missing folder, a junction when it can be made in a test.
- Bridge tests: with a covered current folder, a covered `CLAUDE_PROJECT_DIR`, and a covered `repo` argument given as a path and as an id, every tool returns the refusal text and the backend stub receives no request beyond the id lookup (the bridge already has a stand-in backend in its tests); in an uncovered session the cross-project tools leave covered repositories out; an unreadable list refuses everything; with no entry the tools behave as today.
- Hook test: covered folder → empty output, exit 0.
- CLI tests for `agents off/on/list/status`.
- App: page builders of Settings → Repositories and Home rows with and without entries (the pages have `Doc` tests).
- The clean-VM rig: one case with the plugin (`-Plugin`): add the guest's demo repository to the list, start Claude Code there, ask it to make the wiki; check that no `obsidian/` folder and no registration appear and that the answer mentions the refusal. **Builder's call** whether it fits the next release run.

## Calls made in the design, for the owner to check

- The bridge checks its current folder and `CLAUDE_PROJECT_DIR`; a `repo` given as an id or name is looked up through the backend.
- The hook stays silent when the project folder or its vault root is covered.
- An unreadable list fails closed, with a message that names the file and a reset in the app.
- Stored entries are the absolute, normalized folder; links resolved at check time.
- The app reads and writes the list file directly (an exception recorded in AGENTS.md).
- Home judges a project by its registered folder only.

## Left to the builder

- Module, type and function names; the exact refusal wording within the meaning above; folding of nested entries; the look of Home's marker; whether the bridge also changes its `instructions`; whether the VM case lands now.
