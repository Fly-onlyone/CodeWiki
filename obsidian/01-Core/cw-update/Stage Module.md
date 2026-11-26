---
tags: [core, cw-update, module]
---

# Stage Module

> Unpacking a verified package and checking that its programs start.

## What it does

Takes the verified zip from [[Download Module]], unpacks it into a staging folder, and proves the result is usable before anything is swapped in. It checks that every program is present and that the new `codewiki.exe` starts and reports the expected version.

## Source

- `crates/cw-update/src/stage.rs` — module `cw_update::stage`

## How it works

`stage` empties the target folder if it exists, creates it, and extracts the zip. It then checks that each name in `PROGRAMS` is a file there. Next it calls `program_version` on the new `codewiki.exe` and compares the result to the version of the release. Any mismatch is an error.

`program_version` runs `<exe> --version` with no input, captures standard output, and polls `try_wait` every 50 ms. If the program is still running after `VERSION_TIMEOUT` (20 seconds), it is killed and an error returns. A non-zero exit is an error. `parse_version_line` takes the second word of the line, as in `codewiki 0.2.0`, and parses it as a version.

`install` in [[cw-update]] also calls `program_version` directly.

## Items

- `const VERSION_TIMEOUT: Duration` — how long `codewiki --version` may take. It prints at once; a virus scan of a program never run before can hold it for a few seconds.
- `pub fn stage(zip: &Path, to: &Path, version: &Version) -> Result<(), String>` — unpack `zip` into an empty `to` and check that it is a CodeWiki package of `version` whose `codewiki.exe` starts on this machine.
- `pub fn program_version(exe: &Path) -> Result<Version, String>` — the version `<exe> --version` prints. The program is started without a console window and stopped if it does not answer in time.
- `fn parse_version_line(text: &str) -> Option<Version>` — the version in clap's `--version` line, `codewiki 0.2.0`.

## Depends on

- [[cw-update]] — uses `PROGRAMS`

## Used by

- [[cw-update]] — from `install`

## Gotchas

- On Windows the child starts with the no-window flag. Without it the desktop app would flash a terminal window.
- The 20-second limit is deliberate: a virus scan of a program never run before can delay the first start by seconds.
- A failed stage leaves the unpacked files in place. The test shows a zip without programs still leaves its `LICENSE` in the folder. The next `stage` call empties the folder first.
- Running the new program is a real check on the machine, but it executes downloaded code. That is why [[Release Module]] checks where the package comes from and [[Download Module]] checks its hash first.

## See also

- [[_index]]
- [[cw-update]]
