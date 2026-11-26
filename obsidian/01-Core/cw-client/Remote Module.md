---
tags: [core, cw-client, module]
---

# Remote Module

> A backend on another machine, or in Docker, that the user chose in the app instead of naming it with `CODEWIKI_URL`.

## What it does

Lets a user point the app at a backend in Docker or on another machine, without setting environment variables. The address, a use flag and an encrypted token are saved in `remote.json` in the data directory. Only clients write that file. The backend keeps its settings in another file, so neither side overwrites the other. Tokens are encrypted for the current user through [[Secret Module]].

## Source

- `crates/cw-client/src/remote.rs` — module `cw_client::remote`

## How it works

`save_in` checks the address with `normalized` (must start with http:// or https://, trailing slash removed). With a new token, it encrypts it with `protect` and stores a short `hint` (the last characters) beside it. With `None`, it reuses the saved token, but only when the saved address is the same. Then it writes `{url, use: true, token}`.

`write` saves to `remote.json.tmp` and renames it over the old file, so a reader never sees half a file.

`saved_in` returns the address, the use flag and the hint, never the token. `token_in` decrypts it on demand. `set_in_use_in` flips the `use` flag, keeping address and token, so a user can go back to the local backend and return later. `forget_in` deletes the file and treats a missing file as success.

[[Backend]] calls `saved_in` and `token_in` when it chooses a backend.

## Items

- `pub(crate) const FILE: &str` — the file's name in the data directory.
- `pub struct Remote` — the saved backend, without its token.
- `fn file(data_dir: &Path) -> PathBuf` — private function, returns `PathBuf`; called by `forget_in`, `read`, `write`.
- `fn read(data_dir: &Path) -> Option<Value>` — private function, returns `Option<Value>`; called by `save_in`, `saved_in`, `set_in_use_in`, `token_in`.
- `fn write(data_dir: &Path, value: &Value) -> Result<(), String>` — the new file replaces the old one in one step, so a reader never sees half of it.
- `pub(crate) fn saved_in(data_dir: &Path) -> Option<Remote>` — the saved backend, when there is one.
- `pub(crate) fn token_in(data_dir: &Path) -> Result<String, String>` — the saved backend's token.
- `pub(crate) fn normalized(url: &str) -> Result<String, String>` — an address as it is saved and called: with its scheme, without a slash at the end.
- `pub(crate) fn save_in(data_dir: &Path, url: &str, token: Option<&str>) -> Result<(), String>` — save a backend and make clients use it. `token: None` keeps the token saved for the same address.
- `pub(crate) fn set_in_use_in(data_dir: &Path, in_use: bool) -> Result<(), String>` — make clients use the saved backend, or the one on this machine. Nothing saved is the same as not using it.
- `pub(crate) fn forget_in(data_dir: &Path) -> Result<(), String>` — forget the saved backend and its token.

## Depends on

- [[Secret Module]] — uses `hint`, `protect`, `unprotect`

## Used by

- [[cw-client]] — from `check_remote_in`, `forget_remote`, `save_remote`, `saved_remote`
- [[Settings Pages Module]] — from `Model`
- [[Backend]] — from `named`

## Gotchas

- A token belongs to one address. Saving a different address without a token fails with "enter the token of the backend"; a token is never sent to another backend.
- A token is encrypted for one Windows user on one machine. Another user or machine gets "the saved token cannot be read ... enter it again". Where encryption is unavailable, saving fails and the message suggests `CODEWIKI_URL` and `CODEWIKI_TOKEN`.
- `CODEWIKI_URL` wins over this file.
- `set_in_use_in(false)` with nothing saved succeeds, because nothing saved already means local. `set_in_use_in(true)` with nothing saved fails.
- A missing or broken `remote.json` reads as nothing saved, not as an error.

## See also

- [[_index]]
- [[cw-client]]
