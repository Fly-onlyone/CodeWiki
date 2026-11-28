---
tags: [core, codewiki-app, module]
---

# Doc View Module

> Draws a `Doc` with the `mui` components.

## What it does

Draws a generated page (a `Doc`: title, lead and blocks such as text, tables, cards, stats, settings rows and buttons) with the `mui` components. It does not decide what a link, row or button does. Each one reports an `Action` to the caller.

The page is its own view, `DocPage`, drawn cached, so a ripple or hover in the window's chrome does not rebuild a long table on every frame (that once cost 40% of a processor core). GPUI reuses a cached view only while no cached view around it is redrawn. The app is itself cached inside the backdrop's shell, so the shell draws the page beside the app, and the app tells it where through a `PageSlot`.

## Source

- `crates/codewiki-app/src/doc_view.rs` — module `codewiki_app::doc_view`

## How it works

`strips` cuts a page into strips: the title, one strip for each block, and for a table of more than 12 rows a head strip plus one strip for each row. `DocPage` hands them to a GPUI `list`, which builds only the strips on screen plus 400 px beyond. `draw_strip` adds the page's side gutter from `reading::gutter`, spacing, and a staggered entrance for the first strips.

`draw` turns one block into elements. Controls report through `on_action`. `show` swaps the page: a new page starts at the top, the same page with new content keeps its scroll place (`kept_top` clamps it when the page got shorter). A zoom change re-measures the list.

`column` draws a short page whole with live controls, such as a settings category. Only it has `Controls` for text inputs and selects, since those belong to the view that handles their events. Settings blocks share one row layout: name and help on the left, control on the right, and under it the reason it is locked or the news about its last change.

`line` turns spans into one wrapping run of text. A line with links becomes a `LinkedLine`, which keeps the index of the hovered link, so only that link is lightened and underlined. Clicks map the range index to its `Action`.

Three blocks need more than the page. `Progress` draws a label, a bar and a detail line; a bar with no share and no motion stands still and grey. `Logo` draws the app's logo. `Search` draws Home's search field from a `SearchSlot`, which `DocPage::set_search` takes from the app. The app owns the input and the list under it. The field catches Escape, Up and Down and sends them to the app through `on_key`, since the app's own key handlers never see keys typed beside it.

## Items

