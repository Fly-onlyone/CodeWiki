---
tags: [core, codewiki-app, module]
---

# Live Module

> The live pages: one symbol, one file, search results, hotspots, the theme picker, a proposal, the result of a sync, the notes that wait for a coding agent, and Home, the page the app starts on.

## What it does

Like the reports in `pages`, each turns what the backend sent into a `Doc`. These show code that the vault deliberately does not hold as notes.

## Source

- `crates/codewiki-app/src/live.rs` — module `codewiki_app::live`

## How it works

1. `Suggestions::show` in [[Suggestions]] calls `hit_groups`. `hit_groups` calls `list`, then `hit_target`. `hit_target` calls `string`, then `note_stem`, then `plain_snippet`.
2. `AppView::open_symbol` in [[codewiki-app]] calls `symbol_page`. `symbol_page` calls `Doc::new` in [[Doc]], then `string`, then `text`, then `file` (both in [[Doc Module]]), then `number`, then `list`, and 4 more.
3. `AppView::run_search` in [[codewiki-app]] calls `search_page`. `search_page` calls `Doc::new` in [[Doc]], then `list`, then `hit_groups`, then `Row::new` in [[Doc Module]], then `section`.
4. `AppView::open_hotspots` in [[codewiki-app]] calls `hotspots_page`. `hotspots_page` calls `list`, then `Doc::new` in [[Doc]], then `Table::new`, then `Column::new`, then `Column::numeric`, then `file` (all in [[Doc Module]]), and 3 more.
5. `AppView::open_file` in [[codewiki-app]] calls `file_page`. `file_page` calls `string`, then `Doc::new` in [[Doc]], then `Stat::new` in [[Doc Module]], then `number`, then `code` in [[Doc Module]], then `list`, and 8 more.

10 more entry points: `proposal_page`, `sync_page`, `waiting_count`, `waiting_page`, `agent_view_line`, `explanation_lines`, and 4 more.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub fn hit_groups(hits: &Value) -> Vec<(&'static str, Vec<HitTarget>)>` — the hits of `GET /search` that the app opens, under the heading of their kind, in the order of `HIT_GROUPS` and then of the backend's ranking. A kind with no hit has no group.
- `pub struct HitTarget` — what a search hit opens, and how a list names it.
- `pub fn hit_target(hit: &Value) -> Option<HitTarget>` — what `hit`, one result of `GET /search`, opens; `None` for a kind of hit the app does not open.
- `pub fn plain_snippet(snippet: &str) -> String` — the Markdown of a note's search snippet as plain text on one line: no frontmatter, heading or quote marks, list bullets, rules, table pipes, link brackets, code ticks or `**` emphasis. A snippet is a window of the note, with its lines kept (a text match) or flattened (the opening of a note only the embedder found), so a block or link it cuts off keeps its text.
- `pub fn symbol_page(detail: &Value) -> Doc` — one symbol: what it is, where it is, and what it touches.
- `pub fn search_page(query: &str, hits: &Value) -> Doc` — search results, grouped by what was found.
- `pub fn hotspots_page(rows: &Value) -> Doc` — the hotspot ranking.
- `pub fn file_page(context: &Value, risk: &Value, radius: &Value) -> Doc` — one file: risk, symbols, relations and notes.
- `pub fn proposal_page(detail: &Value) -> Doc` — one proposal for review.
- `pub fn sync_page(report: &Value) -> Doc` — what a sync did.
- `pub fn waiting_count(queue: Option<&Value>) -> u64` — how many notes wait for a coding agent, from the queue of `GET /explanations?all=true`: those behind the code and those never explained. 0 until the queue has been read.
- `pub fn waiting_page(queue: Option<&Value>, auto_on: bool) -> Doc` — the notes that wait for a coding agent to explain them, from the queue of `GET /explanations?all=true`, in the backend's order: those behind the code first. `None` while the queue is being read. `auto_on` says whether agents explain some of them by themselves at the start of a session.
- `pub fn agent_view_line(left_out: &[String]) -> String` — the line above a note in the agent view: what the view is, and what it leaves out of the note.
- `pub fn explanation_lines(explanation: &str, by: &str, behind: &[String]) -> Vec<String>` — what the details pane says about a note's explanation, a line for each string: who wrote it, and why it may be behind. `explanation` is the backend's word for where it stands. Nothing for a note no agent explains.
- `pub fn message_page(message: &str) -> Doc` — A page that only says something, such as that there is no wiki yet.
- `pub enum Kept` — whether coding agents are kept out of a project's registered folder.
- `pub fn kept_by(entries: &[PathBuf], folder: &str) -> Kept` — how `entries` keep agents out of `folder`. A parent entry wins over the folder's own: removing the own entry would leave the folder kept. It looks at the file system (a link is followed), so the app works it out when Home's data is refreshed, never while a page is built.
- `pub struct Project<'a>` — A registered repository, as Home lists it.
- `pub fn home_page(home: &HomeInput) -> Doc` — the page the app starts on: a way to open a folder, and every registered repository, the ones opened most recently first.
- `pub fn offline_page(log: &str, logs: &str, remote: Option<&str>) -> Doc` — the page shown when no backend could be reached: where its log is, and a button that tries again. `remote` is the address of the backend saved in the settings, when that is the one that did not answer. The app does not go back to a backend on this machine by itself, which would show other wikis or none; the page offers it.

And 14 more items.

## Depends on

- [[Doc Module]] — uses `Line`, `Row`, `aside`, `code`
- [[Doc]] — uses `Doc`, `heading`, `lead`, `new`
- [[Agents Off Module]] — uses `blocking`, `covering`, `normalize`
- [[Action]] — uses `Action`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `AppView`, `agent_view_notice`, `connect`, `details`
- [[Suggestions]] — from `Suggestions`, `chosen`, `groups`, `show`
- [[Palette Module]] — from `agents_entry`

## See also

- [[_index]]
- [[codewiki-app]]
