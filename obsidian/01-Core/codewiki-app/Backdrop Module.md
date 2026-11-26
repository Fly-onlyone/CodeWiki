---
tags: [core, codewiki-app, module]
---

# Backdrop Module

> The drifting backdrop: soft color under the glass.

## What it does

A few large soft shapes in the theme's colors sit behind every surface and drift slowly. They are what the glass has to show when the wallpaper behind the window is plain.

gpui-kit has no radial gradient, and its box shadow spreads a blur this wide so thin that the color all but vanishes. A shape is therefore a stack of faint discs, each smaller than the last, which adds up to a color that is strongest in the middle and fades to nothing at the rim.

Over the color lies a sky of notes and the links between them (`constellation.rs`), which says what the app is about. It is drawn only in the clear margins beside the reading column (`Margins`): there the page's glass is at its thinnest and carries no text, so the sky can be strong without making anything harder to read. None of it is behind the chrome, whose glass `glass` fits to `STRENGTH` of the shapes alone, nor behind the column, where moving light would pull the eye off the text. Light runs along the links: now and then by itself, and often while a wiki is being written (`set_writing`). What is near the pointer brightens.

Settings → Appearance turns the whole backdrop off (`set_shown`): plain Mica behind the glass, and a page as dense as its column.

`Shell` is the window's outermost view. It draws the shapes and, on top, the app and the generated page, each as a cached view. Only the shell is redrawn when the shapes move; the app and the page keep their last frames until they change themselves, and neither is redrawn when the other is.

