---
tags: [core, cw-core, module]
---

# Secret Module

> Secrets kept in a settings file: API keys and the token of a remote backend.

## What it does

On Windows a secret is encrypted with the Data Protection API for the user who runs the program, so the file is of no use to another account or on another machine. The stored form is the encrypted bytes as hexadecimal text. Other systems have no such store: there a secret cannot be saved and comes from an environment variable only.

## Source

- `crates/cw-core/src/secret.rs` — module `cw_core::secret`

## How it works

1. `save_in` in [[Remote Module]] and `Settings::set` in [[Settings]] call `protect`. `protect` calls `Error::invalid` in [[Error]], then `protect` in [[Platform Module]].
2. `token_in` in [[Remote Module]] and `Resolved::saved_secret` in [[Resolved]] call `unprotect`. `unprotect` calls `Error::invalid` in [[Error]], then `unprotect` in [[Platform Module]].
3. `save_in` in [[Remote Module]] and `Settings::set` in [[Settings]] call `hint`.
4. `apply` and `describe` in [[Settings Module (cw-server)]] call `available`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[cw-core]]
