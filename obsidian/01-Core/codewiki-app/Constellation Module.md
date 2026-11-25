---
tags: [core, codewiki-app, module]
---

# Constellation Module

> The constellation behind the glass: a made-up graph of notes and the links between them, which the backdrop (`backdrop.rs`) draws over its soft color.

## What it does

It is the same everywhere. A fixed seed lays it out, so every start and every repository shows the same sky, and nothing is asked of the backend. Nodes are spread over the whole window as shares of its width and height: the dense reading column hides what falls behind it, and the clear margins beside it show the rest.

The nodes form three clusters, one for each of the theme's brand colors. Inside a cluster the links make a tree around a hub, the way a map of content links its notes, plus a few links that close loops; one bridge joins each pair of clusters where they come closest.

Light travels along the links. A pulse runs from one end of a link to the other, and may go on along another link from the node it reaches. Pulses come now and then by themselves (`Traffic`), and often while the wiki is being written.

No UI types, so all of it is unit-tested.

## Source

- `crates/codewiki-app/src/constellation.rs` — module `codewiki_app::constellation`

## How it works

1. `Shell::new` in [[Backdrop Module]] calls `Constellation::new`. `Constellation::new` calls `Constellation::from_seed`. `Constellation::from_seed` calls `Rng::new`, then `distance`, then `add_link`.
2. `Sky::new` in [[Backdrop Module]] calls `Pulse::brightness`.
3. `Sky::new` in [[Backdrop Module]] calls `Pulse::flash`.
4. `Shell::new` in [[Backdrop Module]] calls `Traffic::new`. `Traffic::new` calls `Rng::new`.
5. `Sky::new` in [[Backdrop Module]] calls `segment_distance`.

1 more entry point: `lit`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const COLUMNS: usize` — nodes in the constellation: a grid of `COLUMNS` by `ROWS`, each moved off its cell's middle.
- `pub const NODES: usize` — public constant; used by `Constellation::from_seed`.
- `pub const CLUSTERS: usize` — clusters, one per brand color.
- `const CENTRES: [(f32, f32); CLUSTERS]` — where each cluster gathers, as shares of the window's width and height: one behind the sidebar and the left margin, one up and right, one low and right, so that each color shows beside the page.
- `const ASPECT: f32` — width over height of the window the distances are measured for, so that "nearest" looks nearest on a wide window.
- `const SEED: u64` — the seed the sky is laid out from. Any value makes a sky; this one was looked at.
- `const REACH: f32` — how far a node wanders from its place, as a share of the window's shorter side.
- `struct Rng` — A small, fast generator of numbers that look random: SplitMix64. The sky and the pulses need nothing better, and no crate is brought in for it.
- `pub struct Node` — one note of the sky.
- `pub struct Constellation` — the whole sky: its nodes and the links between them.
- `fn distance(a: (f32, f32), b: (f32, f32)) -> f32` — distance between two resting places, measured as on a wide window.
- `fn add_link(links: &mut Vec<[usize; 2]>, a: usize, b: usize)` — link `a` and `b`, unless they are one node or linked already.
- `pub struct Pulse` — light running along one link.
- `const FLASH_FROM: f32` — how far along its link a pulse is when the node ahead starts to flash.
- `const SPEED: f32` — speed of a pulse, in the units of `distance` per second.
- `const RUN: (f32, f32)` — the shortest and longest a pulse takes for one link, in seconds.
- `fn run_time(length: f32) -> f32` — seconds a pulse takes to run a link `length` long.
- `pub struct Traffic` — the pulses on their way, and when the next one sets out.
- `pub fn segment_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32` — how far `p` is from the nearest point of the segment from `a` to `b`.
- `pub fn lit(distance: f32, radius: f32) -> f32` — how strongly something at `distance` pixels from the pointer is lit, from 1 under the pointer to 0 at `radius` and beyond, easing out at the rim.

And 9 more items.

## Used by

- [[Backdrop Module]] — from `Shell`, `drift`, `new`

## See also

- [[_index]]
- [[codewiki-app]]
