---
tags: [core, codewiki-app]
---

# Category

> A group of settings, with a page of its own.

## Source

- `crates/codewiki-app/src/settings.rs` — definition of `Category`

## How it works

Declared as `pub enum Category`.

The order is the order of the list.

Public API:

- `pub fn slug(self) -> &'static str` — the category's name in a startup argument: `settings:<slug>`.
- `pub fn from_slug(slug: &str) -> Self` — the category a slug names.
- `pub const ALL: [Self; 8] = [ Self::Appearance, Self::AiModel, Self::Search, Self::Backend, Self::Repositories, Self::Updates, Self::Keyboard, Self::Diagnostics, ]`
- `pub fn label(self) -> &'static str`
- `pub fn icon(self) -> Glyph`

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
