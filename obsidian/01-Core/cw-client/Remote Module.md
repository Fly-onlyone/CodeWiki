---
tags: [core, cw-client, module]
---

# Remote Module

> A backend on another machine, or in Docker, that the user chose in the app instead of naming it with `CODEWIKI_URL`.

## What it does

Its address and token are in `remote.json` in the data directory. Only clients write that file: the backend keeps its own settings in another one, so neither can overwrite the other's change. The token is stored encrypted for the current user (`cw_core::secret`).

## Source

- `crates/cw-client/src/remote.rs` — module `cw_client::remote`

## How it works

1. `saved_remote` in [[cw-client]] and `Backend::named` in [[Backend]] call `saved_in`. `saved_in` calls `read`. `read` calls `file`.
2. `check_remote_in` in [[cw-client]] and `Backend::named` in [[Backend]] call `token_in`. `token_in` calls `read`, then `unprotect` in [[Secret Module]].
3. `check_remote_in` in [[cw-client]] calls `normalized`.
4. `save_remote` in [[cw-client]] calls `save_in`. `save_in` calls `normalized`, then `protect`, then `hint` (both in [[Secret Module]]), then `read`, then `write`. `write` calls `file`.
5. `use_local` in [[cw-client]] calls `set_in_use_in`. `set_in_use_in` calls `read`, then `write`.

1 more entry point: `forget_in`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[cw-client]]
