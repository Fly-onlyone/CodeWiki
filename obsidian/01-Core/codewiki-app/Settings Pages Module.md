---
tags: [core, codewiki-app, module]
---

# Settings Pages Module

> The pages of the Settings: one for each `Category`, built from what the app knows and from what the backend says about its own settings (`GET /v1/settings`).

## What it does

Each is a `Doc`, so a test reads it without a window.

A setting the backend keeps can be out of the user's hands here for one of three reasons, and its control then says which: an environment variable of the backend sets it; the backend runs on another machine; or the backend cannot save a key at all.

## Source

- `crates/codewiki-app/src/settings_pages.rs` — module `codewiki_app::settings_pages`

## How it works

1. `AppView::settings_frame` in [[codewiki-app]] calls `page`. `page` calls `Doc::new` in [[Doc]], then `appearance`, then `search`, then `backend`, then `repositories`, then `updates`, and 2 more. `appearance` calls `card`, then `for_theme` in [[Icons Module]], then `text`, then `code` (both in [[Doc Module]]), then `zoom_card`, then `startup`. `search` calls `without_settings`, then `provider`, then `server_rows`, then `in_use`, then `text`, then `Press::new` (both in [[Doc Module]]), and 5 more.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub enum Tested` — where a test of a provider stands.
- `pub struct Model<'a>` — everything the pages read.
- `pub fn page(category: Category, model: &Model) -> Doc` — the page of `category`.
- `fn card(title: &str, body: Vec<Block>) -> Block` — private function, returns `Block`; called by `appearance`, `backend`, `diagnostics`, `error_reports` and 4 more.
- `fn appearance(model: &Model) -> Vec<Block>` — private function, returns `Vec<Block>`; called by `page`.
- `fn zoom_card(model: &Model) -> Block` — how large the whole app is drawn.
- `fn startup(model: &Model) -> Block` — where the app starts, whether Windows starts it at sign-in, and what closing the window does: the last two belong together, since an app started that way is seen only as its tray icon.
- `fn without_settings(model: &Model) -> Vec<Block>` — what a page that needs the backend's settings shows without them.
- `fn lock(model: &Model, settings: &Value, entry: &Value) -> Option<String>` — why the setting `entry` describes cannot be changed here, when it cannot.
- `provider` — A select over the `options` the backend lists for `section.provider`.
- `select` — A select over the `options` the backend lists for `section.field`.
- `fn in_use(settings: &Value, section: &str, nothing: &str) -> Block` — what the settings of `section` select, or why they select nothing usable.
- `fn search(model: &Model) -> Vec<Block>` — private function, returns `Vec<Block>`; called by `page`.
- `fn server_rows(model: &Model, settings: &Value, provider: &str) -> Vec<Block>` — the address and key rows of a server, then its model, then how to start it. Nothing for the word-form embedder and for none.
- `fn lists(models: &[Value], provider: &str, chosen: &str) -> bool` — whether the models a server listed hold `chosen`. Ollama takes a name without a tag as its `latest`: "The tag is optional and, if not provided, will default to latest" (https://github.com/ollama/ollama/blob/main/docs/api.md).
- `pub fn job_runs(job: &Value) -> bool` — whether an embedding job is under way.
- `fn job(job: &Value) -> Vec<Block>` — what the embedding job that runs, or the last that ran, has to say.
- `fn backend(model: &Model) -> Vec<Block>` — private function, returns `Vec<Block>`; called by `page`.
- `fn repositories(model: &Model) -> Vec<Block>` — private function, returns `Vec<Block>`; called by `page`.
- `fn updates(model: &Model) -> Vec<Block>` — the version, what the last check found, and whether the app checks by itself.

And 14 more items.

## Depends on

- [[Doc Module]] — uses `Choice`, `chip`, `code`, `new`
- [[Zoom Module]] — uses `DEFAULT`, `LEVELS`, `label`
- [[Update Module]] — uses `State`, `describe`
- [[Doc]] — uses `Doc`, `new`
- [[MotionPref]] — uses `ALL`, `MotionPref`
- [[FieldId]] — uses `FieldId`, `place`
- [[cw-client]] — uses `Origin`
- [[cw-telemetry]] — uses `DSN_ENV`

## Used by

- [[Settings View Module]] — from `State`, `job_ended`, `test_line`
- [[codewiki-app]] — from `settings_frame`, `take_embed_job`

## See also

- [[_index]]
- [[codewiki-app]]
