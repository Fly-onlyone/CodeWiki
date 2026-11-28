---
tags: [core, codewiki-app]
---

# Category

> A group of settings, with a page of its own.

## What it does

`Category` is a group of settings with a page of its own in the app's Settings. There are seven: Appearance, Semantic search, Backend, Repositories, Updates, Keyboard and Diagnostics. The order of `ALL` is the order the list shows. Each category has a label, an icon and a slug.

## Source

- `crates/codewiki-app/src/settings.rs` — definition of `Category`

## How it works

`label` and `icon` give what the settings list draws. `slug` is the category's short name in a startup argument of the form `settings:<slug>`. `AppView::open_startup_view` in [[codewiki-app]] passes the text after the colon to `from_slug`. `from_slug` trims it, lowercases it and finds the category whose slug matches. The [[Settings Pages Module]] builds a category's page, and the [[Settings View Module]] draws it.

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
- [[Ready Module]] — from `kind`
- [[Settings Pages Module]] — from `page`
- [[Settings View Module]] — from `State`
- [[Action]] — from `Action`

## Gotchas

- `from_slug` never fails. An empty or unknown slug opens Appearance, the first category. A test shows `ai`, the slug of a page CodeWiki once had, does this.
- The match is case-insensitive and ignores spaces around the slug.
- Each category must have its own icon. A test checks this.
- The count in `ALL` is fixed at seven. Adding a category means adding it to `ALL`, `label`, `icon` and `slug`, and the page for it.

## See also

- [[_index]]
- [[Settings Module]]
