---
tags: [integration]
---

# sha2

> External crate `sha2`, imported by 3 source files.

## What it does

`sha2` supplies SHA-256. CodeWiki uses it for two jobs. The first is change detection: a file's hash tells whether it changed. The second is trust: the updater checks that a downloaded package matches the checksum the release published, and a webhook signature check uses HMAC built on the same hash.

## Used for

Imported by 3 source files. Main users:

- [[Download Module]] — imports `Digest`, `Sha256`
- [[Hash Module]] — imports `Digest`, `Sha256`
- [[Wire Tests Module]] — imports `Digest`, `Sha256`

## Configuration

- `crates/cw-core/Cargo.toml`, `crates/cw-update/Cargo.toml` — declare the dependency: version `0.11.1`, locked `0.11.1`

## Wire-up

In the [[Hash Module]], `sha256_hex` hashes bytes into lowercase hex. `hmac_sha256` builds HMAC (RFC 2104) by hand from `Sha256`, and `constant_time_eq` compares two values without stopping at the first difference. `normalize_text` strips the byte order mark, trailing spaces and line-ending differences before hashing.

In the [[Download Module]], `fetch` streams the package to a `.part` file in 64 KB chunks and feeds each chunk to a `Sha256` hasher. `download_limited` compares the result with the hash read from the release's `.sha256` file. On a match it renames the file. On a mismatch it deletes it. The [[Wire Tests Module]] uses `Sha256` to build fake releases.

## Auth mode

No sign-in. HMAC keys come from the caller, such as a webhook secret; this crate holds none.

## Gotchas

- The package is hashed while it downloads, so a bad file is caught without a second read. The temp file is removed on any error.
- A download over 1 GB is refused. A real release is about 490 MB.
- The checksum file may be upper case; `parse_sha256` lowercases it and requires exactly 64 hex digits.
- Compare signatures with `constant_time_eq`, never with `==`.
- Hash text after `normalize_text`, or a CRLF checkout looks like an edit.

## See also

- [[_index]]
