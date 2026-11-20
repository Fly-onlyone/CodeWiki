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

1. `AppView::new` in [[codewiki-app]] calls `State::new`. `State::new` calls `read` in [[Prefs Module]].
2. `AppView::take_settings` in [[codewiki-app]] calls `shown_value`.
3. `AppView::test_setting` in [[codewiki-app]] calls `test_line`.
4. `AppView::settings_menu_view` in [[codewiki-app]] calls `lists_themes`.
5. `AppView::commit_settings_field` and `AppView::run_settings_action` in [[codewiki-app]] call `change`.

2 more entry points: `changes_the_embedder`, `settings_behind`.

## Items

- `pub const LOG_LEVEL_PREF: &str` — the name the app's log level is saved under (`prefs.rs`).
- `pub const MODEL_HINT: &str` — file of the app's settings that says the offer of a built-in model on Home was turned down.
- `const MODELS_PAUSE: std::time::Duration` — how long the app waits before it asks again how far a model's job is.
- `struct OpenMenu` — the list of a select that is open.
- `pub struct State` — what the Settings page remembers between frames.
- `fn shown_value(entry: &Value) -> String` — what a field shows for the setting `entry` describes: the value that is saved or that the environment gives. A default is not typed in; the input's placeholder stands for it.
- `fn test_line(result: Result<Value, String>) -> Tested` — the line a test of a provider left.
- `fn lists_themes(options: &[Choice]) -> bool` — whether the options of a select are the themes.
- `fn change(section: &str, field: &str, value: Value) -> Value` — the body of a `PUT /v1/settings` that changes one field.
- `fn changes_the_embedder(body: &Value) -> bool` — whether a change touches the embedder, after which what the backend said about its models (the one in use, the embedder chosen) is out of date.
- `fn settings_behind(models: &Value, settings: Option<&Value>) -> bool` — whether the models tell of another embedder than the settings read before: a job switched it since.

## Depends on

- [[FieldId]] — uses `ALL`, `FieldId`, `placeholder`, `secret`
- [[codewiki-app]] — uses `AppView`, `settings_field_event`
- [[Doc Module]] — uses `Choice`
- [[Prefs Module]] — uses `read`
- [[Settings Module]] — uses `Note`
- [[Settings Pages Module]] — uses `Tested`
- [[Category]] — uses `Category`

## Used by

- [[codewiki-app]] — from `AppView`, `commit_settings_field`, `main`, `new`

## See also

- [[_index]]
- [[codewiki-app]]
