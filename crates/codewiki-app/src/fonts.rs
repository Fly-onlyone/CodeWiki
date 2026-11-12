//! Fonts bundled in the binary.
//!
//! The CodeWiki themes name these families. They are embedded so the app looks
//! the same on a machine that has neither installed. Both are licensed under
//! the SIL Open Font License; the license texts sit beside the font files.
//!
//! The files are static instances, one per weight. The variable builds are
//! smaller, but DirectWrite lists variable Inter under the family name
//! "Inter Variable Text", which a theme cannot name portably.

use std::borrow::Cow;

use gpui_kit::App;

/// Family name of the bundled interface font. The CodeWiki themes must name
/// exactly this; a test compares them.
pub const UI_FAMILY: &str = "Inter";
/// Family name of the bundled code font, checked the same way.
pub const MONO_FAMILY: &str = "JetBrains Mono";

/// Register the bundled fonts with the text system. Call before any theme is
/// applied.
pub fn register(cx: &App) {
    let fonts: Vec<Cow<'static, [u8]>> = vec![
        Cow::Borrowed(include_bytes!("../fonts/Inter-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../fonts/Inter-Italic.ttf")),
        Cow::Borrowed(include_bytes!("../fonts/Inter-Medium.ttf")),
        Cow::Borrowed(include_bytes!("../fonts/Inter-SemiBold.ttf")),
        Cow::Borrowed(include_bytes!("../fonts/Inter-Bold.ttf")),
        Cow::Borrowed(include_bytes!("../fonts/JetBrainsMono-Regular.ttf")),
        Cow::Borrowed(include_bytes!("../fonts/JetBrainsMono-Italic.ttf")),
        Cow::Borrowed(include_bytes!("../fonts/JetBrainsMono-Bold.ttf")),
    ];
    if let Err(error) = cx.text_system().add_fonts(fonts) {
        eprintln!("codewiki: bundled fonts were not loaded: {error}");
        return;
    }
    // A family the text system does not list falls back to the system font
    // without any error. Say so in debug builds, where someone is watching.
    if cfg!(debug_assertions) {
        let known = cx.text_system().all_font_names();
        for family in [UI_FAMILY, MONO_FAMILY] {
            if !known.iter().any(|name| name == family) {
                eprintln!("codewiki: bundled font family {family:?} is not registered");
            }
        }
    }
}
