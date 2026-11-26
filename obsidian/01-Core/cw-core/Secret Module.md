---
tags: [core, cw-core, module]
---

# Secret Module

> Secrets kept in a settings file: API keys and the token of a remote backend.

## What it does

Stores secrets, such as API keys and a remote backend's token, in settings files without writing them in plain text. On Windows a secret is encrypted with the Data Protection API for the signed-in user, and the file holds the encrypted bytes as hex text. Another account or computer cannot read it. Other systems have no such store, so there a secret comes from an environment variable only.

## Source

- `crates/cw-core/src/secret.rs` — module `cw_core::secret`

## How it works

`protect` rejects an empty secret, encrypts the rest through the platform submodule, and returns hex. `unprotect` reverses it: it decodes the hex, rejects empty data, decrypts, and requires valid UTF-8. Any failure becomes an error with a plain message. `available` tells callers whether saving can work at all (true only on Windows), so settings screens can say so.

`hint` returns the last 4 characters of a secret so the owner can tell which key is saved. Secrets shorter than 12 characters give an empty hint. [[Remote Module]] and [[Settings]] store the hint next to the encrypted value.

On Windows the call also mixes in a fixed text (`CodeWiki settings v1`) so another program run by the same user does not decrypt a CodeWiki secret by accident. It is not a key. The user account is the protection.

## Items

- `const HINT_FLOOR: usize` — A secret shorter than this shows no hint: its last characters would be too large a part of it.
- `const HINT_CHARS: usize` — characters of a secret's end that the hint shows.
- `pub fn available() -> bool` — whether this system can save a secret.
- `pub fn protect(plain: &str) -> Result<String>` — encrypt `plain` for the current user and return the text to store.
- `pub fn unprotect(stored: &str) -> Result<String>` — the secret behind text that `protect` returned, when the same user on the same machine asks.
- `pub fn hint(plain: &str) -> String` — the end of a secret, shown so that its owner can tell which one is saved. Empty for a short secret.

Submodules: [[Platform Module]].

## Depends on

- [[Platform Module]] — uses `protect`, `unprotect`
- [[Error Module]] — uses `Result`
- [[Error]] — uses `invalid`

## Used by

- [[Remote Module]] — from `save_in`, `token_in`
- [[Settings Module (cw-server)]] — from `apply`, `describe`
- [[Resolved]] — from `saved_secret`
- [[Settings]] — from `set`

## Gotchas

- A stored secret cannot be moved. Copy the settings file to another user or machine and `unprotect` fails; the user must enter the secret again.
- On non-Windows systems `protect` cannot save. Use environment variables.
- Never log a plain secret. Only the hint is meant to be shown.
- The Windows code uses `unsafe` calls and frees the system buffer with `LocalFree` after copying.

## See also

- [[_index]]
- [[cw-core]]
