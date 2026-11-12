//! Components drawn to Material UI's measurements.
//!
//! The app is native, so Material UI itself cannot be used. Each component
//! here follows the anatomy, sizes and behaviour of its MUI 6 namesake, with
//! the changes the TradingAgent theme makes to it. Colors come from the
//! active CodeWiki theme.
//!
//! Three things GPUI cannot do shape the result: it cannot scale an element,
//! blur what is behind one, or space letters apart.

pub mod base;
pub mod button;
#[expect(dead_code, reason = "presets wait for the components that use them")]
pub mod motion;
pub mod ripple;
// A table copied whole from MUI; not every entry has a user.
#[allow(dead_code)]
pub mod tokens;
