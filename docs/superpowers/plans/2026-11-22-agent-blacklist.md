# Keep coding agents out of chosen folders — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A local list of folders in which Claude Code and Codex get no CodeWiki: every MCP tool refuses, the session-start hook is silent, and listed projects are hidden from agents' cross-project tools; managed from Settings, Home, the palette and `codewiki agents`.

**Architecture:** A new `cw_core::agents_off` module owns `agents-off.json` in the data folder (load, add, remove, reset, path matching). The MCP bridge (`codewiki-mcp`) checks it in one place before any tool runs and filters the cross-project answers; the `codewiki` binary checks it in the session-start hook and offers `codewiki agents …`; the desktop app edits it directly through `cw-core`. The backend does not change.

**Tech Stack:** Rust 2024 workspace; rmcp 3.9 (MCP server), clap (CLI), GPUI with the app's own `Doc`/`Block` page model; serde_json.

**Spec:** `docs/superpowers/specs/2026-11-22-agent-blacklist-design.md`

## Global Constraints

- The list file is `agents-off.json` in `cw_core::paths::data_dir()`; shape `{"version": 1, "folders": [<absolute folder>, …]}`. A missing file is an empty list.
- An unreadable or unparsable list **fails closed**: agents get no CodeWiki anywhere until it is fixed.
- An entry covers its folder and everything below it, compared component by component, case-insensitively on Windows, after making both sides absolute and normalized; links resolved when the path exists. `D:\work\client` does not cover `D:\work\clients`.
- Refusal text, exact (used by the bridge, the skills and the plugin test): `CodeWiki is turned off for this project ({entry} is kept from coding agents in CodeWiki's settings). Do not call CodeWiki tools here; continue without them.`
- Unreadable-list text, exact: `CodeWiki's list of folders kept from coding agents ({file}) cannot be read; fix or delete it. Until then CodeWiki's tools are off for coding agents.`
- A refusal is a normal tool result (`CallToolResult::success` with text), never a protocol error, and makes no HTTP request and starts no backend — except one `GET /v1/repos` when a `repo` argument is an id or a name, not a path.
- The desktop app, the command line's other commands and the backend keep working on listed folders exactly as before. Nothing existing (registrations, indexes, `obsidian/`) is removed.
- After every `.rs` edit: RustRover `get_file_problems`, then `cargo fmt --all` (not `reformat_file`). The four checks of AGENTS.md pass before each commit. Commits: Conventional Commits, `type(scope): imperative subject`, through the `qingyi:commit` skill.

## Review Focus

