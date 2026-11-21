---
tags: [core, codewiki-app]
---

# Category

> A group of settings, with a page of its own.

## What it does

The order is the order of the list.

## Source

- `crates/codewiki-app/src/settings.rs` — definition of `Category`

## How it works

1. `AppView::open_startup_view` in [[codewiki-app]] calls `from_slug`. `from_slug` calls `slug`.

## Items

Declared as `pub enum Category`.

Members:

- `pub const ALL: [Self; 7]` — public constant; used by `AppView::settings_frame`, `Category::from_slug`, `actions`.
- `pub fn label(self) -> &'static str` — public method, returns `&'static str`.
- `pub fn icon(self) -> Glyph` — public method, returns `Glyph`.
- `pub fn slug(self) -> &'static str` — the category's name in a startup argument: `settings:<slug>`.
- `pub fn from_slug(slug: &str) -> Self` — the category a slug names. An empty or unknown one is the first.

## Depends on

- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `Page`, `open_settings`, `open_startup_view`, `settings_frame`
- [[Palette Module]] — from `Command`, `actions`
- [[Settings Pages Module]] — from `page`
- [[Settings View Module]] — from `State`
- [[Action]] — from `Action`

## See also

- [[_index]]
- [[Settings Module]]
