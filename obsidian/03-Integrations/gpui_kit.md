---
tags: [integration]
---

# gpui_kit

> External crate `gpui_kit`, imported by 35 source files.

## What it does

`gpui_kit` is the user interface toolkit of the desktop app. One dependency brings GPUI (the GPU-drawn UI framework with windows, elements, styling and animation), its base helpers, a set of ready components such as inputs, scroll areas and a title bar, and default assets. All 35 UI source files of `codewiki-app` import from it, and the app has no other UI library. Building blocks written in the app, such as buttons, checkboxes and autocomplete, sit on top of it.

## Used for

Imported by 35 source files. Main users:

- [[Autocomplete Module]] — imports `Anchor`, `App`, `Bounds`, `ElementId` and 17 more
- [[Backdrop Module]] — imports `AnyElement`, `App`, `Bounds`, `ContentMask` and 24 more
- [[Base Module]] — imports `App`, `BoxShadow`, `ClickEvent`, `Div` and 14 more
- [[Button Module]] — imports `App`, `BoxShadow`, `ClickEvent`, `ElementId` and 22 more
- [[Checkbox Module]] — imports `App`, `ElementId`, `Hsla`, `InteractiveElement` and 11 more
- [[Diagrams Module]] — imports `Image`
- [[Doc View Module]] — imports `AnimationExt`, `AnyElement`, `App`, `Bounds` and 36 more
- [[Feedback Module]] — imports `Animation`, `AnimationExt`, `AnyView`, `App` and 15 more

## Configuration

- `crates/codewiki-app/Cargo.toml` — declares the dependency: version `=0.9.1`, locked `0.9.1`

## Wire-up

The version is pinned exactly in the app's manifest, because the API changes between releases. Files import the pieces they need from `gpui_kit`, from `gpui_kit::component` (theme, icons, inputs, title bar), `gpui_kit::base` (markdown and motion) and `gpui_kit::prelude`.

At startup the app registers its bundled fonts through the text system (the [[Fonts Module]], before any theme is applied). The [[Icons Module]] plugs into the toolkit's asset source and icon naming. The [[Glass Module]] and the [[Backdrop Module]] read the active theme and draw see-through surfaces on top of the Windows Mica backdrop, since the toolkit cannot blur inside a window.

Most views implement `Render` and build `div()` trees with the toolkit's styling and flex helpers.

## Auth mode

No sign-in. It is a local library with no network access.

## Gotchas

- The version is pinned with `=`. Changing it can break many files at once, so upgrade it on purpose and rebuild everything.
- A font family that the text system does not list falls back to the system font without an error. The Fonts Module warns about this only in debug builds, and a test compares the family names with the themes.
- The toolkit cannot blur content inside a window. The glass look comes from window backdrop plus partly transparent colors, so text contrast must be checked against the theme.
- `main.rs` imports `gpui_kit::*`, so a name clash is easy to cause when adding a type with a common name.

## See also

- [[_index]]
