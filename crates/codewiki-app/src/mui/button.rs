//! `Button` and `IconButton`.
//!
//! Sizes and paddings are MUI's (`Button/Button.js`, `IconButton/IconButton.js`).
//! The look of the contained button, the two pixel outline and the focus ring
//! are TradingAgent's (`components.ts`, `MuiButton`): a filled button rises
//! one pixel and glows on hover, and settles when pressed. TradingAgent also
//! shrinks it to 97% on press, which GPUI cannot do; the ripple marks the
//! press instead.

use std::time::Duration;

use gpui_kit::base::motion as kit;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, BoxShadow, ClickEvent, ElementId, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, SharedString, StatefulInteractiveElement as _, Styled as _,
    Window, div, hsla, linear_color_stop, linear_gradient, point, px,
};

use super::base::{Base, Ink, OnClick, focus_ring};
use super::feedback;
use super::motion::{SNAPPY, scaled};
use super::tokens::{self, Variant, curve, duration, easing};
use crate::icons::Glyph;

/// `Button.js`: the narrowest a button gets.
const MIN_WIDTH: f32 = 64.0;
/// `Button.js`, `ButtonStartIcon`: space between the icon and the label.
const ICON_GAP: f32 = 8.0;
/// TradingAgent, `outlined`: `borderWidth: 2`.
const OUTLINE: f32 = 2.0;
/// TradingAgent: how far a filled button rises on hover.
const LIFT: f32 = 1.0;
/// TradingAgent: `transition: … 0.2s ease`, and `0.1s` once pressed.
const HOVER_TIME: Duration = Duration::from_millis(200);
const PRESS_TIME: Duration = Duration::from_millis(100);

/// MUI's `variant`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Kind {
    /// Filled: the main action.
    Contained,
    /// A border and no fill.
    #[default]
    Outlined,
    /// Text only.
    Text,
}

/// MUI's `color`. Primary is MUI's default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    #[default]
    Primary,
    Error,
}

/// MUI's `size`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scale {
    Small,
    #[default]
    Medium,
}

/// What a size comes to for a button of one kind. `Button.js`, `variants`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// Space left and right, and above and below, between the edge of the
    /// button and its content. An outlined button's border is part of it.
    pub padding: (f32, f32),
    pub font: f32,
    pub icon: f32,
    /// How far the start icon is pulled into the left padding.
    pub icon_inset: f32,
}

impl Metrics {
    pub fn of(kind: Kind, scale: Scale) -> Self {
        // MUI gives an outlined button one pixel less padding each way, to
        // make room for its border. The sum is the same as for a filled one.
        let padding = match (kind, scale) {
            (Kind::Text, Scale::Small) => (5.0, 4.0),
            (Kind::Text, Scale::Medium) => (8.0, 6.0),
            (_, Scale::Small) => (10.0, 4.0),
            (_, Scale::Medium) => (16.0, 6.0),
        };
        let (font, icon, icon_inset) = match scale {
            Scale::Small => (13.0, 18.0, 2.0),
            Scale::Medium => (14.0, 20.0, 4.0),
        };
        Self {
            padding,
            font,
            icon,
            icon_inset,
        }
    }

    /// Height of the label's line: the button text style's line height.
    pub fn line(self) -> f32 {
        self.font * Variant::Button.spec().line_height
    }

    /// Outer height of the button.
    #[cfg(test)]
    pub fn height(self) -> f32 {
        self.line() + 2.0 * self.padding.1
    }
}

/// `IconButton.js`: padding around the icon, and the icon's size.
pub fn icon_button_metrics(scale: Scale) -> (f32, f32) {
    match scale {
        Scale::Small => (5.0, 18.0),
        Scale::Medium => (8.0, 24.0),
    }
}

/// The colors one tone gives a button.
struct Palette {
    /// Text and border of an outlined or text button.
    main: Hsla,
    /// Fill of a contained button, at rest and under the pointer, each as
    /// the two ends of its gradient.
    fill: [Hsla; 2],
    fill_hover: [Hsla; 2],
    on_fill: Hsla,
}

