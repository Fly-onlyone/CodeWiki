---
tags: [core, cw-wiki, module]
---

# Security Module

> Security pattern findings for Rust source.

## Source

- `crates/cw-wiki/src/security.rs` — module `cw_wiki::security`

## How it works

Main items:

- `scan` — findings across every indexed Rust file, by file, line and rule.
- `scan_source` — findings in one Rust source file.
- `ENV_SET_VAR` — `std::env::set_var` or `remove_var`.
- `UNSAFE_BLOCK` — an `unsafe { … }` block.
- `SecurityFinding` — struct
- `INSECURE_HTTP` — an `http://` URL literal whose host is not local, private or reserved.
- `TLS_DISABLED` — certificate or hostname verification switched off on a TLS client.
- `WORLD_WRITABLE` — A Unix file mode whose "other" digit includes write.

## Depends on

- [[Scanned]] — uses `Scanned`, `in_test`, `line_of`, `literal_at`
- [[Index]] — uses `Index`, `files`
- [[Error Module]] — uses `Result`
- [[Rustscan Module]] — uses `is_test_path`
- [[Level]] — uses `Level`
- [[Literal]] — uses `Literal`

## Used by

- [[codewiki]] — from `run`
- [[cw-wiki]] — from `scans_source_files_and_skips_tests`
- [[Api Module]] — from `security`
- [[Ci Module]] — from `check`

## See also

- [[_index]]
- [[cw-wiki]]
