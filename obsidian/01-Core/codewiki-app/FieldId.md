---
tags: [core, codewiki-app]
---

# FieldId

> A setting the user types: each has one text input, which lives as long as the app does.

## Source

- `crates/codewiki-app/src/settings.rs` — definition of `FieldId`

## Items

Declared as `pub enum FieldId`.

Members:

- `pub const ALL: [Self; 13]` — public constant; used by `AppView::commit_settings_fields`, `AppView::take_settings`, `State::new`.
- `pub fn place(self) -> Option<(&'static str, &'static str)>` — where the backend keeps the setting: its section and field in `GET /v1/settings`. `None` for the two the app keeps itself, the address and token of a backend on another machine.
- `pub fn secret(self) -> bool` — whether what is typed is hidden, and never shown again once saved.
- `pub fn placeholder(self) -> &'static str` — what an empty input shows.

## Used by

- [[codewiki-app]] — from `commit_settings_field`, `commit_settings_fields`, `connect_remote`, `forget_saved_note`
- [[Settings Pages Module]] — from `Model`, `entry`, `field`, `secret`
- [[Doc View Module]] — from `Controls`, `secret_control`
- [[Settings View Module]] — from `State`, `new`
- [[Action]] — from `Action`
- [[Block]] — from `Block`
- [[Doc]] — from `fields`

## See also

- [[_index]]
- [[Settings Module]]
