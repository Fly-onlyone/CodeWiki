---
tags: [core, cw-index]
---

# Index

> `Index` struct defined in `cw_index::store` with 19 methods.

## Source

- `crates/cw-index/src/store.rs` — definition of `Index`

## How it works

1. `open_existing_index_in` and `open_index` in [[Service Module]], and 1 more, call `open`. `open` calls `Error::io` in [[Error]], then `db` in [[Store Module]], then `init`. `init` calls `db` in [[Store Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct Index`.

Members:

- `pub fn open(path: &Path) -> Result<Self>` — public method, returns `Result<Self>`; called by `open_existing_index_in`, `open_index`, `open_index_in`.
- `pub fn open_in_memory() -> Result<Self>` — public method, returns `Result<Self>`.
- `fn init(conn: Connection) -> Result<Self>` — private method, returns `Result<Self>`; called by `Index::open`, `Index::open_in_memory`.
- `pub fn replace_facts(&mut self, facts: &RepoFacts) -> Result<()>` — replace every structural fact in one transaction.
- `pub fn set_meta(&self, key: &str, value: &str) -> Result<()>` — public method, returns `Result<()>`.
- `pub fn meta(&self, key: &str) -> Result<Option<String>>` — public method, returns `Result<Option<String>>`.
- `pub fn files(&self) -> Result<Vec<FileRecord>>` — public method, returns `Result<Vec<FileRecord>>`.
- `pub fn symbols(&self) -> Result<Vec<Symbol>>` — public method, returns `Result<Vec<Symbol>>`.
- `pub fn symbol(&self, qualified: &str) -> Result<Option<Symbol>>` — public method, returns `Result<Option<Symbol>>`.
- `pub fn symbols_in_file(&self, file: &str) -> Result<Vec<Symbol>>` — public method, returns `Result<Vec<Symbol>>`.
- `pub fn find_symbols(&self, needle: &str, limit: usize) -> Result<Vec<Symbol>>` — symbols whose bare or qualified name contains `needle`, exact bare-name matches first.
- `pub fn edges(&self) -> Result<Vec<Edge>>` — public method, returns `Result<Vec<Edge>>`.
- `pub fn edges_from(&self, node: &str) -> Result<Vec<Edge>>` — public method, returns `Result<Vec<Edge>>`.
- `pub fn edges_to(&self, node: &str) -> Result<Vec<Edge>>` — public method, returns `Result<Vec<Edge>>`.
- `pub fn crates(&self) -> Result<Vec<CrateInfo>>` — public method, returns `Result<Vec<CrateInfo>>`.
- `pub fn external_uses(&self) -> Result<Vec<ExternalUse>>` — public method, returns `Result<Vec<ExternalUse>>`.
- `pub fn replace_metrics(&mut self, metrics: &[(String, NodeMetrics)]) -> Result<()>` — public method, returns `Result<()>`.
- `pub fn metrics(&self) -> Result<Vec<(String, NodeMetrics)>>` — public method, returns `Result<Vec<(String, NodeMetrics)>>`; called by `Index::replace_metrics`.

And 1 more private item with no comment.

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

- [[Semantic Module]] — from `behind`, `embed_documents`, `embed_documents_with`, `embed_within`
- [[cw-wiki]] — from `a_healthy_repository_gets_no_suggestions`, `apply_writes_both_files_and_preserves_the_authors_text`, `fixture`, `indexed`
- [[Service Module]] — from `built`, `index_notes`, `open_existing_index_in`, `open_index`
- [[Gitintel Module]] — from `blast_radius`, `change_risk`, `dependents_by_file`, `file_risk`
- [[Query Module]] — from `answer`, `context`, `dependency_path`, `overview`
- [[Coverage Module]] — from `import`, `import_text`, `load`, `report`
- [[Api Module]] — from `find_symbols`, `graph`, `index`, `list_files`
- [[Health Module]] — from `dead_code`, `doc_drift`, `file_health`

## See also

- [[_index]]
- [[Store Module]]
