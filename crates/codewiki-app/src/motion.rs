//! Motion: whether things move at all.
//!
//! gpui-kit reads the Windows "animation effects" setting at startup and
//! holds animations still when it is off. This module adds a choice on top:
//! follow Windows (the default), always reduce, or always animate. The choice
//! is remembered between runs.
//!
//! Animations need no check of their own. Under reduced motion a one-shot
//! animation is drawn in its end state, a repeating one in its start state
//! and a spring at its target, and none asks for more frames. How things
//! move when they do is in `mui::motion`.

use gpui_kit::{App, Global};

use crate::prefs;

/// Name of the setting in `prefs`.
const SETTING: &str = "motion";

/// What the user asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MotionPref {
    /// Do what Windows says.
    #[default]
    System,
    /// Hold animations still.
    Reduced,
    /// Animate even when Windows says not to.
    Full,
}

impl MotionPref {
    pub const ALL: [Self; 3] = [Self::System, Self::Reduced, Self::Full];

    /// The word this choice is stored and linked under.
    pub fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Reduced => "reduced",
            Self::Full => "full",
        }
    }

    /// The choice stored under `key`, if it is one.
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|pref| pref.key() == key.trim())
    }

    /// How the choice reads in the settings page and the palette.
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "follow Windows",
            Self::Reduced => "reduced",
            Self::Full => "full",
        }
    }

    /// Whether animations are held still, given what Windows says.
    fn reduces(self, system_reduces: bool) -> bool {
        match self {
            Self::System => system_reduces,
            Self::Reduced => true,
            Self::Full => false,
        }
    }
}

struct State {
    pref: MotionPref,
    /// What Windows said at startup. Kept because the app's own choice
    /// overwrites the flag gpui-kit set from it.
    system_reduces: bool,
}

impl Global for State {}

/// Apply the remembered choice. Call after `gpui_kit::init`, which is what
/// reads the Windows setting.
pub fn init(cx: &mut App) {
    let system_reduces = cx.reduce_motion();
    let pref = prefs::read(SETTING)
        .and_then(|key| MotionPref::parse(&key))
        .unwrap_or_default();
    cx.set_global(State {
        pref,
        system_reduces,
    });
    cx.set_reduce_motion(pref.reduces(system_reduces));
}

/// The current choice.
pub fn current(cx: &App) -> MotionPref {
    cx.try_global::<State>()
        .map(|state| state.pref)
        .unwrap_or_default()
}

/// Make a choice, apply it and remember it.
pub fn set(pref: MotionPref, cx: &mut App) {
    let system_reduces = cx
        .try_global::<State>()
        .map(|state| state.system_reduces)
        .unwrap_or_else(|| cx.reduce_motion());
    cx.set_global(State {
        pref,
        system_reduces,
    });
    cx.set_reduce_motion(pref.reduces(system_reduces));
    prefs::write(SETTING, pref.key());
}

/// The current choice and what it comes to, for the settings page: for
/// example "follow Windows (animations are on)".
pub fn describe(cx: &App) -> String {
    let state = if cx.reduce_motion() { "off" } else { "on" };
    format!("{} (animations are {state})", current(cx).label())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_choice_survives_being_stored() {
        for pref in MotionPref::ALL {
            assert_eq!(MotionPref::parse(pref.key()), Some(pref));
        }
    }

    #[test]
    fn stored_text_is_read_leniently_but_not_guessed() {
        assert_eq!(MotionPref::parse(" reduced\n"), Some(MotionPref::Reduced));
        assert_eq!(MotionPref::parse("Reduced"), None);
        assert_eq!(MotionPref::parse(""), None);
        assert_eq!(MotionPref::parse("off"), None);
    }

    #[test]
    fn the_default_follows_windows() {
        assert_eq!(MotionPref::default(), MotionPref::System);
        assert!(MotionPref::System.reduces(true));
        assert!(!MotionPref::System.reduces(false));
    }

    #[test]
    fn an_explicit_choice_overrides_windows_either_way() {
        for system in [true, false] {
            assert!(MotionPref::Reduced.reduces(system));
            assert!(!MotionPref::Full.reduces(system));
        }
    }

    #[test]
    fn keys_and_labels_are_distinct() {
        for (index, a) in MotionPref::ALL.into_iter().enumerate() {
            for b in MotionPref::ALL.into_iter().skip(index + 1) {
                assert_ne!(a.key(), b.key());
                assert_ne!(a.label(), b.label());
            }
        }
    }
}