- `pub type OnAction = Rc<dyn Fn(&Action, &mut Window, &mut App)>` — what a link, row or button of a page does when used.
- `pub type Shortcuts = Rc<dyn Fn(Div) -> Div>` — gives the page's root the app's shortcut handlers. The shell draws the page beside the app, so a focused input or button in it is not under the app's root, which is where the shortcuts are handled.
- `pub type OnSelect = Rc<dyn Fn(SelectRequest, &mut Window, &mut App)>` — what a click on a select does: its owner opens the list.
- `pub(crate) const TILE_MIN: f32` — the narrowest a stat tile or a theme card gets before the row wraps.
- `pub struct Controls` — what the blocks that hold live controls need from the view that owns them: the text inputs, and the list of a select. Only a page drawn with `column` has these. The page view, `DocPage`, is drawn by the shell from its cache, which suits a report that is read; a text input and a menu belong to the view that handles their events.
- `pub struct SearchSlot` — the input of a page's `Block::Search`, as Home's. The app owns it and handles its events, and draws the list of what it finds under the field. The cached page can hold a text input because the input is a view of its own: what is typed redraws it and the views around it.
- `pub enum SearchKey` — A key typed in a page's search field that acts on its list.
- `pub type OnSearchKey = Rc<dyn Fn(SearchKey, &mut Window, &mut App) -> bool>` — what a key of a page's search field does; see `SearchSlot::on_key`.
- `pub struct SelectRequest` — A select that was clicked: where it is, and what it offers.
- `struct Page` — what every block needs to draw itself.
- `pub struct PageSlot` — where in the window the page goes. The app, which lays the window out, writes it; the shell, which draws the page, reads it.
- `pub fn notify_after_draw(view: EntityId, cx: &mut App)` — ask for `view` to be drawn again, from a place that runs while a frame is being drawn. GPUI wakes the window only for a notify made outside drawing. One made during a frame marks the view and asks for no frame, so the change waited for whatever asked next: the backdrop's drift, or any input. Under reduced motion nothing drifts, and a new page stayed blank until the pointer moved. Deferred, the notify runs once the frame is done, and wakes the window.
- `fn strips(doc: &Doc) -> Vec<Strip>` — the strips of a page, top to bottom.
- `pub struct DocPage` — A generated page as a view of its own, so that it can be drawn cached. The page is a list of strips, and the list builds only the strips on screen. A table of two hundred rows then costs what the thirty in view cost, on every frame of a scroll, a hover or an entrance.
- `fn draw(block: &Block, path: &str, page: &Page, window: &mut Window, cx: &mut App) -> AnyElement` — private function, returns `AnyElement`; called by `column`, `draw_strip`.
- `column` — A short page drawn whole, with live controls: a category of the settings. The caller scrolls it.
- `pub fn choice_tint(choice: &Choice, cx: &App) -> Option<Hsla>` — the color of a choice's icon. A theme's icon is in a color of that theme, as in the theme menu; any other takes the hue its kind has everywhere.
- `fn line(spans: &[Span], path: &str, page: &Page, cx: &App) -> AnyElement` — A line of spans as one run of text that wraps, with its links live.
- `pub fn stat_tile(stat: &Stat, id: ElementId, cx: &App) -> Div` — TradingAgent's metric tile: a label, a large figure that counts up when the tile appears, and a note under it. `id` names the tile.
- `draw_table` — A whole table in one card: for a short table, or one inside a card.

And 32 more items.

## Depends on

- [[Motion Module (mui)]] — uses `EnterExt`, `GENTLE`, `PAGE`, `STAGGER_CAP`
- [[Doc Module]] — uses `Cell`, `Choice`, `Row`, `SecretState`
- [[Feedback Module]] — uses `alert`, `alert_action`, `chip`, `linear_progress`
- [[Surfaces Module]] — uses `CARD_PADDING`, `card`, `card_content`, `card_piece`
- [[Switch Module]] — uses `INSET`, `new`, `on`, `on_change`
- [[Settings Layout Module]] — uses `CONTROL_WIDTH`, `LABEL_WIDTH`, `ROW_GAP`, `ROW_SPACING`
- [[Text Field Module]] — uses `HEIGHT`, `helper`, `new`
- [[Reading Module]] — uses `WIDE_MEASURE`, `gutter`, `link_hover`

## Used by

- [[codewiki-app]] — from `AppView`, `body`, `diagram_viewer_view`, `doc_element`
- [[Backdrop Module]] — from `Shell`, `new`, `render`, `set`

## Gotchas

- `notify_after_draw` defers the notify. A notify made during a frame marks the view but asks for no frame, so with reduced motion a new page stayed blank until the pointer moved. The search field uses it to keep the open list under the field while the page scrolls.
- `PageSlot::set` is cheap each frame. It tells the reader only when the place changed.
- The page ids include a serial, so state kept for one page does not leak into the next.
- `LinkedLine` is used only when a line has a link. Its highlight index points into the highlight list, so keep link and highlight pushes in step.
- The list keeps measured heights. A strip off screen is not re-measured on a zoom change unless `remeasure` is called, which `render` does.
- `DocPage` has no `Controls`. A `Field` or `Secret` input draws nothing there, and a `Select` shows only its chosen label. Pages with those controls must go through `column`.
- `Block::Search` draws nothing without a `SearchSlot`. `column` never has one.
- A table inside a card is never split into row strips, however long it is.

## See also

- [[_index]]
- [[codewiki-app]]
