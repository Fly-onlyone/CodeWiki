---
tags: [core, cw-update, module]
---

# Stage Module

> Unpacking a verified package and checking that its programs start.

## Source

- `crates/cw-update/src/stage.rs` — module `cw_update::stage`

## How it works

1. `install` in [[cw-update]] calls `stage`. `stage` calls `program_version`.
2. `install` in [[cw-update]] calls `program_version`. `program_version` calls `parse_version_line`.

## Items

- `const VERSION_TIMEOUT: Duration` — how long `codewiki --version` may take. It prints at once; a virus scan of a program never run before can hold it for a few seconds.
- `pub fn stage(zip: &Path, to: &Path, version: &Version) -> Result<(), String>` — unpack `zip` into an empty `to` and check that it is a CodeWiki package of `version` whose `codewiki.exe` starts on this machine.
- `pub fn program_version(exe: &Path) -> Result<Version, String>` — the version `<exe> --version` prints. The program is started without a console window and stopped if it does not answer in time.
- `fn parse_version_line(text: &str) -> Option<Version>` — the version in clap's `--version` line, `codewiki 0.2.0`.

## Depends on

- [[cw-update]] — uses `PROGRAMS`

## Used by

- [[cw-update]] — from `install`

## See also

- [[_index]]
- [[cw-update]]
