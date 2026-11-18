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
- `LOG_LEVEL_PREF` — the name the app's log level is saved under (`prefs.rs`).
- `shown_value` — what a field shows for the setting `entry` describes: the value that is saved or that the environment gives.
- `test_line` — the line a test of a provider left.
- `change` — the body of a `PUT /v1/settings` that changes one field.
- `OpenMenu` — the list of a select that is open.
- `lists_themes` — whether the options of a select are the themes.

## Depends on

- [[FieldId]] — uses `ALL`, `FieldId`, `placeholder`, `secret`
- [[codewiki-app]] — uses `AppView`, `settings_field_event`
- [[Doc Module]] — uses `Choice`
- [[Settings Module]] — uses `Note`
- [[Settings Pages Module]] — uses `Tested`
- [[Category]] — uses `Category`

## Used by

- [[codewiki-app]] — from `AppView`, `commit_settings_field`, `main`, `new`

## See also

- [[_index]]
- [[codewiki-app]]
