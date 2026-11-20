---
tags: [core, cw-models, module]
---

# Game Module

> Whether a game is running.

## Source

- `crates/cw-models/src/game.rs` — module `cw_models::game`

## How it works

Main items:

- `Detector` — the rule: both readings, for long enough.
- `busy` — whether a game is running on this machine.
- `Reading` — what the machine says at one moment.
- `Watch` — struct
- `FRESH` — A look is this old at the most; who asks sooner gets the last answer.
- `FORGET` — after this long without a look, what was seen before counts no more.
- `LOAD` — percent of a 3D engine that the other programs must keep busy.
- `SUSTAIN` — how long the load must last.

Submodules: [[Win32 Module (game)]].

## Depends on

- [[Win32 Module (game)]] — uses `full_screen`, `load_3d_of_others`

## Used by

- [[Worker Module]] — from `global`
- [[Host]] — from `default`

## See also

- [[_index]]
- [[cw-models]]
