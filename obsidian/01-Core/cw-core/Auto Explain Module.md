---
tags: [core, cw-core, module]
---

# Auto Explain Module

> The switch for auto explaining.

## What it does

Two small settings that say whether coding agents explain waiting wiki notes by themselves at session start, and how many. Each lives in its own text file in the data directory: `auto-explain.txt` holds the on or off switch, and `auto-explain-count.txt` holds how many notes one session explains. The session-start hook in [[codewiki]] and the desktop app read them. Only the app writes them. Auto explaining is on by default, with a count of 20.

## Source

- `crates/cw-core/src/auto_explain.rs` — module `cw_core::auto_explain`

## How it works

`load` returns a bool. A missing file means on. Otherwise the trimmed text must equal `on`, ignoring case. Anything else (empty, `yes`, `true`, a folder in place of the file, an unreadable file) means off.

`load_count` reads the second file and returns a [[Count]]: a positive number, or `all`. A missing file, an unreadable file, `0`, a negative number or any other text gives `DEFAULT`, which is 20 notes. `Count::of` caps a number of waiting notes by the count. `PRESETS` lists the choices the app offers: 5, 10, 20, 50, 100 and all.

`save` and `save_count` create the data directory if needed, write to a temporary file, then rename it over the real one, so a reader never sees a half-written file. `save_count` never writes a count below 1.

In the hook, a result of on lets `hook_input` read the host's JSON from stdin and choose whether the briefing orders the agent to explain notes, and how many. With off, stdin is not even read.

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

- Fail closed for the switch. A damaged file turns the feature off, so a broken file never spends anyone's usage allowance. Only a missing file means on.
- Fail to the default for the count. A damaged count file gives 20, not off and not unlimited.
- Only the exact words `on` and `off` count for the switch. `true` or `1` written by hand mean off.
- The count is kept while the switch is off. Turning the switch on again uses the saved count.
- `save` and `save_count` return an error for I/O failures, but `load` and `load_count` have no error case at all.

## See also

- [[_index]]
- [[cw-core]]
