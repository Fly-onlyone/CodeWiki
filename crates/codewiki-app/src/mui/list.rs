//! `List`: rows that can be pressed, a heading over a group of them, and a
//! folder that opens and closes.
//!
//! Row anatomy is MUI's `ListItemButton` in a dense list
//! (`ListItemButton/ListItemButton.js`, `ListSubheader/ListSubheader.js`).
//! How a row answers the pointer follows TradingAgent's navigation
//! (`components/layout/Sidebar.tsx`), at the strengths in `interaction`: it
//! shifts to the right, its icon grows and takes the primary color, and the
//! selected row carries a glowing bar on its left edge, which grows and
//! slides in. The bar is in the theme's tertiary color, so that it stands
//! out from the row's primary fill; the selected label takes an accent. A
//! row may instead give its icon a hue of its own, which it keeps in every
//! state, so that kinds of row stay apart: it only brightens, and glows
//! while selected. A pressed row fills up and gives back part of its shift.

use gpui_kit::base::motion::{self as kit, MotionReveal};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, BoxShadow, ClickEvent, Div, ElementId, FontWeight, Hsla,
    InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce, SharedString,
    Styled as _, Transformation, Window, div, point, radians, size,
};

use super::base::{Base, Ink, OnClick, focus_ring};
use super::interaction::{self, Interaction, lift, pressed};
use super::motion::{SNAPPY, scaled};
use super::tokens::{curve, duration, easing};
use crate::icons::Glyph;
use crate::themes;
use crate::zoom::px;

/// A dense `ListItemButton`: 4 px above and below a 14 px label with MUI's
/// 4 px text margins.
pub const ROW_HEIGHT: f32 = 36.0;
/// A dense row with MUI's `secondary` text under its label, at 12 px.
const TWO_LINE_HEIGHT: f32 = 52.0;
const SECONDARY_FONT: f32 = 12.0;
/// `ListItemButton.js`: `paddingLeft: 16`. Half of it is the list's own
/// inset here, so the row's fill stops short of the edges.
const INSET: f32 = 8.0;
const PADDING: f32 = 8.0;
/// Each level of nesting; MUI's examples indent by `pl: 4`.
const INDENT: f32 = 16.0;
/// `Sidebar.tsx`: `fontSize: "0.875rem"`, a small icon.
const FONT: f32 = 14.0;
const ICON: f32 = 20.0;
const ICON_GAP: f32 = 12.0;
/// `Sidebar.tsx`: the bar of the selected row is 24 px high. Its width,
/// and how far a row shifts and its icon grows, are in `interaction`.
const BAR_HEIGHT: f32 = 24.0;
/// How far beyond its own width the bar starts when it slides in.
const BAR_TRAVEL: f32 = 4.0;
/// `ListSubheader.js`: `lineHeight: '48px'`, 14 px text.
const SUBHEADER_HEIGHT: f32 = 48.0;

/// How strongly the primary color fills a row.
fn fill(primary: Hsla, selected: bool, hovered: bool, look: &Interaction) -> Hsla {
    primary.opacity(match (selected, hovered) {
        (true, true) => look.row_selected_hover,
        (true, false) => look.row_selected,
        (false, true) => look.row_hover,
        (false, false) => 0.0,
    })
}

/// `Sidebar.tsx`: the glow of the selected row's bar, in the bar's color,
/// at `strength` times TradingAgent's.
fn bar_glow(color: Hsla, strength: f32) -> Vec<BoxShadow> {
    [(16.0, 0.7), (6.0, 0.5), (30.0, 0.2)]
        .into_iter()
        .map(|(blur, opacity)| BoxShadow {
            color: color.opacity(opacity * strength),
            offset: point(px(0.), px(0.)),
            blur_radius: px(blur),
            spread_radius: px(0.),
            inset: false,
        })
        .collect()
}

/// The glow around a selected row's tinted icon.
fn icon_glow(tint: Hsla, opacity: f32) -> BoxShadow {
    BoxShadow {
        color: tint.opacity(opacity),
        offset: point(px(0.), px(0.)),
        blur_radius: px(ICON / 2.0),
        spread_radius: px(0.),
        inset: false,
    }
}

