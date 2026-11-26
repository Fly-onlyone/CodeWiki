---
tags: [core, codewiki-app, module]
---

# Settings Module

> What the Settings page is made of, apart from how it is drawn: its categories, the fields a user types into, and the news shown beside one.

## What it does

This module holds the data side of the Settings page, with no UI. It names the seven categories of settings ([[Category]]), the thirteen fields a user types into ([[FieldId]]), and the news shown beside a setting (`Note`). The Settings Pages Module builds a category's page from these, and the Settings View Module draws it.

## Source

- `crates/codewiki-app/src/settings.rs` — module `codewiki_app::settings`

## How it works

`Note` has three states. `Working` means the change is on its way to the backend. `Saved` means it went through. `Error` carries the reason the change was refused. A `Field`, `Select` or `Secret` [[Block]] can carry a `Note`, and the view draws it beside the setting.

A [[FieldId]] tells where its value lives, whether it is a secret, and what an empty input shows. A [[Category]] gives the label, the icon and the slug for a startup argument.

## Items

- [[Category]] — A group of settings, with a page of its own.
- [[FieldId]] — A setting the user types: each has one text input, which lives as long as the app does.
- `pub enum Note` — news about one setting, shown beside it.

## Used by

- [[Doc View Module]] — from `secret_control`, `under`
- [[Doc Module]] — from `with_state`
- [[Settings Pages Module]] — from `Model`
- [[Settings View Module]] — from `State`
- [[Block]] — from `Block`

## Gotchas

- Nothing here draws. Changing how a setting looks happens in the pages and view modules, not here.
- A category or field added here is not shown until a page builds it.

## See also

- [[_index]]
- [[codewiki-app]]
