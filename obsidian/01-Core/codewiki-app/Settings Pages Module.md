---
tags: [core, codewiki-app, module]
---

# Settings Pages Module

> The pages of the Settings: one for each `Category`, built from what the app knows and from what the backend says about its own settings (`GET /v1/settings`).

## What it does

This module builds the content of each Settings page as a [[Doc]]: Appearance, Semantic search, Backend, Repositories, Updates, Keyboard and Diagnostics. It mixes two sources: what the app itself knows (theme, zoom, autostart) and what the backend reports about its own settings. Because a page is plain data, tests read it without opening a window. The drawing and the clicks are handled in [[Settings View Module]].

## Source

- `crates/codewiki-app/src/settings_pages.rs` — module `codewiki_app::settings_pages`

## How it works

`page` picks a builder for the category. The caller fills a `Model`, a bundle of everything the pages read: the theme list, backend settings once they arrive, news beside each field, the embedding job, the kept-folder list and more. Each builder returns blocks: cards, switches, selects, text fields and secret fields.

The Appearance page holds the cards Theme, Motion, Zoom, Tabs and Startup. The Tabs card has a switch for tabs, and a second switch, "Open notes in a new tab", that is locked until tabs are on. The Repositories page lists the registered folders, then the Auto explaining card (a switch, and while it is on a select for how many notes a session explains), then the folders kept from coding agents.

Fields that belong to the backend go through `field` or `secret`. Both call `lock`, which says why a setting cannot be changed here. There are three reasons: the backend is on another machine; the backend takes its settings from its environment only; or an environment variable sets that value. A locked control shows the reason. `secret` adds a fourth: a backend that cannot save keys safely says to set the variable instead. A secret shows as typing, saved with a hint, unreadable, or empty.

The search page picks a provider, then `server_rows` shows its address, key, model and how to start the server. `job` reports the running or last embedding job. While settings are not yet read, `without_settings` shows a loading line or the read error.

## Items

- `pub enum Tested` — where a test of a provider stands.
- `pub struct KeptFolder` — A folder on the list of those kept from coding agents. Whether it still exists is looked up when the list is read, never while a page is drawn: a folder on a share that is offline can take long to answer.
- `pub struct Model<'a>` — everything the pages read.
- `pub fn page(category: Category, model: &Model) -> Doc` — the page of `category`.
- `fn card(title: &str, body: Vec<Block>) -> Block` — private function, returns `Block`; called by `agents_card`, `appearance`, `auto_explain_card`, `backend` and 7 more.
- `fn appearance(model: &Model) -> Vec<Block>` — private function, returns `Vec<Block>`; called by `page`.
- `fn tabs_card(model: &Model) -> Block` — whether the app has tabs, and where a note the user opens goes.
- `fn zoom_card(model: &Model) -> Block` — how large the whole app is drawn.
- `fn without_settings(model: &Model) -> Vec<Block>` — what a page that needs the backend's settings shows without them.
- `fn lock(model: &Model, settings: &Value, entry: &Value) -> Option<String>` — why the setting `entry` describes cannot be changed here, when it cannot.
- `provider` — A select over the `options` the backend lists for `section.provider`.
- `fn in_use(settings: &Value, section: &str, nothing: &str) -> Block` — what the settings of `section` select, or why they select nothing usable.
- `fn search(model: &Model) -> Vec<Block>` — private function, returns `Vec<Block>`; called by `page`.
- `fn server_rows(model: &Model, settings: &Value, provider: &str) -> Vec<Block>` — the address and key rows of a server, then its model, then how to start it. Nothing for the word-form embedder and for none.
- `pub(crate) fn server_label(provider: &str) -> &str` — how the page names the server of `provider`, as its address row does.
- `pub fn job_runs(job: &Value) -> bool` — whether an embedding job is under way.
- `fn job(job: &Value) -> Vec<Block>` — what the embedding job that runs, or the last that ran, has to say.
- `fn backend(model: &Model) -> Vec<Block>` — private function, returns `Vec<Block>`; called by `page`.
- `fn repositories(model: &Model) -> Vec<Block>` — private function, returns `Vec<Block>`; called by `page`.
- `fn updates(model: &Model) -> Vec<Block>` — the version, what the last check found, and whether the app checks by itself.

And 19 more items.

## Depends on

- [[Doc Module]] — uses `Choice`, `chip`, `code`, `new`
- [[Zoom Module]] — uses `DEFAULT`, `LEVELS`, `label`
- [[Update Module]] — uses `State`, `describe`
- [[Auto Explain Module]] — uses `DEFAULT`, `PRESETS`
- [[Doc]] — uses `Doc`, `new`
- [[MotionPref]] — uses `ALL`, `MotionPref`
- [[cw-client]] — uses `Origin`
- [[cw-telemetry]] — uses `DSN_ENV`

## Used by

- [[Settings View Module]] — from `State`, `job_ended`, `read_agents_off`, `test_line`
- [[codewiki-app]] — from `probe_embed_models`, `settings_frame`, `take_embed_job`

## Gotchas

- A page that needs backend settings shows only a loading line or an error until they arrive.
- Start with Windows is locked for a copy not installed, since a build run from `target` would leave Windows a program the next build replaces.
- "Open notes in a new tab" shows as off and locked while tabs are off, whatever its saved value is.
- A notes-per-session count in the file that is not one of the presets is still offered, placed by size, so the select never loses the current value.
- The kept-folder list never checks whether a folder exists while drawing, because a folder on an offline share can answer slowly. The check happens when the list is read.
- Secrets are never shown again once saved; only a hint is.

## See also

- [[_index]]
- [[codewiki-app]]
