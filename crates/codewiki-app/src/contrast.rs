//! WCAG contrast arithmetic for translucent surfaces.
//!
//! The window is glass: what sits behind a surface is not known. These
//! functions answer "is this text still readable if the backdrop is as bad as
//! it can be", which for any backdrop color is bounded by black and white.

/// An sRGB color, each channel in `0.0..=1.0`.
pub type Rgb = [f32; 3];

pub const BLACK: Rgb = [0.0; 3];
pub const WHITE: Rgb = [1.0; 3];

/// The contrast ratio WCAG AA asks of body text.
pub const AA: f32 = 4.5;

/// The least opaque the reading pane may be. Every text color of the CodeWiki
/// themes is checked at this value; at 0.85 the status colors of the light
/// themes fall below AA.
pub const PANE_FLOOR: f32 = 0.88;

/// Parse `#RRGGBB`. An alpha pair after it is ignored.
pub fn parse_hex(hex: &str) -> Option<Rgb> {
    let digits = hex.strip_prefix('#')?;
    if digits.len() != 6 && digits.len() != 8 {
        return None;
    }
    let channel = |at: usize| {
        u8::from_str_radix(digits.get(at..at + 2)?, 16)
            .ok()
            .map(|value| f32::from(value) / 255.0)
    };
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// `top` at `alpha` painted over an opaque `under`.
pub fn over(top: Rgb, alpha: f32, under: Rgb) -> Rgb {
    std::array::from_fn(|i| top[i] * alpha + under[i] * (1.0 - alpha))
}

fn luminance(color: Rgb) -> f32 {
    let linear = |c: f32| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color[0]) + 0.7152 * linear(color[1]) + 0.0722 * linear(color[2])
}

/// WCAG contrast ratio between two opaque colors, from 1 to 21.
pub fn ratio(a: Rgb, b: Rgb) -> f32 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// The lowest contrast `text` can have on `surface` painted at `alpha` over
/// a backdrop that lies between the two given colors.
pub fn worst_ratio(text: Rgb, surface: Rgb, alpha: f32, backdrops: [Rgb; 2]) -> f32 {
    let [a, b] = backdrops;
    ratio(text, over(surface, alpha, a)).min(ratio(text, over(surface, alpha, b)))
}

/// A theme whose text is below AA even on its opaque surface cannot be held
/// to AA. Glass may cost such a text at most this share of what it has.
const KEEP: f32 = 0.9;

/// The lowest opacity, from `floor` up, at which every color in `texts` stays
/// readable on `surface` over any backdrop between the two given colors.
///
/// Readable means AA, or `KEEP` of the text's own contrast where that is
/// lower. Returns 1.0 when no translucent surface manages it.
pub fn fit_alpha(surface: Rgb, texts: &[Rgb], floor: f32, backdrops: [Rgb; 2]) -> f32 {
    const STEP: f32 = 0.02;
    let readable = |alpha: f32| {
        texts.iter().all(|&text| {
            let required = AA.min(ratio(text, surface) * KEEP);
            worst_ratio(text, surface, alpha, backdrops) >= required
        })
    };
    let mut alpha = floor;
    while alpha < 1.0 {
        if readable(alpha) {
            return alpha;
        }
        alpha += STEP;
    }
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_with_and_without_alpha() {
        assert_eq!(parse_hex("#ffffff"), Some(WHITE));
        assert_eq!(parse_hex("#00000080"), Some(BLACK));
        assert_eq!(parse_hex("ffffff"), None);
        assert_eq!(parse_hex("#fff"), None);
        assert_eq!(parse_hex("#gggggg"), None);
    }

    #[test]
    fn ratio_matches_the_wcag_reference_values() {
        assert!((ratio(BLACK, WHITE) - 21.0).abs() < 0.01);
        assert!((ratio(WHITE, WHITE) - 1.0).abs() < 0.01);
        // #767676 on white is the well-known 4.54:1 boundary gray.
        let gray = parse_hex("#767676").unwrap();
        assert!((ratio(gray, WHITE) - 4.54).abs() < 0.01);
    }

    #[test]
    fn an_opaque_surface_ignores_the_backdrop() {
        let surface = parse_hex("#0E0C1A").unwrap();
        assert_eq!(over(surface, 1.0, WHITE), surface);
        let text = parse_hex("#E9E7F7").unwrap();
        let any = [BLACK, WHITE];
        assert!((worst_ratio(text, surface, 1.0, any) - ratio(text, surface)).abs() < 0.001);
    }

    #[test]
    fn a_translucent_surface_is_judged_by_its_worse_backdrop() {
        let dark = parse_hex("#0E0C1A").unwrap();
        let text = parse_hex("#E9E7F7").unwrap();
        // Light text on dark glass is worst over a white backdrop.
        let worst = worst_ratio(text, dark, 0.5, [BLACK, WHITE]);
        assert!((worst - ratio(text, over(dark, 0.5, WHITE))).abs() < 0.001);
        assert!(worst < ratio(text, over(dark, 0.5, BLACK)));
        // A backdrop known to be dark costs far less.
        assert!(worst_ratio(text, dark, 0.5, [BLACK, [0.25; 3]]) > worst);
    }

    #[test]
    fn fit_alpha_returns_the_floor_when_the_floor_is_readable() {
        let surface = parse_hex("#0E0C1A").unwrap();
        let text = parse_hex("#E9E7F7").unwrap();
        assert_eq!(fit_alpha(surface, &[text], 0.5, [BLACK, BLACK]), 0.5);
    }

    #[test]
    fn fit_alpha_rises_until_the_weakest_text_is_readable() {
        let surface = parse_hex("#0E0C1A").unwrap();
        let strong = parse_hex("#E9E7F7").unwrap();
        let weak = parse_hex("#A29DBF").unwrap();
        let any = [BLACK, WHITE];
        let for_strong = fit_alpha(surface, &[strong], 0.3, any);
        let for_both = fit_alpha(surface, &[strong, weak], 0.3, any);
        assert!(for_both > for_strong);
        assert!(worst_ratio(weak, surface, for_both, any) >= AA);
        // One step clearer would not do.
        assert!(worst_ratio(weak, surface, for_both - 0.02, any) < AA);
    }

    #[test]
    fn fit_alpha_asks_less_of_a_text_that_is_weak_even_when_opaque() {
        // About 3:1 on its own surface: AA is out of reach at any opacity.
        let surface = parse_hex("#1a1b26").unwrap();
        let weak = parse_hex("#565f89").unwrap();
        assert!(ratio(weak, surface) < AA);
        let alpha = fit_alpha(surface, &[weak], 0.5, [BLACK, WHITE]);
        assert!(alpha < 1.0, "glass is still possible");
        assert!(worst_ratio(weak, surface, alpha, [BLACK, WHITE]) >= ratio(weak, surface) * 0.9);
    }
}
