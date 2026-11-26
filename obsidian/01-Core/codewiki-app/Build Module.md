---
tags: [core, codewiki-app, module]
---

# Build Module

> Embeds the app's icon in the Windows program.

## What it does

This is the build script of the app crate. On Windows it compiles a resource file into the program, so that `codewiki-app.exe` carries its icon in Explorer, the taskbar and the window. On other systems it does nothing.

## Source

- `crates/codewiki-app/build.rs` — module `codewiki_app::build`

## How it works

Cargo runs `main` before it builds the crate. The script tells Cargo to run it again when `resources/codewiki.rc` or `resources/codewiki.ico` change. Then it checks `CARGO_CFG_TARGET_OS`. This is the target system, not the system running the build, so a cross build to Windows works. If the target is Windows, `embed_resource::compile` builds the `.rc` file with the `resources` folder as include path. The linker puts the result into the program.

## Items

- `fn main()` — private function.

## Gotchas

- The resource step uses `manifest_optional`, so a missing manifest is not an error. A resource that fails to compile still stops the build, with the message "the icon resource compiles".
- Only the two files named in the `rerun-if-changed` lines trigger a new run. A new file included from the `.rc` file would need its own line.

## See also

- [[_index]]
- [[codewiki-app]]
