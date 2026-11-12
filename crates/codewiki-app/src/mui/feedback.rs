//! `Alert`, `Chip`, `LinearProgress` and the tooltip: how the app tells the
//! user something.
//!
//! Sizes are MUI's (`Alert/Alert.js`, `Chip/Chip.js`,
//! `LinearProgress/LinearProgress.js`, `Tooltip/Tooltip.js`); colors and
//! corners are TradingAgent's (`components.ts`).

use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex};
use gpui_kit::{
    Animation, AnimationExt as _, AnyView, App, Div, FontWeight, Hsla, IntoElement,
    ParentElement as _, SharedString, Styled as _, Window, div, px, relative,
};

use super::motion::scaled;
use super::tokens::easing;
use crate::icons;

/// What kind of news something is. MUI's `severity`, and the `color` of a
/// chip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    /// No particular kind.
    #[default]
    Neutral,
    Primary,
    Success,
    Info,
    Warning,
    Error,
}

impl Tone {
    /// The color that stands for this tone.
    pub fn color(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        match self {
            Self::Neutral => theme.muted_foreground,
            Self::Primary => theme.primary,
            Self::Success => theme.success,
            Self::Info => theme.info,
            Self::Warning => theme.warning,
            Self::Error => theme.danger,
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Success => icons::SUCCESS,
            Self::Warning => icons::WARNING,
            Self::Error => icons::ERROR,
            Self::Neutral | Self::Primary | Self::Info => icons::INFO,
        }
    }
}

/// `MuiAlert`, `standard`: the tone at 15% behind text in the tone's color.
const ALERT_FILL: f32 = 0.15;
/// `Alert.js`: `padding: '6px 16px'`, a 22 px icon 12 px before the message,
/// which has 8 px above and below it.
const ALERT_PADDING: (f32, f32) = (16.0, 6.0);
const ALERT_ICON: f32 = 22.0;
const ALERT_GAP: f32 = 12.0;
const ALERT_FONT: f32 = 14.0;

/// A message in a tinted box with an icon: MUI's standard `Alert`.
pub fn alert(tone: Tone, message: impl Into<SharedString>, cx: &App) -> Div {
    let color = tone.color(cx);
    h_flex()
        .w_full()
        .px(px(ALERT_PADDING.0))
        .py(px(ALERT_PADDING.1))
        .gap(px(ALERT_GAP))
        .items_start()
        .rounded(cx.theme().radius)
        .bg(color.opacity(ALERT_FILL))
        .text_color(cx.theme().foreground)
        .text_size(px(ALERT_FONT))
        .line_height(px(ALERT_FONT * 1.43))
        .child(
            div().py(px(7.)).child(
                Icon::new(tone.icon())
                    .with_size(px(ALERT_ICON))
                    .text_color(color),
            ),
        )
        .child(div().flex_1().min_w_0().py(px(8.)).child(message.into()))
}

/// `Chip.js`, `size: 'small'`: 24 px high, with 8 px beside a 13 px label.
const CHIP_SMALL: (f32, f32) = (24.0, 8.0);
const CHIP_FONT: f32 = 13.0;
/// `MuiChip`: `borderRadius: max(br - 2, 4)`.
const CHIP_RADIUS_INSET: f32 = 2.0;
/// `MuiChip`, `filled`: the color at 15% behind the label.
const CHIP_FILL: f32 = 0.15;

/// A small label in a rounded box, at MUI's small size. Outlined, as most of
/// TradingAgent's are, unless `filled`.
pub fn chip(label: impl Into<SharedString>, tone: Tone, filled: bool, cx: &App) -> Div {
    let (height, padding) = CHIP_SMALL;
    let theme = cx.theme();
    let color = tone.color(cx);
    let text = if tone == Tone::Neutral {
        theme.foreground
    } else {
        color
    };
    let radius = (theme.radius / px(1.) - CHIP_RADIUS_INSET).max(4.0);
    let chip = h_flex()
        .flex_shrink_0()
        .h(px(height))
        .px(px(padding))
        .items_center()
        .rounded(px(radius))
        .text_size(px(CHIP_FONT))
        .font_weight(FontWeight::MEDIUM)
        .text_color(text)
        .whitespace_nowrap()
        .child(label.into());
    if filled {
        chip.bg(color.opacity(CHIP_FILL))
    } else {
        // `Chip.js`, `outlined`: the color at 70%; a neutral chip uses the
        // outline color of a text field.
        let border = if tone == Tone::Neutral {
            theme.foreground.opacity(0.23)
        } else {
            color.opacity(0.7)
        };
        chip.border_1().border_color(border)
    }
}

