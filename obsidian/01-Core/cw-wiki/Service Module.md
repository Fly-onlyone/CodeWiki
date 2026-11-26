---
tags: [core, cw-wiki, module]
---

# Service Module

> High-level operations shared by the CLI, the server and the MCP tools.

## What it does

The entry points for the main jobs of CodeWiki: open the index of a repository, rebuild it from the code, load the notes into search, and bring index and vault up to date in one call. The CLI, the server and the MCP tools all go through here, so they behave the same. The index is a cache. The code and the vault in the repository are the truth.

## Source

- `crates/cw-wiki/src/service.rs` — module `cw_wiki::service`

## How it works

`open_index` and `open_index_in` open the index file for a repository. They then call `built`, which checks the `built` meta key. If it is missing, the index has never been filled, so `built` runs `reindex` and, when a vault folder exists, `index_notes`. Without this, a new data folder or a schema change would leave an empty index, and every query would answer as if the repository had nothing in it.

`reindex` scans the repository, computes graph metrics, and replaces facts and metrics in the index. It also writes search documents for symbols and files, refreshes their vectors, records the head commit, and saves mined git data. It does not touch notes.

`index_notes` reads every note listed in the vault metadata and replaces the note documents. A note file that cannot be read is skipped.

`update` (or `update_in` with an explicit data folder) is the full run: open, reindex, generate the vault notes, then index them. It logs a summary of written, proposed, removed and kept notes. `is_stale` compares the commit the vault was made at with the current head.

## Items

- `pub const META_COMMIT: &str` — meta key holding the commit the index was last built from.
- `pub fn open_index(root: &Path) -> Result<Index>` — open the index of the repository at `root` in the default data directory.
- `pub fn open_index_in(data_dir: &Path, root: &Path) -> Result<Index>` — open the index of the repository at `root` under `data_dir`.
- `pub fn open_existing_index_in(data_dir: &Path, root: &Path) -> Result<Option<Index>>` — open the index of the repository at `root` under `data_dir` when it has been built; `None` when there is none yet. Unlike `open_index_in` this never scans the repository: work that only wants to add to an index, as embedding its documents again does, must not build one as a side effect.
- `pub const DOCUMENT_KINDS: [&str; 3]` — the kinds of search document an index holds: `reindex` writes symbols and files, `index_notes` the notes.
- `const META_BUILT: &str` — meta key set once the index has been filled from the repository.
- `fn built(root: &Path, mut index: Index) -> Result<Index>` — fill an index that has never been built. The index is a cache: a new data directory, or a schema change, leaves it empty while the vault already exists, and every query would then answer as if the repository were empty.
- `pub fn reindex(root: &Path, index: &mut Index) -> Result<RepoFacts>` — scan the repository and rewrite the structural index. Notes are not touched.
- `pub fn index_notes(root: &Path, index: &mut Index) -> Result<usize>` — load every vault note into the search index.
- `pub fn update(root: &Path) -> Result<Report>` — bring index and vault up to date with the working tree.
- `pub fn update_in(data_dir: &Path, root: &Path) -> Result<Report>` — `update` with an explicit data directory.
- `pub fn is_stale(root: &Path) -> Result<bool>` — true when code was committed after the vault was last generated.

And 1 more private item with no comment.

## Depends on

- [[Index]] — uses `Index`, `meta`, `metrics`, `open`
- [[Paths Module]] — uses `data_dir`, `index_path`, `index_path_in`
- [[Scan Module]] — uses `RepoFacts`, `scan_repo`
- [[Generate Module]] — uses `Report`, `generate_with`
- [[Git Module]] — uses `head_commit`, `vault_is_behind`
- [[Gitintel Module]] — uses `mine`, `save`
- [[cw-core]] — uses `VAULT_DIR`
- [[cw-index]] — uses `replace_documents`

## Used by

- [[cw-wiki]] — from `a_healthy_repository_gets_no_suggestions`, `an_index_that_was_never_built_is_not_opened`, `apply_writes_both_files_and_preserves_the_authors_text`, `fixture`
- [[Api Module]] — from `approve_proposal`, `decide_proposals`, `index`, `reindex_repo`
- [[Embed Job Module]] — from `after_update`, `embed_repo`, `resume`
- [[codewiki]] — from `run`
- [[cw-server]] — from `waits`
- [[Workspace Module (cw-server)]] — from `open`

## Gotchas

- `open_existing_index_in` never builds anything. It returns `None` when the index is missing or was never built. Background jobs such as embedding use it, so they cannot start a repository scan by accident.
- `reindex` does not index notes and `index_notes` does not scan code. After changing both you need both calls, or `update`.
- The `built` key is set at the start of a reindex, so a scan that fails later may still leave the index marked as built.
- Vector refresh runs inside `reindex` and `index_notes`, and a slow model can make them take minutes unless an inline budget is set.

## See also

- [[_index]]
- [[cw-wiki]]
