//! `Menu`: a short list of choices that opens beside the control that asked
//! for it.
//!
//! Anatomy and behaviour are MUI's (`Menu/Menu.js`, `Popover/Popover.js`,
//! `MenuList/MenuList.js`, `MenuItem/MenuItem.js`); TradingAgent changes none
//! of it. The surface is a `Paper` at elevation 8 that hangs under its
//! anchor and stays inside the window. A click outside it, or Escape, closes
//! it. The arrow keys, Home and End move a highlight that wraps at the ends,
//! and Enter or Space choose the highlighted row.
//!
//! The owner keeps the state: whether the menu is open, which row is
//! highlighted, and the focus handle the menu takes its keys through. Tab is
//! bound by the kit's root, which moves the focus on; the owner closes the
//! menu when its handle loses the focus, as MUI closes a menu on Tab.
//!
//! Three things MUI does are left out. Its `Grow` entrance scales the paper,
//! which GPUI cannot do, so the menu fades and drops into place instead. Its
//! subheaders stick to the top while their group scrolls; GPUI has no sticky
//! position. And typing a letter does not jump to a row.

use std::rc::Rc;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Anchor, AnyElement, App, Bounds, Div, ElementId, FocusHandle, Hsla, InteractiveElement as _,
    IntoElement, KeyDownEvent, MouseButton, ParentElement as _, Pixels, RenderOnce, ScrollHandle,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, anchored, deferred, div,
    point,
};

use super::list::{self, ListItemButton};
use super::motion::{Enter, EnterExt as _};
use super::surfaces;
use crate::icons::Glyph;
use crate::zoom::px;

/// `Popover.js`: `elevation = 8`.
const ELEVATION: usize = 8;
/// `Popover.js`: `marginThreshold = 16`, the least distance to the window's
/// edge.
const WINDOW_MARGIN: f32 = 16.0;
/// `Menu.js`: `maxHeight: 'calc(100% - 96px)'`.
const HEIGHT_INSET: f32 = 96.0;
/// `List.js`: `paddingTop: 8, paddingBottom: 8`.
const LIST_PADDING: f32 = 8.0;
/// Between the anchor and the menu. MUI lays a menu over its anchor; here it
/// hangs under it, so the control that opened it stays in view.
const GAP: f32 = 4.0;
/// MUI sizes a menu to its widest row. A row here is a flex line that
/// truncates, so the menu has a width of its own.
const WIDTH: f32 = 300.0;

/// What a chosen row does: it is given the row's place in the menu.
type OnIndex = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type OnClose = Rc<dyn Fn(&mut Window, &mut App)>;

/// A key that moves the highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Down,
    Up,
    Home,
    End,
}

/// The row the highlight moves to from `current` in a menu of `count` rows.
/// `MenuList.js`: `disableListWrap = false`, so it wraps at both ends.
pub fn step(current: usize, count: usize, key: Key) -> usize {
    if count == 0 {
        return 0;
    }
    let current = current.min(count - 1);
    match key {
        Key::Down => (current + 1) % count,
        Key::Up => (current + count - 1) % count,
        Key::Home => 0,
        Key::End => count - 1,
    }
}

/// Where a control is in the window, so that a menu can open beside it.
///
/// Wrap the control with [`AnchorBounds::track`]; its bounds are then known
/// from the first frame it is drawn in.
#[derive(Clone, Default)]
pub struct AnchorBounds(Rc<std::cell::Cell<Option<Bounds<Pixels>>>>);

impl AnchorBounds {
    /// The control's bounds in the last frame, once it has been drawn.
    pub fn get(&self) -> Option<Bounds<Pixels>> {
        self.0.get()
    }

    /// `control` in an element that reports where it is drawn. `moved` is
    /// called, while a frame is being drawn, whenever that changes.
    pub fn track(
        &self,
        control: impl IntoElement,
        moved: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Div {
        let bounds = self.0.clone();
        div()
            .flex_shrink_0()
            .on_children_prepainted(move |children, window, cx| {
                let now = children.first().copied();
                if bounds.get() != now {
                    bounds.set(now);
                    moved(window, cx);
                }
            })
            .child(control)
    }
}

/// One choice.
pub struct MenuItem {
    label: SharedString,
    icon: Option<Glyph>,
    tint: Option<Hsla>,
    selected: bool,
    trailing: Option<AnyElement>,
}

impl MenuItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            tint: None,
            selected: false,
            trailing: None,
        }
    }

    pub fn icon(mut self, icon: Glyph) -> Self {
        self.icon = Some(icon);
        self
    }

    /// The color of the icon; see [`ListItemButton::tint`].
    pub fn tint(mut self, tint: Option<Hsla>) -> Self {
        self.tint = tint;
        self
    }

    /// Mark the row as the choice in effect.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Something at the far end of the row.
    pub fn trailing(mut self, trailing: impl IntoElement) -> Self {
        self.trailing = Some(trailing.into_any_element());
        self
    }
}