The title bar is the exception: the shell draws it itself, in every frame. GPUI keeps the hit areas of a cached view but not its window control areas (the minimize, maximize and close buttons and the part that drags the window; `window_control_hitboxes` in gpui's `window.rs` is cleared for each frame and filled only by a view that is drawn anew). While the title bar was part of the cached app, the buttons and the bar did nothing in every frame where only the shapes had moved, which with the drift on is nearly every frame.

The shell also takes Ctrl+wheel for `zoom`, ahead of everything under the pointer, and draws the title bar at 100% whatever the zoom. Its shapes are shares of the window and keep GPUI's own pixels.

The shapes stand still when the window is inactive and under reduced motion, so an idle or backgrounded window costs nothing; so do the sky, its pulses and the pointer's light. They also stand still when Direct3D runs on the processor (a virtual machine, a remote session, a broken graphics driver): there every frame of the drift is drawn by the processor, and in a clean-Windows test an idle window kept both cores of the machine busy and starved everything else.

## Source

- `crates/codewiki-app/src/backdrop.rs` — module `codewiki_app::backdrop`

## How it works

Setup: `main` calls `init`, which reads the "backdrop" setting from [[Prefs Module]] into a global `Shown`. Only the value "off" turns it off. `set_shown` writes the setting and the global. `main` builds the `Shell`, which makes the sky and its traffic from [[Constellation Module]].

Each frame, `Shell` takes a step of at most `MAX_STEP` seconds (`step`), moves the clock, and draws:

1. Four `Blob`s, each a stack of 48 faint discs (`RINGS`). `ring_radius` and `ring_alpha` make their sum reach the shape's share of `STRENGTH` at the middle and nothing at the rim.
2. The sky, only in the two `Margins` the app sets beside the reading column. Links, nodes, pulses and the pointer's light are painted there.
3. The app and the generated page, each as a cached view, on top.
4. The title bar, redrawn in every frame by the shell itself.

`set_writing` tells the shell whether a wiki is being written. While it is, pulses come often. The shell hears of a change only when the value changes.

The drift stops when the window is inactive, under reduced motion, and when `draws_on_processor` says Direct3D runs on the processor. That check goes by the software flag, by Microsoft's vendor ID, or by the name "Microsoft Basic Render".

## Items

- `const MAX_STEP: f32` — the longest step one frame may take, so that a stalled frame does not show as a jump.
- `pub const STRENGTH: f32` — how strongly a shape colors the backdrop at its middle. `glass` allows for this much when it works out how opaque the chrome must be.
- `const RINGS: usize` — discs in one shape. With 18 the steps between them showed as faint rings; at 48 each step is smaller than one level of an 8-bit display.
- `const DISC_CORNERS: usize` — corners of the polygon that draws a disc.
- `struct Shown` — whether the backdrop is drawn, as the setting says. A global, so that the shell and the app read it in every frame without a file.
- `pub fn init(cx: &mut App)` — read the setting. Call once at start.
- `pub fn shown(cx: &App) -> bool` — whether the backdrop is drawn: on until the user turns it off.
- `pub fn set_shown(on: bool, cx: &mut App)` — turn the backdrop on or off, and remember it.
- `struct Writing` — whether a wiki is being written: the app's own sync, or an update the backend runs for anybody, a coding agent included.
- `fn writing(cx: &App) -> bool` — private function, returns `bool`; called by `Shell::drift`, `Shell::new`, `set_writing`.
- `pub fn set_writing(on: bool, cx: &mut App)` — say whether a wiki is being written. Cheap to call often: the shell hears of it only when it changes.
- `pub struct Margins` — the clear margins on the left and right of the reading column, in window coordinates: where the sky is drawn. The app, which lays the page out, writes them; the shell reads them.
- `const MICROSOFT_VENDOR: &str` — how GPUI names an adapter from Microsoft's vendor ID, 0x1414.
- `fn draws_on_processor(software_flag: bool, device: &str, vendor: &str) -> bool` — whether an adapter draws on the processor, from what GPUI reports of it. DXGI's software flag is not enough. Without a graphics driver (a virtual machine without a graphics card, some remote sessions) Windows renders with the "Microsoft Basic Render Driver", which is software but does not carry the flag. Microsoft's own adapters are known by their vendor ID.
- `struct Blob` — one shape: where it rests, how far it wanders, and how fast.
- `fn step(seconds: f32) -> f32` — the time one frame moves things on by: `seconds`, never backwards and never more than `MAX_STEP`.
- `pub type TitleBarFn = Box<dyn Fn(&mut Window, &mut App) -> AnyElement>` — draws the window's title bar.
- `pub struct Shell<V: Render>` — the window's outermost view: the drifting shapes, and `content` over them.
- `type At = (f32, f32)` — A point in the screen's pixels, from the window's top left corner.
- `struct Sky` — the sky of one frame, in the screen's pixels, ready to paint.

And 25 more items.

## Depends on

- [[Constellation Module]] — uses `Constellation`, `Pulse`, `Traffic`, `advance`
- [[Zoom Module]] — uses `Indicator`, `Wheel`, `base_rem`, `feed`
- [[Doc View Module]] — uses `DocPage`, `PageSlot`, `bounds`, `notify_after_draw`
- [[Prefs Module]] — uses `read`, `write`
- [[Glass Module]] — uses `debug_backdrop`
- [[Themes Module]] — uses `of`
- [[Win32 Module]] — uses `wheel_scroll_lines`

## Used by

- [[codewiki-app]] — from `AppView`, `main`, `margin_width`, `run_action_with`
- [[Glass Module]] — from `matched_backdrop`

## Gotchas

- The title bar must stay out of the cached app view. GPUI keeps the hit areas of a cached view but clears the window control areas each frame, so the buttons and the drag area died in every frame where only the shapes moved.
- `STRENGTH` is asserted at most 0.3 at compile time. `glass` widens what it expects behind the chrome by this amount, and a stronger backdrop would force the chrome nearly opaque.
- 48 rings is deliberate. With 18 the steps showed as faint rings.
- The sky's alphas and widths were chosen by eye on 2026-11-24. The margins carry no text, so no contrast limit applies to them.
- Ctrl+wheel is taken by the shell before anything under the pointer, for zoom. The title bar is drawn at 100 percent whatever the zoom.
- With software drawing, an idle window once kept both cores of a clean-Windows test machine busy, which is why the drift stands still there.
- With the backdrop off, the page is as dense as its column and plain Mica is behind the glass.

## See also

- [[_index]]
- [[codewiki-app]]
