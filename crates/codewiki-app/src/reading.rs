//! Typography of the reading pane.
//!
//! Most of what the app shows is long Markdown, so the pane is set for
//! reading: a bounded line length, open line spacing, headings on a clear
//! scale, and code and tables that stand apart from the prose.
//!
//! Notes are drawn with gpui-kit's base `TextView`, whose style can give a
//! heading a color; the component wrapper only lets a heading change size.
//! The style is therefore built here in full. Headings, list markers, inline
//! code, code blocks, table heads and quotes take the theme's hues.

use gpui_kit::base::{TextView, TextViewStyle};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{
    App, FontWeight, HighlightStyle, Overflow, StyleRefinement, Styled as _, px, relative, rems,
};

use crate::glass;
use crate::icons::Hue;

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

/// Share of the primary color in a code block; a table head takes twice as
/// much.
const CODE_FILL: f32 = 0.1;
/// Opacity of the link color behind inline code.
const INLINE_CODE_FILL: f32 = 0.14;
/// Opacity of the primary color laid over a quote, chosen by eye on
/// 2026-11-13.
const QUOTE_FILL: f32 = 0.14;

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

/// The hue of a heading. The top three levels each take one of the theme's
/// text colors, which the CodeWiki themes hold to AA; deeper levels keep the
/// body color.
fn heading_hue(level: u8) -> Option<Hue> {
    match level {
        1 => Some(Hue::Link),
        2 => Some(Hue::Magenta),
        3 => Some(Hue::Cyan),
        _ => None,
    }
}

/// Side padding that centres a column of `measure` in a pane `pane_width`
/// wide. It is never less than `MIN_GUTTER`.
pub fn gutter(pane_width: f32, measure: f32) -> f32 {
    ((pane_width - measure) / 2.0).max(MIN_GUTTER)
}

/// The text view's whole style: what gpui-component 0.9.1 derives from its
/// theme (`text/mod.rs`, `base_text_view_style`), with this pane's spacing
/// folded in as its `text/compat.rs` folds it, and the theme's hues on top.
fn style(cx: &App) -> TextViewStyle {
    let theme = cx.theme();
    let base_size = theme.font_size;

    let mut table = StyleRefinement::default().rounded(theme.radius);
    // Let a wide table scroll sideways once its columns cannot shrink more.
    table.overflow.x = Some(Overflow::Scroll);
    let code_block = StyleRefinement::default()
        .p_3()
        .rounded(theme.radius)
        .border_1()
        .bg(glass::tint(theme.muted, theme.primary, CODE_FILL))
        .border_color(glass::tinted_border(cx));
    let table_head = StyleRefinement::default()
        .font_weight(FontWeight::SEMIBOLD)
        .bg(glass::tint(
            theme.table_head,
            theme.primary,
            CODE_FILL * 2.0,
        ))
        .text_color(theme.link);
    let inline_code = HighlightStyle {
        color: Some(Hue::Cyan.color(theme)),
        background_color: Some(theme.link.opacity(INLINE_CODE_FILL)),
        ..Default::default()
    };
    let heading_colors = [1, 2, 3].map(|level| heading_hue(level).map(|hue| hue.color(theme)));

    TextViewStyle::default()
        .with_foreground(theme.foreground)
        .with_muted_foreground(theme.muted_foreground)
        .with_link(theme.link)
        .with_selection(theme.selection)
        .with_code_background(theme.muted)
        .with_border(theme.border)
        .with_paragraph_gap(rems(1.0))
        .with_heading(move |level| {
            let heading = StyleRefinement::default().text_size(base_size * heading_scale(level));
            let color = usize::from(level)
                .checked_sub(1)
                .and_then(|index| heading_colors.get(index).copied().flatten());
            match color {
                Some(color) => heading.text_color(color),
                None => heading,
            }
        })
        .with_code_block(code_block)
        .with_table(table)
        .with_table_head(table_head)
        .with_table_cell(StyleRefinement::default().px_3().py_2())
        .with_table_row_hover(theme.table_hover)
        .with_inline_code(inline_code)
        .with_list_style(StyleRefinement::default().text_color(theme.link))
        .with_dark(theme.is_dark())
}

/// Set `view` for reading in a pane `pane_width` wide, with lines no longer
/// than `measure`.
pub fn frame(view: TextView, pane_width: f32, measure: f32, cx: &App) -> TextView {
    let primary = cx.theme().primary;
    let quote = StyleRefinement::default()
        .border_l_3()
        .border_color(primary)
        .py_2()
        .bg(primary.opacity(QUOTE_FILL));
    view.style(style(cx))
        .blockquote_style(quote)
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

    #[test]
    fn the_top_three_heading_levels_each_take_their_own_hue() {
        let hues: Vec<Hue> = (1..=3).map(|level| heading_hue(level).unwrap()).collect();
        assert!(hues[0] != hues[1] && hues[1] != hues[2] && hues[0] != hues[2]);
        assert!((4..=6).all(|level| heading_hue(level).is_none()));
    }
}
