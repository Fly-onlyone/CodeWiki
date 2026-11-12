//! `List`: rows that can be pressed, a heading over a group of them, and a
//! folder that opens and closes.
//!
//! Row anatomy is MUI's `ListItemButton` in a dense list
//! (`ListItemButton/ListItemButton.js`, `ListSubheader/ListSubheader.js`).
//! How a row answers the pointer is TradingAgent's navigation
//! (`components/layout/Sidebar.tsx`): it shifts four pixels to the right, its
//! icon grows and takes the primary color, and the selected row carries a
//! glowing bar on its left edge.

use gpui_kit::assets::IconName;
use gpui_kit::base::motion::{self as kit, MotionReveal};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, BoxShadow, ClickEvent, Div, ElementId, FontWeight, Hsla,
    InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce, SharedString,
    Styled as _, Transformation, Window, div, point, px, radians, size,
};

use super::base::{Base, Ink, OnClick, focus_ring};
use super::motion::{SNAPPY, scaled};
use super::tokens::{curve, duration, easing};

/// A dense `ListItemButton`: 4 px above and below a 14 px label with MUI's
/// 4 px text margins.
pub const ROW_HEIGHT: f32 = 36.0;
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
/// `Sidebar.tsx`: `translateX(4px)` and an icon at 115% under the pointer.
const SHIFT: f32 = 4.0;
const ICON_GROWTH: f32 = 0.15;
/// `Sidebar.tsx`: the bar of the selected row, 3 by 24 px.
const BAR: (f32, f32) = (3.0, 24.0);
/// `ListSubheader.js`: `lineHeight: '48px'`, 14 px text.
const SUBHEADER_HEIGHT: f32 = 48.0;

/// `Sidebar.tsx`: how strongly the primary color fills a row.
fn fill(primary: Hsla, selected: bool, hovered: bool) -> Hsla {
    primary.opacity(match (selected, hovered) {
        (true, true) => 0.15,
        (true, false) => 0.1,
        (false, true) => 0.08,
        (false, false) => 0.0,
    })
}

/// `Sidebar.tsx`: the glow of the selected row's bar.
fn bar_glow(primary: Hsla) -> Vec<BoxShadow> {
    [(16.0, 0.7), (6.0, 0.5), (30.0, 0.2)]
        .into_iter()
        .map(|(blur, opacity)| BoxShadow {
            color: primary.opacity(opacity),
            offset: point(px(0.), px(0.)),
            blur_radius: px(blur),
            spread_radius: px(0.),
            inset: false,
        })
        .collect()
}

/// The heading over a group of rows.
pub fn subheader(label: impl Into<SharedString>, cx: &App) -> Div {
    div()
        .px(px(INSET + PADDING))
        .h(px(SUBHEADER_HEIGHT))
        .flex()
        .items_center()
        .text_size(px(FONT))
        .font_weight(FontWeight::MEDIUM)
        .text_color(cx.theme().muted_foreground)
        .child(label.into())
}

/// A row that can be pressed.
#[derive(IntoElement)]
pub struct ListItemButton {
    id: ElementId,
    label: SharedString,
    icon: Option<IconName>,
    selected: bool,
    level: usize,
    trailing: Option<AnyElement>,
    on_click: Option<OnClick>,
}

impl ListItemButton {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            selected: false,
            level: 0,
            trailing: None,
            on_click: None,
        }
    }

    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// How deeply the row is nested: 0 for a top row.
    pub fn level(mut self, level: usize) -> Self {
        self.level = level;
        self
    }

    /// Something at the far end of the row, such as a folder's arrow.
    fn trailing(mut self, trailing: impl IntoElement) -> Self {
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
        let theme = cx.theme();
        let (primary, foreground, muted) =
            (theme.primary, theme.foreground, theme.muted_foreground);
        let radius = theme.radius / px(1.);
        let (selected, hovered) = (self.selected, base.hovered);

        let policy =
            kit::Transition::new(scaled(duration::SHORTEST)).ease(easing(curve::EASE_IN_OUT));
        let fill = kit::transition(
            (self.id.clone(), "fill"),
            fill(primary, selected, hovered),
            policy,
            window,
            cx,
        );
        let hover_to = if hovered { 1.0 } else { 0.0 };
        let hover = kit::spring(
            (self.id.clone(), "hover"),
            hover_to,
            SNAPPY.policy(0.01),
            window,
            cx,
        );
        let icon_color = if selected || hovered { primary } else { muted };
        let grown = 1.0 + ICON_GROWTH * hover;

        let element = h_flex()
            .id(self.id.clone())
            .relative()
            .left(px(SHIFT * hover))
            .mx(px(INSET))
            .h(px(ROW_HEIGHT))
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
            .when(selected, |row| {
                row.child(
                    div()
                        .absolute()
                        .left_0()
                        .top(px((ROW_HEIGHT - BAR.1) / 2.0))
                        .w(px(BAR.0))
                        .h(px(BAR.1))
                        .rounded_r(px(4.))
                        .bg(primary)
                        .shadow(bar_glow(primary)),
                )
            })
            .when_some(self.icon, |row, icon| {
                row.child(
                    Icon::new(icon)
                        .with_size(px(ICON))
                        .text_color(icon_color)
                        .transform(Transformation::scale(size(grown, grown))),
                )
            })
            .child(div().flex_1().min_w_0().truncate().child(self.label))
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
    icon: IconName,
    arrow: IconName,
    open_at_first: bool,
    rows: Vec<ListItemButton>,
}

impl ListFolder {
    /// `arrow` points down; it turns to point up while the folder is open.
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        icon: IconName,
        arrow: IconName,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            arrow,
            open_at_first: false,
            rows: Vec::new(),
        }
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
    fn a_row_fills_as_trading_agent_fills_it() {
        let primary = hsla(0.7, 0.8, 0.6, 1.0);
        assert_eq!(fill(primary, false, false).a, 0.0);
        assert_eq!(fill(primary, false, true).a, 0.08);
        assert_eq!(fill(primary, true, false).a, 0.1);
        assert_eq!(fill(primary, true, true).a, 0.15);
    }

    #[test]
    fn the_selected_bar_fits_in_a_row_and_glows_three_times() {
        assert!(BAR.1 < ROW_HEIGHT);
        let glow = bar_glow(hsla(0.7, 0.8, 0.6, 1.0));
        assert_eq!(glow.len(), 3);
        assert!(glow.iter().all(|shadow| !shadow.inset));
    }
}