- **The same folder written another way** (`d:/Work/Clients/`, `D:\work\clients\.\api`, a relative `..\clients`): covered, like `D:\work\clients`. Test in Task 1.
- **A sibling with a common prefix** (`D:\work\client` vs `D:\work\clients`, `D:\a` vs `D:\ab`): not covered. Test in Task 1.
- **A drive root entry** (`D:\`): covers every folder on D:, and only D:. Test in Task 1.
- **A broken list** (empty file, `[]`, `{"version":2}`, truncated JSON): the bridge refuses everything with the unreadable-list text; the hook prints nothing; `codewiki agents list` reports it and exits 1; the app shows it with a reset. Tests in Tasks 1, 2, 3, 4.
- **An agent that names the project by id or folder name** (`repo: "cw-core-9bb4c92c"`, `repo: "clients"`) while its session folder is elsewhere: refused when the backend's path for it is covered. Test in Task 4.

---

### Task 1: `cw_core::agents_off`

**Files:**
- Create: `crates/cw-core/src/agents_off.rs`
- Modify: `crates/cw-core/src/lib.rs` (add `pub mod agents_off;`)

**Interfaces:**
- Produces:
  - `pub const FILE: &str = "agents-off.json";`
  - `pub fn file(data_dir: &Path) -> PathBuf`
  - `pub fn load(data_dir: &Path) -> Result<Vec<PathBuf>>` — `Ok(vec![])` when the file is missing; `Err(Error::Invalid(..))` when it cannot be read or parsed, or `version != 1`.
  - `pub fn normalize(path: &Path, base: &Path) -> PathBuf` — absolute (joined to `base` when relative), `.`/`..` resolved lexically, separators unified to the platform's, no trailing separator (except a drive root).
  - `pub fn covering(entries: &[PathBuf], path: &Path) -> Option<PathBuf>` — the entry equal to or an ancestor of `path` (both compared in canonical form when they exist, `\\?\` stripped as `paths::repo_id` does; else normalized; case-insensitive on Windows). `path` must already be absolute.
  - `pub enum Added { Added(PathBuf), CoveredBy(PathBuf) }`, `pub fn add(data_dir: &Path, folder: &Path) -> Result<Added>`
  - `pub enum Removed { Removed(PathBuf), CoveredBy(PathBuf), NotListed }`, `pub fn remove(data_dir: &Path, folder: &Path) -> Result<Removed>` — `CoveredBy` when only a parent entry covers it.
  - `pub fn reset(data_dir: &Path) -> Result<()>` — writes an empty list over whatever is there.
  - `add`/`remove` normalize `folder` against `std::env::current_dir()`, store the normalized form, write atomically (`<file>.tmp` then rename, as `settings::update` does at `settings.rs:319-322`). They return `Err` on an unreadable list rather than overwrite it (only `reset` overwrites).

- [ ] **Step 1: Write the failing tests** in `agents_off.rs` (`#[cfg(test)] mod tests`, with `tempfile` dirs as `settings.rs` tests do):
  - `missing_file_is_an_empty_list`: `load(empty_dir) == Ok(vec![])`.
  - `an_entry_covers_itself_and_below`: entries `[D:\work\clients]` cover `D:\work\clients`, `D:\work\clients\api\src`.
  - `case_separators_and_trailing_slash_do_not_matter` (Windows only): `d:/Work/Clients/` and `D:\work\clients\.\api` are covered.
  - `a_sibling_with_a_common_prefix_is_not_covered`: `D:\work\client` and `D:\work\clientsX` are not covered by `D:\work\clients`.
  - `a_drive_root_covers_the_drive_only` (Windows only): entry `D:\` covers `D:\x`, not `C:\x`.
  - `relative_paths_are_made_absolute`: `normalize(Path::new("..\\b"), Path::new("C:\\a\\c")) == C:\a\b`.
  - `add_then_load_then_remove` round trip; `add` of a covered folder gives `Added::CoveredBy(parent)` and writes nothing; `remove` of a child of an entry gives `Removed::CoveredBy(parent)`; `remove` of an unknown folder gives `NotListed`.
  - `a_broken_file_is_an_error_and_is_not_overwritten`: for each of `""`, `"[]"`, `"{\"version\":2,\"folders\":[]}"`, `"{\"version\":1,"`: `load` is `Err`, `add` is `Err`, the file's bytes are unchanged; `reset` makes `load` `Ok(vec![])`.
  - `a_linked_folder_matches_its_target` (Windows, skip when `mklink /J` fails): a junction to a listed folder is covered.

- [ ] **Step 2: Run them to verify they fail**
  Run: `cargo test -p cw-core agents_off`
  Expected: compile errors (module missing).

- [ ] **Step 3: Implement the module** with the signatures above. Components compared as lowercase strings on Windows (`cfg(windows)`), exact elsewhere.

- [ ] **Step 4: Run the tests to verify they pass**
  Run: `cargo test -p cw-core agents_off` → all pass; then the four checks.

- [ ] **Step 5: Commit** — `feat(core): keep a list of folders closed to coding agents`

---

### Task 2: The session-start hook stays silent

**Files:**
- Modify: `crates/codewiki/src/main.rs:164-181` (`session_start_hook`)
- Test: `crates/codewiki/tests/hook.rs`

**Interfaces:**
- Consumes: `cw_core::agents_off::{load, covering}`, `cw_core::paths::data_dir()`.

- [ ] **Step 1: Write the failing tests** in `hook.rs`, with the existing `codewiki(dir, args)` helper plus `.env("CODEWIKI_DATA_DIR", data)`:
  - `a_listed_project_gets_no_briefing`: a project with a wiki (as the existing hook tests make one) whose folder is listed in `<data>/agents-off.json` → stdout empty, exit 0.
  - `a_listed_parent_silences_a_subfolder`: list the parent, run from `<project>/src` → stdout empty.
  - `a_listed_vault_root_silences_the_hook`: list the project root, start in a subfolder without a wiki of its own → empty (root found by `vault_root`).
  - `a_broken_list_silences_the_hook`: `agents-off.json` = `"{"` → empty, exit 0.
  - `an_unlisted_project_still_gets_its_briefing`: a list naming another folder → briefing printed as today.

- [ ] **Step 2: Run** `cargo test -p codewiki --test hook` → the new tests fail (briefing printed).

- [ ] **Step 3: In `session_start_hook`**, after `start` is canonicalized and `root` found: `match agents_off::load(&data_dir())` — `Err(_)` → return; `Ok(entries)` → return when `covering(&entries, &start)` or `covering(&entries, &root)` is `Some`.

- [ ] **Step 4: Run** `cargo test -p codewiki --test hook` → pass; the four checks.

- [ ] **Step 5: Commit** — `feat(cli): say nothing at session start in a folder closed to agents`

---

### Task 3: `codewiki agents off|on|list|status`

**Files:**
- Modify: `crates/codewiki/src/main.rs` (`enum Command` at `:31`, dispatch before `run` like `Serve` at `:225`)
- Test: `crates/codewiki/tests/agents.rs` (new; copy the `codewiki` helper from `hook.rs:7`)

**Interfaces:**
- Consumes: Task 1's `add`, `remove`, `load`, `covering`, `normalize`.
- Produces (CLI): `Command::Agents { action: AgentsAction }`; `enum AgentsAction { Off { folder: Option<PathBuf> }, On { folder: Option<PathBuf> }, List, Status { folder: Option<PathBuf> } }`. `folder` defaults to the current folder. Printed lines (exact):
  - off: `kept from coding agents: <folder>` / `already kept from coding agents by <entry>`
  - on: `coding agents allowed again: <folder>` / `not listed: <folder>` / `kept from coding agents by <entry>; remove that entry to let agents in` (exit 1)
  - list: one folder per line, or `no folder is kept from coding agents`
  - status: `kept from coding agents by <entry>` or `open to coding agents`
  - any command on a broken list: the unreadable-list text of Global Constraints on stderr, exit 1 (`off`/`on` never overwrite it).

- [ ] **Step 1: Write the failing tests** `off_lists_a_folder_and_status_sees_it`, `off_twice_names_the_entry`, `on_removes_it`, `on_a_child_of_a_parent_entry_exits_1`, `list_is_empty_at_first`, `a_broken_list_is_reported_and_left_alone` — each runs the binary with `CODEWIKI_DATA_DIR` set and asserts the exact lines above and the exit code.
- [ ] **Step 2: Run** `cargo test -p codewiki --test agents` → fail (unknown subcommand).
- [ ] **Step 3: Add the subcommand** and its handler; `--json` (global flag) prints `{"folders": [...]}` for `list` and `{"covered_by": <entry|null>}` for `status`.
- [ ] **Step 4: Run** the tests → pass; the four checks.
- [ ] **Step 5: Commit** — `feat(cli): add codewiki agents to keep coding agents out of folders`

---

### Task 4: The bridge's guard — decisions as plain functions

**Files:**
- Create: `crates/codewiki-mcp/src/guard.rs` (module declared in `main.rs`)

**Interfaces:**
- Consumes: Task 1.
- Produces:
  - `pub enum Verdict { Open, Refused(String) }`
  - `pub fn refusal(entry: &Path) -> String` and `pub fn unreadable(file: &Path) -> String` — the exact texts of Global Constraints.
  - `pub fn session_folders(cwd: &Path, project_dir: Option<&Path>) -> Vec<PathBuf>` — `cwd`, plus `CLAUDE_PROJECT_DIR`'s value when given.
  - `pub enum RepoArg { Path(PathBuf), IdOrName(String) }`, `pub fn repo_arg(value: &str, cwd: &Path) -> RepoArg` — a path when it is absolute, starts with `.`, or contains a separator; else an id or name.
  - `pub fn resolve_id(listing: &serde_json::Value, id_or_name: &str) -> Option<PathBuf>` — the `path` of the `/v1/repos` entry whose `id` equals it or whose `name` equals it case-insensitively (the same rule as `cw-client`'s `repo_id` fallback, `lib.rs:471-500`).
  - `pub fn verdict(list: &cw_core::Result<Vec<PathBuf>>, list_file: &Path, folders: &[PathBuf]) -> Verdict` — `Refused(unreadable(..))` on `Err`; `Refused(refusal(entry))` when any folder is covered; else `Open`.
  - `pub fn hide_repos(listing: &mut Value, entries: &[PathBuf]) -> HashSet<String>` — removes covered entries from a `/v1/repos` array in place, returns their names.
  - `pub fn hide_in_workspace(workspace: &mut Value, hidden: &HashSet<String>)` — drops `repos[]` rows whose `name` is hidden, and hidden names from each `shared_dependencies[].repos` (dropping a dependency left with fewer than two repos).
  - `pub fn hide_in_contracts(contracts: &mut Value, hidden: &HashSet<String>)` — drops rows whose `repo` is hidden (read the shape at `cw-server/src/workspace.rs:167` and the formatter `workspace_contracts_text` at `codewiki-mcp/src/main.rs:331`, including matches that name a hidden repo on either side).

- [ ] **Step 1: Write the failing tests** in `guard.rs` with `json!` fixtures: `a_covered_session_folder_is_refused`, `a_covered_claude_project_dir_is_refused`, `an_uncovered_session_is_open`, `a_broken_list_refuses_with_the_file_name` (asserts the exact unreadable text), `the_refusal_text_is_exact`, `repo_args_are_classified` (`"D:\\x"`, `".."`, `"src/x"` → Path; `"cw-core-9bb4c92c"`, `"clients"` → IdOrName), `an_id_or_name_resolves_to_its_path`, `hidden_repos_leave_the_listing`, `hidden_repos_leave_the_workspace_and_shared_dependencies`, `hidden_repos_leave_contract_matches`.
- [ ] **Step 2: Run** `cargo test -p codewiki-mcp guard` → fail.
- [ ] **Step 3: Implement** the functions above.
- [ ] **Step 4: Run** → pass; the four checks.
- [ ] **Step 5: Commit** — `feat(mcp): decide which tool calls coding agents may make`

---

### Task 5: The bridge applies the guard

**Files:**
- Modify: `crates/codewiki-mcp/src/main.rs` — the `ServerHandler` impl (`:1554-1558`), `list_repos` (`:404`), `get_workspace` (`:411`), `get_contracts` (`:467`, the `workspace: true` branch)

**Interfaces:**
- Consumes: Task 4.
- Produces: `impl Bridge { fn screen(&self, arguments: Option<&serde_json::Map<String, Value>>) -> Option<String> }` — returns the refusal text, or `None`. Reads the list (`agents_off::load(&data_dir())`) on every call; session folders from `std::env::current_dir()` and `CLAUDE_PROJECT_DIR`; a `repo` argument per `repo_arg` (an `IdOrName` resolved with one `GET /v1/repos` through `connected(..)`; if that fails, only the session folders decide).

- [ ] **Step 1: Check rmcp 3.9's generated `call_tool`.** Read what `#[tool_handler]` expands to in rmcp 3.9 (crate source under the cargo registry, or docs.rs) to find how to wrap the router: either implement `ServerHandler::call_tool` by hand (`self.tool_router.call(ToolCallContext::new(self, request, context))` or what 3.9 names it) with `#[tool_handler]` kept for `list_tools`, or a router layer if 3.9 has one. Note the exact API in the commit message.
- [ ] **Step 2: Write the failing test** `screen_refuses_before_any_backend_starts`: with `CODEWIKI_DATA_DIR` set to a temp folder whose list covers the test's current folder (set the process's current folder in the test, serialized with a mutex as other env-changing tests in the workspace do), `Bridge::default().screen(None)` returns the refusal text and `bridge.backend.lock().unwrap().is_none()` afterwards.
- [ ] **Step 3: Run** `cargo test -p codewiki-mcp screen` → fail.
- [ ] **Step 4: Implement `screen`**, call it first in `call_tool`; on `Some(text)` return `Ok(CallToolResult::success(vec![ContentBlock::text(text)]))` without calling the router. In `list_repos`, `get_workspace` and `get_contracts { workspace: true }`, after the backend answers, load the list (an unreadable list never gets here: `screen` refused already), get `/v1/repos`, call `hide_repos`, then `hide_in_workspace` / `hide_in_contracts` before formatting.
- [ ] **Step 5: Run** `cargo test -p codewiki-mcp` → pass; the four checks.
- [ ] **Step 6: Commit** — `feat(mcp): refuse every tool in a folder closed to coding agents`

