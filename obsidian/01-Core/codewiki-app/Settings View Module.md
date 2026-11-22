---
tags: [core, codewiki-app, module]
---

# Settings View Module

> The Settings page as the app's view draws and runs it.

## What it does

The page fills the window, as Home does: the categories at the left, the page of the chosen one at the right. A category's page is a `Doc` (`settings_pages.rs`), drawn here with `doc_view::column`, in this view and not in the cached page view: its text inputs and the list of a select take their events through this view, as the search box and the theme menu do.

A change takes effect when it is made. A switch, a select or a choice acts on its click; a field that is typed is saved on Enter or when the focus leaves it, and says "Saved" or why not beside itself. The one exception is the backend on another machine, whose address and token go together and are checked before they are used.

## Source

- `crates/codewiki-app/src/settings_view.rs` — module `codewiki_app::settings_view`

## How it works

1. `AppView::new` in [[codewiki-app]] calls `State::new`. `State::new` calls `remove` in [[Prefs Module]], then `read_agents_off`.
2. `AppView::take_settings` in [[codewiki-app]] calls `shown_value`.
3. `AppView::test_embedder` in [[codewiki-app]] calls `test_line`.
4. `AppView::refresh_agents_off` in [[codewiki-app]] calls `read_agents_off`. `read_agents_off` calls `data_dir` in [[Paths Module]], then `load`, then `unreadable`, then `file` (all in [[Agents Off Module]]).
5. `AppView::settings_menu_view` in [[codewiki-app]] calls `lists_themes`.

4 more entry points: `change`, `changes_the_embedder`, `job_in`, `job_ended`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

- [[FieldId]] — uses `ALL`, `FieldId`, `placeholder`, `secret`
- [[Settings Pages Module]] — uses `KeptFolder`, `Tested`, `job_runs`
- [[Agents Off Module]] — uses `file`, `load`, `unreadable`
- [[codewiki-app]] — uses `AppView`, `settings_field_event`
- [[Doc Module]] — uses `Choice`
- [[Prefs Module]] — uses `remove`
- [[Settings Module]] — uses `Note`
- [[Paths Module]] — uses `data_dir`

## Used by

- [[codewiki-app]] — from `AppView`, `commit_settings_field`, `load_embed_job`, `main`

## See also

- [[_index]]
- [[codewiki-app]]