/// The color of a row's icon. One with a hue of its own keeps it, made
/// `lifted` lighter. Else it is muted at rest, primary under the pointer,
/// and the selected label's accent while selected.
fn icon_color(
    tint: Option<Hsla>,
    (primary, muted, accent): (Hsla, Hsla, Hsla),
    (selected, active): (bool, bool),
    lifted: f32,
) -> Hsla {
    match tint {
        Some(tint) => lift(tint, lifted),
        None if selected => accent,
        None if active => primary,
        None => muted,
    }
}

/// Where the bar of a selected row is, `shown` of the way in: its left edge
/// and its height, centred on the row. It grows from its middle as it
/// slides in from the left.
fn bar_place(width: f32, shown: f32) -> (f32, f32) {
    let left = -(width + BAR_TRAVEL) * (1.0 - shown);
    (left, BAR_HEIGHT * shown.clamp(0.0, 1.0))
}

/// The heading over a group of rows, in the theme's secondary color, which
/// is its link color.
pub fn subheader(label: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .px(px(INSET + PADDING))
        .h(px(SUBHEADER_HEIGHT))
        .flex()
        .items_center()
        .text_size(px(FONT))
        .font_weight(FontWeight::MEDIUM)
        .text_color(themes::brand(cx).secondary)
        .child(label.into())
}

/// A row that can be pressed.
#[derive(IntoElement)]
pub struct ListItemButton {
    id: ElementId,
    label: SharedString,
    icon: Option<Glyph>,
    tint: Option<Hsla>,
    selected: bool,
    highlighted: bool,
    level: usize,
    code: bool,
    secondary: Option<SharedString>,
    trailing: Option<AnyElement>,
    on_click: Option<OnClick>,
}

