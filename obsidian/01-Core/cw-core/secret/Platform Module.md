---
tags: [core, cw-core, module]
---

# Platform Module

> Module `cw_core::secret::platform` defining 7 items, including `unprotect`, `protect`, and `blob`.

## What it does

This private module does the real encryption for [[Secret Module]]. Two versions exist, chosen at compile time. On Windows it calls the Data Protection API (`CryptProtectData` and `CryptUnprotectData`), which ties a secret to the signed-in user on that machine. On other systems it only returns an error, because no store exists there.

## Source

- `crates/cw-core/src/secret.rs` — module `cw_core::secret::platform`

## How it works

`protect` wraps the input and a fixed `ENTROPY` value in system blobs with `blob`, calls the system, and passes the result to `take`. `take` copies the returned bytes into a `Vec` and frees the system memory with `LocalFree`. `unprotect` does the same in reverse. If the system call returns 0, `failed` builds an error from the last OS error. On other systems both functions return the error from `unavailable`, which tells the user to set the environment variable instead.

The flags forbid any window, so a call never blocks on a dialog.

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

## Gotchas

- `ENTROPY` is not a key. The user's account protects the secret; the entropy only stops another program of the same user from decrypting it by accident. Changing the value makes every stored secret unreadable.
- A secret saved by another user or on another machine fails in `unprotect` with a clear message.
- `blob` casts a read-only pointer to mutable because the API wants it; the calls only read it. Inputs over 4 GiB are refused.
- The Windows code is `unsafe`; each block has a SAFETY comment that must stay true if the code changes.

## See also

- [[_index]]
- [[cw-core]]