impl Tone {
    fn palette(self, cx: &App) -> Palette {
        let theme = cx.theme();
        match self {
            Self::Primary => Palette {
                main: theme.primary,
                fill: [theme.button_primary, theme.button_primary_active],
                fill_hover: [theme.button_primary_hover, theme.button_primary],
                on_fill: theme.button_primary_foreground,
            },
            Self::Error => Palette {
                main: theme.danger,
                fill: [theme.danger, theme.danger_active],
                fill_hover: [theme.danger_hover, theme.danger],
                on_fill: theme.danger_foreground,
            },
        }
    }
}

/// The shadow of a filled button, `hover` of the way to its hovered state
/// and `press` of the way to its pressed one. TradingAgent, `MuiButton`,
/// `contained`.
fn glow(tint: Hsla, hover: f32, press: f32) -> Vec<BoxShadow> {
    let mix = |rest: f32, hovered: f32, pressed: f32| {
        let lifted = rest + (hovered - rest) * hover;
        (lifted + (pressed - lifted) * press).max(0.0)
    };
    let shadow = |color: Hsla, y: f32, blur: f32, inset: bool| BoxShadow {
        color,
        offset: point(px(0.), px(y)),
        blur_radius: px(blur),
        spread_radius: px(0.),
        inset,
    };
    let white = |alpha: f32| hsla(0., 0., 1., alpha);
    let black = |alpha: f32| hsla(0., 0., 0., alpha);
    vec![
        // `0 2px 8px p/.25`, hovered `0 4px 16px p/.4`, pressed `0 1px 4px p/.3`.
        shadow(
            tint.opacity(mix(0.25, 0.4, 0.3)),
            mix(2.0, 4.0, 1.0),
            mix(8.0, 16.0, 4.0),
            false,
        ),
        // Hovered only: `0 0 20px p/.15`.
        shadow(
            tint.opacity(mix(0.0, 0.15, 0.0)),
            0.0,
            mix(0.0, 20.0, 0.0),
            false,
        ),
        // `inset 0 1px 0 white/.15`, hovered `.2`; gone when pressed.
        shadow(white(mix(0.15, 0.2, 0.0)), 1.0, 0.0, true),
        // Pressed only: `inset 0 2px 4px black/.2`.
        shadow(
            black(mix(0.0, 0.0, 0.2)),
            mix(0.0, 0.0, 2.0),
            mix(0.0, 0.0, 4.0),
            true,
        ),
    ]
}

fn ease(time: Duration) -> kit::Transition {
    kit::Transition::new(scaled(time)).ease(easing(curve::EASE_IN_OUT))
}

