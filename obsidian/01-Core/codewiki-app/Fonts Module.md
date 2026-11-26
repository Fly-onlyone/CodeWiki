---
tags: [core, codewiki-app, module]
---

# Fonts Module

> Fonts bundled in the binary.

## What it does

This module puts two fonts inside the app program: Inter for the interface and JetBrains Mono for code. The CodeWiki themes name these two families. Because the font files are embedded, the app looks the same on a computer that has neither font installed. Both fonts use the SIL Open Font License, and the license texts sit beside the font files.

The files are static, one per weight. Variable builds are smaller, but DirectWrite lists variable Inter as "Inter Variable Text", a name a theme cannot use on every system.

## Source

- `crates/codewiki-app/src/fonts.rs` — module `codewiki_app::fonts`

## How it works

`main` calls `register` once at startup, before `themes::init`. `register` builds a list of eight font files (five Inter, three JetBrains Mono) with `include_bytes!`, so the bytes live in the executable. It hands the list to GPUI's text system with `add_fonts`.

If `add_fonts` fails, `register` prints a line to standard error and returns. The app then runs with the system fonts. In debug builds only, `register` also checks that both family names appear in the text system's font list, and prints a warning if one does not.

## Items

- `pub const UI_FAMILY: &str` — family name of the bundled interface font. The CodeWiki themes must name exactly this; a test compares them.
- `pub const MONO_FAMILY: &str` — family name of the bundled code font, checked the same way.
- `pub fn register(cx: &App)` — register the bundled fonts with the text system. Call before any theme is applied.

## Used by

- [[codewiki-app]] — from `main`

## Gotchas

- `UI_FAMILY` and `MONO_FAMILY` must match the family names the themes use, letter for letter. A test compares them.
- A family the text system does not know falls back to the system font with no error. Only the debug check shows it, so a wrong name goes unseen in a release build.
- Failure is reported with `eprintln!`, not the log. A release build has no console, so a failed load is silent there.
- Call `register` before any theme is applied, or the theme may resolve a family before it exists.

## See also

- [[_index]]
- [[codewiki-app]]