---

### Task 6: Settings → Repositories card

**Files:**
- Modify: `crates/codewiki-app/src/doc.rs` (`Action` at `:15`), `crates/codewiki-app/src/settings_pages.rs` (`Model` at `:40`, `repositories` at `:840`, tests near `:1814`), `crates/codewiki-app/src/settings_view.rs` (`run_settings_action` at `:647`, model filling at `:895`), `crates/codewiki-app/src/main.rs` (a folder picker that does not need the backend)

**Interfaces:**
- Consumes: Task 1.
- Produces:
  - `Action::KeepAgentsOut(String)`, `Action::LetAgentsIn(String)`, `Action::PickAgentsFolder`, `Action::ResetAgentsList` (folders as strings, like `ForgetRepo(String)`).
  - `Model.agents_off: &'a Result<Vec<PathBuf>, String>` (the error is the unreadable-list text).
  - A card **"Kept from coding agents"** after the repositories table: the line `Claude Code and Codex get no CodeWiki tools in these folders and everything below them. The app and the command line still work there.`; one `Block::Setting { label: <folder>, value: "" or "Folder not found", button: Some(("Remove", Action::LetAgentsIn(<folder>))) }` per entry; `Block::Buttons(vec![Press::new("Add folder…", Action::PickAgentsFolder)])`; on `Err`, a `Block::Alert { tone: Error, text: <unreadable text>, button: Some(("Start a new list", Action::ResetAgentsList)) }` instead of the rows.

