---
tags: [core, cw-wiki, module]
---

# Contracts Module

> HTTP contracts: the endpoints a repository provides and the ones it calls.

## What it does

Both sides are found lexically in Rust source (`crate::rustscan`), with no type information:

- **Provided**: axum `.route("/path", get(h).post(h2))` calls, with the prefix of a literal `.nest("/v1", …)` in the same file applied, and attribute macros in the actix and rocket style (`#[get("/path")]`).
- **Consumed**: string literals and `format!` templates that look like a URL path, such as `/v1/repos/{id}/notes` or `{url}/v1/health`, other than the literals that define a provided route.

Test code is skipped on both sides. Path parameters (`{id}`, `:id`, `<id>`, `{}` and any interpolation) are normalised to `{}` before two paths are compared, and a parameter on either side matches any one segment: a consumer that interpolates the last segment is taken to call every route of that shape. The rules favour silence over recall.

## Source

- `crates/cw-wiki/src/contracts.rs` — module `cw_wiki::contracts`

## How it works

1. `run` in [[codewiki]] and `repo_contracts` in [[Workspace Module (cw-server)]] call `scan`. `scan` calls `is_test_path` in [[Rustscan Module]], then `scan_source`. `scan_source` calls `is_test_path` in [[Rustscan Module]], then `Scanned::new` in [[Scanned]], then `router_routes`, then `attribute_routes`, then `nests`, then `prefix_at`, and 2 more.
2. `workspace_contracts` in [[Workspace Module (cw-server)]] calls `match_workspace`. `match_workspace` calls `normalize`, then `same_shape`. `normalize` calls `is_parameter`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub struct Provided` — an endpoint the repository serves.
- `pub struct Consumed` — A URL path the repository's code refers to.
- `pub struct RepoContracts` — public struct; used by `match_workspace`, `repo_contracts`, `scan`, `scan_source`.
- `pub struct RouteRef` — A provided route, named by the repository that serves it.
- `pub struct CallRef` — A consumed path, named by the repository that calls it.
- `pub struct ProviderRef` — A route that answers a consumed path.
- `pub struct Link` — one consumed path and the routes that answer it.
- `pub struct RepoCount` — public struct; used by `WorkspaceContracts`, `match_workspace`.
- `pub struct WorkspaceContracts` — contracts matched across repositories.
- `fn call_open(s: &Scanned, from: usize) -> Option<usize>` — offset of the bracket that opens right after `from`, whitespace aside.
- `fn router_routes(s: &Scanned, defining: &mut Vec<usize>) -> Vec<Route>` — routes declared with `.route` and `.route_service`. The offsets of the literals that hold their paths are added to `defining`.
- `fn attribute_routes(s: &Scanned, defining: &mut Vec<usize>) -> Vec<Route>` — routes declared with an attribute macro: `#[get("/path")]`, `#[actix_web::post("/path")]`.
- `fn nests(s: &Scanned, defining: &mut Vec<usize>) -> Vec<Nest>` — every `.nest` call with a literal prefix whose target is in this file: a router built inline, a `let` binding, or a function.
- `fn prefix_at(nests: &[Nest], offset: usize, seen: &mut Vec<usize>) -> String` — prefix that applies to code at `offset`: that of the innermost nest whose target contains it, behind the prefix that applies to that nest itself.
- `fn is_parameter(segment: &str) -> bool` — private function, returns `bool`; called by `normalize`, `url_path`.
- `pub fn normalize(path: &str) -> String` — A path with every parameter segment replaced by `{}`.
- `fn same_shape(a: &str, b: &str) -> bool` — true when two normalised paths have the same shape: the same number of segments, each equal or a parameter on either side.
- `pub fn scan_source(path: &str, source: &str) -> RepoContracts` — contracts of one Rust source file. `path` is repo-relative.
- `pub fn scan(root: &Path, index: &Index) -> Result<RepoContracts>` — contracts across every indexed Rust file, by file and line.
- `pub fn match_workspace(repos: &[(String, RepoContracts)]) -> WorkspaceContracts` — match what each repository consumes against what every repository, itself included, provides. `repos` pairs a repository name with its contracts.

And 15 more items.

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