/// `LinearProgress.js`: a 4 px track; `MuiLinearProgress`: corners of 4 px.
const PROGRESS_HEIGHT: f32 = 4.0;
/// `LinearProgress.js`, `indeterminate1Keyframe`: one pass takes 2.1 s.
const PROGRESS_PERIOD: Duration = Duration::from_millis(2100);

/// Where the two bars of an indeterminate progress are, `time` of the way
/// through a pass: for each, the left edge and the right edge as shares of
/// the track. `LinearProgress.js`, `indeterminate1Keyframe` and
/// `indeterminate2Keyframe`, the second starting 1.15 s after the first.
pub fn indeterminate(time: f32) -> [(f32, f32); 2] {
    // Each bar's `left` and `right` insets run between these over the first
    // 60% of its pass and then hold.
    let bar = |time: f32, ease: [f32; 4], from: (f32, f32), to: (f32, f32)| {
        let share = easing(ease)((time / 0.6).clamp(0.0, 1.0));
        let left = from.0 + (to.0 - from.0) * share;
        let right = from.1 + (to.1 - from.1) * share;
        (left, 1.0 - right)
    };
    // The second bar runs 1.15 s late, in a pass of 2.1 s.
    let late = (time - 1.15 / 2.1).rem_euclid(1.0);
    [
        bar(time, [0.65, 0.815, 0.735, 0.395], (-0.35, 1.0), (1.0, -0.9)),
        bar(late, [0.165, 0.84, 0.44, 1.0], (-2.0, 1.0), (1.07, -0.08)),
    ]
}

/// A thin bar that shows work is going on, without saying how much is left.
pub fn linear_progress(id: &'static str, cx: &App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    let (color, track) = (theme.primary, theme.muted_foreground.opacity(0.3));
    let reduced = cx.reduce_motion();
    div()
        .w_full()
        .h(px(PROGRESS_HEIGHT))
        .flex_shrink_0()
        .relative()
        .overflow_hidden()
        .rounded(px(PROGRESS_HEIGHT))
        .bg(track)
        .with_animation(
            id,
            Animation::new(scaled(PROGRESS_PERIOD)).repeat(),
            move |track, time| {
                // With motion reduced the bar is a still, half-filled track.
                let bars = if reduced {
                    vec![(0.25, 0.75)]
                } else {
                    indeterminate(time).to_vec()
                };
                track.children(bars.into_iter().filter_map(|(left, right)| {
                    let (left, right) = (left.clamp(0.0, 1.0), right.clamp(0.0, 1.0));
                    (right > left).then(|| {
                        div()
                            .absolute()
                            .top_0()
                            .h_full()
                            .left(relative(left))
                            .w(relative(right - left))
                            .bg(color)
                    })
                }))
            },
        )
}

/// `MuiTooltip`: 12 px text with 6 by 12 px of padding and corners of 6 px.
const TOOLTIP_FONT: f32 = 12.0;

/// Builds the tooltip a control shows: pass to GPUI's `.tooltip(…)`.
pub fn tooltip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView {
    let text: SharedString = text.into();
    move |window, cx| {
        let border = cx.theme().primary.opacity(0.12);
        Tooltip::new(text.clone())
            .px(px(12.))
            .py(px(6.))
            .rounded(px(6.))
            .border_1()
            .border_color(border)
            .text_size(px(TOOLTIP_FONT))
            .font_weight(FontWeight::MEDIUM)
            .build(window, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_progress_bars_cross_the_track_from_left_to_right() {
        // At the start the first bar is off the left edge.
        let [(left, right), _] = indeterminate(0.0);
        assert!((left - -0.35).abs() < 1e-3 && (right - 0.0).abs() < 1e-3);
        // By 60% of the pass it has left by the right edge, and stays there.
        let [(left, _), _] = indeterminate(0.6);
        assert!((left - 1.0).abs() < 1e-3);
        assert_eq!(indeterminate(0.6)[0], indeterminate(0.9)[0]);
        // In between, part of it is on the track.
        let [(left, right), _] = indeterminate(0.3);
        assert!(left < 1.0 && right > 0.0 && right > left);
    }

    #[test]
    fn the_second_bar_runs_behind_the_first() {
        // It starts 1.15 s into the 2.1 s pass.
        let start = 1.15 / 2.1;
        let (left, right) = indeterminate(start)[1];
        assert!((left - -2.0).abs() < 1e-2 && right.abs() < 1e-2);
        let (left, right) = indeterminate((start + 0.3).rem_euclid(1.0))[1];
        assert!(right > left);
        assert!(right > 0.0 && left < 1.0, "on the track: {left}..{right}");
    }

    #[test]
    fn a_bar_never_has_its_ends_the_wrong_way_round() {
        for step in 0..=100 {
            for (left, right) in indeterminate(step as f32 / 100.0) {
                assert!(right >= left - 1e-3, "{left}..{right} at step {step}");
            }
        }
    }
}
