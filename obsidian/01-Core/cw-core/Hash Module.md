---
tags: [core, cw-core, module]
---

# Hash Module

> Content hashing used for change detection.

## Source

- `crates/cw-core/src/hash.rs` — module `cw_core::hash`

## How it works

1. `repo_id` in [[Paths Module]] and `scan_repo` in [[Scan Module]], and 4 more, call `sha256_hex`.
2. `webhook_signed` in [[Api Module]] calls `hmac_sha256`.
3. `require_token` and `webhook_signed` in [[Api Module]] call `constant_time_eq`.
4. `body_hash` in [[Manifest Module]] calls `normalize_text`.

## Items

- `pub fn sha256_hex(bytes: impl AsRef<[u8]>) -> String` — lowercase hex SHA-256 of `bytes`.
- `const BLOCK: usize` — SHA-256 block size in bytes.
- `pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32]` — HMAC-SHA256 of `message` under `key` (RFC 2104), used to verify webhook signatures.
- `pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool` — compare without leaking how many leading bytes matched.
- `pub fn normalize_text(text: &str) -> String` — normalize text before hashing so that line-ending and trailing-whitespace differences (git autocrlf, editor settings) are not reported as edits.

## Used by

- [[Api Module]] — from `require_token`, `webhook_signed`
- [[Generate Module]] — from `file_hash`, `generate_with`
- [[cw-server]] — from `signature`
- [[Paths Module]] — from `repo_id`
- [[Scan Module]] — from `scan_repo`
- [[Manifest Module]] — from `body_hash`
- [[Proposal Module]] — from `create_agent`
- [[Run]] — from `plan`

## See also

- [[_index]]
- [[cw-core]]
