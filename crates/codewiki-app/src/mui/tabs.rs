//! `Tabs` and `Tab`.
//!
//! Sizes are TradingAgent's (`components.ts`, `MuiTab`): 44 px high, 8 px of
//! padding each side, 4 px between tabs, and under the selected tab a bar
//! three pixels thick and 60% of the tab wide. TradingAgent scales that bar
//! in from its middle. GPUI cannot scale an element, so the bar slides from
//! the old tab to the new one on a spring instead, as the indicator of MUI
//! itself slides (`Tabs/Tabs.js`).

use std::rc::Rc;

use gpui_kit::base::motion as kit;
use gpui_kit::component::{ActiveTheme as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, SharedString, Styled as _, Window, div, px,
};

use super::base::{Base, Ink, OnClick, focus_ring};
use super::motion::SNAPPY;

/// `MuiTab`: `minHeight: 44`.
pub const HEIGHT: f32 = 44.0;
/// `MuiTab`: `paddingLeft: 8`, `paddingRight: 8`.
const PADDING: f32 = 8.0;
/// `MuiTabs`, `flexContainer`: `gap: 4`.
const GAP: f32 = 4.0;
/// `Tab/Tab.js`: the button text size at a line height of 1.25.
const FONT: f32 = 14.0;
const LINE: f32 = FONT * 1.25;
/// `MuiTab`, `::after`: `height: 3`, `width: 60%`, `maxWidth: 88`.
const BAR: f32 = 3.0;
const BAR_SHARE: f32 = 0.6;
const BAR_MAX: f32 = 88.0;
/// The widest a label gets before it is cut short.
const LABEL_MAX: f32 = 180.0;

/// Where the bar sits under a tab that starts at `left` and is `width` wide:
/// its own left edge and width.
pub fn bar(left: f32, width: f32) -> (f32, f32) {
    let bar = (width * BAR_SHARE).min(BAR_MAX);
    (left + (width - bar) / 2.0, bar)
}

/// What choosing the tab at an index does.
type OnSelect = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// One tab: its label and, optionally, something after it, such as a close
/// button.
pub struct Tab {
    label: SharedString,
    suffix: Option<AnyElement>,
}

impl Tab {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            suffix: None,
        }
    }

    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }
}

impl gpui_kit::prelude::FluentBuilder for Tab {}

/// A row of tabs with a bar under the selected one.
#[derive(IntoElement)]
pub struct Tabs {
    id: SharedString,
    tabs: Vec<Tab>,
    selected: usize,
    on_select: Option<OnSelect>,
}

impl Tabs {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            tabs: Vec::new(),
            selected: 0,
            on_select: None,
        }
    }

    pub fn tabs(mut self, tabs: impl IntoIterator<Item = Tab>) -> Self {
        self.tabs.extend(tabs);
        self
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    pub fn on_select(mut self, on_select: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(on_select));
        self
    }
}

impl RenderOnce for Tabs {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (primary, muted, radius) =
            (theme.primary, theme.muted_foreground, theme.radius / px(1.));
        let row_id = ElementId::Name(self.id.clone());
        // Where each tab was drawn last frame, from the left of the row.
        let places = window.use_keyed_state(row_id.clone(), cx, |_, _| Vec::<(f32, f32)>::new());
        let place = places.read(cx).get(self.selected).copied();
        let count = self.tabs.len();

        let mut row = h_flex()
            .relative()
            .h(px(HEIGHT))
            .gap(px(GAP))
            .on_children_prepainted({
                let places = places.clone();
                move |bounds, _, cx| {
                    let Some(origin) = bounds.first().map(|first| first.origin.x) else {
                        return;
                    };
                    // The bar, when it is drawn, is the last child.
                    let measured: Vec<(f32, f32)> = bounds
                        .iter()
                        .take(count)
                        .map(|tab| ((tab.origin.x - origin) / px(1.), tab.size.width / px(1.)))
                        .collect();
                    places.update(cx, |places, cx| {
                        if *places != measured {
                            *places = measured;
                            cx.notify();
                        }
                    });
                }
            });

        for (index, tab) in self.tabs.into_iter().enumerate() {
            let id = ElementId::NamedInteger(self.id.clone(), index as u64);
            let base = Base::new(&id, false, window, cx);
            let color = if index == self.selected {
                primary
            } else {
                muted
            };
            let on_click = self.on_select.clone().map(|on_select| {
                Rc::new(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                    on_select(index, window, cx)
                }) as OnClick
            });
            let element = h_flex()
                .id(id)
                .relative()
                .flex_shrink_0()
                .h_full()
                .px(px(PADDING))
                .gap(px(GAP))
                .items_center()
                .rounded_t(px(radius))
                .text_color(color)
                .text_size(px(FONT))
                .line_height(px(LINE))
                .font_weight(FontWeight::MEDIUM)
                .when(base.focused, |tab| tab.shadow(vec![focus_ring(primary)]))
                .child(div().max_w(px(LABEL_MAX)).truncate().child(tab.label))
                .children(tab.suffix);
            let ink = Ink {
                radius: 0.0,
                centered: false,
                color,
            };
            row = row.child(base.attach(element, ink, false, on_click, cx));
        }

        // The bar appears once the selected tab has been measured, and from
        // then on follows the selection on a spring.
        if let Some((left, width)) = place {
            let (left, width) = bar(left, width);
            let policy = SNAPPY.policy(0.1);
            let left = kit::spring((row_id.clone(), "bar-left"), left, policy, window, cx);
            let width = kit::spring((row_id, "bar-width"), width, policy, window, cx);
            row = row.child(
                div()
                    .absolute()
                    .bottom_0()
                    .left(px(left))
                    .w(px(width.max(0.0)))
                    .h(px(BAR))
                    .rounded_t(px(BAR))
                    .bg(primary),
            );
        }
        row
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_is_centred_under_its_tab() {
        // 60% of a 100 px tab, centred.
        for (start, expected) in [(0.0, 20.0), (50.0, 70.0)] {
            let (left, width) = bar(start, 100.0);
            assert!((left - expected).abs() < 1e-3, "{left}");
            assert!((width - 60.0).abs() < 1e-3, "{width}");
        }
    }

    #[test]
    fn the_bar_stops_growing_under_a_wide_tab() {
        let (left, width) = bar(10.0, 300.0);
        assert_eq!(width, 88.0);
        assert_eq!(left, 10.0 + (300.0 - 88.0) / 2.0);
    }

    #[test]
    fn the_bar_never_leaves_its_tab() {
        for width in [0.0, 12.0, 90.0, 146.0, 360.0] {
            let (left, bar_width) = bar(7.0, width);
            assert!(left >= 7.0 && left + bar_width <= 7.0 + width + 1e-3);
        }
    }
}
