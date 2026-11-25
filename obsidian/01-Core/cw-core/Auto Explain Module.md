---
tags: [core, cw-core, module]
---

# Auto Explain Module

> The switch for auto explaining.

## What it does

When it is on, coding agents explain the waiting wiki notes on their own at the start of a session. One small text file in the data directory holds the switch. `codewiki hook session-start` and the desktop app read it; the app changes it. It is on by default. A file that cannot be read or understood means off, so a broken file never spends anyone's allowance.

## Source

- `crates/cw-core/src/auto_explain.rs` — module `cw_core::auto_explain`

## How it works

1. `session_start_hook` in [[codewiki]] and `read_auto_explain` in [[codewiki-app]], and 3 more, call `load`. `load` calls `file`.
2. `AppView::run_settings_action` in [[codewiki-app]] calls `save`. `save` calls `file`, then `Error::io` in [[Error]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const FILE: &str` — name of the switch file inside the data directory.
- `pub fn file(data_dir: &Path) -> PathBuf` — path of the switch file.
- `pub fn load(data_dir: &Path) -> bool` — whether auto explaining is on. A missing file means on. The file holds `on` or `off`; anything else, or a file that cannot be read, means off.
- `pub fn save(data_dir: &Path, on: bool) -> Result<()>` — turn auto explaining on or off. Creates the data directory when it is missing.

## Depends on

- [[Error Module]] — uses `Result`
- [[Error]] — uses `io`

## Used by

- [[codewiki-app]] — from `open_settings`, `read_auto_explain`, `run_settings_action`, `show_settings_category`
- [[codewiki]] — from `session_start_hook`
- [[Settings View Module]] — from `new`

## See also

- [[_index]]
- [[cw-core]]