- [ ] **Step 1: Write the failing page tests**: `agents_card_lists_entries_with_remove`, `agents_card_offers_add_folder_when_empty`, `agents_card_shows_a_broken_list_with_reset`, `agents_card_marks_a_missing_folder` — via `Fixture::new()` and `fixture.page(Category::Repositories)`, asserting words and `doc.actions()`.
- [ ] **Step 2: Run** `cargo test -p codewiki-app settings_pages` → fail.
- [ ] **Step 3: Build the card**; fill `agents_off` from `agents_off::load(&data_dir())` when the Settings view refreshes; handle the four actions in `run_settings_action` (`PickAgentsFolder` uses `cx.prompt_for_paths(PathPromptOptions { files: false, directories: true, multiple: false, prompt: Some("Keep agents out".into()) })` like `open_folder` at `main.rs:1173`, but without its backend check), then refresh the page.
- [ ] **Step 4: Run** the tests → pass; the four checks. Check the page by eye as AGENTS.md "Checking the app without taking focus" says (`settings:repositories`, with two entries and with a broken list).
- [ ] **Step 5: Commit** — `feat(app): manage the folders kept from coding agents in Settings`

---

### Task 7: Home and the palette

**Files:**
- Modify: `crates/codewiki-app/src/doc.rs` (`Row` at `:321`), the renderer of `Block::Rows` (find where `Row`s are drawn), `crates/codewiki-app/src/live.rs` (`home_page` at `:466`, `Project` at `:440`, `HomeInput` at `:448`, test helper `home` at `:632`), `crates/codewiki-app/src/palette.rs` (`Command` at `:8`), `crates/codewiki-app/src/main.rs` (`palette_matches` at `:1673`, `run_command` at `:1748`, `home_page` call at `:3468`), `crates/codewiki-app/src/icons.rs` (`for_command` at `:137`)

