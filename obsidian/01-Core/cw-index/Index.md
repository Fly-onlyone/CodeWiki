---
tags: [core, cw-index]
---

# Index

> `Index` struct defined in `cw_index::store` with 19 methods.

## Source

- `crates/cw-index/src/store.rs` — definition of `Index`

## How it works

Declared as `pub struct Index`.

Public API:

- `pub fn find_symbols(&self, needle: &str, limit: usize) -> Result<Vec<Symbol>>` — symbols whose bare or qualified name contains `needle`, exact bare-name matches first.
- `pub fn replace_facts(&mut self, facts: &RepoFacts) -> Result<()>` — replace every structural fact in one transaction.
- `pub fn open_in_memory() -> Result<Self>`
- `pub fn open(path: &Path) -> Result<Self>`
- `pub fn files(&self) -> Result<Vec<FileRecord>>`
- `pub fn symbols(&self) -> Result<Vec<Symbol>>`

## Depends on

- [[Store Module]] — uses `NodeMetrics`, `SYMBOL_COLUMNS`, `db`, `edge_from_row`
- [[Schema Module]] — uses `DROP_ALL`, `SCHEMA`, `SCHEMA_VERSION`
- [[Model Module]] — uses `Edge`, `FileRecord`
- [[Error]] — uses `external`, `io`
- [[Error Module]] — uses `Result`
- [[Resolve Module]] — uses `ExternalUse`
- [[Scan Module]] — uses `RepoFacts`
- [[Workspace Module]] — uses `CrateInfo`

## Used by

- [[cw-wiki]] — from `a_healthy_repository_gets_no_suggestions`, `apply_writes_both_files_and_preserves_the_authors_text`, `fixture`, `indexed`
- [[Gitintel Module]] — from `blast_radius`, `change_risk`, `dependents_by_file`, `file_risk`
- [[Query Module]] — from `answer`, `context`, `dependency_path`, `overview`
- [[Service Module]] — from `built`, `index_notes`, `open_index`, `open_index_in`
- [[Coverage Module]] — from `import`, `import_text`, `load`, `report`
- [[Api Module]] — from `find_symbols`, `graph`, `index`, `list_files`
- [[Semantic Module]] — from `embed_documents`, `refresh`, `search`, `search_with`
- [[Health Module]] — from `dead_code`, `doc_drift`, `file_health`

## See also

- [[_index]]
- [[Store Module]]
