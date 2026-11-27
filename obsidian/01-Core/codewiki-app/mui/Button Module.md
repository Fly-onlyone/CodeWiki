---
tags: [core, codewiki-app, module]
---

# Button Module

> `Button` and `IconButton`.

## What it does

The clickable controls of the app: `Button` (label, optional start icon), `IconButton` (round, icon only) and `ToggleButtonGroup` (a row of buttons, one chosen). Sizes follow MUI. The contained look, the two pixel outline and the focus ring follow TradingAgent.

A filled button rises one pixel and glows on hover. TradingAgent also shrinks it on press; GPUI cannot scale an element, so a pressed filled button darkens and sinks one pixel instead. Outlined and text buttons fill up when pressed. An `IconButton` grows and brightens its icon under the pointer, and `toggled(true)` shows it switched on, as the sidebar and details toggles do in the main view. How strong each state is comes from [[Interaction Module]].

## Source

- `crates/codewiki-app/src/mui/button.rs` — module `codewiki_app::mui::button`

## How it works

`Button::render` makes a `Base` (see [[Base Module]]) for hover, press and focus, then decides where each color is heading: gradient ends, text and border, from the kind (contained, outlined, text) and the disabled flag. Transitions move the colors there over 200 ms.

On press, `base.press` (0 to 1) acts in two ways. A contained button's gradient stops are darkened with `lift` by `press_darken`. Other kinds get `pressed`, which raises their fill toward the pressed fill. The element also moves down by `sink` times the press. A spring raises a contained button by `LIFT` while hovered and not pressed, and `glow` builds the shadow stack from the hover and press progress.

`IconButton::render` picks a fill: primary at the toggled strength (plus hover) when switched on, the icon color at hover strength when hovered. `pressed` adds the press. A hover spring grows the icon and lifts its lightness. The press sinks the icon, not the button, because GPUI can move an icon but not an element.

`ToggleButtonGroup::render` builds one `Base` per label. The chosen one is tinted with primary at the selected strength. Buttons overlap by 1 px so neighbours share a border, and a press only fills, never sinks.

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
- `pub struct Button` — A button with a label, and optionally an icon before it.
- `pub struct IconButton` — A round button that is only an icon.
- `const TOGGLE_PADDING: f32` — `ToggleButton.js`, `size: 'small'`: 7 px of padding around 13 px text.
- `type OnSelect = std::rc::Rc<dyn Fn(usize, &mut Window, &mut App)>` — what choosing the button at an index does.
- `pub struct ToggleButtonGroup` — A few buttons side by side, one of them chosen: MUI's exclusive `ToggleButtonGroup` in the primary color.

And 3 more private items with no comment.

## Depends on

- [[Base Module]] — uses `Ink`, `OnClick`, `focus_ring`, `new`
- [[Interaction Module]] — uses `current`, `lift`, `pressed`
- [[Tokens Module]] — uses `easing`, `spec`, `states`
- [[Motion Module (mui)]] — uses `SNAPPY`, `scaled`
- [[Feedback Module]] — uses `tooltip`
- [[Curve Module]] — uses `EASE_IN_OUT`
- [[Duration Module]] — uses `SHORTEST`
- [[Zoom Module]] — uses `px`

## Used by

- [[codewiki-app]] — from `diagram_viewer_view`, `edit_view`, `review_actions`, `settings_frame`
- [[Doc View Module]] — from `choices_row`, `draw`, `draw_row`, `secret_control`
- [[Snackbar Module]] — from `snackbar`

## Gotchas

- `glow` is given spring values that may overshoot. It clamps blur and alpha at zero, so a bounce never gives a negative shadow.
- The tooltip of an `IconButton` hangs on its own layer named after its text. GPUI keeps a waiting tooltip with its element, so a toggle whose text changes on press would show the old text. A new text makes a new layer.
- A tinted `IconButton` uses the tint for icon, hover fill and ripple. A disabled one stays neutral.
- Disabled buttons show no hover, lift or glow.
- Outlined buttons take the 2 px border out of their padding, so all kinds are the same height.

## See also

- [[_index]]
- [[codewiki-app]]
