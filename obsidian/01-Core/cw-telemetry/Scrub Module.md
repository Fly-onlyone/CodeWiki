---
tags: [core, cw-telemetry, module]
---

# Scrub Module

> What is taken out of a report before it leaves the machine.

## What it does

The logs follow rules of their own (no token, key, query, prompt, LLM answer or note text), but they do name folders, and a folder names its repository and often the user. So every absolute path in a message, an exception or a breadcrumb becomes `PATH`. The host name, the user, and the query string, headers, cookies and body of a request are dropped. A loaded program is named by its file name alone.

## Source

- `crates/cw-telemetry/src/scrub.rs` — module `cw_telemetry::scrub`

## How it works

1. `start` in [[cw-telemetry]] calls `event`. `event` calls `scrub_option`, then `scrub_string`, then `file_name`. `scrub_option` calls `scrub_string`. `scrub_string` calls `paths`.
2. `start` in [[cw-telemetry]] calls `breadcrumb`. `breadcrumb` calls `scrub_option`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const PATH: &str` — what an absolute path is replaced by.
- `pub fn event(mut event: Event<'static>, program: &str) -> Event<'static>` — `event` without what must not leave the machine, tagged with the program that sent it.
- `pub fn breadcrumb(mut breadcrumb: Breadcrumb) -> Breadcrumb` — `breadcrumb` with the paths in its message replaced.
- `fn scrub_option(text: &mut Option<String>)` — private function; called by `breadcrumb`, `event`.
- `fn scrub_string(text: &mut String)` — private function; called by `event`, `scrub_option`.
- `fn file_name(path: &str) -> &str` — the last part of a Windows or Unix path.
- `pub fn paths(text: &str) -> Cow<'_, str>` — `text` with every absolute Windows path replaced by `PATH`: one that starts with a drive (`C:\`, `D:/`), a share (`\\server\`) or the verbatim prefix (`\\?\`). A path runs to a quote, a colon, a pipe, an angle bracket or a space; past a space only when one of the next few words holds its separator, as in `C:\Users\Software Tester\AppData`. Punctuation that ends a sentence stays outside it.
- `fn path_at(text: &str, at: usize) -> Option<usize>` — the end of the absolute path that starts at byte `at`, if one does.
- `const WORDS_AHEAD: usize` — how many words after a space are looked at for the rest of a path, as in `C:\Program Files (x86)\CodeWiki`.
- `fn continued(bytes: &[u8], end: usize, separator: u8) -> Option<usize>` — where a path that stopped at a space at `end` goes on to: the end of the first of the next `WORDS_AHEAD` words that holds its `separator`, or `None` when none does and the path ends at `end`.
- `fn is_drive(bytes: &[u8]) -> bool` — A drive and a separator: `C:\` or `C:/`.
- `fn is_part(byte: u8) -> bool` — whether `byte` can be part of a path. Spaces are handled by the caller.

## Used by

- [[cw-telemetry]] — from `start`

## See also

- [[_index]]
- [[cw-telemetry]]
