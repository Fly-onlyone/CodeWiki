//! `TextField`, in MUI's outlined variant and small size.
//!
//! The editing itself is gpui-kit's `Input`, with its own border and fill
//! turned off. This draws what MUI draws around an input: the outline, which
//! takes the primary color and a second pixel when the field has focus, an
//! icon before the text, and a line of help or of news under it
//! (`OutlinedInput/OutlinedInput.js`,
//! `InputAdornment/InputAdornment.js`, `FormHelperText/FormHelperText.js`).
//! The outline's colors at rest and under the pointer, and the fill of a
//! disabled field, are TradingAgent's (`components.ts`, `MuiTextField` and
//! `MuiInputBase`).
//!
//! A field that hides what is typed is the same field over an input that is
//! masked (`InputState::masked`).

use gpui_kit::base::motion as kit;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, BoxShadow, ElementId, Entity, Focusable as _, InteractiveElement as _, IntoElement,
    ParentElement as _, RenderOnce, SharedString, Styled as _, Window, div, point, px,
};

use super::base::Hover;
use super::feedback::Tone;
use super::motion::scaled;
use super::tokens::{self, Variant, curve, duration, easing};
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
/// `MuiInputBase`, disabled: the sidebar color at 50% behind the field.
const DISABLED_FILL: f32 = 0.5;
/// `FormHelperText.js`, `size: 'small'`: `marginTop: 4`; contained:
/// `marginLeft: 14`, `marginRight: 14`.
const HELPER_TOP: f32 = 4.0;
const HELPER_SIDE: f32 = 14.0;

/// A single-line text field.
#[derive(IntoElement)]
pub struct TextField {
    id: ElementId,
    state: Entity<InputState>,
    icon: Option<Glyph>,
    disabled: bool,
    helper: Option<(SharedString, Tone)>,
}

impl TextField {
    pub fn new(id: impl Into<ElementId>, state: &Entity<InputState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            icon: None,
            disabled: false,
            helper: None,
        }
    }

    /// MUI's start `InputAdornment`.
    pub fn icon(mut self, icon: Glyph) -> Self {
        self.icon = Some(icon);
        self
    }

    /// A field that shows its value and takes no input.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// A line under the field: help in the neutral tone, or news about the
    /// value, such as that it was saved or why it was refused. An error also
    /// colors the outline.
    pub fn helper(mut self, text: impl Into<SharedString>, tone: Tone) -> Self {
        self.helper = Some((text.into(), tone));
        self
    }
}

impl RenderOnce for TextField {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let hover = Hover::new(&self.id, window, cx);
        let focused = !self.disabled && self.state.read(cx).focus_handle(cx).is_focused(window);
        let theme = cx.theme();
        let dark = theme.is_dark();
        let states = tokens::states(dark);
        let (radius, primary, muted, foreground) = (
            theme.radius,
            theme.primary,
            theme.muted_foreground,
            theme.foreground,
        );
        let disabled_fill = theme.sidebar.opacity(DISABLED_FILL);
        let error = self
            .helper
            .as_ref()
            .is_some_and(|(_, tone)| *tone == Tone::Error);
        // `OutlinedInput.js`: an error keeps its color whatever else is true.
        let accent = if error {
            Tone::Error.color(cx)
        } else {
            primary
        };
        let outline_to = if self.disabled {
            // `OutlinedInput.js`, disabled: `action.disabled`.
            foreground.opacity(states.disabled)
        } else if focused || error {
            accent
        } else if hover.hovered {
            primary.opacity(0.5)
        } else {
            foreground.opacity(OUTLINE_OPACITY)
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
        let second_to = if focused { accent } else { accent.opacity(0.0) };
        let second = kit::transition((self.id.clone(), "second"), second_to, policy, window, cx);
        let helper = self.helper.map(|(text, tone)| {
            let caption = Variant::Caption.spec();
            div()
                .mt(px(HELPER_TOP))
                .mx(px(HELPER_SIDE))
                .text_size(px(caption.size))
                .line_height(px(caption.size * caption.line_height))
                .text_color(tone.color(cx))
                .truncate()
                .child(text)
        });

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
            .when(self.disabled, |field| field.bg(disabled_fill))
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
                        .disabled(self.disabled)
                        .px_0()
                        .text_size(px(FONT)),
                ),
            );
        let field = if self.disabled {
            field
        } else {
            hover.attach(field)
        };
        div().w_full().child(field).children(helper)
    }
}
