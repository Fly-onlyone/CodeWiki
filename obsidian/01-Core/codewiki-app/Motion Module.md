---
tags: [core, codewiki-app, module]
---

# Motion Module

> Motion: whether things move at all.

## Source

- `crates/codewiki-app/src/motion.rs` — module `codewiki_app::motion`

## How it works

Main items:

- [[MotionPref]] — what the user asked for.
- `set` — make a choice, apply it and remember it.
- `init` — apply the remembered choice.
- `current` — the current choice.
- `State` — struct
- `SETTING` — name of the setting in `prefs`.

## Depends on

- [[MotionPref]] — uses `MotionPref`, `key`, `parse`, `reduces`
- [[Prefs Module]] — uses `read`, `write`

## Used by

- [[codewiki-app]] — from `main`, `run_action`, `run_command`, `settings_frame`

## See also

- [[_index]]
- [[codewiki-app]]