/// A group of choices, under a heading when it has one.
pub struct MenuSection {
    label: Option<SharedString>,
    items: Vec<MenuItem>,
}

impl MenuSection {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: Some(label.into()),
            items: Vec::new(),
        }
    }

    /// A group with no heading.
    pub fn plain() -> Self {
        Self {
            label: None,
            items: Vec::new(),
        }
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.items.extend(items);
        self
    }
}

/// Which edge of its anchor a menu lines up with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// The left edges meet: the menu of a select, which is as wide as the
    /// select (`Select/SelectInput.js`: centred under it, and at least as
    /// wide).
    Start,
    /// The right edges meet: a menu opened by a button at the end of a bar.
    #[default]
    End,
}

/// A menu that hangs under `anchor`.
#[derive(IntoElement)]
pub struct Menu {
    id: SharedString,
    anchor: Bounds<Pixels>,
    focus: FocusHandle,
    sections: Vec<MenuSection>,
    width: f32,
    align: Align,
    highlighted: usize,
    open: bool,
    shown: f32,
    on_select: Option<OnIndex>,
    on_highlight: Option<OnIndex>,
    on_close: Option<OnClose>,
}

impl Menu {
    /// `focus` is the handle the menu takes its keys through; give it the
    /// focus when the menu opens.
    pub fn new(id: impl Into<SharedString>, anchor: Bounds<Pixels>, focus: &FocusHandle) -> Self {
        Self {
            id: id.into(),
            anchor,
            focus: focus.clone(),
            sections: Vec::new(),
            width: WIDTH,
            align: Align::default(),
            highlighted: 0,
            open: true,
            shown: 1.0,
            on_select: None,
            on_highlight: None,
            on_close: None,
        }
    }

    pub fn section(mut self, section: MenuSection) -> Self {
        self.sections.push(section);
        self
    }

    /// The width of the menu, in pixels.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Which edge of the anchor the menu lines up with.
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// The row the keys act on, counted through every section.
    pub fn highlighted(mut self, highlighted: usize) -> Self {
        self.highlighted = highlighted;
        self
    }

    /// Whether the menu takes input. A menu that is fading out is still
    /// drawn, with `open` false.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// How visible the menu is, from 0 to 1, while it closes.
    pub fn shown(mut self, shown: f32) -> Self {
        self.shown = shown;
        self
    }

    /// Called with a row's place when it is clicked or chosen with a key.
    pub fn on_select(mut self, on_select: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(on_select));
        self
    }

    /// Called with the row a key moved the highlight to.
    pub fn on_highlight(
        mut self,
        on_highlight: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_highlight = Some(Rc::new(on_highlight));
        self
    }

    /// Called on Escape and on a click outside the menu.
    pub fn on_close(mut self, on_close: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(on_close));
        self
    }
}

/// The scroll position of a menu, and the row it last brought into view.
struct Scroll {
    handle: ScrollHandle,
    shown: Option<usize>,
}

