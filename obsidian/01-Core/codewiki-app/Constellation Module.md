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

`Constellation::new` calls `from_seed` with a fixed `SEED`. A `Rng` (SplitMix64) gives the numbers.

1. Layout: a grid of 7 by 6 nodes. Each node moves off its cell's middle by up to 38 percent of the cell. Each takes one of three clusters, whose centres sit in set places of the window.
2. Links: inside a cluster the nodes form a tree around a hub. About 30 percent of nodes get one more link, to their second nearest neighbour, which closes a loop. `add_link` skips a link between a node and itself and a link that exists. One bridge joins each pair of clusters where they come closest. `distance` measures as on a window 1.6 times wider than tall.
3. Drift: `Node::place` gives where a node is at a phase of the drift, a small wander of 1.2 percent of the window's shorter side.
4. Light: `Traffic` holds the pulses. `advance` is called each frame with the clock and a `busy` flag. A pulse runs a link, then may go on along another link from the node it reaches, until its hops are used up. Quiet, a pulse starts every 3.5 seconds on average, with at most 4 at once and 3 hops. Busy (a wiki is being written), every 0.35 seconds, at most 18 at once and 6 hops.
5. Drawing support: `Pulse::brightness` and `Pulse::flash` give the glow along a link and at its end node. `segment_distance` and `lit` give the pointer's light on links and nodes.

The shell in [[Backdrop Module]] builds the sky and the traffic, and draws them.

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

## Gotchas

- The sky never changes, and nothing is asked of the backend. Changing `SEED` or the grid redraws every user's sky.
- Positions are shares of the window, so the layout stretches with it. The `ASPECT` only affects which node counts as nearest.
- A pulse takes between 0.7 and 2.6 seconds for a link, whatever its length (`run_time` clamps it).
- The module has no UI types, so tests check it without a window: the sky is the same each time, every node has a link, and the whole sky is one graph.
- The nodes stay put when the backdrop stands still. The motion setting, an inactive window and software drawing all stop the drift and pulses in [[Backdrop Module]], not here.

## See also

- [[_index]]
- [[codewiki-app]]
