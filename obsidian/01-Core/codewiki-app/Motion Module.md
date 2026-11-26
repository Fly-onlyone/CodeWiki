---
tags: [core, codewiki-app, module]
---

# Motion Module

> Motion: whether things move at all.

## What it does

gpui-kit reads the Windows "animation effects" setting at startup and holds animations still when it is off. This module lets the user override that. There are three choices, see [[MotionPref]]: follow Windows (the default), always reduce, or always animate. The choice is saved between runs.

Animations need no check of their own. Under reduced motion a one-shot animation is drawn in its end state, a repeating one in its start state, and a spring at its target. None asks for more frames.

## Source

- `crates/codewiki-app/src/motion.rs` — module `codewiki_app::motion`

## How it works

`main` calls `init` right after `gpui_kit::init`, which is what reads the Windows setting. `init` records what Windows said (`system_reduces`) in a global `State`. It then reads the saved choice from the preferences, with `MotionPref::parse`. A missing or unknown value means "follow Windows". It then calls `set_reduce_motion` on the app with the result of `MotionPref::reduces`.

`current` returns the choice, for the settings page. `set` does the same steps for a new choice and also writes it with `prefs::write`. It is called from `run_action` (settings page) and `run_command` (palette).

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

## Gotchas

- `init` must run after `gpui_kit::init`. Before that, there is no Windows value to remember.
- The module keeps the Windows value itself. The app's own choice overwrites the flag that gpui-kit set from it, so without a copy a return to "follow Windows" would not know what Windows says.
- The Windows value is read once, at startup. A change in Windows while the app runs is not seen until the next start.
- Saved text is read with trimming but not guessed: "Reduced" with a capital letter is not accepted and falls back to the default.

## See also

- [[_index]]
- [[codewiki-app]]
