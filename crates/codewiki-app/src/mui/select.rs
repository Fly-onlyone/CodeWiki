//! `Select`, in MUI's outlined variant and small size.
//!
//! The closed control: the outline of a text field around the choice in
//! effect, with an arrow at its end (`Select/SelectInput.js`,
//! `NativeSelect/NativeSelectInput.js`, `OutlinedInput/OutlinedInput.js`).
//! The outline's colors are those of [`super::text_field`], which are
//! TradingAgent's, so the two controls match side by side.
//!
//! A click, Enter or Space asks for the list of choices, which is a
//! [`Menu`] that hangs under the control, as wide as it; [`menu`] builds one.
//! The owner keeps the menu's state, as for every menu.
//!
//! MUI turns the arrow upside down while the list is open. The kit's icon
//! cannot be turned, so the arrow stays as it is.

use gpui_kit::base::motion as kit;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, Bounds, BoxShadow, ClickEvent, ElementId, FocusHandle, Hsla, InteractiveElement as _,
    IntoElement, ParentElement as _, Pixels, RenderOnce, SharedString, Styled as _, Window, div,
    point,
};

use super::base::{Base, Ink, OnClick, focus_ring};
use super::interaction;
use super::menu::{Align, Menu, MenuSection};
use super::motion::scaled;
use super::text_field::HEIGHT;
use super::tokens::{self, curve, duration, easing};
use crate::icons::{self, Glyph};
use crate::zoom::px;

/// `OutlinedInput.js`, `size: 'small'`: `padding: '8.5px 14px'`.
const PADDING: f32 = 14.0;
const FONT: f32 = 16.0;
/// `NativeSelectInput.js`, outlined: `paddingRight: 32`, the room the arrow
/// takes.
const ARROW_ROOM: f32 = 32.0;
/// `NativeSelectInput.js`, `NativeSelectIcon`, outlined: `right: 7`.
const ARROW_RIGHT: f32 = 7.0;
/// `SvgIcon.js`: an icon is 24 px.
const ARROW: f32 = 24.0;
/// The icon of the choice in effect: the size and the gap of a text field's
/// start adornment (`text_field.rs`), so that the two controls match.
const ICON: f32 = 20.0;
const ICON_GAP: f32 = 8.0;
/// `OutlinedInput.js`: the outline is the text color at 23%.
const OUTLINE_OPACITY: f32 = 0.23;
/// `styles/createPalette.js`, `text.disabled`, light and dark.
const TEXT_DISABLED: (f32, f32) = (0.38, 0.5);
/// The narrowest menu: a control cut short by a narrow window still opens a
/// list its choices can be read in.
const MENU_MIN_WIDTH: f32 = 160.0;

/// How wide the menu of a select is whose control is `control` wide.
/// `SelectInput.js`: `minWidth` is the width of the control.
pub fn menu_width(control: f32) -> f32 {
    control.max(MENU_MIN_WIDTH)
}

/// The list of choices of a select, under the control at `anchor`: its
/// rows, in one group or several, as the caller made them. The row of the
/// choice in effect is the one made `selected`.
pub fn menu(
    id: impl Into<SharedString>,
    anchor: Bounds<Pixels>,
    focus: &FocusHandle,
    sections: impl IntoIterator<Item = MenuSection>,
) -> Menu {
    sections.into_iter().fold(
        Menu::new(id, anchor, focus)
            .align(Align::Start)
            .width(menu_width(anchor.size.width / px(1.))),
        Menu::section,
    )
}

/// A setting with a few named values, one of them in effect.
#[derive(IntoElement)]
pub struct Select {
    id: ElementId,
    value: SharedString,
    icon: Option<(Glyph, Option<Hsla>)>,
    open: bool,
    disabled: bool,
    on_click: Option<OnClick>,
}

impl Select {
    /// `value` is the label of the choice in effect.
    pub fn new(id: impl Into<ElementId>, value: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            icon: None,
            open: false,
            disabled: false,
            on_click: None,
        }
    }

    /// The icon of the choice in effect, before its label, as its row in
    /// the list has it. `tint` is its color; without one it takes the color
    /// of the arrow.
    pub fn icon(mut self, icon: Glyph, tint: Option<Hsla>) -> Self {
        self.icon = Some((icon, tint));
        self
    }

    /// Whether its list is open: the control then looks focused.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called when the list of choices is asked for.
    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(on_click));
        self
    }
}

