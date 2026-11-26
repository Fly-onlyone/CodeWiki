---
tags: [core, cw-wiki, module]
---

# Contracts Module

> HTTP contracts: the endpoints a repository provides and the ones it calls.

## What it does

When several services talk over HTTP, a renamed route can silently break a caller. This module finds, in Rust source, the routes a repository serves and the URL paths it calls. Across repositories it matches the two sides, so you can see calls nobody answers and routes nobody calls. It reads text only, with no type information, and it prefers to stay silent over guessing.

## Source

- `crates/cw-wiki/src/contracts.rs` — module `cw_wiki::contracts`

## How it works

`scan_source` handles one file. It skips test paths, then builds a `Scanned` view of the code (comments and strings separated).

Provided side: `router_routes` finds `.route("/p", get(h).post(h2))` and `.route_service` calls and reads the method names with `route_methods`. A route with no method name is `ANY`. `attribute_routes` finds `#[get("/p")]` style attributes. `nests` finds `.nest("/v1", target)` calls whose target is an inline router, a `let` binding or a function in the same file. `prefix_at` stacks the prefixes of the innermost enclosing nests onto each route.

Consumed side: every string literal that is not the path of a provided route, not in test code, and not the argument of a text method like `starts_with` goes to `url_path`. That keeps rooted paths, `http(s)://host/path` URLs and templates like `{base_url}/v1/health`.

`scan` runs this over every indexed Rust file. `match_workspace` takes several repositories. It normalises every path with `normalize` (parameters become `{}`) and compares with `same_shape`: same segment count, and each segment equal or a parameter on either side. The result has matched links, unprovided calls and unconsumed routes.

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

## Gotchas

- Only Rust is scanned. A client written in another language is invisible.
- A path built at run time is not seen, only literals and `format!` templates.
- A parameter segment on either side matches any segment, so one interpolated call can match many routes. Treat "matched" as likely, not proven.
- `url_path` rejects filesystem roots (`/usr`), paths ending in file extensions, bare API roots (`/v1`), and one-letter paths (`/c`) to avoid false calls.
- Nesting is applied only within the same file. A router nested from another file loses its prefix.
- Provided paths with a query string are cut at the `?`.

## See also

- [[_index]]
- [[cw-wiki]]
