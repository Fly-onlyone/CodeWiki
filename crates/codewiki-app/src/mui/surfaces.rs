//! `Paper`, `Card` and `Divider`: what content sits on, and `Typography`,
//! how it is set.
//!
//! The card is TradingAgent's (`components.ts`, `MuiCard`): tinted glass with
//! a thin primary border, corners four pixels rounder than a button's, a
//! soft shadow and a line of accent color along its top edge. TradingAgent
//! blurs what is behind a card; GPUI cannot, so the card is kept dense
//! enough to read on whatever is behind it.

use gpui_kit::component::{ActiveTheme as _, v_flex};
use gpui_kit::{
    App, BoxShadow, Div, Hsla, ParentElement as _, SharedString, Styled as _, div, hsla, point, px,
};

use super::tokens::{self, Variant};
use crate::glass;

/// MUI's `CardContent`: `padding: 16`.
pub const CARD_PADDING: f32 = 16.0;
/// `MuiCard`: the share of primary color mixed into a card.
const CARD_TINT: f32 = 0.08;

/// `MuiCard`, `boxShadow`: `0 4px 20px black/.2, inset 0 1px 0 white/.05`.
fn card_shadow() -> Vec<BoxShadow> {
    vec![
        BoxShadow {
            color: hsla(0., 0., 0., 0.2),
            offset: point(px(0.), px(4.)),
            blur_radius: px(20.),
            spread_radius: px(0.),
            inset: false,
        },
        BoxShadow {
            color: hsla(0., 0., 1., 0.05),
            offset: point(px(0.), px(1.)),
            blur_radius: px(0.),
            spread_radius: px(0.),
            inset: true,
        },
    ]
}

/// The fill of a card: the theme's surface color, as dense as the reading
/// pane, so text on it keeps its contrast.
pub fn card_fill(cx: &App) -> Hsla {
    let levels = cx
        .try_global::<glass::Levels>()
        .copied()
        .unwrap_or(glass::Levels::OPAQUE);
    levels.pane(cx.theme().secondary)
}

/// A card: a bordered, tinted surface for one group of content. Its children
/// are laid out in a column with MUI's `CardContent` padding.
pub fn card(cx: &App) -> Div {
    let theme = cx.theme();
    v_flex()
        .relative()
        .overflow_hidden()
        .rounded(theme.radius_lg)
        .border_1()
        .border_color(glass::tinted_border(cx))
        .bg(card_fill(cx))
        .shadow(card_shadow())
        // The primary tint lies over the fill, as in TradingAgent's
        // `linear-gradient(cardTint, cardTint), …`.
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .bg(theme.primary.opacity(CARD_TINT * 0.5)),
        )
        .child(glass::hairline(cx))
}

/// The padded body of a [`card`].
pub fn card_content() -> Div {
    v_flex()
        .relative()
        .p(px(CARD_PADDING))
        .gap(px(tokens::SPACING))
}

/// A raised surface for something that floats over the page, such as a
/// dialog: opaque, with MUI's elevation shadow.
pub fn paper(elevation: usize, cx: &App) -> Div {
    let theme = cx.theme();
    v_flex()
        .rounded(theme.radius_lg)
        .border_1()
        .border_color(glass::tinted_border(cx))
        .bg(theme.popover)
        .text_color(theme.popover_foreground)
        .shadow(tokens::elevation(elevation))
}

/// A one pixel line between two things.
pub fn divider(cx: &App) -> Div {
    div()
        .h(px(1.))
        .w_full()
        .flex_shrink_0()
        .bg(cx.theme().border)
}

/// Text in one of MUI's styles.
pub fn text(variant: Variant, content: impl Into<SharedString>) -> Div {
    let spec = variant.spec();
    let content: SharedString = content.into();
    let content = if spec.uppercase {
        SharedString::from(content.to_uppercase())
    } else {
        content
    };
    div()
        .text_size(px(spec.size))
        .line_height(px(spec.size * spec.line_height))
        .font_weight(spec.weight)
        .child(content)
}
