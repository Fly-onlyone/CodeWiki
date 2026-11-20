---
tags: [core, codewiki-app, module]
---

# Backdrop Module

> The drifting backdrop: soft color under the glass.

## What it does

A few large soft shapes in the theme's colors sit behind every surface and drift slowly. They are what the glass has to show when the wallpaper behind the window is plain.

gpui-kit has no radial gradient, and its box shadow spreads a blur this wide so thin that the color all but vanishes. A shape is therefore a stack of faint discs, each smaller than the last, which adds up to a color that is strongest in the middle and fades to nothing at the rim.

`Shell` is the window's outermost view. It draws the shapes and, on top, the app and the generated page, each as a cached view. Only the shell is redrawn when the shapes move; the app and the page keep their last frames until they change themselves, and neither is redrawn when the other is.

The title bar is the exception: the shell draws it itself, in every frame. GPUI keeps the hit areas of a cached view but not its window control areas (the minimize, maximize and close buttons and the part that drags the window; `window_control_hitboxes` in gpui's `window.rs` is cleared for each frame and filled only by a view that is drawn anew). While the title bar was part of the cached app, the buttons and the bar did nothing in every frame where only the shapes had moved, which with the drift on is nearly every frame.

The shell also takes Ctrl+wheel for `zoom`, ahead of everything under the pointer, and draws the title bar at 100% whatever the zoom. Its shapes are shares of the window and keep GPUI's own pixels.

The shapes stand still when the window is inactive and under reduced motion, so an idle or backgrounded window costs nothing. They also stand still when Direct3D runs on the processor (a virtual machine, a remote session, a broken graphics driver): there every frame of the drift is drawn by the processor, and in a clean-Windows test an idle window kept both cores of the machine busy and starved everything else.

## Source

- `crates/codewiki-app/src/backdrop.rs` — module `codewiki_app::backdrop`

## How it works

1. `main` in [[codewiki-app]] calls `Shell::new`. `Shell::new` calls `draws_on_processor`, then `debug_backdrop` in [[Glass Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const PERIOD: f32` — seconds for the shapes to come back to where they started.
- `const TICK: Duration` — time between two positions. About 24 a second: the drift is too slow to need more.
- `const MAX_STEP: f32` — the longest step one frame may take, so that a stalled frame does not show as a jump.
- `pub const STRENGTH: f32` — how strongly a shape colors the backdrop at its middle. `glass` allows for this much when it works out how opaque the chrome must be.
- `const RINGS: usize` — discs in one shape. With 18 the steps between them showed as faint rings; at 48 each step is smaller than one level of an 8-bit display.
- `fn ring_radius(index: usize) -> f32` — radius of disc `index` of a shape, as a share of the shape's radius. The first is the whole shape; they shrink quickly at first and slowly near the middle, so the color rises gently from the rim.
- `fn ring_alpha(total: f32) -> f32` — the opacity of each disc such that all of them stacked reach `total`.
- `const MICROSOFT_VENDOR: &str` — how GPUI names an adapter from Microsoft's vendor ID, 0x1414.
- `fn draws_on_processor(software_flag: bool, device: &str, vendor: &str) -> bool` — whether an adapter draws on the processor, from what GPUI reports of it. DXGI's software flag is not enough. Without a graphics driver (a virtual machine without a graphics card, some remote sessions) Windows renders with the "Microsoft Basic Render Driver", which is software but does not carry the flag. Microsoft's own adapters are known by their vendor ID.
- `struct Blob` — one shape: where it rests, how far it wanders, and how fast.
- `const BLOBS: [Blob; 4]` — the shapes sit behind the chrome — the sidebar, the top band, the details column — and stay off the middle, where the page is.
- `fn advance(phase: f32, seconds: f32) -> f32` — `phase` moved on by `seconds`, wrapped into 0 to 1.
- `pub type TitleBarFn = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>` — draws the window's title bar.
- `pub struct Shell<V: Render>` — the window's outermost view: the drifting shapes, and `content` over them.

## Depends on

- [[Zoom Module]] — uses `Indicator`, `Wheel`, `base_rem`, `feed`
- [[Doc View Module]] — uses `DocPage`, `PageSlot`, `bounds`, `read_by`
- [[Glass Module]] — uses `debug_backdrop`
- [[Themes Module]] — uses `of`
- [[Win32 Module]] — uses `wheel_scroll_lines`

## Used by

- [[codewiki-app]] — from `main`
- [[Glass Module]] — from `matched_backdrop`

## See also

- [[_index]]
- [[codewiki-app]]