impl ListItemButton {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            tint: None,
            selected: false,
            highlighted: false,
            level: 0,
            code: false,
            secondary: None,
            trailing: None,
            on_click: None,
        }
    }

    pub fn icon(mut self, icon: Glyph) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Draw the icon in this color at rest, under the pointer and when
    /// selected, in place of the muted color that turns primary. The row's
    /// fill, bar and ripple stay primary.
    pub fn tint(mut self, tint: Option<Hsla>) -> Self {
        self.tint = tint;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Draw the row as if the pointer were over it: the row the keys act on
    /// in a menu, MUI's `focusVisible`.
    pub fn highlighted(mut self, highlighted: bool) -> Self {
        self.highlighted = highlighted;
        self
    }

    /// How deeply the row is nested: 0 for a top row.
    pub fn level(mut self, level: usize) -> Self {
        self.level = level;
        self
    }

    /// Set the label in the code font.
    pub fn code(mut self, code: bool) -> Self {
        self.code = code;
        self
    }

    /// A second, quieter line under the label: MUI's `secondary` text. The
    /// row grows to hold it.
    pub fn secondary(mut self, text: impl Into<SharedString>) -> Self {
        self.secondary = Some(text.into());
        self
    }

    /// Something at the far end of the row, such as a folder's arrow.
    pub fn trailing(mut self, trailing: impl IntoElement) -> Self {
        self.trailing = Some(trailing.into_any_element());
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

impl RenderOnce for ListItemButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let base = Base::new(&self.id, false, window, cx);
        let look = interaction::current(cx);
        let theme = cx.theme();
        let (primary, foreground, muted) =
            (theme.primary, theme.foreground, theme.muted_foreground);
        let radius = theme.radius / px(1.);
        let bar = themes::Brand::of(theme).tertiary;
        let accent = lift(look.selected_label.color(theme), look.label_lift);
        let (selected, hovered) = (self.selected, base.hovered || self.highlighted);
        let mono = theme.mono_font_family.clone();
        let height = if self.secondary.is_some() {
            TWO_LINE_HEIGHT
        } else {
            ROW_HEIGHT
        };

        let policy =
            kit::Transition::new(scaled(duration::SHORTEST)).ease(easing(curve::EASE_IN_OUT));
        let fill = kit::transition(
            (self.id.clone(), "fill"),
            fill(primary, selected, hovered, &look),
            policy,
            window,
            cx,
        );
        let fill = pressed(fill, look.row_pressed, base.press);
        // Pressed, a shifted row gives back part of its shift.
        let recoil = look.row_recoil / look.row_shift * base.press;
        let hover_to = if hovered { 1.0 - recoil } else { 0.0 };
        let hover = kit::spring(
            (self.id.clone(), "hover"),
            hover_to,
            SNAPPY.policy(0.01),
            window,
            cx,
        );
        let selected_to = if selected { 1.0 } else { 0.0 };
        // How far the bar is in. The spring starts at its target, so a row
        // that appears selected shows its bar at once.
        let shown = kit::spring(
            (self.id.clone(), "bar"),
            selected_to,
            look.bar_spring.policy(0.001),
            window,
            cx,
        );
        let lifted = look.icon_lift * hover.max(selected_to).clamp(0.0, 1.0);
        let icon_color = icon_color(
            self.tint,
            (primary, muted, accent),
            (selected, selected || hovered),
            lifted,
        );
        let glow = self
            .tint
            .filter(|_| selected)
            .map(|tint| icon_glow(tint, look.icon_glow));
        let grown = 1.0 + look.icon_growth * hover;
        let (bar_left, bar_height) = bar_place(look.bar_width, shown);
        let label_color = selected.then_some(accent);

        let element = h_flex()
            .id(self.id.clone())
            .relative()
            .left(px(look.row_shift * hover))
            .mx(px(INSET))
            .h(px(height))
            .pl(px(PADDING + INDENT * self.level as f32))
            .pr(px(PADDING))
            .gap(px(ICON_GAP))
            .items_center()
            .rounded(px(radius))
            .bg(fill)
            .text_size(px(FONT))
            .text_color(foreground)
            .font_weight(if selected {
                FontWeight::SEMIBOLD
            } else {
                FontWeight::NORMAL
            })
            .when(base.focused, |row| row.shadow(vec![focus_ring(primary)]))
            .when(shown > 0.001, |row| {
                row.child(
                    div()
                        .absolute()
                        .left(px(bar_left))
                        .top(px((height - bar_height) / 2.0))
                        .w(px(look.bar_width))
                        .h(px(bar_height))
                        .rounded_r(px(4.))
                        .bg(bar)
                        .shadow(bar_glow(bar, look.bar_glow * shown.clamp(0.0, 1.0))),
                )
            })
            .when_some(self.icon, |row, icon| {
                let icon = Icon::new(icon)
                    .with_size(px(ICON))
                    .text_color(icon_color)
                    .transform(Transformation::scale(size(grown, grown)));
                match glow {
                    // The glow is the shadow of a clear disc behind the icon.
                    Some(glow) => row.child(
                        div()
                            .flex_shrink_0()
                            .size(px(ICON))
                            .rounded_full()
                            .shadow(vec![glow])
                            .child(icon),
                    ),
                    None => row.child(icon),
                }
            })
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .truncate()
                            .when(self.code, |label| label.font_family(mono))
                            .when_some(label_color, |label, color| label.text_color(color))
                            .child(self.label),
                    )
                    .when_some(self.secondary, |text, secondary| {
                        text.child(
                            div()
                                .truncate()
                                .text_size(px(SECONDARY_FONT))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(muted)
                                .child(secondary),
                        )
                    }),
            )
            .children(self.trailing);
        let ink = Ink {
            radius,
            centered: false,
            color: primary,
        };
        base.attach(element, ink, false, self.on_click, cx)
    }
}

/// A row that opens and closes the rows under it.
#[derive(IntoElement)]
pub struct ListFolder {
    id: SharedString,
    label: SharedString,
    icon: Glyph,
    tint: Option<Hsla>,
    arrow: Glyph,
    open_at_first: bool,
    rows: Vec<ListItemButton>,
}

impl ListFolder {
    /// `arrow` points down; it turns to point up while the folder is open.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: Glyph,
        arrow: Glyph,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            tint: None,
            arrow,
            open_at_first: false,
            rows: Vec::new(),
        }
    }

    /// The hue of the folder's icon; see [`ListItemButton::tint`].
    pub fn tint(mut self, tint: Option<Hsla>) -> Self {
        self.tint = tint;
        self
    }

    /// Whether the folder is open when it first appears. After that it is
    /// as the user left it.
    pub fn open_at_first(mut self, open: bool) -> Self {
        self.open_at_first = open;
        self
    }

    pub fn rows(mut self, rows: impl IntoIterator<Item = ListItemButton>) -> Self {
        self.rows.extend(rows);
        self
    }
}

