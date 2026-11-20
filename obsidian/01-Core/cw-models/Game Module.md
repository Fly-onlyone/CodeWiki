---
tags: [core, cw-models, module]
---

# Game Module

> Whether a game is running.

## What it does

A model's worker holds memory, and embedding in the background takes the processor or the graphics card. While the user plays, both give way: the worker does not linger, and a background job waits.

A game is two things at once:

1. Windows says a full-screen app is in front. This is what Windows itself goes by to hold notifications back.
2. The programs other than this one keep the graphics card's 3D engines busy.

One without the other is not a game: a full-screen video leaves the card almost idle, and a busy card with no full-screen app is some other work. The load must also last, so that a scene change in a video counts for nothing.

`Detector` is the rule, over readings it is given. The readings come from the two Win32 calls in the `win32` module below.

## Source

- `crates/cw-models/src/game.rs` — module `cw_models::game`

## How it works

What the functions of this module call:

1. `busy` calls `full_screen`, then `load_3d_of_others` (both in [[Win32 Module (game)]]).

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const LOAD: f64` — percent of a 3D engine that the other programs must keep busy.
- `const SUSTAIN: Duration` — how long the load must last.
- `const FORGET: Duration` — after this long without a look, what was seen before counts no more.
- `const FRESH: Duration` — A look is this old at the most; who asks sooner gets the last answer. One look takes a second, to measure a rate.
- `pub struct Reading` — what the machine says at one moment.
- `pub struct Detector` — the rule: both readings, for long enough.
- `pub fn busy() -> bool` — whether a game is running on this machine. Cheap to ask often: the machine is looked at every few seconds at the most, and its graphics load only while a full-screen app is in front. Always false where the readings do not exist.

And 1 more private item with no comment.

Submodules: [[Win32 Module (game)]].

## Depends on

- [[Win32 Module (game)]] — uses `full_screen`, `load_3d_of_others`

## Used by

- [[Worker Module]] — from `global`
- [[Host]] — from `default`

## See also

- [[_index]]
- [[cw-models]]
