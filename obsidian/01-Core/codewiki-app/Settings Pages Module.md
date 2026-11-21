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

1. `AppView::settings_frame` in [[codewiki-app]] calls `page`. `page` calls `Doc::new` in [[Doc]], then `appearance`, then `search`, then `backend`, then `repositories`, then `updates`, and 2 more. `appearance` calls `card`, then `for_theme` in [[Icons Module]], then `text`, then `code` (both in [[Doc Module]]), then `zoom_card`, then `startup`. `search` calls `without_settings`, then `lock`, then `provider`, then `builtin_choice`, then `field`, then `in_use`, and 7 more.
2. `AppView::take_models` in [[codewiki-app]] calls `job_runs`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub enum Tested` — where a test of a provider stands.
- `pub struct Model<'a>` — everything the pages read.
- `pub fn page(category: Category, model: &Model) -> Doc` — the page of `category`.
- `fn card(title: &str, body: Vec<Block>) -> Block` — private function, returns `Block`; called by `appearance`, `backend`, `built_in`, `diagnostics` and 5 more.
- `fn appearance(model: &Model) -> Vec<Block>` — private function, returns `Vec<Block>`; called by `page`.
- `fn zoom_card(model: &Model) -> Block` — how large the whole app is drawn.
- `fn startup(model: &Model) -> Block` — where the app starts, whether Windows starts it at sign-in, and what closing the window does: the last two belong together, since an app started that way is seen only as its tray icon.
- `fn without_settings(model: &Model) -> Vec<Block>` — what a page that needs the backend's settings shows without them.
- `fn lock(model: &Model, settings: &Value, entry: &Value) -> Option<String>` — why the setting `entry` describes cannot be changed here, when it cannot.
- `fn field(model: &Model, settings: &Value, id: FieldId, label: &str, help: &str) -> Block` — private function, returns `Block`; called by `entry`, `search`, `select`.
- `provider` — A select over the `options` the backend lists for `section.provider`.
- `select` — A select over the `options` the backend lists for `section.field`.
- `fn in_use(settings: &Value, section: &str, nothing: &str) -> Block` — what the settings of `section` select, or why they select nothing usable.
- `fn search(model: &Model) -> Vec<Block>` — private function, returns `Vec<Block>`; called by `page`.
- `fn builtin_choice(models: Option<&Value>, settings: &Value) -> Action` — what choosing the built-in embedder does. An installed model is used at once, the recommended one first, and one in use is left as it is. With none installed the choice is saved, and waits for Install or Use to name a model. What is in use comes from `settings`, which every change brings back; the models may have been read before the last change.
- `fn size(bytes: u64) -> String` — bytes as a person reads them: megabytes below a gigabyte.
- `pub fn job_runs(models: &Value) -> bool` — whether a job is under way: installing, switching, embedding or paused.
- `fn backend(model: &Model) -> Vec<Block>` — private function, returns `Vec<Block>`; called by `page`.
- `fn repositories(model: &Model) -> Vec<Block>` — private function, returns `Vec<Block>`; called by `page`.
- `fn updates(model: &Model) -> Vec<Block>` — the version, what the last check found, and whether the app checks by itself.

And 14 more items.

## Depends on

- [[Doc Module]] — uses `Choice`, `Press`, `chip`, `code`
- [[Zoom Module]] — uses `DEFAULT`, `LEVELS`, `label`
- [[Update Module]] — uses `State`, `describe`
- [[Doc]] — uses `Doc`, `new`
- [[MotionPref]] — uses `ALL`, `MotionPref`
- [[FieldId]] — uses `FieldId`, `place`
- [[cw-client]] — uses `Origin`
- [[cw-telemetry]] — uses `DSN_ENV`

## Used by

- [[codewiki-app]] — from `settings_frame`, `take_models`
- [[Settings View Module]] — from `State`, `test_line`

## See also

- [[_index]]
- [[codewiki-app]]
