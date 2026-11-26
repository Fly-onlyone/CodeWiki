---
tags: [core, cw-index]
---

# Index

> `Index` struct defined in `cw_index::store` with 19 methods.

## What it does

`Index` is the SQLite database that holds everything CodeWiki knows about a repository: files, symbols, the edges between them (calls, uses), crate info, graph metrics, search documents and vectors. The scanner writes facts into it. The query, health, risk and API code read facts out. It is the one place the rest of the program asks "what is in this code".

## Source

- `crates/cw-index/src/store.rs` — definition of `Index`

## How it works

`open` makes the parent folder, opens the file, and sets WAL mode (readers do not block the writer) with `synchronous = NORMAL`. `open_in_memory` skips both; tests use it. Both call `init`.

`init` reads `PRAGMA user_version`. If it differs from `SCHEMA_VERSION`, it drops all tables, creates them again from `SCHEMA` and stamps the new version. See [[Schema Module]].

`replace_facts` is a full rewrite. In one transaction it empties files, symbols, edges, crates and external uses, then inserts the new [[Scan Module]] output. The reading methods (`files`, `symbols`, `edges_from`, `edges_to`, and so on) all go through one private `collect` helper and return rows in a fixed order.

`replace_metrics` follows the same delete-then-insert pattern for graph scores. `set_meta` and `meta` keep small key/value facts such as the indexed commit.

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
- [[Gitintel Module]] — from `blast_radius`, `change_risk`, `dependents_by_file`, `file_risk`
- [[Query Module]] — from `answer`, `context`, `dependency_path`, `overview`
- [[Service Module]] — from `built`, `index_notes`, `open_existing_index_in`, `open_index`
- [[Coverage Module]] — from `import`, `import_text`, `load`, `report`
- [[Api Module]] — from `find_symbols`, `graph`, `index`, `list_files`
- [[Health Module]] — from `dead_code`, `doc_drift`, `file_health`

## Gotchas

- A changed schema version wipes the whole index, search documents and vectors too. It is rebuilt, not migrated.
- `replace_facts` inserts symbols with `INSERT OR IGNORE`. If two symbols share a qualified name, the first one wins and the rest vanish silently.
- `find_symbols` strips `%` from the needle and escapes `_`, so both act as plain characters. Exact bare-name matches come first, then shorter qualified names.
- A stored external-use row whose names do not decode becomes a row with no names, not an error.
- A symbol row with an unknown kind reads back as a function. An unknown edge kind reads back as a reference.

## See also

- [[_index]]
- [[Store Module]]