/// A button with a label, and optionally an icon before it.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: SharedString,
    icon: Option<Glyph>,
    kind: Kind,
    tone: Tone,
    scale: Scale,
    disabled: bool,
    tooltip: Option<SharedString>,
    on_click: Option<OnClick>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: SharedString::default(),
            icon: None,
            kind: Kind::default(),
            tone: Tone::default(),
            scale: Scale::default(),
            disabled: false,
            tooltip: None,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    /// MUI's `startIcon`.
    pub fn icon(mut self, icon: Glyph) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn contained(mut self) -> Self {
        self.kind = Kind::Contained;
        self
    }

    pub fn text(mut self) -> Self {
        self.kind = Kind::Text;
        self
    }

    pub fn error(mut self) -> Self {
        self.tone = Tone::Error;
        self
    }

    pub fn small(mut self) -> Self {
        self.scale = Scale::Small;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(on_click));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let base = Base::new(&self.id, self.disabled, window, cx);
        let theme = cx.theme();
        let states = tokens::states(theme.is_dark());
        let radius = theme.radius / px(1.);
        let (foreground, primary) = (theme.foreground, theme.primary);
        let palette = self.tone.palette(cx);
        let metrics = Metrics::of(self.kind, self.scale);
        let (hovered, pressed) = (base.hovered, base.pressed);
        let clear = palette.main.opacity(0.0);

        // Where each color is heading; the transitions below take it there.
        let (fill, text, border) = match (self.kind, self.disabled) {
            (Kind::Contained, true) => {
                let fill = foreground.opacity(states.disabled_background);
                ([fill, fill], foreground.opacity(states.disabled), clear)
            }
            (_, true) => (
                [clear, clear],
                foreground.opacity(states.disabled),
                foreground.opacity(states.disabled_background),
            ),
            (Kind::Contained, false) => {
                let fill = if hovered {
                    palette.fill_hover
                } else {
                    palette.fill
                };
                (fill, palette.on_fill, clear)
            }
            (kind, false) => {
                let tint = if hovered {
                    palette.main.opacity(states.hover)
                } else {
                    clear
                };
                let border = match (kind, hovered) {
                    (Kind::Text, _) => clear,
                    (_, true) => palette.main,
                    // `--variant-outlinedBorder`: the color at half strength.
                    (_, false) => palette.main.opacity(0.5),
                };
                ([tint, tint], palette.main, border)
            }
        };
        let id = &self.id;
        let from = kit::transition((id.clone(), "from"), fill[0], ease(HOVER_TIME), window, cx);
        let to = kit::transition((id.clone(), "to"), fill[1], ease(HOVER_TIME), window, cx);
        let border = kit::transition((id.clone(), "border"), border, ease(HOVER_TIME), window, cx);

        let raised = self.kind == Kind::Contained && !self.disabled;
        let lift_to = if raised && hovered && !pressed {
            1.0
        } else {
            0.0
        };
        let lift = kit::spring(
            (id.clone(), "lift"),
            lift_to,
            SNAPPY.policy(0.01),
            window,
            cx,
        );
        let press_to = if raised && pressed { 1.0 } else { 0.0 };
        let press = kit::transition(
            (id.clone(), "press"),
            press_to,
            ease(PRESS_TIME),
            window,
            cx,
        );

        let mut shadows = if raised {
            glow(palette.main, lift, press)
        } else {
            Vec::new()
        };
        if base.focused {
            shadows.push(focus_ring(primary));
        }

        // The start icon is pulled into the padding, and a border takes its
        // width out of it.
        let inset = if self.icon.is_some() {
            metrics.icon_inset
        } else {
            0.0
        };
        let edge = if self.kind == Kind::Outlined {
            OUTLINE
        } else {
            0.0
        };
        let element = h_flex()
            .id(self.id.clone())
            .relative()
            .top(px(-LIFT * lift))
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .min_w(px(MIN_WIDTH))
            .pl(px(metrics.padding.0 - edge - inset))
            .pr(px(metrics.padding.0 - edge))
            .py(px(metrics.padding.1 - edge))
            .gap(px(ICON_GAP))
            .rounded(px(radius))
            .bg(linear_gradient(
                135.,
                linear_color_stop(from, 0.),
                linear_color_stop(to, 1.),
            ))
            .when(self.kind == Kind::Outlined, |button| {
                button.border(px(OUTLINE)).border_color(border)
            })
            .shadow(shadows)
            .text_color(text)
            .text_size(px(metrics.font))
            .line_height(px(metrics.line()))
            .font_weight(Variant::Button.spec().weight)
            .whitespace_nowrap()
            .when_some(self.icon, |button, icon| {
                button.child(Icon::new(icon).with_size(px(metrics.icon)).text_color(text))
            })
            .child(self.label)
            .when_some(self.tooltip, |button, text| {
                button.tooltip(feedback::tooltip(text))
            });
        let ink = Ink {
            radius,
            centered: false,
            color: text,
        };
        base.attach(element, ink, self.disabled, self.on_click, cx)
    }
}

