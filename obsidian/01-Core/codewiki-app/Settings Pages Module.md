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
- `job_runs` — whether a job is under way: installing, switching, embedding or paused.
- `page` — the page of `category`.
- `entry` — function
- `size` — bytes as a person reads them: megabytes below a gigabyte.
- `field` — function
- `provider` — A select over the `options` the backend lists for `section.provider`.

## Depends on

- [[Doc Module]] — uses `Choice`, `Press`, `chip`, `code`
- [[Update Module]] — uses `State`, `describe`
- [[Doc]] — uses `Doc`, `new`
- [[MotionPref]] — uses `ALL`, `MotionPref`
- [[FieldId]] — uses `FieldId`, `place`
- [[cw-client]] — uses `Origin`
- [[Icons Module]] — uses `for_theme`
- [[Settings Module]] — uses `Note`

## Used by

- [[codewiki-app]] — from `settings_frame`, `take_models`
- [[Settings View Module]] — from `State`, `test_line`

## See also

- [[_index]]
- [[codewiki-app]]
