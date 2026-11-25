---
tags: [core, codewiki-app, module]
---

# Duration Module

> How long a transition takes.

## What it does

`createTransitions.js`, `duration`.

## Source

- `crates/codewiki-app/src/mui/tokens.rs` — module `codewiki_app::mui::tokens::duration`

## Items

- `pub const SHORTEST: Duration` — public constant; used by `Checkbox::render`, `IconButton::render`, `ListItemButton::render`, `Switch::render` and 1 more.
- `pub const SHORTER: Duration` — public constant; used by `Select::render`, `TextField::render`, `table_row`.
- `pub const SHORT: Duration` — public constant.
- `pub const STANDARD: Duration` — public constant; used by `ListFolder::render`.
- `pub const COMPLEX: Duration` — public constant.
- `pub const ENTERING_SCREEN: Duration` — public constant; used by `drawer`.
- `pub const LEAVING_SCREEN: Duration` — public constant; used by `drawer`.

## Used by

- [[Doc View Module]] — from `table_row`
- [[Button Module]] — from `render`
- [[Checkbox Module]] — from `render`
- [[List Module]] — from `render`
- [[Motion Module (mui)]] — from `drawer`
- [[Select Module]] — from `render`
- [[Switch Module]] — from `render`
- [[Tabs Module]] — from `render`

## See also

- [[_index]]
- [[codewiki-app]]