impl RenderOnce for ListFolder {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = ElementId::Name(self.id.clone());
        let open_at_first = self.open_at_first;
        let state = window.use_keyed_state(id.clone(), cx, |_, _| open_at_first);
        let open = *state.read(cx);
        let muted = cx.theme().muted_foreground;

        // MUI's `Collapse` and the half turn of its `ExpandMore` arrow.
        let policy =
            kit::Transition::new(scaled(duration::STANDARD)).ease(easing(curve::EASE_IN_OUT));
        let target = if open { 1.0 } else { 0.0 };
        let shown = kit::transition((id.clone(), "shown"), target, policy, window, cx);

        let toggle = state.clone();
        let header = ListItemButton::new(
            ElementId::NamedChild(id.clone().into(), "row".into()),
            self.label,
        )
        .icon(self.icon)
        .tint(self.tint)
        .trailing(
            Icon::new(self.arrow)
                .with_size(px(ICON))
                .text_color(muted)
                .rotate(radians(std::f32::consts::PI * shown)),
        )
        .on_click(move |_, _, cx| {
            toggle.update(cx, |open, cx| {
                *open = !*open;
                cx.notify();
            });
        });

        v_flex()
            .child(header)
            // Closed and at rest, the rows are not built at all.
            .when(open || shown > 0.0, |folder| {
                let rows = v_flex().children(self.rows).into_any_element();
                folder.child(MotionReveal::new(
                    ElementId::NamedChild(id.into(), "rows".into()),
                    shown,
                    rows,
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::hsla;

    #[test]
    fn a_row_fills_with_the_row_strengths() {
        let primary = hsla(0.7, 0.8, 0.6, 1.0);
        let look = interaction::DARK;
        assert_eq!(fill(primary, false, false, &look).a, 0.0);
        assert_eq!(fill(primary, false, true, &look).a, look.row_hover);
        assert_eq!(fill(primary, true, false, &look).a, look.row_selected);
        assert_eq!(fill(primary, true, true, &look).a, look.row_selected_hover);
    }

    #[test]
    fn a_tinted_icon_keeps_its_hue_in_every_state() {
        let primary = hsla(0.7, 0.8, 0.6, 1.0);
        let muted = hsla(0.7, 0.1, 0.6, 1.0);
        let accent = hsla(0.9, 0.7, 0.7, 1.0);
        let tint = hsla(0.15, 0.9, 0.6, 1.0);
        let colors = (primary, muted, accent);
        for state in [(false, false), (false, true), (true, true)] {
            for lifted in [0.0, 0.1] {
                let color = icon_color(Some(tint), colors, state, lifted);
                assert_eq!((color.h, color.s), (tint.h, tint.s));
            }
        }
        assert_eq!(icon_color(None, colors, (false, false), 0.0), muted);
        assert_eq!(icon_color(None, colors, (false, true), 0.0), primary);
        assert_eq!(icon_color(None, colors, (true, true), 0.0), accent);
    }

    #[test]
    fn the_selected_bar_fits_in_a_row_and_glows_three_times() {
        const { assert!(BAR_HEIGHT < ROW_HEIGHT) };
        let glow = bar_glow(hsla(0.7, 0.8, 0.6, 1.0), 1.0);
        assert_eq!(glow.len(), 3);
        assert!(glow.iter().all(|shadow| !shadow.inset));
    }

    #[test]
    fn the_bar_grows_and_slides_into_its_place() {
        let width = interaction::DARK.bar_width;
        assert_eq!(bar_place(width, 1.0), (0.0, BAR_HEIGHT));
        // Half way in it is half as high and still left of its place.
        let (left, height) = bar_place(width, 0.5);
        assert!(left < 0.0 && height == BAR_HEIGHT / 2.0);
        // At the start it is out of sight, left of the row.
        assert_eq!(bar_place(width, 0.0), (-(width + BAR_TRAVEL), 0.0));
        // A spring that dips below zero never asks for a negative height.
        assert_eq!(bar_place(width, -0.1).1, 0.0);
        // A spring that overshoots never makes the bar taller than its row.
        assert!(bar_place(width, 1.2).1 <= ROW_HEIGHT);
    }
}
