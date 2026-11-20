---
tags: [core, cw-models]
---

# Store

> `Store` struct defined in `cw_models::store` with 9 methods.

## Source

- `crates/cw-models/src/store.rs` — definition of `Store`

## How it works

Declared as `pub struct Store`.

Public API:

- `pub fn new(data_dir: &Path, base: Option<&str>) -> Self` — the store under `data_dir`.
- `pub fn dir(&self, entry: &Entry) -> PathBuf` — the folder of an installed model.
- `pub fn is_installed(&self, entry: &Entry) -> bool` — whether the model is on disk at the catalog's revision, every file with its size.
- `pub fn install( &self, entry: &Entry, progress: &mut dyn FnMut(u64, u64), cancel: &AtomicBool, ) -> Result<Outcome>` — download, verify and install the model.
- `pub fn root(&self) -> &Path` — the folder that holds the installed models.
- `pub fn installed(&self) -> Vec<&'static Entry>` — the catalog's models that are installed.

## Depends on

- [[Store Module (cw-models)]] — uses `DEFAULT_BASE`, `MANIFEST`, `Outcome`, `PARTIAL`
- [[Error]] — uses `invalid`, `io`
- [[Entry]] — uses `Entry`, `size`
- [[Error Module]] — uses `Result`
- [[Catalog Module]] — uses `all`

## Used by

- [[Models Module]] — from `describe`, `start`, `store`, `work`
- [[Model Module (codewiki)]] — from `install`, `list`, `store`
- [[codewiki]] — from `install_by_hand`
- [[cw-server]] — from `install_model`
- [[Settings Module (cw-server)]] — from `describe`
- [[BuiltinEmbedder]] — from `new`

## See also

- [[_index]]
- [[Store Module (cw-models)]]
