//! `Checkbox`, at MUI's small size.
//!
//! Sizes and colors are MUI's (`Checkbox/Checkbox.js`,
//! `internal/SwitchBase.js`, `SvgIcon/SvgIcon.js`); TradingAgent does not
//! restyle the checkbox. A round area around one of three icons: a blank
//! box, a checked box, and the box of a selection that is partly made. The
//! area tints under the pointer and carries the ripple.
//!
//! MUI swaps the icon at once and does not ease the tint; here the tint eases
//! over `duration.shortest`, as the app's other controls do.

use gpui_kit::base::motion as kit;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, ElementId, Hsla, InteractiveElement as _, IntoElement, ParentElement as _, RenderOnce,
    Styled as _, Window, div,
};

use super::base::{Base, Ink, OnClick, focus_ring};
use super::motion::scaled;
use super::tokens::{self, curve, duration, easing};
use crate::icons::{self, Glyph};
use crate::zoom::px;

/// `SvgIcon.js`, `fontSize: 'small'`: `pxToRem(20)`. `Checkbox.js` gives
/// its icons the checkbox's own size.
const ICON: f32 = 20.0;
/// `SwitchBase.js`, `SwitchBaseRoot`: `padding: 9`, `borderRadius: '50%'`.
const PADDING: f32 = 9.0;
/// The checkbox with the round area around its icon.
pub const SIZE: f32 = ICON + 2.0 * PADDING;
/// `SwitchBase.js`: `edge: 'start'` at `size: 'small'` is `marginLeft: -3`,
/// so that the box lines up with the text above it.
const EDGE_START: f32 = 3.0;

/// What a checkbox shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Check {
    #[default]
    Unchecked,
    Checked,
    /// Some of what the checkbox stands for is checked: MUI's
    /// `indeterminate`.
    Indeterminate,
}

impl Check {
    pub fn of(checked: bool) -> Self {
        if checked {
            Self::Checked
        } else {
            Self::Unchecked
        }
    }

    /// `Checkbox.js`: `CheckBoxIcon`, `CheckBoxOutlineBlankIcon` and
    /// `IndeterminateCheckBoxIcon`.
    pub fn icon(self) -> Glyph {
        match self {
            Self::Unchecked => icons::CHECK_BOX_BLANK,
            Self::Checked => icons::CHECK_BOX,
            Self::Indeterminate => icons::CHECK_BOX_INDETERMINATE,
        }
    }
}

/// The colors of a checkbox in one state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// The icon, and the ripple, which is MUI's `currentColor`.
    pub icon: Hsla,
    /// What tints the round area under the pointer.
    pub hover: Hsla,
}

impl Look {
    /// `muted` stands for MUI's `text.secondary`, which the theme's muted
    /// text color is.
    pub fn of(check: Check, disabled: bool, dark: bool, theme: (Hsla, Hsla, Hsla)) -> Self {
        let (foreground, muted, primary) = theme;
        let states = tokens::states(dark);
        // `Checkbox.js`: `text.secondary` at rest; `palette[color].main`
        // when checked or indeterminate; `action.disabled` when disabled.
        let icon = match (check, disabled) {
            (_, true) => foreground.opacity(states.disabled),
            (Check::Unchecked, false) => muted,
            (Check::Checked | Check::Indeterminate, false) => primary,
        };
        // `Checkbox.js`, `&:hover`: `alpha(palette[color].main,
        // action.hoverOpacity)`, in the checkbox's color whether or not it
        // is checked.
        Self {
            icon,
            hover: primary.opacity(states.hover),
        }
    }
}

/// A box that is checked, unchecked or partly checked, and says when it is
/// clicked. What a click changes is the owner's to decide.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    check: Check,
    edge_start: bool,
    on_click: Option<OnClick>,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            check: Check::Unchecked,
            edge_start: false,
            on_click: None,
        }
    }

    pub fn check(mut self, check: Check) -> Self {
        self.check = check;
        self
    }

    /// Pull the checkbox left, so that its box and not its round area lines
    /// up with what is above it: MUI's `edge="start"`.
    pub fn edge_start(mut self) -> Self {
        self.edge_start = true;
        self
    }

    pub fn on_click(
        mut self,
        on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(on_click));
        self
    }
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let base = Base::new(&self.id, false, window, cx);
        let theme = cx.theme();
        let primary = theme.primary;
        let look = Look::of(
            self.check,
            false,
            theme.is_dark(),
            (theme.foreground, theme.muted_foreground, primary),
        );
        let policy =
            kit::Transition::new(scaled(duration::SHORTEST)).ease(easing(curve::EASE_IN_OUT));
        let hover_to = if base.hovered {
            look.hover
        } else {
            look.hover.opacity(0.0)
        };
        let hover = kit::transition((self.id.clone(), "hover"), hover_to, policy, window, cx);

        let element = div()
            .id(self.id.clone())
            .relative()
            .flex_shrink_0()
            .size(px(SIZE))
            .when(self.edge_start, |area| area.ml(px(-EDGE_START)))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(hover)
            .when(base.focused, |area| area.shadow(vec![focus_ring(primary)]))
            .child(
                Icon::new(self.check.icon())
                    .with_size(px(ICON))
                    .text_color(look.icon),
            );
        let ink = Ink {
            radius: SIZE / 2.0,
            centered: true,
            color: look.icon,
        };
        base.attach(element, ink, false, self.on_click, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::hsla;

    fn theme() -> (Hsla, Hsla, Hsla) {
        (
            hsla(0.0, 0.0, 0.95, 1.0),
            hsla(0.0, 0.0, 0.7, 1.0),
            hsla(0.72, 0.6, 0.6, 1.0),
        )
    }

    #[test]
    fn the_checkbox_has_muis_small_measurements() {
        assert_eq!(SIZE, 38.0);
        assert_eq!(ICON, 20.0);
        assert_eq!((PADDING, EDGE_START), (9.0, 3.0));
    }

    #[test]
    fn a_checked_or_partly_checked_box_is_in_the_primary_color() {
        let (_, muted, primary) = theme();
        for dark in [false, true] {
            assert_eq!(Look::of(Check::Checked, false, dark, theme()).icon, primary);
            assert_eq!(
                Look::of(Check::Indeterminate, false, dark, theme()).icon,
                primary
            );
            assert_eq!(Look::of(Check::Unchecked, false, dark, theme()).icon, muted);
        }
    }

    #[test]
    fn the_hover_tint_is_primary_at_muis_hover_opacity() {
        let (_, _, primary) = theme();
        for (dark, opacity) in [(false, 0.04), (true, 0.08)] {
            for check in [Check::Unchecked, Check::Checked, Check::Indeterminate] {
                let hover = Look::of(check, false, dark, theme()).hover;
                assert_eq!(hover.h, primary.h);
                assert!((hover.a - opacity).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn a_disabled_box_is_in_the_disabled_text_color() {
        let (foreground, _, _) = theme();
        for (dark, opacity) in [(false, 0.26), (true, 0.3)] {
            let icon = Look::of(Check::Checked, true, dark, theme()).icon;
            assert_eq!(icon.l, foreground.l);
            assert!((icon.a - opacity).abs() < 1e-6);
        }
    }

    #[test]
    fn each_state_has_its_own_icon() {
        assert_eq!(Check::of(true), Check::Checked);
        assert_eq!(Check::of(false), Check::Unchecked);
        let icons = [Check::Unchecked, Check::Checked, Check::Indeterminate].map(Check::icon);
        assert_ne!(icons[0], icons[1]);
        assert_ne!(icons[1], icons[2]);
        assert_ne!(icons[0], icons[2]);
    }
}
