---
tags: [core, cw-wiki, module]
---

# Service Module

> High-level operations shared by the CLI, the server and the MCP tools.

## Source

- `crates/cw-wiki/src/service.rs` — module `cw_wiki::service`

## How it works

Main items:

- `reindex` — scan the repository and rewrite the structural index.
- `open_index_in` — open the index of the repository at `root` under `data_dir`.
- `index_notes` — load every vault note into the search index.
- `update_in` — `update` with an explicit data directory.
- `META_COMMIT` — meta key holding the commit the index was last built from.
- `is_stale` — true when code was committed after the vault was last generated.
- `open_index` — open the index of the repository at `root` in the default data directory.
- `update` — bring index and vault up to date with the working tree.

## Depends on

- [[Index]] — uses `Index`, `meta`, `metrics`, `open`
- [[Paths Module]] — uses `data_dir`, `index_path`, `index_path_in`
- [[Scan Module]] — uses `RepoFacts`, `scan_repo`
- [[Generate Module]] — uses `Report`, `generate`
- [[Git Module]] — uses `head_commit`, `vault_is_behind`
- [[Gitintel Module]] — uses `mine`, `save`
- [[cw-core]] — uses `VAULT_DIR`
- [[cw-index]] — uses `replace_documents`

## Used by

- [[cw-wiki]] — from `a_healthy_repository_gets_no_suggestions`, `apply_writes_both_files_and_preserves_the_authors_text`, `fixture`, `indexed`
- [[Api Module]] — from `approve_proposal`, `index`, `reindex_repo`, `run_update`
- [[codewiki]] — from `run`
- [[Workspace Module (cw-server)]] — from `open`

## See also

- [[_index]]
- [[cw-wiki]]
