---
tags: [core, codewiki-app, module]
---

# Button Module

> `Button` and `IconButton`.

## What it does

Sizes and paddings are MUI's (`Button/Button.js`, `IconButton/IconButton.js`). The look of the contained button, the two pixel outline and the focus ring are TradingAgent's (`components.ts`, `MuiButton`): a filled button rises one pixel and glows on hover, and settles when pressed. TradingAgent also shrinks it to 97% on press, which GPUI cannot do; the ripple marks the press instead.

## Source

- `crates/codewiki-app/src/mui/button.rs` — module `codewiki_app::mui::button`

## How it works

1. `Button::render` in [[Button]] calls `Metrics::of`.
2. `Button::render` in [[Button]] calls `glow`. `glow` calls `px` in [[Zoom Module]].
3. `Button::render` in [[Button]] calls `ease`. `ease` calls `scaled` in [[Motion Module (mui)]], then `easing` in [[Tokens Module]].
4. `AppView::diagram_viewer_view` and `AppView::tab_bar` in [[codewiki-app]], and 2 more, call `IconButton::new`.
5. `choices_row` in [[Doc View Module]] calls `ToggleButtonGroup::new`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const MIN_WIDTH: f32` — `Button.js`: the narrowest a button gets.
- `const ICON_GAP: f32` — `Button.js`, `ButtonStartIcon`: space between the icon and the label.
- `const OUTLINE: f32` — TradingAgent, `outlined`: `borderWidth: 2`.
- `const LIFT: f32` — TradingAgent: how far a filled button rises on hover.
- `const HOVER_TIME: Duration` — TradingAgent: `transition: … 0.2s ease`, and `0.1s` once pressed.
- `pub enum Kind` — MUI's `variant`.
- `pub enum Tone` — MUI's `color`. Primary is MUI's default.
- `pub enum Scale` — MUI's `size`.
- `pub struct Metrics` — what a size comes to for a button of one kind. `Button.js`, `variants`.
- `pub fn icon_button_metrics(scale: Scale) -> (f32, f32)` — `IconButton.js`: padding around the icon, and the icon's size.
- `struct Palette` — the colors one tone gives a button.
- `fn glow(tint: Hsla, hover: f32, press: f32) -> Vec<BoxShadow>` — the shadow of a filled button, `hover` of the way to its hovered state and `press` of the way to its pressed one. TradingAgent, `MuiButton`, `contained`.
- `fn ease(time: Duration) -> kit::Transition` — private function, returns `kit::Transition`; called by `Button::render`, `ToggleButtonGroup::render`.
- [[Button]] — A button with a label, and optionally an icon before it.
- `pub struct IconButton` — A round button that is only an icon.
- `const TOGGLE_PADDING: f32` — `ToggleButton.js`, `size: 'small'`: 7 px of padding around 13 px text.
- `type OnSelect = std::rc::Rc<dyn Fn(usize, &mut Window, &mut App)>` — what choosing the button at an index does.
- `pub struct ToggleButtonGroup` — A few buttons side by side, one of them chosen: MUI's exclusive `ToggleButtonGroup` in the primary color.

And 2 more private items with no comment.

## Depends on

- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Tokens Module]] — uses `easing`, `spec`, `states`
- [[Feedback Module]] — uses `tooltip`
- [[Motion Module (mui)]] — uses `scaled`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Duration Module]] — uses `SHORTEST`
- [[Zoom Module]] — uses `px`
- [[Glyph]] — uses `Glyph`

## Used by

- [[codewiki-app]] — from `diagram_viewer_view`, `settings_frame`, `tab_bar`, `toolbar`
- [[Button]] — from `Button`, `render`
- [[Doc View Module]] — from `choices_row`

## See also

- [[_index]]
- [[codewiki-app]]
