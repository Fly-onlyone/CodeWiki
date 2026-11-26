---
tags: [core, codewiki-app]
---

# Glyph

> An icon of the app: the path its SVG is served under.

## What it does

A `Glyph` is one icon of the app. It is a small copy-able value that holds the asset path of an SVG, for example `icons/material/home.svg`. Buttons, menus, lists and rows take a `Glyph` instead of a string, so a wrong icon name is a compile error, not a missing picture.

## Source

- `crates/codewiki-app/src/icons.rs` — definition of `Glyph`

## How it works

The [[Icons Module]] creates every `Glyph` with the `app_icons!` macro. For each name the macro makes a constant and embeds the SVG file with `include_bytes!`. `Glyph` implements gpui-kit's `IconNamed`, whose `path` method returns the stored path. When a component draws the icon, GPUI asks the asset source (`AppAssets`) for that path and gets the embedded bytes.

## Items

Declared as `pub struct Glyph`.

Members:

- `fn path(self) -> SharedString` — public method, returns `SharedString`.

## Used by

- [[Icons Module]] — from `for_command`, `for_theme`, `hue`, `tint`
- [[Button Module]] — from `Button`, `IconButton`, `icon`, `new`
- [[List Module]] — from `ListFolder`, `ListItemButton`, `icon`, `new`
- [[Doc Module]] — from `Choice`, `Row`, `new`
- [[Autocomplete Module]] — from `Choice`, `new`
- [[Menu Module]] — from `MenuItem`, `icon`
- [[Select Module]] — from `Select`, `icon`
- [[Text Field Module]] — from `TextField`, `icon`

## Gotchas

- The field is private. Make new glyphs only through `app_icons!`, so the file is embedded and the path starts with `icons/material/`.
- Two glyphs are equal when their paths are equal. `hue` matches on glyph constants, so a new icon stays neutral until it gets an arm there.

## See also

- [[_index]]
- [[Icons Module]]
