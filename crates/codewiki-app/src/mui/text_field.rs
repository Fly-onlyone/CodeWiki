//! `TextField`, in MUI's outlined variant and small size.
//!
//! The editing itself is gpui-kit's `Input`, with its own border and fill
//! turned off. This draws what MUI draws around an input: the outline, which
//! takes the primary color and a second pixel when the field has focus, and
//! an icon before the text (`OutlinedInput/OutlinedInput.js`,
//! `InputAdornment/InputAdornment.js`). The outline's colors at rest and
//! under the pointer are TradingAgent's (`components.ts`, `MuiTextField`).

use gpui_kit::base::motion as kit;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, BoxShadow, ElementId, Entity, Focusable as _, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, Styled as _, Window, div, point, px,
};

use super::base::Hover;
use super::motion::scaled;
use super::tokens::{curve, duration, easing};
use crate::icons::Glyph;

/// `OutlinedInput.js`, `size: 'small'`: `padding: '8.5px 14px'` around a line
/// of `1.4375em` at 16 px.
pub const HEIGHT: f32 = 40.0;
const PADDING: f32 = 14.0;
const FONT: f32 = 16.0;
/// `InputAdornment.js`: `marginRight: 8`.
const ICON_GAP: f32 = 8.0;
const ICON: f32 = 20.0;
/// `OutlinedInput.js`: the outline is the text color at 23%.
const OUTLINE_OPACITY: f32 = 0.23;

/// A single-line text field.
#[derive(IntoElement)]
pub struct TextField {
    id: ElementId,
    state: Entity<InputState>,
    icon: Option<Glyph>,
}

impl TextField {
    pub fn new(id: impl Into<ElementId>, state: &Entity<InputState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            icon: None,
        }
    }

    /// MUI's start `InputAdornment`.
    pub fn icon(mut self, icon: Glyph) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl RenderOnce for TextField {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let hover = Hover::new(&self.id, window, cx);
        let focused = self.state.read(cx).focus_handle(cx).is_focused(window);
        let theme = cx.theme();
        let (radius, primary, muted) = (theme.radius, theme.primary, theme.muted_foreground);
        let outline_to = if focused {
            primary
        } else if hover.hovered {
            primary.opacity(0.5)
        } else {
            theme.foreground.opacity(OUTLINE_OPACITY)
        };
        let policy = kit::Transition::new(scaled(duration::SHORTER)).ease(easing(curve::EASE_OUT));
        let outline = kit::transition(
            (self.id.clone(), "outline"),
            outline_to,
            policy.clone(),
            window,
            cx,
        );
        // MUI widens the outline to two pixels on focus. A second pixel drawn
        // as an inner shadow does that without moving the text.
        let second_to = if focused {
            primary
        } else {
            primary.opacity(0.0)
        };
        let second = kit::transition((self.id.clone(), "second"), second_to, policy, window, cx);

        let field = h_flex()
            .id(self.id)
            .w_full()
            .h(px(HEIGHT))
            .px(px(PADDING))
            .gap(px(ICON_GAP))
            .items_center()
            .rounded(radius)
            .border_1()
            .border_color(outline)
            .shadow(vec![BoxShadow {
                color: second,
                offset: point(px(0.), px(0.)),
                blur_radius: px(0.),
                spread_radius: px(1.),
                inset: true,
            }])
            .when_some(self.icon, |field, icon| {
                field.child(Icon::new(icon).with_size(px(ICON)).text_color(muted))
            })
            .child(
                div().flex_1().min_w_0().child(
                    Input::new(&self.state)
                        .appearance(false)
                        .px_0()
                        .text_size(px(FONT)),
                ),
            );
        hover.attach(field)
    }
}
