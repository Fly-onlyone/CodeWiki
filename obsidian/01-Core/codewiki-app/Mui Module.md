---
tags: [core, codewiki-app, module]
---

# Mui Module

> Components drawn to Material UI's measurements.

## What it does

This is the home of the app's UI building blocks: buttons, switches, tabs, menus, text fields and so on. The app is native (GPUI, not a browser), so Material UI cannot be used directly. Each component copies the anatomy, sizes and behaviour of its MUI 6 version, plus the changes the TradingAgent theme makes. Colors always come from the active CodeWiki theme, so a theme change restyles every component.

## Source

- `crates/codewiki-app/src/mui/mod.rs` — module `codewiki_app::mui`

## How it works

The file only declares the submodules, one per component family, and holds the shared doc comment. There is no logic here. Screens such as settings and the report pages import the pieces they need from the submodules. [[Tokens Module]] holds the MUI measurement table; [[Motion Module (mui)]] and [[Ripple Module]] hold the animation parts.

## Items

Submodules: [[Autocomplete Module]], [[Base Module]], [[Button Module]], [[Checkbox Module]], [[Feedback Module]], [[Interaction Module]], [[List Module]], [[Menu Module]], [[Motion Module (mui)]], [[Ripple Module]], [[Select Module]], [[Snackbar Module]], [[Surfaces Module]], [[Switch Module]], [[Tabs Module]], [[Text Field Module]], [[Tokens Module]].

## Gotchas

- GPUI cannot scale an element, blur what is behind it, or space letters apart. Components that look different from MUI there are working around these limits, not making mistakes.
- The tokens module is marked to allow dead code. It is a table copied whole from MUI, so unused entries are normal. Do not delete them to silence a warning.

## See also

- [[_index]]
- [[codewiki-app]]
