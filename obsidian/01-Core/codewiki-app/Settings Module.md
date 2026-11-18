---
tags: [core, codewiki-app, module]
---

# Settings Module

> What the Settings page is made of, apart from how it is drawn: its categories, the fields a user types into, and the news shown beside one.

## Source

- `crates/codewiki-app/src/settings.rs` — module `codewiki_app::settings`

## How it works

No UI here; `pages.rs` builds a category's page from these and `settings_view.rs` draws it.

Main items:

- [[FieldId]] — A setting the user types: each has one text input, which lives as long as the app does.
- [[Category]] — A group of settings, with a page of its own.
- `Note` — news about one setting, shown beside it.

## Used by

- [[Doc View Module]] — from `secret_control`, `under`
- [[Doc Module]] — from `with_state`
- [[Settings Pages Module]] — from `Model`
- [[Settings View Module]] — from `State`
- [[Block]] — from `Block`

## See also

- [[_index]]
- [[codewiki-app]]
