---
tags: [core, cw-telemetry, module]
---

# Scrub Module

> What is taken out of a report before it leaves the machine.

## What it does

Removes private data from an error report before it leaves the machine. The logs follow their own rules: no token, key, query, prompt, LLM answer or note text. But they do name folders, and a folder names its repository and often the user. So every absolute path in a message, an exception or a breadcrumb is replaced by `PATH`. The host name, the user, and the query string, headers, cookies and body of a request are dropped. A loaded program is named by its file name alone.

## Source

- `crates/cw-telemetry/src/scrub.rs` — module `cw_telemetry::scrub`

## How it works

`event` clears the server name and user. It strips the request's query, cookies, body, headers and environment, and cuts the query and fragment from its URL. It then runs `scrub_string` over the message, log entry, culprit, exception values and breadcrumb messages. Debug image names are cut to the file name. At last it tags the event with the program name. `breadcrumb` scrubs one breadcrumb message.

`paths` scans the text and, at each position, asks `path_at` whether an absolute Windows path starts there. Three forms count: a drive (`C:\` or `D:/`), a share (`\\server\`) and the verbatim prefix (`\\?\`). The path runs until a quote, colon, pipe, angle bracket or space. Past a space, `continued` looks at the next three words. If one of them holds the path's separator, the path goes on. That is how `C:\Users\Software Tester\AppData` stays whole. Trailing sentence punctuation stays outside the path. If nothing matches, `paths` returns the input without copying.

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

## Gotchas

- Only Windows-style absolute paths are found. A Unix path such as `/home/me/x` is not replaced.
- A path that follows a letter, digit or underscore is not taken as a path start. This keeps text such as `abC:\` from matching.
- A space inside a path is only followed when a separator shows up within three words. A path with a long space-filled name can be cut short, leaving the tail.
- Scrubbing is a text rule, not a guarantee. Keep secrets out of log messages in the first place.

## See also

- [[_index]]
- [[cw-telemetry]]
