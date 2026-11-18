---
tags: [core, codewiki-app]
---

# MotionPref

> What the user asked for.

## Source

- `crates/codewiki-app/src/motion.rs` — definition of `MotionPref`

## How it works

Declared as `pub enum MotionPref`.

Public API:

- `pub fn key(self) -> &'static str` — the word this choice is stored and linked under.
- `pub fn parse(key: &str) -> Option<Self>` — the choice stored under `key`, if it is one.
- `pub fn label(self) -> &'static str` — how the choice reads in the settings page and the palette.
- `pub const ALL: [Self; 3] = [Self::System, Self::Reduced, Self::Full]`

## Used by

- [[Motion Module]] — from `State`, `current`, `init`, `set`
- [[Palette Module]] — from `Command`, `actions`
- [[Settings Pages Module]] — from `Model`, `appearance`
- [[Action]] — from `Action`

## See also

- [[_index]]
- [[Motion Module]]