/// A round button that is only an icon.
#[derive(IntoElement)]
pub struct IconButton {
    id: ElementId,
    icon: Glyph,
    scale: Scale,
    disabled: bool,
    tooltip: Option<SharedString>,
    on_click: Option<OnClick>,
}

impl IconButton {
    pub fn new(id: impl Into<ElementId>, icon: Glyph) -> Self {
        Self {
            id: id.into(),
            icon,
            scale: Scale::default(),
            disabled: false,
            tooltip: None,
            on_click: None,
        }
    }

    pub fn small(mut self) -> Self {
        self.scale = Scale::Small;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(on_click));
        self
    }
}

impl RenderOnce for IconButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let base = Base::new(&self.id, self.disabled, window, cx);
        let theme = cx.theme();
        let states = tokens::states(theme.is_dark());
        // `palette.action.active`: white on dark, black at 54% on light.
        let active = if theme.is_dark() {
            theme.foreground
        } else {
            theme.foreground.opacity(0.54)
        };
        let primary = theme.primary;
        let color = if self.disabled {
            theme.foreground.opacity(states.disabled)
        } else {
            active
        };
        let (padding, icon) = icon_button_metrics(self.scale);
        let side = icon + 2.0 * padding;

        let fill_to = if base.hovered {
            active.opacity(active.a * states.hover)
        } else {
            active.opacity(0.0)
        };
        let policy =
            kit::Transition::new(scaled(duration::SHORTEST)).ease(easing(curve::EASE_IN_OUT));
        let fill = kit::transition((self.id.clone(), "fill"), fill_to, policy, window, cx);

        let element = div()
            .id(self.id.clone())
            .relative()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size(px(side))
            .rounded_full()
            .bg(fill)
            .when(base.focused, |button| {
                button.shadow(vec![focus_ring(primary)])
            })
            .child(Icon::new(self.icon).with_size(px(icon)).text_color(color))
            .when_some(self.tooltip, |button, text| {
                button.tooltip(feedback::tooltip(text))
            });
        let ink = Ink {
            radius: side / 2.0,
            centered: true,
            color,
        };
        base.attach(element, ink, self.disabled, self.on_click, cx)
    }
}

/// `ToggleButton.js`, `size: 'small'`: 7 px of padding around 13 px text.
const TOGGLE_PADDING: f32 = 7.0;
const TOGGLE_FONT: f32 = 13.0;

/// What choosing the button at an index does.
type OnSelect = std::rc::Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A few buttons side by side, one of them chosen: MUI's exclusive
/// `ToggleButtonGroup` in the primary color.
#[derive(IntoElement)]
pub struct ToggleButtonGroup {
    id: SharedString,
    labels: Vec<SharedString>,
    selected: Option<usize>,
    on_select: Option<OnSelect>,
}

impl ToggleButtonGroup {
    pub fn new(id: impl Into<SharedString>, labels: Vec<SharedString>) -> Self {
        Self {
            id: id.into(),
            labels,
            selected: None,
            on_select: None,
        }
    }

    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    pub fn on_select(mut self, on_select: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(std::rc::Rc::new(on_select));
        self
    }
}

