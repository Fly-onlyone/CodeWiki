---
tags: [integration]
---

# serde_json

> External crate `serde_json`, imported by 29 source files.

## What it does

`serde_json` reads and writes JSON. In the desktop app it is the common language with the backend server: every answer from the server arrives as JSON, and the app keeps it as a loose `Value` instead of fixed structs. Pages such as the hotspot list, the search results, the review queue and the settings pages read the fields they need straight from the `Value`. The same crate builds the small messages that two copies of the app send each other.

## Used for

Imported by 29 source files. Main users:

- [[cw-embed]] — imports `Value`, `json`
- [[cw-server]] — imports `Value`, `json`
- [[Activity Module]] — imports `Value`, `json`
- [[Api Module]] — imports `Value`, `json`
- [[Ci Module]] — imports `Value`, `json`
- [[Embed Job Module]] — imports `Value`, `json`
- [[Graph Module (codewiki-app)]] — imports `Value`
- [[Guard Module]] — imports `Value`, `json`

## Configuration

- `crates/codewiki-app/Cargo.toml`, `crates/codewiki-mcp/Cargo.toml`, `crates/codewiki/Cargo.toml`, and 7 more — declare the dependency: version `1`, locked `1.0.154`
- `crates/cw-core/Cargo.toml` — declares the dependency: version `1.0.154`, locked `1.0.154`

## Wire-up

The app asks the backend over HTTP and gets a `Value`. A page function then reads fields by name, for example `value["nodes"].as_array()`. The graph module builds `GraphData` this way in `from_json`.

The [[Instance Module]] uses `json!` to write one line of JSON per launch request. `encode` adds a protocol number, a token, the repo, the view and a tray flag. `decode` parses a line and accepts it only when the number matches and the token matches.

Tests build sample answers with the `json!` macro, so page functions can be checked without a running server.

## Auth mode

No sign-in. The token in the instance message is a random 128-bit value that proves the sender is a local copy of the app.

## Gotchas

- Indexing a `Value` never panics. A missing field gives `Null`, and `as_str` on `Null` gives `None`. Code must pick a default on purpose, such as an empty string or zero. A renamed server field then shows as empty text, not as an error.
- Numbers: `as_u64` returns `None` for a negative or fractional number, and `as_f64` accepts integers too.
- `graph.rs` drops edges whose index is out of range or that point at themselves, so a bad answer cannot crash drawing.
- `decode` returns `None` for any bad line, so a wrong or old message is ignored without a log entry.

## See also

- [[_index]]
