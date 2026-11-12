//! Typography of the reading pane.
//!
//! Most of what the app shows is long Markdown, so the pane is set for
//! reading: a bounded line length, open line spacing, headings on a clear
//! scale, and code and tables that stand apart from the prose.

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::text::{TextView, TextViewStyle};
use gpui_kit::{App, Overflow, StyleRefinement, Styled as _, px, relative, rems};

/// Widest a line of prose may run, in pixels. About 95 characters of the
/// bundled interface font at its default size: longer than a book's line,
/// because notes carry file paths and diagrams as well as prose.
pub const NOTE_MEASURE: f32 = 680.0;
/// Measure for generated pages. They are mostly tables, which need the room.
pub const WIDE_MEASURE: f32 = 1040.0;
/// Space kept on each side of the text however narrow the pane gets.
const MIN_GUTTER: f32 = 24.0;

/// Line height as a multiple of the font size.
const LINE_HEIGHT: f32 = 1.65;

/// Heading size as a multiple of the body size, by level.
fn heading_scale(level: u8) -> f32 {
    match level {
        1 => 2.0,
        2 => 1.5,
        3 => 1.25,
        4 => 1.1,
        _ => 1.0,
    }
}

/// Side padding that centres a column of `measure` in a pane `pane_width`
/// wide. It is never less than `MIN_GUTTER`.
pub fn gutter(pane_width: f32, measure: f32) -> f32 {
    ((pane_width - measure) / 2.0).max(MIN_GUTTER)
}

fn style(cx: &App) -> TextViewStyle {
    let theme = cx.theme();
    let mut table = StyleRefinement::default();
    // Let a wide table scroll sideways once its columns cannot shrink more.
    table.overflow.x = Some(Overflow::Scroll);
    let mut style = TextViewStyle::default()
        .paragraph_gap(rems(1.0))
        .heading_font_size(|level, base| base * heading_scale(level))
        .code_block(
            StyleRefinement::default()
                .p_3()
                .rounded(theme.radius)
                .border_1()
                .border_color(theme.border),
        )
        .table(table)
        .table_head(StyleRefinement::default().font_weight(gpui_kit::FontWeight::SEMIBOLD))
        .table_cell(StyleRefinement::default().px_3().py_2());
    style.heading_base_font_size = theme.font_size;
    style
}

/// Set `view` for reading in a pane `pane_width` wide, with lines no longer
/// than `measure`.
pub fn frame(view: TextView, pane_width: f32, measure: f32, cx: &App) -> TextView {
    let primary = cx.theme().primary;
    view.style(style(cx))
        .blockquote_style(
            StyleRefinement::default()
                .border_l_3()
                .border_color(primary),
        )
        .line_height(relative(LINE_HEIGHT))
        .px(px(gutter(pane_width, measure)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gutter_centres_the_measure_in_a_wide_pane() {
        assert_eq!(gutter(1000.0, NOTE_MEASURE), 160.0);
        assert_eq!(gutter(1000.0, NOTE_MEASURE) * 2.0 + NOTE_MEASURE, 1000.0);
    }

    #[test]
    fn the_gutter_never_closes_in_a_narrow_pane() {
        assert_eq!(gutter(NOTE_MEASURE, NOTE_MEASURE), MIN_GUTTER);
        assert_eq!(gutter(300.0, NOTE_MEASURE), MIN_GUTTER);
        assert_eq!(gutter(0.0, WIDE_MEASURE), MIN_GUTTER);
    }

    #[test]
    fn headings_shrink_with_depth_and_never_below_body_size() {
        let sizes: Vec<f32> = (1..=6).map(heading_scale).collect();
        assert!(sizes.windows(2).all(|pair| pair[0] >= pair[1]), "{sizes:?}");
        assert!(sizes.iter().all(|&size| size >= 1.0));
        assert!(sizes[0] > sizes[1] && sizes[1] > sizes[2]);
    }
}
