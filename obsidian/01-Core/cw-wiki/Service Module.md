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
- `open_existing_index_in` — open the index of the repository at `root` under `data_dir` when it has been built; `None` when there is none yet.
- `index_notes` — load every vault note into the search index.
- `DOCUMENT_KINDS` — the kinds of search document an index holds: `reindex` writes symbols and files, `index_notes` the notes.
- `update_in` — `update` with an explicit data directory.
- `META_COMMIT` — meta key holding the commit the index was last built from.
- `reembed` — bring the search vectors of an index in line with the documents it already holds, with `embedder`.

## Depends on

- [[Index]] — uses `Index`, `meta`, `metrics`, `open`
- [[Paths Module]] — uses `data_dir`, `index_path`, `index_path_in`
- [[Semantic Module]] — uses `Progress`, `embed_documents_with`, `refresh`
- [[cw-index]] — uses `documents`, `replace_documents`
- [[Scan Module]] — uses `RepoFacts`, `scan_repo`
- [[Generate Module]] — uses `Report`, `generate`
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