impl RenderOnce for ToggleButtonGroup {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let states = tokens::states(theme.is_dark());
        let (primary, muted, foreground, line) = (
            theme.primary,
            theme.muted_foreground,
            theme.foreground,
            theme.border,
        );
        let radius = theme.radius;
        let last = self.labels.len().saturating_sub(1);
        let mut group = h_flex().flex_shrink_0();
        for (index, label) in self.labels.into_iter().enumerate() {
            let id = ElementId::NamedInteger(self.id.clone(), index as u64);
            let base = Base::new(&id, false, window, cx);
            let chosen = self.selected == Some(index);
            // `ToggleButton.js`: the chosen button is tinted with its color
            // at the selected opacity, and more under the pointer.
            let fill_to = match (chosen, base.hovered) {
                (true, true) => primary.opacity(states.selected + states.hover),
                (true, false) => primary.opacity(states.selected),
                (false, true) => foreground.opacity(states.hover),
                (false, false) => foreground.opacity(0.0),
            };
            let fill = kit::transition((id.clone(), "fill"), fill_to, ease(HOVER_TIME), window, cx);
            let color = if chosen { primary } else { muted };
            let on_click = self.on_select.clone().map(|on_select| {
                std::rc::Rc::new(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                    on_select(index, window, cx)
                }) as OnClick
            });
            let element = h_flex()
                .id(id)
                .relative()
                .p(px(TOGGLE_PADDING))
                .px(px(TOGGLE_PADDING + 4.0))
                .items_center()
                .justify_center()
                .border_1()
                .border_color(line)
                // Neighbours share one line between them.
                .when(index > 0, |button| button.ml(px(-1.)))
                .when(index == 0, |button| button.rounded_l(radius))
                .when(index == last, |button| button.rounded_r(radius))
                .bg(fill)
                .text_color(color)
                .text_size(px(TOGGLE_FONT))
                .line_height(px(TOGGLE_FONT * Variant::Button.spec().line_height))
                .font_weight(Variant::Button.spec().weight)
                .whitespace_nowrap()
                .when(base.focused, |button| {
                    button.shadow(vec![focus_ring(primary)])
                })
                .child(label);
            let ink = Ink {
                radius: 0.0,
                centered: false,
                color,
            };
            group = group.child(base.attach(element, ink, false, on_click, cx));
        }
        group
    }
}

/// TradingAgent, `createCardHover`: a card that can be pressed rises under
/// the pointer and its shadow deepens. There it rises 8 px and grows 3%;
/// GPUI cannot scale it, so here it only rises, by half as much.
const CARD_LIFT: f32 = 4.0;

/// The shadow of a pressable card, `hover` of the way to its hovered state.
/// `createCardHover`: `0 4px 20px black/.25` at rest; hovered
/// `0 20px 40px black/.35, 0 0 20px primary/.15`.
fn card_glow(tint: Hsla, hover: f32) -> Vec<BoxShadow> {
    let mix = |rest: f32, hovered: f32| (rest + (hovered - rest) * hover).max(0.0);
    vec![
        BoxShadow {
            color: hsla(0., 0., 0., mix(0.25, 0.35)),
            offset: point(px(0.), px(mix(4.0, 20.0))),
            blur_radius: px(mix(20.0, 40.0)),
            spread_radius: px(0.),
            inset: false,
        },
        BoxShadow {
            color: tint.opacity(mix(0.0, 0.15)),
            offset: point(px(0.), px(0.)),
            blur_radius: px(mix(0.0, 20.0)),
            spread_radius: px(0.),
            inset: false,
        },
    ]
}

/// A card that can be pressed, such as one of a set to choose from.
#[derive(IntoElement)]
pub struct CardButton {
    id: ElementId,
    selected: bool,
    on_click: Option<OnClick>,
    children: Vec<gpui_kit::AnyElement>,
}

