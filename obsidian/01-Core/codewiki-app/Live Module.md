---
tags: [core, codewiki-app, module]
---

# Live Module

> The live pages: one symbol, one file, search results, hotspots, the theme picker, a proposal, the result of a sync, the notes that wait for a coding agent, and Home, the page the app starts on.

## What it does

Most pages of the app are notes that the vault holds as Markdown files. Some things are not notes: one symbol, one file, search results, hotspots, a proposal, the result of a sync, the notes that wait for a coding agent, Home, and the offline page. The backend sends these as JSON. This module turns each JSON answer into a `Doc`, the page model that [[Doc Module]] and the doc view draw. Each page is a plain function from data to `Doc`, so it can be tested without a window.

## Source

- `crates/codewiki-app/src/live.rs` — module `codewiki_app::live`

## How it works

The app calls a page function after it has the backend's answer. `AppView::open_symbol` calls `symbol_page`. `run_search` calls `search_page`. `open_hotspots` calls `hotspots_page`. `open_file` calls `file_page`, which takes three answers: context, risk and radius. Home is built by `home_page` from a `HomeInput`, on every frame.

The functions read JSON with small helpers. `string` gives an empty text for a missing field, `list` gives an empty list for a missing array, and `number` shows a dash for null. Rows are made by `symbol_row`, `file_row` and `note_rows`, each with an `Action` that says what a click does.

Search hits go through `hit_groups`, which sorts them under Notes, Symbols and Files, in that order, then by the backend's ranking. `hit_target` decides what a hit opens, and `plain_snippet` strips Markdown from a note's snippet so it reads as one plain line. The search suggestions in [[Suggest Module]] share this code with the search page.

`kept_by` checks the file system to see if coding agents are kept out of a folder.

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
- `pub fn review_empty_page() -> Doc` — the review page when no proposal waits.
- `pub fn sync_page(report: &Value) -> Doc` — what a sync did.
- `pub fn waiting_count(queue: Option<&Value>) -> u64` — how many notes wait for a coding agent, from the queue of `GET /explanations?all=true`: those behind the code and those never explained. 0 until the queue has been read.
- `pub fn waiting_page(queue: Option<&Value>, auto_on: bool) -> Doc` — the notes that wait for a coding agent to explain them, from the queue of `GET /explanations?all=true`, in the backend's order: those behind the code first. `None` while the queue is being read. `auto_on` says whether agents explain some of them by themselves at the start of a session.
- `pub fn agent_view_line(left_out: &[String]) -> String` — the line above a note in the agent view: what the view is, and what it leaves out of the note.
- `pub fn explanation_lines(explanation: &str, by: &str, behind: &[String]) -> Vec<String>` — what the details pane says about a note's explanation, a line for each string: who wrote it, and why it may be behind. `explanation` is the backend's word for where it stands. Nothing for a note no agent explains.
- `pub fn message_page(message: &str) -> Doc` — A page that only says something, such as that there is no wiki yet.
- `pub enum Kept` — whether coding agents are kept out of a project's registered folder.
- `pub fn kept_by(entries: &[PathBuf], folder: &str) -> Kept` — how `entries` keep agents out of `folder`. A parent entry wins over the folder's own: removing the own entry would leave the folder kept. It looks at the file system (a link is followed), so the app works it out when Home's data is refreshed, never while a page is built.
- `pub fn home_page(home: &HomeInput) -> Doc` — the page the app starts on: a way to open a folder, and every registered repository, the ones opened most recently first.
- `pub fn offline_page(log: &str, logs: &str, remote: Option<&str>) -> Doc` — the page shown when no backend could be reached: that the app keeps looking for one, where its log is, and a button that tries again at once. `remote` is the address of the backend saved in the settings, when that is the one that did not answer. The app does not go back to a backend on this machine by itself, which would show other wikis or none; the page offers it.

And 15 more items.

## Depends on

- [[Doc Module]] — uses `Line`, `Row`, `aside`, `code`
- [[Doc]] — uses `Doc`, `heading`, `lead`, `new`
- [[Agents Off Module]] — uses `blocking`, `covering`, `normalize`
- [[Action]] — uses `Action`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `AppView`, `agent_view_notice`, `connect`, `details`
- [[Suggest Module]] — from `Suggestions`, `chosen`, `groups`, `show`
- [[Palette Module]] — from `agents_entry`
- [[Review View Module]] — from `empty_review`

## Gotchas

- Missing JSON fields never raise an error. They show as empty text or a dash. An old backend gives a thin page, not a crash.
- A search hit of a kind the app does not open gives `None` from `hit_target` and is skipped.
- A note row opens the note by its whole vault path. A bare folder path without `.md` is not found by the backend.
- `kept_by` reads the file system and follows links. Call it when Home's data is refreshed, never while a page is built, since Home is built every frame.
- `waiting_count` is 0 until the queue has been read, and `waiting_page` takes `None` while it is being read.
- The offline page does not return to a backend on this machine by itself. It offers a button, because that backend could show other wikis or none.

## See also

- [[_index]]
- [[codewiki-app]]
