---
tags: [core, codewiki-app]
---

# MotionPref

> What the user asked for.

## What it does

`MotionPref` is the user's choice about animation. It has three values. `System` follows the Windows animation setting and is the default. `Reduced` always holds animations still. `Full` always animates, even if Windows says not to. The [[Motion Module]] stores it, and the settings page and the command palette let the user pick it.

## Source

- `crates/codewiki-app/src/motion.rs` — definition of `MotionPref`

## How it works

Each value has a short key (`system`, `reduced`, `full`) used to save it and to link to it, and a label for people ("follow Windows", "reduced", "full"). `init` in [[Motion Module]] reads the saved text and calls `parse`, which matches it against the key of each value in `ALL`. `reduces` answers the one question the rest of the app needs: given what Windows says, should animations be held still? `System` gives Windows's answer back, `Reduced` is always yes, and `Full` is always no.

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

## Gotchas

- `parse` trims spaces and newlines but is case-sensitive, and it does not guess. An unknown text gives `None`, and the caller falls back to `System`.
- The list `ALL` drives both `parse` and the menus. A new value must be added there, with a key and a label that differ from the others. A test checks this.

## See also

- [[_index]]
- [[Motion Module]]
