//! `Switch`.
//!
//! Sizes, opacities and timing are MUI's (`Switch/Switch.js` and
//! `internal/SwitchBase.js`); TradingAgent does not restyle the switch. A
//! track with a round thumb that slides along it; around the thumb a round
//! area that tints under the pointer and carries the ripple.
//!
//! In MUI an invisible input covers the whole switch, so a click anywhere on
//! it toggles. Here the thumb's area is the control (hover, ripple, keyboard)
//! and the track takes clicks of its own.

use std::rc::Rc;

use gpui_kit::base::motion as kit;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, ClickEvent, ElementId, Hsla, InteractiveElement as _, IntoElement, ParentElement as _,
    RenderOnce, Rgba, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    hsla,
};

use super::base::{Base, Ink, OnClick, focus_ring};
use super::motion::scaled;
use super::tokens::{self, curve, duration, easing};
use crate::zoom::px;

/// `Switch.js`, `SwitchRoot`: `padding: 12` around the track.
const PADDING: f32 = 12.0;
/// How far the track is from the switch's left edge. A row that lines the
/// track up with what is above it pulls the switch left by this much, as
/// MUI's `FormControlLabel` does (`marginLeft: -11`).
pub const INSET: f32 = PADDING;
/// `Switch.js`, `SwitchRoot`: `width: 34 + 12 * 2`, `height: 14 + 12 * 2`.
const TRACK_WIDTH: f32 = 34.0;
const TRACK_HEIGHT: f32 = 14.0;
pub const WIDTH: f32 = TRACK_WIDTH + 2.0 * PADDING;
pub const HEIGHT: f32 = TRACK_HEIGHT + 2.0 * PADDING;
/// `Switch.js`, `SwitchThumb`: `width: 20`, `height: 20`.
const THUMB: f32 = 20.0;
/// `SwitchBase.js`, `SwitchBaseRoot`: `padding: 9` around the thumb.
const THUMB_PADDING: f32 = 9.0;
/// The round area around the thumb.
const HALO: f32 = THUMB + 2.0 * THUMB_PADDING;
/// `Switch.js`, `SwitchSwitchBase`, checked: `translateX(20px)`.
const TRAVEL: f32 = 20.0;

/// `Switch.js`, `SwitchTrack`: `opacity` of an off track, light and dark.
const TRACK_OFF: (f32, f32) = (0.38, 0.3);
/// `Switch.js`, checked: the track at `opacity: 0.5`, in the primary color.
const TRACK_ON: f32 = 0.5;
/// `Switch.js`, disabled: the track's `opacity`, light and dark.
const TRACK_DISABLED: (f32, f32) = (0.12, 0.2);
/// `Switch.js`, disabled and checked: `lighten(main, 0.62)` on a light theme,
/// `darken(main, 0.55)` on a dark one.
const DISABLED_LIGHTEN: f32 = 0.62;
const DISABLED_DARKEN: f32 = 0.55;

/// Lightness of the greys MUI gives the thumb (`colors/grey.js`): `common.white`
/// off on a light theme and `grey[300]` on a dark one; disabled, `grey[100]`
/// and `grey[600]`. They are greys in every theme: the thumb of an off switch
/// carries no hue.
const THUMB_OFF: (f32, f32) = (1.0, 0.878);
const THUMB_OFF_DISABLED: (f32, f32) = (0.961, 0.459);

fn grey(lightness: f32) -> Hsla {
    hsla(0.0, 0.0, lightness, 1.0)
}

fn pick(pair: (f32, f32), dark: bool) -> f32 {
    if dark { pair.1 } else { pair.0 }
}

/// MUI's `lighten` and `darken` (`@mui/system`, `colorManipulator.js`): each
/// channel moves toward white, or toward black, by `amount`.
fn lighten(color: Hsla, amount: f32) -> Hsla {
    let Rgba { r, g, b, a } = Rgba::from(color);
    let toward_white = |channel: f32| channel + (1.0 - channel) * amount;
    Rgba {
        r: toward_white(r),
        g: toward_white(g),
        b: toward_white(b),
        a,
    }
    .into()
}

fn darken(color: Hsla, amount: f32) -> Hsla {
    let Rgba { r, g, b, a } = Rgba::from(color);
    Rgba {
        r: r * (1.0 - amount),
        g: g * (1.0 - amount),
        b: b * (1.0 - amount),
        a,
    }
    .into()
}

