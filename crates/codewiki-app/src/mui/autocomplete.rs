//! `Autocomplete`'s list: the choices that hang under a text field while it
//! is typed in.
//!
//! Anatomy is MUI's (`Autocomplete/Autocomplete.js`): a `Paper` as wide as
//! the field hangs under it and holds a listbox at most 40% of the window
//! high, with 8 pixels above and below its rows. Choices are grouped under a
//! `ListSubheader` each. When nothing matches, a quiet line says so, padded
//! 14 by 16 pixels.
//!
//! The field keeps the focus and its keys: the owner keeps which row is
//! highlighted, moves it with the arrow keys and chooses it on Enter. A press
//! on the list does not take the focus from the field (a row's press never
//! moves the focus, and the paper takes the rest); MUI's listbox keeps a
//! press from taking the focus the same way. A click on a row chooses it. A
//! press anywhere else does move the focus, and the owner closes the list
//! when the field loses it. Nothing covers the window, as a menu's scrim
//! does: the field under the pointer must still take presses.
//!
//! Two things MUI does are left out. Its rows are 48 pixels high on touch
//! screens; here they are always the dense list's. And its group headings
//! stick to the top while their group scrolls; GPUI has no sticky position.

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Anchor, App, Bounds, ElementId, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, Pixels, RenderOnce, ScrollHandle, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, anchored, deferred, div, point,
};

use super::list::{self, ListItemButton};
use super::motion::{Enter, EnterExt as _};
use super::surfaces;
use crate::icons::Glyph;
use crate::zoom::px;

/// `Autocomplete.js` gives its `Paper` no elevation of its own: `Paper.js`
/// defaults to 1.
const ELEVATION: usize = 1;
/// `Autocomplete.js`, listbox: `maxHeight: '40vh'`.
const HEIGHT_SHARE: f32 = 0.4;
/// `Autocomplete.js`, listbox: `padding: '8px 0'`.
const LIST_PADDING: f32 = 8.0;
/// `Autocomplete.js`, `noOptions`: `padding: '14px 16px'`.
const EMPTY_PADDING: (f32, f32) = (14.0, 16.0);
/// Between the field and the list. MUI's popper touches the field; the
/// app's menus hang 4 pixels under theirs, and so does this list.
const GAP: f32 = 4.0;
/// `Popover.js`: `marginThreshold = 16`, the least distance to the window's
/// edge, as for the app's menus.
const WINDOW_MARGIN: f32 = 16.0;

/// What a chosen row does: it is given the row's place in the list.
type OnIndex = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// One choice.
pub struct Choice {
    icon: Glyph,
    label: SharedString,
    secondary: Option<SharedString>,
    code: bool,
}

impl Choice {
    pub fn new(icon: Glyph, label: impl Into<SharedString>) -> Self {
        Self {
            icon,
            label: label.into(),
            secondary: None,
            code: false,
        }
    }

    /// A quieter line under the label.
    pub fn secondary(mut self, text: impl Into<SharedString>) -> Self {
        self.secondary = Some(text.into());
        self
    }

    /// Set the label in the code font.
    pub fn code(mut self, code: bool) -> Self {
        self.code = code;
        self
    }
}

/// Choices under one heading.
pub struct Group {
    label: SharedString,
    choices: Vec<Choice>,
}

impl Group {
    pub fn new(label: impl Into<SharedString>, choices: Vec<Choice>) -> Self {
        Self {
            label: label.into(),
            choices,
        }
    }
}

/// The list under a field. Rows are counted across the groups, in order.
#[derive(IntoElement)]
pub struct Listbox {
    id: SharedString,
    anchor: Bounds<Pixels>,
    groups: Vec<Group>,
    highlighted: Option<usize>,
    empty: SharedString,
    on_select: Option<OnIndex>,
}

impl Listbox {
    /// `anchor` is where the field is drawn.
    pub fn new(id: impl Into<SharedString>, anchor: Bounds<Pixels>) -> Self {
        Self {
            id: id.into(),
            anchor,
            groups: Vec::new(),
            highlighted: None,
            empty: "No options".into(),
            on_select: None,
        }
    }

    /// Add a group; a group with no choices is not shown.
    pub fn group(mut self, group: Group) -> Self {
        self.groups.push(group);
        self
    }

    /// The row the keys act on; none while the keys act on the field.
    pub fn highlighted(mut self, highlighted: Option<usize>) -> Self {
        self.highlighted = highlighted;
        self
    }

    /// What the list says when no group has a choice: MUI's `noOptionsText`.
    pub fn empty(mut self, text: impl Into<SharedString>) -> Self {
        self.empty = text.into();
        self
    }

    /// Called with a row's place when it is clicked.
    pub fn on_select(mut self, on_select: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(on_select));
        self
    }
}

/// The scroll position of a list, and the row it last brought into view.
struct Scroll {
    handle: ScrollHandle,
    shown: Option<usize>,
}

impl RenderOnce for Listbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let max_height = window.viewport_size().height * HEIGHT_SHARE;
        let named = |name: &'static str| {
            ElementId::NamedChild(Arc::new(ElementId::Name(self.id.clone())), name.into())
        };

        let mut rows = div().py(px(LIST_PADDING));
        // The place of the highlighted row among the list's children, for
        // the scroll handle, which counts headings too.
        let (mut index, mut child, mut highlighted_child) = (0, 0, None);
        for group in self.groups {
            if group.choices.is_empty() {
                continue;
            }
            rows = rows.child(list::subheader(group.label, cx));
            child += 1;
            for choice in group.choices {
                let highlighted = self.highlighted == Some(index);
                if highlighted {
                    highlighted_child = Some(child);
                }
                let on_select = self.on_select.clone();
                let row = ListItemButton::new(
                    ElementId::NamedInteger(self.id.clone(), index as u64),
                    choice.label,
                )
                .icon(choice.icon)
                .code(choice.code)
                .highlighted(highlighted)
                .when_some(choice.secondary, ListItemButton::secondary)
                .on_click(move |_, window, cx| {
                    if let Some(on_select) = &on_select {
                        on_select(index, window, cx);
                    }
                });
                rows = rows.child(row);
                index += 1;
                child += 1;
            }
        }
        if index == 0 {
            rows = rows.child(
                div()
                    .py(px(EMPTY_PADDING.0 - LIST_PADDING))
                    .px(px(EMPTY_PADDING.1))
                    .text_color(muted)
                    .child(self.empty),
            );
        }

        let scroll_id = named("scroll");
        let scroll = window.use_keyed_state(scroll_id.clone(), cx, |_, _| Scroll {
            handle: ScrollHandle::new(),
            shown: None,
        });
        let handle = scroll.update(cx, |scroll, _| {
            if scroll.shown != highlighted_child {
                scroll.shown = highlighted_child;
                if let Some(child) = highlighted_child {
                    scroll.handle.scroll_to_item(child);
                }
            }
            scroll.handle.clone()
        });

        let paper = surfaces::paper(ELEVATION, cx)
            .w(self.anchor.size.width)
            .overflow_hidden()
            .occlude()
            // A press on the list, rows or not, leaves the focus in the field.
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
            .child(
                rows.id(scroll_id)
                    .max_h(max_height)
                    .overflow_y_scroll()
                    .track_scroll(&handle),
            )
            .enter(named("in"), Enter::MENU);

        deferred(
            anchored()
                .attach(self.anchor, Anchor::BottomLeft)
                .anchor(Anchor::TopLeft)
                .offset(point(px(0.), px(GAP)))
                .snap_to_window_with_margin(px(WINDOW_MARGIN))
                .child(paper),
        )
        .with_priority(1)
    }
}
