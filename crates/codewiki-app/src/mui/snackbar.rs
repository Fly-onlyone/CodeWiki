//! `Snackbar`: a short message in the window's corner.
//!
//! Sizes are MUI's (`Snackbar/Snackbar.js`, `SnackbarContent/SnackbarContent.js`).
//! MUI's snackbar carries a message and an action; this one also has the
//! tone icon of an alert (`feedback::alert`) and a close button.

use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, Div, ElementId, FontWeight, InteractiveElement as _, ParentElement as _,
    SharedString, Stateful, Styled as _, div,
};

use super::base::OnClick;
use super::button::{Button, IconButton};
use super::feedback::Tone;
use super::surfaces;
use super::tokens::Variant;
use crate::icons;
use crate::zoom::px;

/// `Snackbar.js`: `bottom: 24` and `right: 24` from the `sm` breakpoint up.
pub const OFFSET: f32 = 24.0;
/// `SnackbarContent.js`: `minWidth: 288` from the `sm` breakpoint up.
pub const MIN_WIDTH: f32 = 288.0;
/// MUI sets no largest width. This is the app's own, so that a long line wraps
/// before it fills the window.
pub const MAX_WIDTH: f32 = 568.0;
/// The space between two toasts: `theme.spacing(1)`.
pub const GAP: f32 = 8.0;
/// `SnackbarContent.js`: `elevation: 6`.
const ELEVATION: usize = 6;
/// `SnackbarContent.js`: `padding: '6px 16px'`.
const PADDING: (f32, f32) = (16.0, 6.0);
/// `SnackbarContent.js`, `SnackbarContentMessage`: `padding: '8px 0'`.
const MESSAGE_PADDING: f32 = 8.0;
/// `SnackbarContent.js`, `SnackbarContentAction`: `paddingLeft: 16`,
/// `marginRight: -8`.
const ACTION_PADDING: f32 = 16.0;
const ACTION_END: f32 = -8.0;
/// The tone icon, as in `feedback::alert`, and its distance from the message.
const ICON: f32 = 22.0;
const ICON_GAP: f32 = 12.0;
/// The line under the title is a size smaller than the title.
const BODY_FONT: f32 = 13.0;

/// What a toast shows and does.
pub struct Content {
    pub tone: Tone,
    pub title: SharedString,
    pub body: Option<SharedString>,
    /// The label of the text button in the action slot, and what it does.
    pub action: Option<(SharedString, OnClick)>,
    pub on_close: OnClick,
}

/// One snackbar. `id` tells it apart from the others, for its buttons.
pub fn snackbar(id: impl Into<ElementId>, content: Content, cx: &App) -> Stateful<Div> {
    let id: ElementId = id.into();
    let theme = cx.theme();
    let color = content.tone.color(cx);
    let body = content.body.map(|body| {
        div()
            .text_size(px(BODY_FONT))
            .text_color(theme.muted_foreground)
            .child(body)
    });
    let action = content.action.map(|(label, on_click)| {
        Button::new((id.clone(), "action"))
            .label(label)
            .text()
            .small()
            .on_click(move |event, window, cx| on_click(event, window, cx))
    });
    let on_close = content.on_close;
    surfaces::paper(ELEVATION, cx)
        .id(id.clone())
        // The paper is made for dialogs; a snackbar's corners are the
        // theme's.
        .rounded(theme.radius)
        // A toast takes the clicks that land on it and no others.
        .occlude()
        .min_w(px(MIN_WIDTH))
        .max_w(px(MAX_WIDTH))
        .child(
            h_flex()
                .w_full()
                .px(px(PADDING.0))
                .py(px(PADDING.1))
                .gap(px(ICON_GAP))
                .items_center()
                .child(
                    Icon::new(content.tone.icon())
                        .with_size(px(ICON))
                        .text_color(color),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .py(px(MESSAGE_PADDING))
                        .child(
                            surfaces::text(Variant::Subtitle2, content.title)
                                .font_weight(FontWeight::MEDIUM),
                        )
                        .children(body),
                )
                .child(
                    h_flex()
                        .flex_shrink_0()
                        .items_center()
                        .ml_auto()
                        .pl(px(ACTION_PADDING))
                        .mr(px(ACTION_END))
                        .children(action)
                        .child(
                            IconButton::new((id, "close"), icons::CLOSE)
                                .small()
                                .tooltip("Close")
                                .on_click(move |event, window, cx| on_close(event, window, cx)),
                        ),
                ),
        )
}

/// The toasts in the window's corner, oldest first: the newest sits nearest
/// the corner. The stack takes no room and no clicks; each toast takes its
/// own.
pub fn stack(toasts: impl IntoIterator<Item = AnyElement>) -> Div {
    v_flex()
        .absolute()
        .bottom(px(OFFSET))
        .right(px(OFFSET))
        .gap(px(GAP))
        .items_end()
        .children(toasts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sizes_are_the_ones_mui_names() {
        assert_eq!((MIN_WIDTH, MAX_WIDTH), (288.0, 568.0));
        assert_eq!((OFFSET, GAP, ELEVATION), (24.0, 8.0, 6));
    }
}