/// How far the thumb's area is from the left of the switch, for a thumb
/// `progress` of the way from off (0) to on (1).
pub fn halo_left(progress: f32) -> f32 {
    TRAVEL * progress
}

/// The colors of a switch in one state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    pub thumb: Hsla,
    pub track: Hsla,
    /// What tints the area around the thumb under the pointer, and its
    /// ripple: MUI's `action.active` off, the primary color on.
    pub ink: Hsla,
}

impl Look {
    pub fn of(on: bool, disabled: bool, dark: bool, foreground: Hsla, primary: Hsla) -> Self {
        let thumb = match (on, disabled) {
            (true, false) => primary,
            (true, true) if dark => darken(primary, DISABLED_DARKEN),
            (true, true) => lighten(primary, DISABLED_LIGHTEN),
            (false, false) => grey(pick(THUMB_OFF, dark)),
            (false, true) => grey(pick(THUMB_OFF_DISABLED, dark)),
        };
        // The track is the page's text color when off (MUI's `onBackground`).
        let color = if on { primary } else { foreground };
        let opacity = if disabled {
            pick(TRACK_DISABLED, dark)
        } else if on {
            TRACK_ON
        } else {
            pick(TRACK_OFF, dark)
        };
        // `palette.action.active`: white on dark, black at 54% on light.
        let active = if dark {
            foreground
        } else {
            foreground.opacity(0.54)
        };
        Self {
            thumb,
            track: color.opacity(opacity),
            ink: if on { primary } else { active },
        }
    }
}

/// What a change of the switch does; it is given the new value.
type OnChange = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// A setting that is on or off, and takes effect when it is clicked.
#[derive(IntoElement)]
pub struct Switch {
    id: SharedString,
    on: bool,
    disabled: bool,
    on_change: Option<OnChange>,
}

