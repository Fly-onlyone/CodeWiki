---
tags: [core, codewiki-app, module]
---

# Live Module

> The live pages: one symbol, one file, search results, hotspots, the theme picker, a proposal, the result of a sync, and Home, the page the app starts on.

## What it does

Like the reports in `pages`, each turns what the backend sent into a `Doc`. These show code that the vault deliberately does not hold as notes.

## Source

- `crates/codewiki-app/src/live.rs` — module `codewiki_app::live`

## How it works

1. `AppView::open_symbol` in [[codewiki-app]] calls `symbol_page`. `symbol_page` calls `Doc::new` in [[Doc]], then `string`, then `text`, then `file` (both in [[Doc Module]]), then `number`, then `list`, and 4 more.
2. `AppView::run_search` in [[codewiki-app]] calls `search_page`. `search_page` calls `Doc::new` in [[Doc]], then `list`, then `string`, then `Row::new` in [[Doc Module]], then `symbol_row`, then `file_row`, and 1 more. `symbol_row` calls `Row::new` in [[Doc Module]]. `file_row` calls `Row::new` in [[Doc Module]].
3. `AppView::open_hotspots` in [[codewiki-app]] calls `hotspots_page`. `hotspots_page` calls `list`, then `Doc::new` in [[Doc]], then `Table::new`, then `Column::new`, then `Column::numeric`, then `file` (all in [[Doc Module]]), and 3 more.
4. `AppView::open_file` in [[codewiki-app]] calls `file_page`. `file_page` calls `string`, then `Doc::new` in [[Doc]], then `Stat::new` in [[Doc Module]], then `number`, then `code` in [[Doc Module]], then `list`, and 8 more.
5. `AppView::open_proposal` in [[codewiki-app]] calls `proposal_page`. `proposal_page` calls `Doc::new` in [[Doc]], then `code` in [[Doc Module]], then `string`, then `text` in [[Doc Module]].

7 more entry points: `sync_page`, `agent_view_line`, `explanation_lines`, `message_page`, `model_offer`, `home_page`, and 1 more.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `fn list(value: &Value) -> Vec<&Value>` — private function, returns `Vec<&Value>`; called by `file_page`, `hotspots_page`, `note_rows`, `related` and 2 more.
- `fn string<'a>(value: &'a Value, key: &str) -> &'a str` — private function, returns `&'a str`; called by `file_page`, `hotspots_page`, `proposal_page`, `related` and 2 more.
- `fn number(value: &Value) -> String` — private function, returns `String`; called by `file_page`, `hotspots_page`, `symbol_page`, `sync_page`.
- `fn symbol_row(qualified: &str, detail: impl Into<String>) -> Row` — A row that opens a symbol, with its signature or relation under its name.
- `fn file_row(path: &str) -> Row` — A row that opens a file.
- `pub fn symbol_page(detail: &Value) -> Doc` — one symbol: what it is, where it is, and what it touches.
- `pub fn search_page(query: &str, hits: &Value) -> Doc` — search results, grouped by what was found.
- `pub fn hotspots_page(rows: &Value) -> Doc` — the hotspot ranking.
- `pub fn file_page(context: &Value, risk: &Value, radius: &Value) -> Doc` — one file: risk, symbols, relations and notes.
- `pub fn proposal_page(detail: &Value) -> Doc` — one proposal for review.
- `pub fn sync_page(report: &Value) -> Doc` — what a sync did.
- `pub fn agent_view_line(left_out: &[String]) -> String` — the line above a note in the agent view: what the view is, and what it leaves out of the note.
- `pub fn explanation_lines(explanation: &str, by: &str, behind: &[String]) -> Vec<String>` — what the details pane says about a note's explanation, a line for each string: who wrote it, and why it may be behind. `explanation` is the backend's word for where it stands. Nothing for a note no agent explains.
- `pub fn message_page(message: &str) -> Doc` — A page that only says something, such as that there is no wiki yet.
- `pub struct Project<'a>` — A registered repository, as Home lists it.
- `pub struct HomeInput<'a>` — what Home is made from.
- `pub struct ModelOffer` — the offer of a built-in embedding model on Home.
- `pub fn model_offer(models: &Value, dismissed: bool) -> Option<ModelOffer>` — the model Home offers, from what the backend says about its models (`GET /v1/models`). There is an offer only for someone who searches with the embedder that needs no model, has no model installed, and has not turned the offer down. Once a model is installed the offer is gone: the settings are then where its models are.
- `pub fn home_page(home: &HomeInput) -> Doc` — the page the app starts on: a way to open a folder, and every registered repository, the ones opened most recently first.
- `pub fn offline_page(log: &str, logs: &str, remote: Option<&str>) -> Doc` — the page shown when no backend could be reached: where its log is, and a button that tries again. `remote` is the address of the backend saved in the settings, when that is the one that did not answer. The app does not go back to a backend on this machine by itself, which would show other wikis or none; the page offers it.

And 4 more items.

## Depends on

- [[Doc Module]] — uses `Line`, `Row`, `code`, `detail`
- [[Doc]] — uses `Doc`, `heading`, `lead`, `new`

## Used by

- [[codewiki-app]] — from `agent_view_notice`, `connect`, `details`, `home_doc`

## See also

- [[_index]]
- [[codewiki-app]]
