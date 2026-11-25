//! `Snackbar`: a short message in the window's corner.
//!
//! Sizes are MUI's (`Snackbar/Snackbar.js`, `SnackbarContent/SnackbarContent.js`).
//! MUI's snackbar carries a message and an action. This one also has a
//! header, as a Windows notification does: the app's logo and name, and a
//! close button. Before the message is the icon of what it is about, in the
//! tone's color, where an alert (`feedback::alert`) has its tone icon.

use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::{
    AnyElement, App, Div, ElementId, FontWeight, InteractiveElement as _, ParentElement as _,
    SharedString, Stateful, Styled as _, div, img,
};

use super::base::OnClick;
use super::button::{Button, IconButton};
use super::feedback::Tone;
use super::surfaces;
use super::tokens::Variant;
use crate::icons::{self, Glyph};
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
/// The icon before the message, at the size of the tone icon in
/// `feedback::alert`, and its distance from the message.
const ICON: f32 = 22.0;
const ICON_GAP: f32 = 12.0;
/// The line under the title is a size smaller than the title.
const BODY_FONT: f32 = 13.0;
/// The app's own: the logo in the header, as large as in the title bar, and
/// its distance from the app's name.
const LOGO: f32 = 16.0;
const LOGO_GAP: f32 = 8.0;
/// The name in the header: the one Windows shows above the app's
/// notifications, "CodeWiki Dev" for a development build.
const APP_NAME: &str = cw_core::APP_NAME;

/// What a toast shows and does.
pub struct Content {
    pub tone: Tone,
    /// The icon of what the message is about, drawn in the tone's color.
    pub icon: Glyph,
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
    let header = h_flex()
        .w_full()
        .gap(px(LOGO_GAP))
        .items_center()
        .child(img(icons::LOGO).size(px(LOGO)).flex_none())
        .child(
            surfaces::text(Variant::Caption, APP_NAME)
                .flex_1()
                .text_color(theme.muted_foreground),
        )
        .child(
            div().flex_none().mr(px(ACTION_END)).child(
                IconButton::new((id.clone(), "close"), icons::CLOSE)
                    .small()
                    .tooltip("Close")
                    .on_click(move |event, window, cx| on_close(event, window, cx)),
            ),
        );
    let message = h_flex()
        .w_full()
        .gap(px(ICON_GAP))
        .items_center()
        .child(
            Icon::new(content.icon)
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
        .children(action.map(|action| {
            div()
                .flex_shrink_0()
                .ml_auto()
                .pl(px(ACTION_PADDING))
                .mr(px(ACTION_END))
                .child(action)
        }));
    surfaces::paper(ELEVATION, cx)
        .id(id)
        // The paper is made for dialogs; a snackbar's corners are the
        // theme's.
        .rounded(theme.radius)
        // A toast takes the clicks that land on it and no others.
        .occlude()
        .min_w(px(MIN_WIDTH))
        .max_w(px(MAX_WIDTH))
        .child(
            v_flex()
                .w_full()
                .px(px(PADDING.0))
                .py(px(PADDING.1))
                .child(header)
                .child(message),
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
        assert_eq!(PADDING, (16.0, 6.0));
        assert_eq!(
            (MESSAGE_PADDING, ACTION_PADDING, ACTION_END),
            (8.0, 16.0, -8.0)
        );
    }

    #[test]
    fn the_header_logo_fits_its_caption_line() {
        // A 12 px caption at a line height of 1.66 is 19.92 px high.
        let caption = Variant::Caption.spec();
        assert!(LOGO <= caption.size * caption.line_height);
    }
}
