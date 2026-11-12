---
tags: [core, cw-core, module]
---

# Hash Module

> Content hashing used for change detection.

## Source

- `crates/cw-core/src/hash.rs` — module `cw_core::hash`

## How it works

Main items:

- `sha256_hex` — lowercase hex SHA-256 of `bytes`.
- `hmac_sha256` — HMAC-SHA256 of `message` under `key` (RFC 2104), used to verify webhook signatures.
- `constant_time_eq` — compare without leaking how many leading bytes matched.
- `normalize_text` — normalize text before hashing so that line-ending and trailing-whitespace differences (git autocrlf, editor settings) are not reported as edits.
- `BLOCK` — SHA-256 block size in bytes.

## Used by

- [[Api Module]] — from `require_token`, `webhook_signed`
- [[cw-server]] — from `signature`
- [[Paths Module]] — from `repo_id`
- [[Scan Module]] — from `scan_repo`
- [[Manifest Module]] — from `body_hash`
- [[Generate Module]] — from `generate`
- [[Semantic Module]] — from `embed_documents`

## See also

- [[_index]]
- [[cw-core]]
