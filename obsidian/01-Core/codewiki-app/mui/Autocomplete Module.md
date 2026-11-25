---
tags: [core, codewiki-app, module]
---

# Autocomplete Module

> `Autocomplete`'s list: the choices that hang under a text field while it is typed in.

## What it does

This module draws the list of choices that hangs under a text field while you type in it, like Material UI's Autocomplete. The app uses it for the search box in the toolbar: [[codewiki-app]] builds a `Listbox` in `AppView::suggestions_view` from the hits of the text typed so far.

It only draws. The owner, here the app's main view, keeps the state: which choices there are, which row is highlighted, and whether the list is open. A `Listbox` is built anew in every frame from that state, as `Choice` rows under `Group` headings, and reports a chosen row by its number through `on_select`.

The sizes follow MUI's `Autocomplete.js`: a paper as wide as the field, at most 40% of the window high, 8 pixels of padding above and below the rows, and a quiet line such as "No results" when no group has a choice.

## Source

- `crates/codewiki-app/src/mui/autocomplete.rs` — module `codewiki_app::mui::autocomplete`

## How it works

`render` walks the groups in order. A group with no choices is skipped. Each group gets a heading from `subheader` in [[List Module]], then one ListItemButton for each choice, numbered across all groups. The number given to `highlighted` draws that row as if the pointer were on it: it takes the row's hover fill, shift and icon growth, so the key-driven row looks the same as a hovered one. Real hover and press use the shared strengths from [[Interaction Module]], including the press fill and recoil.

The rows sit in a scrolling column. A `Scroll` kept per list remembers which row it last brought into view, and scrolls only when the highlighted row changes, so the arrow keys can move the highlight past the visible part.

The column sits on a paper from [[Surfaces Module]] with the width of `anchor`, the field's bounds. `anchored` hangs it 4 pixels under the field's bottom left corner and keeps it 16 pixels inside the window. `deferred` draws it above the page, and it fades and drops in with the menu entrance from [[Motion Module (mui)]].

The field keeps the keyboard focus the whole time. A row's own press never moves the focus, because the row's base stops the press and prevents the default. The paper does the same for every other press on the list. A click on a row calls `on_select` with the row's number.

## Items

- `const ELEVATION: usize` — `Autocomplete.js` gives its `Paper` no elevation of its own: `Paper.js` defaults to 1.
- `const HEIGHT_SHARE: f32` — `Autocomplete.js`, listbox: `maxHeight: '40vh'`.
- `const LIST_PADDING: f32` — `Autocomplete.js`, listbox: `padding: '8px 0'`.
- `const EMPTY_PADDING: (f32, f32)` — `Autocomplete.js`, `noOptions`: `padding: '14px 16px'`.
- `const GAP: f32` — between the field and the list. MUI's popper touches the field; the app's menus hang 4 pixels under theirs, and so does this list.
- `const WINDOW_MARGIN: f32` — `Popover.js`: `marginThreshold = 16`, the least distance to the window's edge, as for the app's menus.
- `type OnIndex = Rc<dyn Fn(usize, &mut Window, &mut App)>` — what a chosen row does: it is given the row's place in the list.
- `pub struct Choice` — one choice.
- `pub struct Group` — choices under one heading.
- `pub struct Listbox` — the list under a field. Rows are counted across the groups, in order.
- `struct Scroll` — the scroll position of a list, and the row it last brought into view.

## Depends on

- [[List Module]] — uses `new`, `secondary`, `subheader`
- [[Motion Module (mui)]] — uses `MENU`, `enter`
- [[Surfaces Module]] — uses `paper`
- [[Zoom Module]] — uses `px`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `suggestions_view`

## Gotchas

- Nothing covers the window behind the list, unlike a menu, which lays a scrim to catch the click that closes it. The field must still take presses while the list is open. So the owner must close the list itself when the field loses the focus.
- A row is chosen on click, not on press. A wrapper's mouse-down handler never fires, because the row's base stops the press there.
- The row numbers that `highlighted` and `on_select` use count rows across groups, skipping empty groups. The owner must count the same way, or the highlighted row and the chosen hit differ.
- `anchor` must be the field's bounds from the last frame. The owner tracks them with `AnchorBounds` and draws no list before the field has been drawn once.

## See also

- [[_index]]
- [[codewiki-app]]
