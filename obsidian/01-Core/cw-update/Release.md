---
tags: [core, cw-update]
---

# Release

> A release newer than the running program.

## What it does

`Release` describes one update the app can install: its version, the zip file name, the address of the zip and the address of its `.sha256` file. It exists only when a newer release was found, so holding one means an update is available.

## Source

- `crates/cw-update/src/release.rs` — definition of `Release`

## How it works

`parse` in [[Release Module]] builds it from GitHub's answer. The zip name is always `CodeWiki-<version>-windows-x64.zip`, and the two addresses are the download links of the assets with the matching names. [[Download Module]] reads `sha256_url` and `zip_url`. `install` in [[cw-update]] passes the version on to unpacking.

## Items

Declared as `pub struct Release`.

## Used by

- [[Update Module]] — from `State`, `available`
- [[Download Module]] — from `download`, `download_limited`
- [[Release Module]] — from `Check`, `parse`
- [[cw-update]] — from `install`
- [[Wire Tests Module]] — from `release_at`

## Gotchas

- A `Release` is never built without both addresses. A release with the zip but no checksum file is reported as unavailable, not as a half-filled `Release`.
- The version comes from the tag with a leading `v` removed, and the asset names are built from that version, so a tag and an asset name that disagree give no `Release`.

## See also

- [[_index]]
- [[Release Module]]