**Interfaces:**
- Consumes: Task 1, Task 6's actions.
- Produces:
  - `Row::aside(self, label: impl Into<String>, action: Action) -> Self` — a small text button at the row's end, drawn with the existing MUI text-button component; a click on it runs its action, not the row's.
  - `HomeInput.agents_off: Vec<PathBuf>` (an unreadable list passes empty here; Settings shows the error).
  - Recent-projects rows: `aside("Keep agents out", KeepAgentsOut(path))` when nothing covers the project's folder; `aside("Let agents in", LetAgentsIn(path))` when an entry equal to it covers it; when a parent entry covers it, no aside and the detail line gains `· kept from agents by <entry>` (a covered project shows this marker in every case).
  - `Command::KeepAgentsOut` / `Command::LetAgentsIn` in the palette, offered for the open project like the Autostart toggle (`main.rs:1683-1692`): labels `Keep agents out of this project` / `Let agents into this project`; neither when a parent entry covers it.

- [ ] **Step 1: Write the failing tests** in `live.rs`: `home_offers_keep_agents_out`, `home_offers_let_agents_in_for_an_exact_entry`, `home_names_the_parent_entry_and_offers_no_button`; a `doc.rs` test that a row's aside action is listed in `doc.actions()`.
- [ ] **Step 2: Run** `cargo test -p codewiki-app live` → fail.
- [ ] **Step 3: Implement** `Row::aside` and its drawing, the Home rows, the palette entries (`icons::for_command` arms), and the dispatch to Task 6's handlers.
- [ ] **Step 4: Run** the tests → pass; the four checks. Check Home by eye (`home:`) with one exact and one parent entry, at 100% and 150% zoom.
- [ ] **Step 5: Commit** — `feat(app): keep agents out of a project from Home and the palette`

