---
tags: [core, codewiki-app, module]
---

# Duration Module

> How long a transition takes.

## What it does

These are MUI's seven standard transition times, copied from its transitions file. Components use a name such as `SHORTEST` instead of a raw number, so all controls animate at the same pace and the app matches MUI's feel.

## Source

- `crates/codewiki-app/src/mui/tokens.rs` — module `codewiki_app::mui::tokens::duration`

## How it works

Each constant is a `Duration`. A component builds a transition from one of them and an easing curve from [[Curve Module]]. The values are 150, 200, 250, 300 and 375 ms for `SHORTEST`, `SHORTER`, `SHORT`, `STANDARD` and `COMPLEX`. `ENTERING_SCREEN` is 225 ms and `LEAVING_SCREEN` is 195 ms, for the drawer.

Small state changes use the short ones: checkbox, switch, icon buttons and list rows use `SHORTEST`; text fields, selects and table rows use `SHORTER`. A folder opening in a list uses `STANDARD`.

## Items

- `pub const SHORTEST: Duration` — public constant; used by `Checkbox::render`, `IconButton::render`, `ListItemButton::render`, `Switch::render` and 1 more.
- `pub const SHORTER: Duration` — public constant; used by `Select::render`, `TextField::render`, `table_row`.
- `pub const SHORT: Duration` — public constant.
- `pub const STANDARD: Duration` — public constant; used by `ListFolder::render`, `glide`.
- `pub const COMPLEX: Duration` — public constant.
- `pub const ENTERING_SCREEN: Duration` — public constant; used by `drawer`.
- `pub const LEAVING_SCREEN: Duration` — public constant; used by `drawer`.

## Used by

- [[Motion Module (mui)]] — from `drawer`, `glide`
- [[Doc View Module]] — from `table_row`
- [[Button Module]] — from `render`
- [[Checkbox Module]] — from `render`
- [[List Module]] — from `render`
- [[Select Module]] — from `render`
- [[Switch Module]] — from `render`
- [[Tabs Module]] — from `render`

## Gotchas

- Wrap a duration in `motion::scaled` before use. The scale follows the user's motion setting, and a raw constant ignores it.
- Leaving is faster than entering, on purpose: 195 against 225 ms. MUI does the same.
- `SHORT` and `COMPLEX` have no user in the app today.

## See also

- [[_index]]
- [[codewiki-app]]
