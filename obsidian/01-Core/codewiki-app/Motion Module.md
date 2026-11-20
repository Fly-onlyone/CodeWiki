---
tags: [core, codewiki-app, module]
---

# Motion Module

> Motion: whether things move at all.

## What it does

gpui-kit reads the Windows "animation effects" setting at startup and holds animations still when it is off. This module adds a choice on top: follow Windows (the default), always reduce, or always animate. The choice is remembered between runs.

Animations need no check of their own. Under reduced motion a one-shot animation is drawn in its end state, a repeating one in its start state and a spring at its target, and none asks for more frames. How things move when they do is in `mui::motion`.

## Source

- `crates/codewiki-app/src/motion.rs` — module `codewiki_app::motion`

## How it works

1. `main` in [[codewiki-app]] calls `init`. `init` calls `read` in [[Prefs Module]], then `MotionPref::parse` in [[MotionPref]].
2. `AppView::settings_frame` in [[codewiki-app]] calls `current`.
3. `AppView::run_action` and `AppView::run_command` in [[codewiki-app]] call `set`. `set` calls `write` in [[Prefs Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const SETTING: &str` — name of the setting in `prefs`.
- [[MotionPref]] — what the user asked for.
- `pub fn init(cx: &mut App)` — apply the remembered choice. Call after `gpui_kit::init`, which is what reads the Windows setting.
- `pub fn current(cx: &App) -> MotionPref` — the current choice.
- `pub fn set(pref: MotionPref, cx: &mut App)` — make a choice, apply it and remember it.

And 1 more private item with no comment.

## Depends on

- [[MotionPref]] — uses `MotionPref`, `key`, `parse`, `reduces`
- [[Prefs Module]] — uses `read`, `write`

## Used by

- [[codewiki-app]] — from `main`, `run_action`, `run_command`, `settings_frame`

## See also

- [[_index]]
- [[codewiki-app]]