impl Switch {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            on: false,
            disabled: false,
            on_change: None,
        }
    }

    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(mut self, on_change: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(on_change));
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = ElementId::Name(self.id.clone());
        let base = Base::new(&id, self.disabled, window, cx);
        let theme = cx.theme();
        let dark = theme.is_dark();
        let states = tokens::states(dark);
        let primary = theme.primary;
        let look = Look::of(self.on, self.disabled, dark, theme.foreground, primary);

        // `transitions.create([...], { duration: duration.shortest })`.
        let policy =
            || kit::Transition::new(scaled(duration::SHORTEST)).ease(easing(curve::EASE_IN_OUT));
        let progress = kit::transition(
            (id.clone(), "progress"),
            if self.on { 1.0 } else { 0.0 },
            policy(),
            window,
            cx,
        );
        let thumb = kit::transition((id.clone(), "thumb"), look.thumb, policy(), window, cx);
        let track = kit::transition((id.clone(), "track"), look.track, policy(), window, cx);
        let halo_to = if base.hovered {
            look.ink.opacity(look.ink.a * states.hover)
        } else {
            look.ink.opacity(0.0)
        };
        let halo = kit::transition((id.clone(), "halo"), halo_to, policy(), window, cx);

        let (on, disabled) = (self.on, self.disabled);
        let toggle = self.on_change.map(|on_change| {
            Rc::new(move |_: &ClickEvent, window: &mut Window, cx: &mut App| {
                on_change(!on, window, cx)
            }) as OnClick
        });

        let knob = div()
            .id(id)
            .absolute()
            .top_0()
            .left(px(halo_left(progress)))
            .size(px(HALO))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .bg(halo)
            // The track under it must not take the same click.
            .occlude()
            .when(base.focused, |knob| knob.shadow(vec![focus_ring(primary)]))
            .child(
                div()
                    .size(px(THUMB))
                    .rounded_full()
                    .bg(thumb)
                    // `SwitchThumb`: `boxShadow: shadows[1]`.
                    .shadow(tokens::elevation(1)),
            );
        let ink = Ink {
            radius: HALO / 2.0,
            centered: true,
            color: look.ink,
        };
        let knob = base.attach(knob, ink, disabled, toggle.clone(), cx);

        let track = div()
            .id(ElementId::NamedInteger(self.id, 1))
            .absolute()
            .top(px(PADDING))
            .left(px(PADDING))
            .w(px(TRACK_WIDTH))
            .h(px(TRACK_HEIGHT))
            // `SwitchTrack`: `borderRadius: 14 / 2`.
            .rounded(px(TRACK_HEIGHT / 2.0))
            .bg(track)
            .when_some(toggle.filter(|_| !disabled), |track, toggle| {
                track
                    .cursor_pointer()
                    .on_click(move |event, window, cx| toggle(event, window, cx))
            });

        div()
            .relative()
            .flex_shrink_0()
            .w(px(WIDTH))
            .h(px(HEIGHT))
            .child(track)
            .child(knob)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    fn foreground(dark: bool) -> Hsla {
        if dark { grey(0.95) } else { grey(0.1) }
    }

    fn primary() -> Hsla {
        hsla(0.72, 0.6, 0.6, 1.0)
    }

    #[test]
    fn the_switch_has_muis_measurements() {
        assert_eq!((WIDTH, HEIGHT), (58.0, 38.0));
        assert_eq!(HALO, 38.0);
        // The thumb sits on the middle line of the track.
        let thumb_middle = THUMB_PADDING + THUMB / 2.0;
        assert_eq!(thumb_middle, PADDING + TRACK_HEIGHT / 2.0);
        assert_eq!(HALO, HEIGHT);
    }

    #[test]
    fn the_thumb_travels_the_track_and_stays_in_the_switch() {
        assert_eq!(halo_left(0.0), 0.0);
        assert_eq!(halo_left(1.0), 20.0);
        assert_eq!(halo_left(0.5), 10.0);
        // The area around the thumb never leaves the switch.
        assert!(halo_left(1.0) + HALO <= WIDTH);
        // The thumb overhangs each end of the track by the same 3 px.
        let off = halo_left(0.0) + THUMB_PADDING;
        let on = halo_left(1.0) + THUMB_PADDING + THUMB;
        assert_eq!(PADDING - off, 3.0);
        assert_eq!(on - (PADDING + TRACK_WIDTH), 3.0);
    }

    #[test]
    fn an_on_switch_is_in_the_primary_color() {
        for dark in [false, true] {
            let look = Look::of(true, false, dark, foreground(dark), primary());
            assert_eq!(look.thumb, primary());
            assert!(near(look.track.a, 0.5));
            assert!(near(look.track.h, primary().h));
            assert_eq!(look.ink, primary());
        }
    }

    #[test]
    fn an_off_switch_is_grey_on_a_track_of_the_text_color() {
        let light = Look::of(false, false, false, foreground(false), primary());
        assert_eq!(light.thumb, grey(1.0));
        assert!(near(light.track.a, 0.38));
        assert!(near(light.track.l, foreground(false).l));
        // `action.active` on a light theme is the text color at 54%.
        assert!(near(light.ink.a, 0.54));

        let dark = Look::of(false, false, true, foreground(true), primary());
        assert_eq!(dark.thumb, grey(0.878));
        assert!(near(dark.track.a, 0.3));
        assert_eq!(dark.ink, foreground(true));
    }

    #[test]
    fn a_disabled_switch_fades() {
        let light_on = Look::of(true, true, false, foreground(false), primary());
        assert!(near(light_on.track.a, 0.12));
        // Lighter than the primary color on a light theme, darker on a dark.
        assert!(light_on.thumb.l > primary().l);
        let dark_on = Look::of(true, true, true, foreground(true), primary());
        assert!(near(dark_on.track.a, 0.2));
        assert!(dark_on.thumb.l < primary().l);

        let light_off = Look::of(false, true, false, foreground(false), primary());
        assert_eq!(light_off.thumb, grey(0.961));
        let dark_off = Look::of(false, true, true, foreground(true), primary());
        assert_eq!(dark_off.thumb, grey(0.459));
        assert!(near(dark_off.track.a, 0.2));
    }

    #[test]
    fn lighten_and_darken_move_each_channel_as_mui_does() {
        let color: Hsla = Rgba {
            r: 0.5,
            g: 0.2,
            b: 0.0,
            a: 1.0,
        }
        .into();
        let lighter = Rgba::from(lighten(color, 0.5));
        assert!(near(lighter.r, 0.75) && near(lighter.g, 0.6) && near(lighter.b, 0.5));
        let darker = Rgba::from(darken(color, 0.5));
        assert!(near(darker.r, 0.25) && near(darker.g, 0.1) && near(darker.b, 0.0));
        assert!(near(Rgba::from(lighten(color, 0.0)).r, 0.5));
    }
}
