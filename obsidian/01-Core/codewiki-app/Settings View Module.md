---
tags: [core, codewiki-app, module]
---

# Settings View Module

> The Settings page as the app's view draws and runs it.

## Source

- `crates/codewiki-app/src/settings_view.rs` — module `codewiki_app::settings_view`

## How it works

Main items:

- `State` — what the Settings page remembers between frames.
- `MODEL_HINT` — file of the app's settings that says the offer of a built-in model on Home was turned down.
- `LOG_LEVEL_PREF` — the name the app's log level is saved under (`prefs.rs`).
- `shown_value` — what a field shows for the setting `entry` describes: the value that is saved or that the environment gives.
- `test_line` — the line a test of a provider left.
- `MODELS_PAUSE` — how long the app waits before it asks again how far a model's job is.
- `change` — the body of a `PUT /v1/settings` that changes one field.
- `OpenMenu` — the list of a select that is open.

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
