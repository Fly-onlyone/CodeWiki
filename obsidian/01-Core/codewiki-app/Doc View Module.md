---
tags: [core, codewiki-app, module]
---

# Doc View Module

> Draws a `Doc` with the `mui` components.

## What it does

The page is a view of its own, `DocPage`, drawn cached. A ripple or a hover in the window's chrome then redraws the chrome and leaves the page as it was; before this, an animation anywhere rebuilt the page on every frame, which on a long table took 40% of a processor core.

GPUI reuses a cached view only while no cached view around it is being redrawn. The app is itself drawn cached, inside the backdrop's shell, so a page drawn inside the app would be rebuilt with it. The shell therefore draws the page, beside the app: the app leaves an empty place for it and says where that is through a `PageSlot`.

A page is a column of blocks, as wide as a table needs and no wider. What a link, a row or a button does is not decided here: every one reports its `Action` to the caller.

## Source

- `crates/codewiki-app/src/doc_view.rs` — module `codewiki_app::doc_view`

## How it works

1. `AppView::diagram_viewer_view` and `AppView::render` in [[codewiki-app]], and 1 more, call `notify_after_draw`.
2. `AppView::new` in [[codewiki-app]] calls `DocPage::new`. `DocPage::new` calls `strips`, then `factor` in [[Zoom Module]].
3. `AppView::settings_frame` in [[codewiki-app]] calls `column`. `column` calls `px` in [[Zoom Module]], then `text` in [[Surfaces Module]], then `Enter::card` in [[Motion Module (mui)]], then `draw`. `draw` calls `text` in [[Surfaces Module]], then `px` in [[Zoom Module]], then `line`, then `stat_tile`, then `draw_table`, then `card_content` in [[Surfaces Module]], and 18 more.
4. `AppView::settings_menu_view` in [[codewiki-app]] calls `choice_tint`. `choice_tint` calls `entries`, then `glyph_tint` (both in [[Themes Module]]), then `tint` in [[Icons Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub type OnAction = Rc<dyn Fn(&Action, &mut Window, &mut App)>` — what a link, row or button of a page does when used.
- `pub type OnSelect = Rc<dyn Fn(SelectRequest, &mut Window, &mut App)>` — what a click on a select does: its owner opens the list.
- `pub(crate) const TILE_MIN: f32` — the narrowest a stat tile or a theme card gets before the row wraps.
- `pub struct Controls` — what the blocks that hold live controls need from the view that owns them: the text inputs, and the list of a select. Only a page drawn with `column` has these. The page view, `DocPage`, is drawn by the shell from its cache, which suits a report that is read; a text input and a menu belong to the view that handles their events.
- `pub struct SelectRequest` — A select that was clicked: where it is, and what it offers.
- `struct Page` — what every block needs to draw itself.
- `pub struct PageSlot` — where in the window the page goes. The app, which lays the window out, writes it; the shell, which draws the page, reads it.
- `pub fn notify_after_draw(view: EntityId, cx: &mut App)` — ask for `view` to be drawn again, from a place that runs while a frame is being drawn. GPUI wakes the window only for a notify made outside drawing. One made during a frame marks the view and asks for no frame, so the change waited for whatever asked next: the backdrop's drift, or any input. Under reduced motion nothing drifts, and a new page stayed blank until the pointer moved. Deferred, the notify runs once the frame is done, and wakes the window.
- `enum Strip` — one strip of a page, as the list scrolls it: the title, a whole block, or a piece of a long table.
- `fn strips(doc: &Doc) -> Vec<Strip>` — the strips of a page, top to bottom.
- `pub struct DocPage` — A generated page as a view of its own, so that it can be drawn cached. The page is a list of strips, and the list builds only the strips on screen. A table of two hundred rows then costs what the thirty in view cost, on every frame of a scroll, a hover or an entrance.
- `fn draw(block: &Block, path: &str, page: &Page, window: &mut Window, cx: &mut App) -> AnyElement` — private function, returns `AnyElement`; called by `column`, `draw_strip`.
- `column` — A short page drawn whole, with live controls: a category of the settings. The caller scrolls it.
- `fn under(locked: &Option<String>, note: &Option<Note>) -> Option<(String, Tone)>` — what stands under a control: why it cannot be changed, or else the news about its last change.
- `row` — `setting_row`, with the choice of how wide the control's side is: `fill` gives it all the room that is left, for a value that is shown and may be long, such as a folder.
- `pub fn choice_tint(choice: &Choice, cx: &App) -> Option<Hsla>` — the color of a choice's icon. A theme's icon is in a color of that theme, as in the theme menu; any other takes the hue its kind has everywhere.
- `fn line(spans: &[Span], path: &str, page: &Page, cx: &App) -> AnyElement` — A line of spans as one run of text that wraps, with its links live.
- `pub fn stat_tile(stat: &Stat, id: ElementId, cx: &App) -> Div` — TradingAgent's metric tile: a label, a large figure that counts up when the tile appears, and a note under it. `id` names the tile.
- `fn table_cell(table: &Table, index: usize) -> Div` — A cell of column `index`, taking that column's share of the width.
- `draw_table` — A whole table in one card: for a short table, or one inside a card.

And 22 more items.

## Depends on

- [[Motion Module (mui)]] — uses `EnterExt`, `GENTLE`, `PAGE`, `STAGGER_CAP`
- [[Doc Module]] — uses `Cell`, `Choice`, `Row`, `SecretState`
- [[Feedback Module]] — uses `alert`, `alert_action`, `chip`, `linear_progress`
- [[Surfaces Module]] — uses `CARD_PADDING`, `card`, `card_content`, `card_piece`
- [[Switch Module]] — uses `INSET`, `new`, `on`, `on_change`
- [[Settings Layout Module]] — uses `CONTROL_WIDTH`, `LABEL_WIDTH`, `ROW_GAP`, `ROW_SPACING`
- [[Text Field Module]] — uses `HEIGHT`, `helper`, `new`
- [[Button]] — uses `contained`, `error`, `new`

## Used by

- [[codewiki-app]] — from `AppView`, `diagram_viewer_view`, `doc_element`, `graph_view`
- [[Backdrop Module]] — from `Shell`, `new`, `render`
- [[Menu Module]] — from `track`
- [[Win32 Module]] — from `on_console_stop`
- [[Zoom Module]] — from `set_factor`

## See also

- [[_index]]
- [[codewiki-app]]
