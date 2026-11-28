---
tags: [core, codewiki-app, module]
---

# Settings View Module

> The Settings page as the app's view draws and runs it.

## What it does

This is the live Settings page: categories on the left, the chosen category's page on the right. The page content is a `Doc` built by [[Settings Pages Module]]. This module draws it and runs what the user does. It is drawn here, not in the cached page view, because text inputs and the open list of a select need their events to come through this view. A change takes effect at once. A switch, a select or a choice acts on its click. A typed field saves on Enter or when focus leaves it, and shows Saved or the reason it failed. The one exception is a backend on another machine: its address and token are checked together before use.

## Source

- `crates/codewiki-app/src/settings_view.rs` — module `codewiki_app::settings_view`

## How it works

`State` holds what the page remembers between frames: one text input per `FieldId` (created once, living as long as the app), what the backend last said each field holds, news beside each field, the open select menu, the embedding job and models, the kept-folders list, and the auto explain switch with its count.

`open_settings` shows the page, resets connect news, re-reads the kept-folder list and the auto explain switch and count (they are files another program may change), fills the remote address field, and loads the backend settings. On the Search category it also loads the embedding job and models. Moving between categories (`show_settings_category`) does not add history, so Back leaves Settings in one step. Opening Repositories re-reads the kept folders and the auto explain files again.

A saved change is a `PUT /v1/settings` with a small body from `change` (section, field, value). `changes_the_embedder` tells whether the change touches the embedder, so the old job reading may be out of date. While an embedding job runs, the app asks again every second (`JOB_PAUSE`). `job_ended` notices when it stops, and the settings are read again. `test_line` turns a provider test answer into one line. `shown_value` shows a saved or environment value in a field; a default is left to the placeholder.

A backend on another machine is not asked for the embedding job or models; those fields stay empty.

The list of folders kept from coding agents is read by the app from the data folder, not through the backend (`read_agents_off`).

## Items

- `pub const LOG_LEVEL_PREF: &str` — the name the app's log level is saved under (`prefs.rs`).
- `const JOB_PAUSE: std::time::Duration` — how long the app waits before it asks again how far the embedding job is.
- `struct OpenMenu` — the list of a select that is open.
- `pub struct State` — what the Settings page remembers between frames.
- `fn shown_value(entry: &Value) -> String` — what a field shows for the setting `entry` describes: the value that is saved or that the environment gives. A default is not typed in; the input's placeholder stands for it.
- `fn test_line(result: Result<Value, String>) -> Tested` — the line a test of a provider left.
- `fn read_agents_off() -> Result<Vec<KeptFolder>, String>` — the list of folders kept from coding agents, or the text that says it cannot be read. The app reads it itself: the backend is not involved.
- `fn lists_themes(options: &[Choice]) -> bool` — whether the options of a select are the themes.
- `fn change(section: &str, field: &str, value: Value) -> Value` — the body of a `PUT /v1/settings` that changes one field.
- `fn changes_the_embedder(body: &Value) -> bool` — whether a change touches the embedder, after which the job the app last read may be out of date.
- `fn job_in(body: &Value) -> Option<Value>` — the job in the body of an answer from the job routes; `None` for `null`.
- `fn job_ended(before: Option<&Value>, after: Option<&Value>) -> bool` — whether a job that ran has ended: the settings are read again then.

## Depends on

- [[Connection Module]] — uses `Backoff`, `EMBED_CAP`, `EMBED_FIRST`, `Sequence`
- [[FieldId]] — uses `ALL`, `FieldId`, `placeholder`, `secret`
- [[Settings Pages Module]] — uses `KeptFolder`, `Tested`, `job_runs`
- [[Agents Off Module]] — uses `file`, `load`, `unreadable`
- [[codewiki-app]] — uses `AppView`, `settings_field_event`
- [[Auto Explain Module]] — uses `load`, `load_count`
- [[Doc Module]] — uses `Choice`
- [[Prefs Module]] — uses `remove`

## Used by

- [[codewiki-app]] — from `AppView`, `announce_page`, `commit_settings_field`, `load_embed_job`

## Gotchas

- Unsaved edits can keep the old page open: `open_settings` checks that the page really changed before it sets state.
- Questions to the embedding server are numbered (`Sequence`). Leaving the page, a new embedder, another backend and a hidden window each take a number, so a late answer or retry from before is dropped.
- The folder existence check runs once per read, never during drawing, because a folder on an offline share can answer slowly.
- The auto explain switch and count are read from files when the page opens, so a change by another program shows up only after the page is opened or Repositories is shown again.
- Secret inputs are masked, and a saved secret is never shown again.
- `State::new` removes an old preference (model hint) left from a removed offer.

## See also

- [[_index]]
- [[codewiki-app]]
