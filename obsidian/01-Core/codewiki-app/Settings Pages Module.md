---
tags: [core, codewiki-app, module]
---

# Settings Pages Module

> The pages of the Settings: one for each `Category`, built from what the app knows and from what the backend says about its own settings (`GET /v1/settings`).

## Source

- `crates/codewiki-app/src/settings_pages.rs` — module `codewiki_app::settings_pages`

## How it works

Each is a `Doc`, so a test reads it without a window.

Main items:

- `Model` — everything the pages read.
- `Tested` — where a test of a provider stands.
- `page` — the page of `category`.
- `entry` — function
- `provider` — A select over the `options` the backend lists for `section.provider`.
- `lock` — why the setting `entry` describes cannot be changed here, when it cannot.
- `field` — function
- `card` — function

## Depends on

- [[Doc Module]] — uses `Choice`, `chip`, `code`, `new`
- [[Doc]] — uses `Doc`, `new`
- [[MotionPref]] — uses `ALL`, `MotionPref`
- [[FieldId]] — uses `FieldId`, `place`
- [[cw-client]] — uses `Origin`
- [[Icons Module]] — uses `for_theme`
- [[Settings Module]] — uses `Note`
- [[Update Module]] — uses `State`

## Used by

- [[Settings View Module]] — from `State`, `test_line`
- [[codewiki-app]] — from `settings_frame`

## See also

- [[_index]]
- [[codewiki-app]]
