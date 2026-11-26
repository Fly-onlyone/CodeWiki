---
tags: [core, cw-core, module]
---

# Auto Explain Module

> The switch for auto explaining.

## What it does

A one-word switch that says whether coding agents should explain waiting wiki notes by themselves at session start. The setting lives in `auto-explain.txt` in the data directory. The session-start hook in [[codewiki]] and the desktop app read it. Only the app writes it. It is on by default.

## Source

- `crates/cw-core/src/auto_explain.rs` — module `cw_core::auto_explain`

## How it works

`load` returns a bool. A missing file means on. Otherwise the trimmed text must equal `on`, ignoring case. Anything else (empty, `yes`, `true`, a folder in place of the file, an unreadable file) means off.

`save` creates the data directory if needed, writes `on` or `off` to a temporary file, then renames it over the real one, so a reader never sees a half-written file.

In the hook, a result of on lets `hook_input` read the host's JSON from stdin and choose whether the briefing orders the agent to explain notes. With off, stdin is not even read.

## Items

- `pub const FILE: &str` — name of the switch file inside the data directory.
- `pub const COUNT_FILE: &str` — name of the file inside the data directory that holds the `Count`.
- [[Count]] — how many waiting notes a session explains.
- `pub const DEFAULT: Count` — the count when the user chose none.
- `pub const PRESETS: [Count; 6]` — the counts the app offers, in its order.
- `pub fn file(data_dir: &Path) -> PathBuf` — path of the switch file.
- `pub fn count_file(data_dir: &Path) -> PathBuf` — path of the count file.
- `pub fn load(data_dir: &Path) -> bool` — whether auto explaining is on. A missing file means on. The file holds `on` or `off`; anything else, or a file that cannot be read, means off.
- `pub fn save(data_dir: &Path, on: bool) -> Result<()>` — turn auto explaining on or off. Creates the data directory when it is missing.
- `pub fn load_count(data_dir: &Path) -> Count` — how many notes a session explains. A missing file, one that cannot be read, and one that names no positive number or `all` mean `DEFAULT`.
- `pub fn save_count(data_dir: &Path, count: Count) -> Result<()>` — save how many notes a session explains. Creates the data directory when it is missing.
- `fn write(data_dir: &Path, name: &str, text: &str) -> Result<()>` — write `text` to `name` in the data directory through a draft, so a reader never sees half a file.

## Depends on

- [[Count]] — uses `Count`, `parse`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `io`

## Used by

- [[codewiki-app]] — from `read_auto_explain`, `reload_auto_explain`, `run_settings_action`
- [[cw-wiki]] — from `the_line_about_waiting_notes_goes_when_none_waits`, `the_waiting_line_orders_a_batch_for_each_host`
- [[codewiki]] — from `session_start_hook`
- [[Settings Pages Module]] — from `count_choices`
- [[Settings View Module]] — from `new`

## Gotchas

- Fail closed. A damaged file turns the feature off, so a broken file never spends anyone's usage allowance. Only a missing file means on.
- Only the exact words `on` and `off` count. `true` or `1` written by hand mean off.
- `save` returns an error for I/O failures, but `load` has no error case at all.

## See also

- [[_index]]
- [[cw-core]]
