---
tags: [core, cw-wiki, module]
---

# Service Module

> High-level operations shared by the CLI, the server and the MCP tools.

## Source

- `crates/cw-wiki/src/service.rs` — module `cw_wiki::service`

## How it works

1. `Repo::index` in [[Api Module]] and `open` in [[Workspace Module (cw-server)]] call `open_index_in`. `open_index_in` calls `built`, then `Index::open` in [[Index]], then `index_path_in` in [[Paths Module]]. `built` calls `reindex`, then `index_notes`.
2. `run` in [[codewiki]] and `reindex_repo` in [[Api Module]] call `reindex`. `reindex` calls `scan_repo` in [[Scan Module]], then `analyze` in [[Analyze Module]], then `refresh` in [[Semantic Module]], then `head_commit` in [[Git Module]], then `save`, then `mine` (both in [[Gitintel Module]]).
3. `run` in [[codewiki]] calls `open_index`. `open_index` calls `built`, then `Index::open` in [[Index]], then `index_path` in [[Paths Module]].
4. `after_update` and `embed_repo` in [[Models Module]], and 1 more, call `open_existing_index_in`. `open_existing_index_in` calls `index_path_in` in [[Paths Module]], then `Index::open` in [[Index]].
5. `approve_proposal` and `write_note` in [[Api Module]] call `index_notes`. `index_notes` calls `VaultMeta::load` in [[VaultMeta]], then `Note::parse` in [[Note]], then `refresh` in [[Semantic Module]].

3 more entry points: `update`, `update_in`, `is_stale`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const META_COMMIT: &str` — meta key holding the commit the index was last built from.
- `pub fn open_index(root: &Path) -> Result<Index>` — open the index of the repository at `root` in the default data directory.
- `pub fn open_index_in(data_dir: &Path, root: &Path) -> Result<Index>` — open the index of the repository at `root` under `data_dir`.
- `pub fn open_existing_index_in(data_dir: &Path, root: &Path) -> Result<Option<Index>>` — open the index of the repository at `root` under `data_dir` when it has been built; `None` when there is none yet. Unlike `open_index_in` this never scans the repository: work that only wants to add to an index, as embedding its documents again does, must not build one as a side effect.
- `pub const DOCUMENT_KINDS: [&str; 3]` — the kinds of search document an index holds: `reindex` writes symbols and files, `index_notes` the notes.
- `reembed` — bring the search vectors of an index in line with the documents it already holds, with `embedder`. Nothing is scanned, generated or written to the vault, and no LLM is called. `on_batch` hears how far the run is after each batch and may stop it; the next call goes on from there.
- `const META_BUILT: &str` — meta key set once the index has been filled from the repository.
- `fn built(root: &Path, mut index: Index) -> Result<Index>` — fill an index that has never been built. The index is a cache: a new data directory, or a schema change, leaves it empty while the vault already exists, and every query would then answer as if the repository were empty.
- `pub fn reindex(root: &Path, index: &mut Index) -> Result<RepoFacts>` — scan the repository and rewrite the structural index. Notes are not touched.
- `pub fn index_notes(root: &Path, index: &mut Index) -> Result<usize>` — load every vault note into the search index.
- `pub fn update(root: &Path, writer: Option<&dyn ProseWriter>) -> Result<Report>` — bring index and vault up to date with the working tree.
- `pub fn update_in(data_dir: &Path, root: &Path, writer: Option<&dyn ProseWriter>) -> Result<Report>` — `update` with an explicit data directory.
- `pub fn is_stale(root: &Path) -> Result<bool>` — true when code was committed after the vault was last generated.

And 1 more private item with no comment.

## Depends on

- [[Index]] — uses `Index`, `meta`, `metrics`, `open`
- [[Paths Module]] — uses `data_dir`, `index_path`, `index_path_in`
- [[Semantic Module]] — uses `Progress`, `embed_documents_with`, `refresh`
- [[cw-index]] — uses `documents`, `replace_documents`
- [[Scan Module]] — uses `RepoFacts`, `scan_repo`
- [[Generate Module]] — uses `Report`, `generate_with`
- [[Git Module]] — uses `head_commit`, `vault_is_behind`
- [[Gitintel Module]] — uses `mine`, `save`

## Used by

- [[cw-wiki]] — from `a_healthy_repository_gets_no_suggestions`, `an_index_that_was_never_built_is_not_opened`, `apply_writes_both_files_and_preserves_the_authors_text`, `fixture`
- [[Api Module]] — from `approve_proposal`, `index`, `reindex_repo`, `run_update`
- [[Models Module]] — from `after_update`, `embed_repo`, `resume`
- [[codewiki]] — from `run`
- [[cw-server]] — from `waits`
- [[Workspace Module (cw-server)]] — from `open`

## See also

- [[_index]]
- [[cw-wiki]]
