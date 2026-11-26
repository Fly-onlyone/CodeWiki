---
tags: [integration]
---

# gpui_kit

> External crate `gpui_kit`, imported by 36 source files.

## What it does

`gpui_kit` is the user interface toolkit of the desktop app. One dependency brings GPUI (the GPU-drawn UI framework with windows, elements, styling and animation), base helpers such as markdown and motion, ready components (inputs, scroll areas, a title bar, a theme), and default assets. The UI source files of `codewiki-app` import from it, and the app has no other UI library. Building blocks written in the app, such as buttons, checkboxes and autocomplete, sit on top of it.

## Used for

Imported by 36 source files. Main users:

- [[Autocomplete Module]] — imports `Anchor`, `App`, `Bounds`, `ElementId` and 17 more
- [[Backdrop Module]] — imports `AnyElement`, `App`, `Bounds`, `ContentMask` and 24 more
- [[Base Module]] — imports `App`, `BoxShadow`, `ClickEvent`, `Div` and 16 more
- [[Button Module]] — imports `App`, `BoxShadow`, `ClickEvent`, `ElementId` and 22 more
- [[Checkbox Module]] — imports `App`, `ElementId`, `Hsla`, `InteractiveElement` and 11 more
- [[Diagrams Module]] — imports `Image`
- [[Doc View Module]] — imports `AnimationExt`, `AnyElement`, `App`, `Bounds` and 36 more
- [[Feedback Module]] — imports `Animation`, `AnimationExt`, `AnyView`, `App` and 15 more

## Configuration

- `crates/codewiki-app/Cargo.toml` — declares the dependency: version `=0.9.1`, locked `0.9.1`

## Wire-up

The version is pinned exactly in the app's manifest, because the API changes between releases. Files import the pieces they need from `gpui_kit`, from `gpui_kit::component` (theme, icons, inputs, title bar), `gpui_kit::base` (markdown and motion) and `gpui_kit::prelude`.

In `main`, the app is built with `gpui_kit::application().with_assets(icons::AppAssets)`. Inside `run` the setup order is: `gpui_kit::init`, then the app's `motion`, `backdrop` and `tab_prefs` init, then `fonts::register`, then `themes::init`, then `zoom::init`. The fonts are registered through the text system before any theme is applied. Zoom comes after the themes because it reads the theme's sizes as its 100% sizes.

The window is opened with `gpui_kit::open_window` and the Windows Mica backdrop. The [[Icons Module]] supplies `AppAssets`. It serves the app's own SVG icons and logo first, then falls back to the kit's default assets, which the kit's own components need. The [[Glass Module]] and the [[Backdrop Module]] read the active theme and draw see-through surfaces over Mica, since the toolkit cannot blur inside a window.

Most views implement `Render` and build `div()` trees with the toolkit's styling and flex helpers.

## Auth mode

No sign-in. It is a local library with no network access.

## Gotchas

- The version is pinned with `=`. Changing it can break many files at once, so upgrade it on purpose and rebuild everything.
- A font family that the text system does not list falls back to the system font without an error. `fonts::register` warns about this only in debug builds, and a test compares the family names with the themes.
- The toolkit paints the theme background over the whole window, which hides the backdrop. After opening the window, `main` clears the root's background to transparent. The app's own surfaces are the glass.
- The toolkit cannot blur content inside a window. The glass look comes from the window backdrop plus partly transparent colors, so check text contrast against each theme.
- The kit's own components draw a few Lucide icons. Those load only because `AppAssets` falls back to the kit's assets. Do not drop that fallback.
- `main.rs` imports `gpui_kit::*`, so a name clash is easy to cause when adding a type with a common name.

## See also

- [[_index]]
