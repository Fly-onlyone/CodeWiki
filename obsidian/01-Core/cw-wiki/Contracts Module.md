---
tags: [core, cw-wiki, module]
---

# Contracts Module

> HTTP contracts: the endpoints a repository provides and the ones it calls.

## Source

- `crates/cw-wiki/src/contracts.rs` — module `cw_wiki::contracts`

## How it works

Main items:

- `RepoContracts` — struct
- `Consumed` — A URL path the repository's code refers to.
- `Provided` — an endpoint the repository serves.
- `ProviderRef` — A route that answers a consumed path.
- `scan` — contracts across every indexed Rust file, by file and line.
- `WorkspaceContracts` — contracts matched across repositories.
- `match_workspace` — match what each repository consumes against what every repository, itself included, provides.
- `scan_source` — contracts of one Rust source file.

## Depends on

- [[Scanned]] — uses `Scanned`, `in_test`, `line_of`, `literal_at`
- [[Index]] — uses `Index`, `files`
- [[Error Module]] — uses `Result`
- [[Rustscan Module]] — uses `is_test_path`
- [[Literal]] — uses `Literal`

## Used by

- [[cw-wiki]] — from `client`, `matches_the_client_to_the_server`, `scan`, `server`
- [[Workspace Module (cw-server)]] — from `WorkspaceContractsView`, `repo_contracts`, `workspace_contracts`
- [[codewiki]] — from `run`
- [[Api Module]] — from `repo_contracts`

## See also

- [[_index]]
- [[cw-wiki]]
