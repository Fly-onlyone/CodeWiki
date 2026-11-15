---
tags: [core, cw-update, module]
---

# Stage Module

> Unpacking a verified package and checking that its programs start.

## Source

- `crates/cw-update/src/stage.rs` — module `cw_update::stage`

## How it works

Main items:

- `program_version` — the version `<exe> --version` prints.
- `stage` — unpack `zip` into an empty `to` and check that it is a CodeWiki package of `version` whose `codewiki.exe` starts on this machine.
- `parse_version_line` — the version in clap's `--version` line, `codewiki 0.2.0`.
- `VERSION_TIMEOUT` — how long `codewiki --version` may take.

## Depends on

- [[cw-update]] — uses `PROGRAMS`

## Used by

- [[cw-update]] — from `install`

## See also

- [[_index]]
- [[cw-update]]
