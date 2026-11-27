//! The outline in the details pane: the `##` headings of the note as its
//! body draws them, which of their sections is being read, and the glide of
//! the body to a heading the user clicks.
//!
//! Places are in design units, measured down from the top of the note: what
//! the body's text view reports, divided by one zoomed pixel.

use std::time::Instant;

use gpui_kit::EntityId;
use gpui_kit::base::text::DocumentHeading;

/// How far below the top of the body the line lies that decides which
/// section is being read: a heading at or above it has begun its section.
pub const SPY_LINE: f32 = 32.0;

/// A place in the body as its list keeps it: a block and how far into it the
/// view starts. The glide remembers the one it wrote, to tell it from a
/// scroll of the user's.
pub type Place = (usize, f32);

/// The entries of the outline: the `##` headings of the body.
pub fn entries(headings: Vec<DocumentHeading>) -> Vec<DocumentHeading> {
    headings
        .into_iter()
        .filter(|heading| heading.level == 2)
        .collect()
}

/// The entry whose section is being read, from where each entry's heading
/// starts (`tops`, in order), where the body is scrolled to and how far it
/// can scroll (`max`). The last heading at or above `line` below the top
/// lights its entry; none has before the first heading. At the very bottom
/// the last entry lights, since a short last section never reaches the line.
pub fn spy(tops: &[f32], scroll: f32, max: f32, line: f32) -> Option<usize> {
    if tops.is_empty() {
        return None;
    }
    if max > 0.0 && scroll >= max - 1.0 {
        return Some(tops.len() - 1);
    }
    tops.iter().rposition(|top| *top <= scroll + line)
}

/// Where the body scrolls to so that a heading starting at `top` is at its
/// top, or as far as the body goes when the note ends before that.
pub fn target(top: f32, max: f32) -> f32 {
    top.clamp(0.0, max.max(0.0))
}

/// Where the body is `progress` of the way from `from` to `to`.
pub fn glide_to(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}

/// Whether the body is somewhere the glide did not put it: the user
/// scrolled it, with the wheel, the scrollbar or by selecting text.
pub fn user_moved(written: Option<Place>, now: Place) -> bool {
    written.is_some_and(|(block, offset)| block != now.0 || (offset - now.1).abs() > 0.5)
}

/// A glide of the body to a heading, which keeps the heading's entry lit
/// after it lands, until the user scrolls.
pub struct Jump {
    /// The text view the glide moves: the note, or the preview of the editor.
    pub body: EntityId,
    /// The heading's block in that view.
    pub block_ix: usize,
    pub from: f32,
    pub to: f32,
    pub started: Instant,
    /// The place the glide last put the body at.
    pub written: Option<Place>,
    /// Whether the glide has arrived; the entry stays lit.
    pub landed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown;
    use gpui_kit::base::text::markdown_headings;

    const TOPS: [f32; 3] = [100.0, 400.0, 900.0];

    #[test]
    fn nothing_is_lit_before_the_first_section() {
        assert_eq!(spy(&TOPS, 0.0, 1_000.0, SPY_LINE), None);
        assert_eq!(spy(&TOPS, 100.0 - SPY_LINE - 1.0, 1_000.0, SPY_LINE), None);
    }

    #[test]
    fn the_last_heading_past_the_line_lights_its_entry() {
        // A heading exactly on the line has begun its section.
        assert_eq!(spy(&TOPS, 100.0 - SPY_LINE, 1_000.0, SPY_LINE), Some(0));
        assert_eq!(spy(&TOPS, 399.0 - SPY_LINE, 1_000.0, SPY_LINE), Some(0));
        assert_eq!(spy(&TOPS, 400.0 - SPY_LINE, 1_000.0, SPY_LINE), Some(1));
        assert_eq!(spy(&TOPS, 700.0, 1_000.0, SPY_LINE), Some(1));
    }

    #[test]
    fn the_last_entry_is_lit_at_the_very_bottom() {
        // The last heading never reaches the line, but the note ends.
        assert_eq!(spy(&TOPS, 800.0, 800.0, SPY_LINE), Some(2));
        assert_eq!(spy(&TOPS, 799.5, 800.0, SPY_LINE), Some(2));
        assert_eq!(spy(&TOPS, 790.0, 800.0, SPY_LINE), Some(1));
    }

    #[test]
    fn a_note_that_fits_its_view_lights_by_the_line_alone() {
        assert_eq!(spy(&[0.0, 300.0], 0.0, 0.0, SPY_LINE), Some(0));
        assert_eq!(spy(&[200.0, 300.0], 0.0, 0.0, SPY_LINE), None);
    }

    #[test]
    fn no_headings_light_nothing() {
        assert_eq!(spy(&[], 0.0, 1_000.0, SPY_LINE), None);
        assert_eq!(spy(&[], 1_000.0, 1_000.0, SPY_LINE), None);
    }

    #[test]
    fn a_jump_puts_the_heading_at_the_top_or_goes_as_far_as_the_note_does() {
        assert_eq!(target(400.0, 1_000.0), 400.0);
        assert_eq!(target(900.0, 800.0), 800.0);
        assert_eq!(target(-5.0, 800.0), 0.0);
        // A note that fits its view does not move.
        assert_eq!(target(300.0, 0.0), 0.0);
    }

    #[test]
    fn a_glide_runs_from_where_the_body_was_to_the_heading() {
        assert_eq!(glide_to(100.0, 500.0, 0.0), 100.0);
        assert_eq!(glide_to(100.0, 500.0, 0.5), 300.0);
        assert_eq!(glide_to(100.0, 500.0, 1.0), 500.0);
        // Up the note as well as down it.
        assert_eq!(glide_to(500.0, 100.0, 0.25), 400.0);
    }

    #[test]
    fn only_a_place_the_glide_did_not_write_is_a_scroll_of_the_users() {
        assert!(!user_moved(None, (3, 10.0)));
        assert!(!user_moved(Some((3, 10.0)), (3, 10.0)));
        assert!(!user_moved(Some((3, 10.0)), (3, 10.4)));
        assert!(user_moved(Some((3, 10.0)), (3, 12.0)));
        assert!(user_moved(Some((3, 10.0)), (4, 10.0)));
    }

    #[test]
    fn the_outline_is_the_level_two_headings_the_body_draws() {
        let note = "---\ntags: [core]\n---\n\n# Title\n\n> Summary.\n\n## Source\n\n- `a.rs`\n\n## How [[Doc Module|the doc]] works\n\nText with **bold**.\n\n### Detail\n\n## `code` and **bold**\n\n```text\n## not a heading\n```\n\n> ## quoted\n\n## Gotchas\n";
        let outline: Vec<(String, usize)> =
            entries(markdown_headings(&markdown::prepare_note(note)))
                .into_iter()
                .map(|heading| (heading.text.to_string(), heading.block_ix))
                .collect();
        // Blocks: the title, its tags, the summary, then each section's
        // heading and body; the quoted heading is inside its quote.
        assert_eq!(
            outline,
            [
                ("Source".to_string(), 3),
                ("How the doc works".to_string(), 5),
                ("code and bold".to_string(), 8),
                ("Gotchas".to_string(), 11),
            ]
        );
    }
}