---

### Task 8: Skills, plugin test and docs

**Files:**
- Modify: `plugins/claude-code/skills/write-notes/SKILL.md`, `plugins/codex/skills/using-codewiki/SKILL.md`, `plugins/codex/skills/wiki-note-style/SKILL.md`, `crates/codewiki/tests/plugins.rs`, `README.md`, `AGENTS.md` (line 8 exception; Architecture/MCP notes), `ROADMAP.md`

- [ ] **Step 1: Write the failing test** in `plugins.rs`: `skills_stop_at_the_agents_refusal` — each of the three skills contains `CodeWiki is turned off for this project` and the sentence telling the agent to stop using CodeWiki tools in the session, and `crates/codewiki-mcp/src/guard.rs` contains the same phrase (the pattern of `write_notes_skill_follows_the_tools_and_the_limits` at `:261`).
- [ ] **Step 2: Run** `cargo test -p codewiki --test plugins` → fail.
- [ ] **Step 3: Add the line** to the three skills: `If a CodeWiki tool answers that CodeWiki is turned off for this project, stop using CodeWiki tools for the rest of this session.`
- [ ] **Step 4: Docs.** README: a section "Keep coding agents out of a folder" (the three ways to manage the list, what agents see, that the app and the command line still work there, that a remote backend's repositories cannot be matched, and Claude Code's per-project plugin switch as the alternative). AGENTS.md line 8: the app reads and writes `agents-off.json` itself, a local agents' setting. ROADMAP: a row for the feature with what is verified.
- [ ] **Step 5: Run** the four checks → pass.
- [ ] **Step 6: Commit** — `docs: describe keeping coding agents out of folders` (tests and skills in a separate `test(plugins)`/`feat(plugins)` commit if the commit skill splits them).

---

### Task 9 (builder's call): A clean-VM case

**Files:** `scripts/vm/test-release.ps1` (a case after CC5, needs `-Plugin`)

- [ ] Add `CC9`: `codewiki agents off <guest demo repo>`; start Claude Code there with `--permission-mode default` and the CodeWiki tools allowed (memory note: nested `claude -p` starts in plan mode); ask it to make the wiki; pass when no `obsidian/` folder and no new `repos.json` entry appear and Claude's answer mentions the refusal; then `codewiki agents on` and remove the folder. Parse-check under Windows PowerShell 5.1 (AGENTS.md lessons). Run with the next release run, not now.