impl RenderOnce for Select {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let base = Base::new(&self.id, self.disabled, window, cx);
        let theme = cx.theme();
        let dark = theme.is_dark();
        let states = tokens::states(dark);
        let (radius, primary, foreground) = (theme.radius, theme.primary, theme.foreground);
        let active = self.open || base.focused;

        let outline_to = if self.disabled {
            // `OutlinedInput.js`, disabled: `action.disabled`.
            foreground.opacity(states.disabled)
        } else if active {
            primary
        } else if base.hovered {
            primary.opacity(interaction::current(cx).outline_hover)
        } else {
            foreground.opacity(OUTLINE_OPACITY)
        };
        let policy =
            || kit::Transition::new(scaled(duration::SHORTER)).ease(easing(curve::EASE_OUT));
        let outline = kit::transition(
            (self.id.clone(), "outline"),
            outline_to,
            policy(),
            window,
            cx,
        );
        // As on a text field: a second pixel inside, not a thicker border,
        // so the text does not move.
        let second_to = if active && !self.disabled {
            primary
        } else {
            primary.opacity(0.0)
        };
        let second = kit::transition((self.id.clone(), "second"), second_to, policy(), window, cx);

        let text = if self.disabled {
            let opacity = if dark {
                TEXT_DISABLED.1
            } else {
                TEXT_DISABLED.0
            };
            foreground.opacity(opacity)
        } else {
            foreground
        };
        // `palette.action.active`: white on dark, black at 54% on light.
        let arrow = if self.disabled {
            foreground.opacity(states.disabled)
        } else if dark {
            foreground
        } else {
            foreground.opacity(0.54)
        };

        let mut shadows = vec![BoxShadow {
            color: second,
            offset: point(px(0.), px(0.)),
            blur_radius: px(0.),
            spread_radius: px(1.),
            inset: true,
        }];
        if base.focused && !self.open {
            shadows.push(focus_ring(primary));
        }

        let element = h_flex()
            .id(self.id)
            .relative()
            .w_full()
            .h(px(HEIGHT))
            .pl(px(PADDING))
            .pr(px(ARROW_ROOM))
            .items_center()
            .rounded(radius)
            .border_1()
            .border_color(outline)
            .shadow(shadows)
            .text_color(text)
            .text_size(px(FONT))
            .gap(px(ICON_GAP))
            .when_some(self.icon, |select, (icon, tint)| {
                // A disabled select fades its icon with its text.
                let color = tint.filter(|_| !self.disabled).unwrap_or(arrow);
                select.child(Icon::new(icon).with_size(px(ICON)).text_color(color))
            })
            .child(div().flex_1().min_w_0().truncate().child(self.value))
            .child(
                div()
                    .absolute()
                    .right(px(ARROW_RIGHT))
                    .top(px((HEIGHT - ARROW) / 2.0))
                    .child(
                        Icon::new(icons::DROP_DOWN)
                            .with_size(px(ARROW))
                            .text_color(arrow),
                    ),
            )
            .when(self.disabled, |select| select.cursor_default());
        // A select does not ripple: it is an input, not a button.
        let ink = Ink {
            radius: radius / px(1.),
            centered: false,
            color: foreground.opacity(0.0),
        };
        base.attach(element, ink, self.disabled, self.on_click, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_is_as_wide_as_its_control() {
        assert_eq!(menu_width(280.0), 280.0);
        assert_eq!(menu_width(420.5), 420.5);
        // A control squeezed by a narrow window still opens a readable list.
        assert_eq!(menu_width(90.0), MENU_MIN_WIDTH);
    }

    #[test]
    fn the_arrow_has_its_room_at_the_end() {
        // The arrow, 7 px from the edge, ends one pixel inside the room kept
        // for it.
        assert_eq!(ARROW_ROOM - (ARROW_RIGHT + ARROW), 1.0);
        // It sits on the middle line of the control.
        assert_eq!((HEIGHT - ARROW) / 2.0 + ARROW / 2.0, HEIGHT / 2.0);
        // A select is as tall as a text field, so the two line up in a row.
        assert_eq!(HEIGHT, 40.0);
    }
}
