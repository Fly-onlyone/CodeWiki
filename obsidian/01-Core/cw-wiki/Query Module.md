---
tags: [core, cw-wiki, module]
---

# Query Module

> Read-side queries served to the app and to agents.

## Source

- `crates/cw-wiki/src/query.rs` — module `cw_wiki::query`

## How it works

Main items:

- `NoteInfo` — struct
- `Related` — A related symbol with how sure the resolver is of the relation.
- `Status` — struct
- `status` — function
- `SymbolDetail` — struct
- `FileContext` — struct
- `Answer` — struct
- `symbol_detail` — detail for one symbol, by qualified or bare name.

## Depends on

- [[Index]] — uses `Index`, `crates`, `edges`, `edges_from`
- [[Error]] — uses `invalid`, `io`, `not_found`
- [[VaultMeta]] — uses `by_path`, `load`, `notes_for_source`
- [[Model Module]] — uses `Edge`, `file_node`
- [[Lifecycle Module]] — uses `NoteState`, `state`
- [[Manifest Module]] — uses `NoteEntry`, `Origin`
- [[Graph]] — uses `from_edges`, `id`
- [[cw-core]] — uses `VAULT_DIR`

## Used by

- [[Api Module]] — from `ask`, `context`, `dependency_path`, `list_notes`
- [[Answer Module]] — from `answer`, `context`
- [[Workspace Module (cw-server)]] — from `summarize`

## See also

- [[_index]]
- [[cw-wiki]]
