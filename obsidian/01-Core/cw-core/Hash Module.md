---
tags: [core, cw-core, module]
---

# Hash Module

> Content hashing used for change detection.

## What it does

Small hashing helpers. CodeWiki hashes file and note content to tell whether anything changed, so regeneration is skipped when nothing did. The same module signs and checks webhook and token data.

## Source

- `crates/cw-core/src/hash.rs` — module `cw_core::hash`

## How it works

`sha256_hex` returns the lowercase hex SHA-256 of bytes. It is the basis of change detection, repository ids and input hashes.

`normalize_text` prepares text before hashing: it drops a leading byte-order mark, trims trailing spaces on each line, ends every line with a single newline, and removes extra blank lines at the end. So Windows and Unix line endings, editor whitespace, and git autocrlf do not look like edits. [[Manifest Module]] uses it for note bodies.

`hmac_sha256` builds HMAC-SHA256 by hand (RFC 2104). A key longer than the 64-byte block is hashed first, a shorter one is padded with zeros. Tests check it against RFC 4231 and GitHub's webhook example. [[Api Module]] uses it to verify webhook signatures.

`constant_time_eq` compares two byte strings without stopping at the first difference.

## Items

- `pub fn sha256_hex(bytes: impl AsRef<[u8]>) -> String` — lowercase hex SHA-256 of `bytes`.
- `const BLOCK: usize` — SHA-256 block size in bytes.
- `pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32]` — HMAC-SHA256 of `message` under `key` (RFC 2104), used to verify webhook signatures.
- `pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool` — compare without leaking how many leading bytes matched.
- `pub fn normalize_text(text: &str) -> String` — normalize text before hashing so that line-ending and trailing-whitespace differences (git autocrlf, editor settings) are not reported as edits.

## Used by

- [[Api Module]] — from `require_token`, `webhook_signed`
- [[Proposal Module]] — from `create_agent`, `create_retire`
- [[Generate Module]] — from `file_hash`, `generate_with`
- [[cw-server]] — from `signature`
- [[Paths Module]] — from `repo_id`
- [[Scan Module]] — from `scan_repo`
- [[Manifest Module]] — from `body_hash`
- [[Run]] — from `plan`

## Gotchas

- Hash only normalized text when comparing across machines. Raw hashes differ for CRLF files.
- `normalize_text` removes a trailing blank line, so a note with and without a final empty line hashes the same.
- Use `constant_time_eq` for tokens and signatures, never `==`; the early exit of `==` leaks how many leading bytes matched.
- Different lengths compare unequal at once. The length itself is not hidden.

## See also

- [[_index]]
- [[cw-core]]
