---
tags: [core, cw-update]
---

# Release

> A release newer than the running program.

## Source

- `crates/cw-update/src/release.rs` — definition of `Release`

## Items

Declared as `pub struct Release`.

## Used by

- [[Update Module]] — from `State`, `available`
- [[Download Module]] — from `download`, `download_limited`
- [[Release Module]] — from `Check`, `parse`
- [[cw-update]] — from `install`
- [[Wire Tests Module]] — from `release_at`

## See also

- [[_index]]
- [[Release Module]]
