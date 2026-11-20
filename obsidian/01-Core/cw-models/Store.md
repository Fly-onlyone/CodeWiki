---
tags: [core, cw-models]
---

# Store

> `Store` struct defined in `cw_models::store` with 9 methods.

## Source

- `crates/cw-models/src/store.rs` — definition of `Store`

## How it works

1. `store` in [[Model Module (codewiki)]] and `store` in [[Models Module]], and 1 more, call `new`.

## Items

Declared as `pub struct Store`.

Members:

- `pub fn new(data_dir: &Path, base: Option<&str>) -> Self` — the store under `data_dir`. `base` replaces the host the files come from; `None` is Hugging Face.
- `pub fn root(&self) -> &Path` — the folder that holds the installed models.
- `pub fn dir(&self, entry: &Entry) -> PathBuf` — the folder of an installed model.
- `pub fn is_installed(&self, entry: &Entry) -> bool` — whether the model is on disk at the catalog's revision, every file with its size. The hashes were checked when it was installed.
- `pub fn installed(&self) -> Vec<&'static Entry>` — the catalog's models that are installed.
- `fn have(&self, entry: &Entry) -> u64` — bytes of the model that are already downloaded.
- `install` — download, verify and install the model. `progress` hears the bytes there are and the bytes there will be, many times a second; `cancel` stops the download at the next chunk.
- `pub fn remove(&self, entry: &Entry) -> Result<()>` — delete the model and whatever was downloaded of it. Not an error when there is nothing.

And 1 more private item with no comment.

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
- [[BuiltinEmbedder]] — from `new`

## See also

- [[_index]]
- [[Store Module (cw-models)]]
