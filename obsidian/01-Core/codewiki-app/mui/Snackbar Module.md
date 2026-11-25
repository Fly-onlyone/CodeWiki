---
tags: [core, codewiki-app, module]
---

# Snackbar Module

> `Snackbar`: a short message in the window's corner.

## What it does

Sizes are MUI's (`Snackbar/Snackbar.js`, `SnackbarContent/SnackbarContent.js`). MUI's snackbar carries a message and an action. This one also has a header, as a Windows notification does: the app's logo and name, and a close button. Before the message is the icon of what it is about, in the tone's color, where an alert (`feedback::alert`) has its tone icon.

## Source

- `crates/codewiki-app/src/mui/snackbar.rs` — module `codewiki_app::mui::snackbar`

## How it works

1. `AppView::toasts_view` in [[codewiki-app]] calls `snackbar`. `snackbar` calls `px` in [[Zoom Module]], then `Button::new` in [[Button Module]], then `text` in [[Surfaces Module]], then `IconButton::new` in [[Button Module]], then `paper` in [[Surfaces Module]].
2. `AppView::toasts_view` in [[codewiki-app]] calls `stack`. `stack` calls `px` in [[Zoom Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const OFFSET: f32` — `Snackbar.js`: `bottom: 24` and `right: 24` from the `sm` breakpoint up.
- `pub const MIN_WIDTH: f32` — `SnackbarContent.js`: `minWidth: 288` from the `sm` breakpoint up.
- `pub const MAX_WIDTH: f32` — MUI sets no largest width. This is the app's own, so that a long line wraps before it fills the window.
- `pub const GAP: f32` — the space between two toasts: `theme.spacing(1)`.
- `const ELEVATION: usize` — `SnackbarContent.js`: `elevation: 6`.
- `const PADDING: (f32, f32)` — `SnackbarContent.js`: `padding: '6px 16px'`.
- `const MESSAGE_PADDING: f32` — `SnackbarContent.js`, `SnackbarContentMessage`: `padding: '8px 0'`.
- `const ACTION_PADDING: f32` — `SnackbarContent.js`, `SnackbarContentAction`: `paddingLeft: 16`, `marginRight: -8`.
- `const ICON: f32` — the icon before the message, at the size of the tone icon in `feedback::alert`, and its distance from the message.
- `const BODY_FONT: f32` — the line under the title is a size smaller than the title.
- `const LOGO: f32` — the app's own: the logo in the header, as large as in the title bar, and its distance from the app's name.
- `const APP_NAME: &str` — the name in the header: the one Windows shows above the app's notifications, "CodeWiki Dev" for a development build.
- `pub struct Content` — what a toast shows and does.
- `pub fn snackbar(id: impl Into<ElementId>, content: Content, cx: &App) -> Stateful<Div>` — one snackbar. `id` tells it apart from the others, for its buttons.
- `pub fn stack(toasts: impl IntoIterator<Item = AnyElement>) -> Div` — the toasts in the window's corner, oldest first: the newest sits nearest the corner. The stack takes no room and no clicks; each toast takes its own.

And 3 more private items with no comment.

## Depends on

- [[Surfaces Module]] — uses `paper`, `text`
- [[cw-core]] — uses `APP_NAME`
- [[Icons Module]] — uses `LOGO`
- [[Base Module]] — uses `OnClick`
- [[Button Module]] — uses `new`
- [[Zoom Module]] — uses `px`
- [[Glyph]] — uses `Glyph`
- [[Tone]] — uses `Tone`

## Used by

- [[codewiki-app]] — from `toasts_view`

## See also

- [[_index]]
- [[codewiki-app]]
