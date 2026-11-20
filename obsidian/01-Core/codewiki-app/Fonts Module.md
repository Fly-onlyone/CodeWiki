---
tags: [core, codewiki-app, module]
---

# Fonts Module

> Fonts bundled in the binary.

## What it does

The CodeWiki themes name these families. They are embedded so the app looks the same on a machine that has neither installed. Both are licensed under the SIL Open Font License; the license texts sit beside the font files.

The files are static instances, one per weight. The variable builds are smaller, but DirectWrite lists variable Inter under the family name "Inter Variable Text", which a theme cannot name portably.

## Source

- `crates/codewiki-app/src/fonts.rs` — module `codewiki_app::fonts`

## How it works

1. `main` in [[codewiki-app]] calls `register`.

## Items

- `pub const UI_FAMILY: &str` — family name of the bundled interface font. The CodeWiki themes must name exactly this; a test compares them.
- `pub const MONO_FAMILY: &str` — family name of the bundled code font, checked the same way.
- `pub fn register(cx: &App)` — register the bundled fonts with the text system. Call before any theme is applied.

## Used by

- [[codewiki-app]] — from `main`

## See also

- [[_index]]
- [[codewiki-app]]
