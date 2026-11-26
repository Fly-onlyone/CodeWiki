---
tags: [core, codewiki-app, module]
---

# Menu Module

> `Menu`: a short list of choices that opens beside the control that asked for it.

## What it does

`Menu` is a short list of choices that opens beside the control that asked for it, such as the theme menu, a select's list or a right-click menu. Anatomy and behaviour are MUI's. The surface is a `Paper` at elevation 8 that hangs under its anchor and stays inside the window. A click outside it with any mouse button, or Escape, closes it. The arrow keys, Home and End move a highlight that wraps at the ends, and Enter or Space choose the highlighted row.

The owner keeps the state: whether the menu is open, which row is highlighted, and the focus handle the menu takes its keys through. Tab is bound by the kit's root, which moves the focus on; the owner closes the menu when its handle loses the focus, as MUI closes a menu on Tab.

Three things MUI does are left out. Its `Grow` entrance scales the paper, which GPUI cannot do, so the menu fades and drops into place instead. Its subheaders stick to the top while their group scrolls; GPUI has no sticky position. And typing a letter does not jump to a row.

## Source

- `crates/codewiki-app/src/mui/menu.rs` — module `codewiki_app::mui::menu`

## How it works

The owner keeps the state and rebuilds a `Menu` each frame. It passes the `anchor` bounds, a focus handle, sections of `MenuItem`s, the highlighted row, and the callbacks `on_select`, `on_highlight` and `on_close`.

`AnchorBounds::track` wraps the control that opens the menu. After each paint it stores the control's bounds and calls `moved` when they change, so the menu has an anchor from the first frame.

On render, each section becomes an optional subheading and a list of `ListItemButton` rows from [[List Module]]. The highlighted row is found by counting rows through every section. A keyed `Scroll` state scrolls the highlighted row into view, but only when the highlight changed.

Keys: `step` moves the highlight with Down, Up, Home and End, and wraps at the ends. Enter or Space call `on_select` with the highlighted row. Escape calls `on_close`. A key with a modifier is ignored.

The layout: a full-window `scrim` takes the press that closes the menu. It listens for the left, right and middle buttons, so a right-click outside also closes the menu. Inside it an `anchored` element hangs the paper under the anchor (4 px gap), lines up its left edge (`Align::Start`) or right edge (`Align::End`), and keeps 16 px from the window edge. The whole thing is `deferred` so it draws on top. The paper has elevation 8 from [[Surfaces Module]] and a fade-and-drop entrance.

## Items

- `const ELEVATION: usize` — `Popover.js`: `elevation = 8`.
- `const WINDOW_MARGIN: f32` — `Popover.js`: `marginThreshold = 16`, the least distance to the window's edge.
- `const HEIGHT_INSET: f32` — `Menu.js`: `maxHeight: 'calc(100% - 96px)'`.
- `const LIST_PADDING: f32` — `List.js`: `paddingTop: 8, paddingBottom: 8`.
- `const GAP: f32` — between the anchor and the menu. MUI lays a menu over its anchor; here it hangs under it, so the control that opened it stays in view.
- `const WIDTH: f32` — MUI sizes a menu to its widest row. A row here is a flex line that truncates, so the menu has a width of its own.
- `type OnIndex = Rc<dyn Fn(usize, &mut Window, &mut App)>` — what a chosen row does: it is given the row's place in the menu.
- `pub enum Key` — A key that moves the highlight.
- `pub fn step(current: usize, count: usize, key: Key) -> usize` — the row the highlight moves to from `current` in a menu of `count` rows. `MenuList.js`: `disableListWrap = false`, so it wraps at both ends.
- `pub struct AnchorBounds` — where a control is in the window, so that a menu can open beside it. Wrap the control with `AnchorBounds::track`; its bounds are then known from the first frame it is drawn in.
- `pub struct MenuItem` — one choice.
- `pub struct MenuSection` — A group of choices, under a heading when it has one.
- `pub enum Align` — which edge of its anchor a menu lines up with.
- `pub struct Menu` — A menu that hangs under `anchor`.
- `struct Scroll` — the scroll position of a menu, and the row it last brought into view.

And 1 more private item with no comment.

## Depends on

- [[List Module]] — uses `icon`, `new`, `subheader`, `trailing`
- [[Motion Module (mui)]] — uses `MENU`, `enter`
- [[Surfaces Module]] — uses `paper`
- [[Zoom Module]] — uses `px`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `AppView`, `body`, `context_menu_view`, `settings_menu_view`
- [[Doc View Module]] — from `select_control`
- [[Select Module]] — from `menu`
- [[Suggest Module]] — from `Suggestions`

## Gotchas

- The owner must give the focus handle focus when the menu opens, or the keys do nothing.
- The owner must close the menu when the handle loses focus. This is how Tab closes it, as in MUI.
- A menu being closed is drawn with `open(false)` and a falling `shown` value. In that state it takes no clicks or keys.
- Highlight indexes count rows through all sections, not within one. Headings do not count.
- `highlighted` is clamped to the last row, so a stale index is safe. An empty menu always gives 0.
- The width is a fixed 300 unless set, since rows truncate rather than size the menu.
- Typing a letter does not jump to a row, and subheadings do not stick.

## See also

- [[_index]]
- [[codewiki-app]]
