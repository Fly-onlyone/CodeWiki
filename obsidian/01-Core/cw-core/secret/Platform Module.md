---
tags: [core, cw-core, module]
---

# Platform Module

> Module `cw_core::secret::platform` defining 7 items, including `unprotect`, `protect`, and `blob`.

## Source

- `crates/cw-core/src/secret.rs` — module `cw_core::secret::platform`

## How it works

1. `protect` in [[Secret Module]] calls `protect`. `protect` calls `blob`, then `failed`, then `take`, then `unavailable`. `blob` calls `Error::invalid` in [[Error]]. `failed` calls `Error::external` in [[Error]].
2. `unprotect` in [[Secret Module]] calls `unprotect`. `unprotect` calls `blob`, then `failed`, then `take`, then `unavailable`. `unavailable` calls `Error::invalid` in [[Error]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const ENTROPY: &[u8]` — mixed into the encryption, so that another program's call with the same user does not decrypt a CodeWiki secret by accident. It is not a key: the user's account is what protects the secret.
- `fn blob(bytes: &[u8]) -> Result<CRYPT_INTEGER_BLOB>` — private function, returns `Result<CRYPT_INTEGER_BLOB>`; called by `protect`, `unprotect`.
- `fn failed(operation: &str) -> Error` — private function, returns `Error`; called by `protect`, `unprotect`.
- `fn take(output: CRYPT_INTEGER_BLOB) -> Vec<u8>` — copy what the system returned, then give its memory back.
- `pub fn protect(plain: &[u8]) -> Result<Vec<u8>>` — public function, returns `Result<Vec<u8>>`; called by `protect`.
- `pub fn unprotect(stored: &[u8]) -> Result<Vec<u8>>` — public function, returns `Result<Vec<u8>>`; called by `unprotect`.
- `fn unavailable() -> Error` — private function, returns `Error`; called by `protect`, `unprotect`.

## Depends on

- [[Error]] — uses `Error`, `external`, `invalid`
- [[Error Module]] — uses `Result`

## Used by

- [[Secret Module]] — from `protect`, `unprotect`

## See also

- [[_index]]
- [[cw-core]]