impl RenderOnce for Menu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let count: usize = self.sections.iter().map(|s| s.items.len()).sum();
        let highlighted = self.highlighted.min(count.saturating_sub(1));
        let open = self.open;
        let max_height = (window.viewport_size().height - px(HEIGHT_INSET)).max(px(0.));

        let mut rows = div().py(px(LIST_PADDING));
        // The place of the highlighted row among the list's children, for
        // the scroll handle, which counts headings too.
        let (mut index, mut child, mut highlighted_child) = (0, 0, 0);
        for section in self.sections {
            if let Some(label) = section.label {
                rows = rows.child(list::subheader(label, cx));
                child += 1;
            }
            for item in section.items {
                if index == highlighted {
                    highlighted_child = child;
                }
                let on_select = self.on_select.clone();
                let row = ListItemButton::new(
                    ElementId::NamedInteger(self.id.clone(), index as u64),
                    item.label,
                )
                .tint(item.tint)
                .selected(item.selected)
                .highlighted(open && index == highlighted)
                .when_some(item.icon, ListItemButton::icon)
                .when_some(item.trailing, ListItemButton::trailing)
                .when(open, |row| {
                    row.on_click(move |_, window, cx| {
                        if let Some(on_select) = &on_select {
                            on_select(index, window, cx);
                        }
                    })
                });
                rows = rows.child(row);
                index += 1;
                child += 1;
            }
        }

        let scroll_id = ElementId::NamedChild(
            std::sync::Arc::new(ElementId::Name(self.id.clone())),
            "scroll".into(),
        );
        let scroll = window.use_keyed_state(scroll_id.clone(), cx, |_, _| Scroll {
            handle: ScrollHandle::new(),
            shown: None,
        });
        let handle = scroll.update(cx, |scroll, _| {
            if scroll.shown != Some(highlighted) {
                scroll.shown = Some(highlighted);
                scroll.handle.scroll_to_item(highlighted_child);
            }
            scroll.handle.clone()
        });

        let (on_select, on_highlight, on_close) =
            (self.on_select, self.on_highlight, self.on_close.clone());
        let on_key = move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
            if event.keystroke.modifiers.modified() {
                return;
            }
            let moved = |key| step(highlighted, count, key);
            let (highlight, select, close) = match event.keystroke.key.as_str() {
                "down" => (Some(moved(Key::Down)), false, false),
                "up" => (Some(moved(Key::Up)), false, false),
                "home" => (Some(moved(Key::Home)), false, false),
                "end" => (Some(moved(Key::End)), false, false),
                "enter" | "space" => (None, count > 0, false),
                "escape" => (None, false, true),
                _ => return,
            };
            cx.stop_propagation();
            if let (Some(row), Some(on_highlight)) = (highlight, &on_highlight) {
                on_highlight(row, window, cx);
            }
            if let (true, Some(on_select)) = (select, &on_select) {
                on_select(highlighted, window, cx);
            }
            if let (true, Some(on_close)) = (close, &on_close) {
                on_close(window, cx);
            }
        };

        let paper = surfaces::paper(ELEVATION, cx)
            .w(px(self.width))
            .when(open, |paper| {
                paper
                    .track_focus(&self.focus)
                    .on_key_down(on_key)
                    // A press on the menu is not a press outside it.
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            })
            .child(
                rows.id(scroll_id)
                    .max_h(max_height)
                    .overflow_y_scroll()
                    .track_scroll(&handle),
            )
            .enter(
                ElementId::NamedChild(std::sync::Arc::new(ElementId::Name(self.id)), "in".into()),
                Enter::MENU,
            );

        let on_close = self.on_close;
        // The corner of the anchor the menu hangs from, and its own corner
        // that meets it.
        let (attach, corner) = match self.align {
            Align::Start => (Anchor::BottomLeft, Anchor::TopLeft),
            Align::End => (Anchor::BottomRight, Anchor::TopRight),
        };
        // MUI's `Modal` under a menu: a backdrop nobody sees, which takes the
        // click that closes it.
        let scrim = div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .opacity(self.shown.clamp(0.0, 1.0))
            .when(open, |scrim| {
                scrim
                    .occlude()
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        if let Some(on_close) = &on_close {
                            on_close(window, cx);
                        }
                    })
            })
            .child(
                anchored()
                    .attach(self.anchor, attach)
                    .anchor(corner)
                    .offset(point(px(0.), px(GAP)))
                    .snap_to_window_with_margin(px(WINDOW_MARGIN))
                    .child(paper),
            );
        deferred(scrim).with_priority(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_highlight_wraps_at_both_ends() {
        assert_eq!(step(0, 3, Key::Down), 1);
        assert_eq!(step(2, 3, Key::Down), 0);
        assert_eq!(step(0, 3, Key::Up), 2);
        assert_eq!(step(2, 3, Key::Up), 1);
        assert_eq!(step(1, 3, Key::Home), 0);
        assert_eq!(step(1, 3, Key::End), 2);
    }

    #[test]
    fn a_highlight_past_the_end_counts_as_the_last_row() {
        assert_eq!(step(9, 3, Key::Down), 0);
        assert_eq!(step(9, 3, Key::Up), 1);
    }

    #[test]
    fn an_empty_menu_has_nowhere_to_go() {
        for key in [Key::Down, Key::Up, Key::Home, Key::End] {
            assert_eq!(step(4, 0, key), 0);
        }
    }

    #[test]
    fn a_menu_row_is_as_tall_as_mui_makes_it() {
        // `MenuItem.js`: 6 px above and below one line of `body1`, 24 px.
        assert_eq!(list::ROW_HEIGHT, 6.0 + 24.0 + 6.0);
        // `Menu.js` keeps 48 px clear above and below the tallest menu.
        assert_eq!(HEIGHT_INSET, 2.0 * 48.0);
    }
}