impl CardButton {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            selected: false,
            on_click: None,
            children: Vec::new(),
        }
    }

    /// Mark the card as the chosen one of its set.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(on_click));
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl RenderOnce for CardButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let base = Base::new(&self.id, false, window, cx);
        let theme = cx.theme();
        let (primary, radius) = (theme.primary, theme.radius_lg);
        let hover_to = if base.hovered && !base.pressed {
            1.0
        } else {
            0.0
        };
        let hover = kit::spring(
            (self.id.clone(), "hover"),
            hover_to,
            SNAPPY.policy(0.01),
            window,
            cx,
        );
        let border_to = if self.selected {
            primary
        } else if base.hovered {
            primary.opacity(0.35)
        } else {
            primary.opacity(0.15)
        };
        let border = kit::transition(
            (self.id.clone(), "border"),
            border_to,
            ease(HOVER_TIME),
            window,
            cx,
        );
        let mut shadows = card_glow(primary, hover);
        if base.focused {
            shadows.push(focus_ring(primary));
        }
        let element = div()
            .id(self.id.clone())
            .relative()
            .top(px(-CARD_LIFT * hover))
            .w_full()
            .p(px(super::surfaces::CARD_PADDING))
            .rounded(radius)
            .border_1()
            .border_color(border)
            .bg(super::surfaces::card_fill(cx))
            .shadow(shadows)
            .children(self.children);
        let ink = Ink {
            radius: radius / px(1.),
            centered: false,
            color: primary,
        };
        base.attach(element, ink, false, self.on_click, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_are_as_tall_as_mui_makes_them() {
        // 14 px text at a line height of 1.75, plus 6 px above and below.
        assert_eq!(Metrics::of(Kind::Contained, Scale::Medium).height(), 36.5);
        assert_eq!(Metrics::of(Kind::Contained, Scale::Small).height(), 30.75);
        // The kind changes the width of a button, never its height.
        for scale in [Scale::Small, Scale::Medium] {
            let heights = [Kind::Contained, Kind::Outlined, Kind::Text]
                .map(|kind| Metrics::of(kind, scale).height());
            assert!(
                heights.windows(2).all(|pair| pair[0] == pair[1]),
                "{scale:?}"
            );
        }
    }

    #[test]
    fn paddings_are_the_ones_in_button_js() {
        assert_eq!(
            Metrics::of(Kind::Contained, Scale::Medium).padding,
            (16.0, 6.0)
        );
        assert_eq!(
            Metrics::of(Kind::Outlined, Scale::Medium).padding,
            (16.0, 6.0)
        );
        assert_eq!(Metrics::of(Kind::Text, Scale::Medium).padding, (8.0, 6.0));
        assert_eq!(
            Metrics::of(Kind::Contained, Scale::Small).padding,
            (10.0, 4.0)
        );
        assert_eq!(Metrics::of(Kind::Text, Scale::Small).padding, (5.0, 4.0));
    }

    #[test]
    fn icons_grow_with_the_button() {
        let icons = [Scale::Small, Scale::Medium].map(|scale| Metrics::of(Kind::Text, scale).icon);
        assert_eq!(icons, [18.0, 20.0]);
        let sides = [Scale::Small, Scale::Medium].map(|scale| {
            let (padding, icon) = icon_button_metrics(scale);
            icon + 2.0 * padding
        });
        assert_eq!(sides, [28.0, 40.0]);
    }

    #[test]
    fn a_filled_button_glows_more_when_hovered_and_less_when_pressed() {
        let tint = hsla(0.7, 0.8, 0.6, 1.0);
        let reach = |shadows: &[BoxShadow]| shadows[0].blur_radius / px(1.);
        let (rest, hovered, pressed) = (
            glow(tint, 0.0, 0.0),
            glow(tint, 1.0, 0.0),
            glow(tint, 0.0, 1.0),
        );
        assert_eq!(
            (reach(&rest), reach(&hovered), reach(&pressed)),
            (8.0, 16.0, 4.0)
        );
        assert_eq!(rest[0].color.a, 0.25);
        assert_eq!(hovered[0].color.a, 0.4);
        // The wide halo is there on hover only.
        assert_eq!(rest[1].color.a, 0.0);
        assert!(hovered[1].color.a > 0.0);
        // Pressed, the top highlight gives way to an inner shadow.
        assert_eq!(pressed[2].color.a, 0.0);
        assert!(pressed[3].inset && pressed[3].color.a > 0.0);
        assert_eq!(rest[3].color.a, 0.0);
        // A spring that overshoots never asks for a negative blur.
        assert!(
            glow(tint, -0.2, 0.0)
                .iter()
                .all(|shadow| shadow.blur_radius >= px(0.) && shadow.color.a >= 0.0)
        );
    }
}
