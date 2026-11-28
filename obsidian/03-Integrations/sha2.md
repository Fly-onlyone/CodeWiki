---
tags: [integration]
---

# sha2

> External crate `sha2`, imported by 3 source files.

## What it does

`sha2` supplies SHA-256. CodeWiki uses it for three jobs. The first is change detection: a source file's hash tells whether it changed, so a note can be marked behind the code. The second is trust: the updater checks that a downloaded package matches the checksum the release published. The third is the webhook signature check, which uses HMAC built on the same hash.

## Used for

Imported by 3 source files. Main users:

- [[Download Module]] — imports `Digest`, `Sha256`
- [[Hash Module]] — imports `Digest`, `Sha256`
- [[Wire Tests Module]] — imports `Digest`, `Sha256`

## Configuration

- `crates/cw-core/Cargo.toml`, `crates/cw-update/Cargo.toml` — declare the dependency: version `0.11.1`, locked `0.11.1`

## Wire-up

In the [[Hash Module]], `sha256_hex` hashes bytes into lowercase hex. `source_hash` hashes a source file with every CRLF read as LF, so a checkout with other line endings gives the same hash. `hmac_sha256` builds HMAC (RFC 2104) by hand from `Sha256`: a key longer than one block is hashed first, a shorter one is padded with zeros. `constant_time_eq` compares two values without stopping at the first difference. `normalize_text` strips the byte order mark, trailing spaces and line-ending differences before hashing text.

In the [[Download Module]], `fetch` streams the package to a `.part` file in 64 KB chunks and feeds each chunk to a `Sha256` hasher. `download_limited` compares the result with the hash read from the release's `.sha256` file. On a match it renames the file. On a mismatch it deletes it. The [[Wire Tests Module]] uses `Sha256` to build fake releases.

## Auth mode

No sign-in. HMAC keys come from the caller, such as a webhook secret; this crate holds none.

## Gotchas

- The package is hashed while it downloads, so a bad file is caught without a second read. The temp file is removed on any error.
- A download over 1 GB is refused. A real release is about 490 MB.
- The checksum file may be upper case; `parse_sha256` lowercases it and requires exactly 64 hex digits.
- Compare signatures with `constant_time_eq`, never with `==`.
- `source_hash` treats only CRLF as a line ending. A lone CR, and every other byte, still counts as a change. Without this, a CRLF checkout marked notes behind and failed the wiki sync check.
- `source_hash` of a file with LF only equals `sha256_hex` of its bytes, so hashes recorded before the change stay valid.
- Hash text after `normalize_text` when trailing spaces or a byte order mark must not count as an edit.

## See also

- [[_index]]
