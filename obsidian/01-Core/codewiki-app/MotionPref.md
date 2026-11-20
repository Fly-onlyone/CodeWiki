---
tags: [core, codewiki-app]
---

# MotionPref

> What the user asked for.

## Source

- `crates/codewiki-app/src/motion.rs` — definition of `MotionPref`

## How it works

1. `init` in [[Motion Module]] calls `parse`. `parse` calls `key`.

## Items

Declared as `pub enum MotionPref`.

Members:

- `pub const ALL: [Self; 3]` — public constant; used by `MotionPref::parse`, `actions`, `appearance`.
- `pub fn key(self) -> &'static str` — the word this choice is stored and linked under.
- `pub fn parse(key: &str) -> Option<Self>` — the choice stored under `key`, if it is one.
- `pub fn label(self) -> &'static str` — how the choice reads in the settings page and the palette.
- `fn reduces(self, system_reduces: bool) -> bool` — whether animations are held still, given what Windows says.

## Used by

- [[Motion Module]] — from `State`, `current`, `init`, `set`
- [[Palette Module]] — from `Command`, `actions`
- [[Settings Pages Module]] — from `Model`, `appearance`
- [[Action]] — from `Action`

## See also

- [[_index]]
- [[Motion Module]]
