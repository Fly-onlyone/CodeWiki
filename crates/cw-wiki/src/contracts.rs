//! HTTP contracts: the endpoints a repository provides and the ones it calls.
//!
//! Both sides are found lexically in Rust source ([`crate::rustscan`]), with
//! no type information:
//!
//! - **Provided**: axum `.route("/path", get(h).post(h2))` calls, with the
//!   prefix of a literal `.nest("/v1", …)` in the same file applied, and
//!   attribute macros in the actix and rocket style (`#[get("/path")]`).
//! - **Consumed**: string literals and `format!` templates that look like a
//!   URL path, such as `/v1/repos/{id}/notes` or `{url}/v1/health`, other than
//!   the literals that define a provided route.
//!
//! Test code is skipped on both sides. Path parameters (`{id}`, `:id`,
//! `<id>`, `{}` and any interpolation) are normalised to `{}` before two paths
//! are compared, and a parameter on either side matches any one segment: a
//! consumer that interpolates the last segment is taken to call every route
//! of that shape. The rules favour silence over recall.

use std::path::Path;

use cw_core::{Language, Result};
use cw_index::Index;
use serde::{Deserialize, Serialize};

use crate::rustscan::{Literal, Scanned, is_test_path};

/// An endpoint the repository serves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provided {
    /// Upper-case HTTP method, or `ANY` when the route does not name one.
    pub method: String,
    /// Path as written, with nesting prefixes applied.
    pub path: String,
    /// Repo-relative file.
    pub file: String,
    /// 1-based line.
    pub line: u32,
}

/// A URL path the repository's code refers to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Consumed {
    /// Path as written, without scheme, host or query string.
    pub path: String,
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoContracts {
    pub provided: Vec<Provided>,
    pub consumed: Vec<Consumed>,
}

/// A provided route, named by the repository that serves it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteRef {
    pub repo: String,
    pub method: String,
    pub path: String,
    pub file: String,
    pub line: u32,
}

/// A consumed path, named by the repository that calls it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallRef {
    pub repo: String,
    pub path: String,
    pub file: String,
    pub line: u32,
}

/// A route that answers a consumed path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRef {
    pub repo: String,
    pub method: String,
    pub path: String,
}

/// One consumed path and the routes that answer it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub repo: String,
    pub path: String,
    pub file: String,
    pub line: u32,
    pub providers: Vec<ProviderRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoCount {
    pub repo: String,
    pub provided: usize,
    pub consumed: usize,
}

/// Contracts matched across repositories.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceContracts {
    pub repos: Vec<RepoCount>,
    /// Consumed paths at least one repository provides.
    pub matched: Vec<Link>,
    /// Consumed paths no repository provides.
    pub unprovided: Vec<CallRef>,
    /// Provided routes nothing consumes.
    pub unconsumed: Vec<RouteRef>,
}

/// Routing functions and attribute macros named after an HTTP method.
const METHODS: [&str; 9] = [
    "get", "post", "put", "delete", "patch", "head", "options", "trace", "connect",
];

/// First segments that mark a filesystem path rather than a URL path.
const FILESYSTEM_ROOTS: [&str; 16] = [
    "bin", "boot", "dev", "etc", "home", "lib", "mnt", "opt", "proc", "root", "sbin", "sys", "tmp",
    "usr", "var", "windows",
];

/// Words in a placeholder name that say it holds the address of a server.
const BASE_WORDS: [&str; 8] = [
    "url", "uri", "host", "endpoint", "origin", "server", "api", "addr",
];

/// String methods that test or cut text. A path-like literal passed to one
/// (`strip_suffix("/index")`) is a pattern, not a request.
const TEXT_METHODS: [&str; 16] = [
    "contains",
    "ends_with",
    "find",
    "matches",
    "replace",
    "rfind",
    "rsplit",
    "rsplit_once",
    "split",
    "split_once",
    "starts_with",
    "strip_prefix",
    "strip_suffix",
    "trim_end_matches",
    "trim_matches",
    "trim_start_matches",
];

fn is_ident(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Offset of the bracket that opens right after `from`, whitespace aside.
fn call_open(s: &Scanned, from: usize) -> Option<usize> {
    let rest = s.code.get(from..)?;
    let open = from + (rest.len() - rest.trim_start().len());
    (s.code.as_bytes().get(open) == Some(&b'(')).then_some(open)
}

/// The string literal that is the first argument of the call opened at
/// `open`, allowing a `&` and a raw prefix.
fn first_literal(s: &Scanned, open: usize) -> Option<&Literal> {
    let rest = s.code.get(open + 1..)?;
    let quote = rest.find('"')?;
    rest[..quote]
        .chars()
        .all(|c| c.is_whitespace() || matches!(c, '&' | 'r' | '#'))
        .then(|| s.literal_at(open + 1 + quote))
        .flatten()
}

/// HTTP methods named by the routing calls in `code[from..to]`, such as
/// `get(list).post(add)`. Calls nested inside arguments are not looked at.
fn route_methods(s: &Scanned, from: usize, to: usize) -> Vec<String> {
    let bytes = s.code.as_bytes();
    let mut methods: Vec<String> = Vec::new();
    let mut add = |method: String| {
        if !methods.contains(&method) {
            methods.push(method);
        }
    };
    let mut i = from;
    while i < to {
        match bytes[i] {
            b'(' | b'[' | b'{' => i = s.matching(i),
            b if is_ident(b) => {
                let start = i;
                while i < to && is_ident(bytes[i]) {
                    i += 1;
                }
                let word = &s.code[start..i];
                let Some(open) = call_open(s, i) else {
                    continue;
                };
                let close = s.matching(open);
                let name = word.strip_suffix("_service").unwrap_or(word);
                if METHODS.contains(&name) {
                    add(name.to_ascii_uppercase());
                } else if name == "any" {
                    add("ANY".to_string());
                } else if name == "on" {
                    // `on(MethodFilter::GET.or(MethodFilter::POST), handler)`.
                    let args = &s.code[open..close.min(to)];
                    for (at, _) in args.match_indices("MethodFilter::") {
                        let method: String = args[at + 14..]
                            .chars()
                            .take_while(|c| c.is_ascii_alphabetic())
                            .collect();
                        if METHODS.contains(&method.to_ascii_lowercase().as_str()) {
                            add(method);
                        }
                    }
                }
                i = close;
            }
            _ => i += 1,
        }
    }
    methods
}

/// A `.route(…)` call or a route attribute, before prefixes are applied.
struct Route {
    offset: usize,
    path: String,
    methods: Vec<String>,
}

/// A `.nest("/prefix", target)` call and the code its prefix applies to.
struct Nest {
    offset: usize,
    prefix: String,
    target: (usize, usize),
}

/// A provider path as written: rooted, on one line, without a query string.
fn provider_path(value: &str) -> Option<String> {
    let path = value.split('?').next().unwrap_or(value);
    (path.starts_with('/') && !path.contains(char::is_whitespace)).then(|| path.to_string())
}

/// Routes declared with `.route` and `.route_service`. The offsets of the
/// literals that hold their paths are added to `defining`.
fn router_routes(s: &Scanned, defining: &mut Vec<usize>) -> Vec<Route> {
    let mut routes = Vec::new();
    for word in [".route", ".route_service"] {
        for pos in s.find(word) {
            let Some(open) = call_open(s, pos + word.len()) else {
                continue;
            };
            let Some(literal) = first_literal(s, open) else {
                continue;
            };
            let Some(path) = provider_path(&literal.value) else {
                continue;
            };
            defining.push(literal.open);
            let mut methods = route_methods(s, literal.close, s.matching(open) - 1);
            if methods.is_empty() {
                methods.push("ANY".to_string());
            }
            routes.push(Route {
                offset: pos,
                path,
                methods,
            });
        }
    }
    routes
}

/// Routes declared with an attribute macro: `#[get("/path")]`,
/// `#[actix_web::post("/path")]`.
fn attribute_routes(s: &Scanned, defining: &mut Vec<usize>) -> Vec<Route> {
    let bytes = s.code.as_bytes();
    let mut routes = Vec::new();
    for (pos, _) in s.code.match_indices("#[") {
        let close = s.matching(pos + 1);
        let Some(inner) = s.code.get(pos + 2..close.saturating_sub(1)) else {
            continue;
        };
        let Some(paren) = inner.find('(') else {
            continue;
        };
        let name = inner[..paren].trim().rsplit("::").next().unwrap_or("");
        if !METHODS.contains(&name) || bytes.get(pos + 2 + paren) != Some(&b'(') {
            continue;
        }
        let Some(literal) = first_literal(s, pos + 2 + paren) else {
            continue;
        };
        let Some(path) = provider_path(&literal.value) else {
            continue;
        };
        defining.push(literal.open);
        routes.push(Route {
            offset: pos,
            path,
            methods: vec![name.to_ascii_uppercase()],
        });
    }
    routes
}

/// The statement that binds `name` with `let`, nearest before `before`.
fn let_binding(s: &Scanned, name: &str, before: usize) -> Option<(usize, usize)> {
    s.find("let")
        .take_while(|pos| *pos < before)
        .filter(|pos| {
            let rest = s.code[pos + 3..].trim_start();
            let rest = rest
                .strip_prefix("mut")
                .filter(|r| r.starts_with(char::is_whitespace))
                .map_or(rest, str::trim_start);
            rest.strip_prefix(name)
                .is_some_and(|after| !after.bytes().next().is_some_and(is_ident))
        })
        .last()
        .map(|pos| (pos, s.statement_end(pos)))
}

/// The body of the function called `name`. A function that contains
/// `caller` is passed over: a router does not nest itself.
fn fn_body(s: &Scanned, name: &str, caller: usize) -> Option<(usize, usize)> {
    let bytes = s.code.as_bytes();
    let body = |pos: usize| {
        let after = s.code[pos + 2..]
            .trim_start()
            .strip_prefix(name)
            .filter(|rest| !rest.bytes().next().is_some_and(is_ident))?;
        let mut j = s.code.len() - after.len();
        while j < bytes.len() {
            match bytes[j] {
                b'(' | b'[' => j = s.matching(j),
                b'{' => return Some((j, s.matching(j))),
                b';' => return None,
                _ => j += 1,
            }
        }
        None
    };
    s.find("fn")
        .filter_map(body)
        .find(|(from, to)| !(*from..*to).contains(&caller))
}

/// Every `.nest` call with a literal prefix whose target is in this file: a
/// router built inline, a `let` binding, or a function.
fn nests(s: &Scanned, defining: &mut Vec<usize>) -> Vec<Nest> {
    let mut out = Vec::new();
    for pos in s.find(".nest") {
        let Some(open) = call_open(s, pos + 5) else {
            continue;
        };
        let Some(literal) = first_literal(s, open) else {
            continue;
        };
        let Some(prefix) = provider_path(&literal.value) else {
            continue;
        };
        defining.push(literal.open);
        let end = s.matching(open) - 1;
        let Some(argument) = s.code.get(literal.close..end) else {
            continue;
        };
        let target = if argument.contains(".route") || argument.contains(".nest") {
            Some((literal.close, end))
        } else {
            // `routes`, `api::routes(state)`, `self.routes()`: the last name
            // before any call.
            let expression = argument.trim_start_matches([',', ' ', '\n', '\r', '\t', '&']);
            let head = expression.split('(').next().unwrap_or("").trim();
            let name = head.rsplit([':', '.']).next().unwrap_or("").trim();
            if name.is_empty() || !name.bytes().all(is_ident) {
                None
            } else if head.len() == expression.trim().len() {
                let_binding(s, name, pos).or_else(|| fn_body(s, name, pos))
            } else {
                fn_body(s, name, pos)
            }
        };
        if let Some(target) = target {
            out.push(Nest {
                offset: pos,
                prefix: prefix.trim_end_matches('/').to_string(),
                target,
            });
        }
    }
    out
}

/// Prefix that applies to code at `offset`: that of the innermost nest whose
/// target contains it, behind the prefix that applies to that nest itself.
fn prefix_at(nests: &[Nest], offset: usize, seen: &mut Vec<usize>) -> String {
    let innermost = nests
        .iter()
        .enumerate()
        .filter(|(i, n)| !seen.contains(i) && (n.target.0..n.target.1).contains(&offset))
        .min_by_key(|(_, n)| n.target.1 - n.target.0);
    match innermost {
        Some((i, nest)) => {
            seen.push(i);
            prefix_at(nests, nest.offset, seen) + &nest.prefix
        }
        None => String::new(),
    }
}

/// The URL path a literal refers to, or `None` when it does not look like
/// one. Accepts a rooted path, an `http(s)://host/path` URL, and a template
/// whose leading placeholder stands for the server (`{url}/v1/health`).
fn url_path(value: &str) -> Option<String> {
    let mut rest = value;
    let mut after_placeholder = false;
    if let Some(after) = rest
        .strip_prefix("http://")
        .or_else(|| rest.strip_prefix("https://"))
    {
        rest = &after[after.find('/')?..];
    } else if rest.starts_with('{') {
        let close = rest.find('}')?;
        let name = rest[1..close].to_ascii_lowercase();
        rest = &rest[close + 1..];
        // `{dir}/src` is a file path; `{base_url}/items` and `{}/v1/items`
        // are requests.
        let first = rest.get(1..)?.split(['/', '?']).next()?;
        if !is_api_root(first) && !BASE_WORDS.iter().any(|word| name.contains(word)) {
            return None;
        }
        after_placeholder = true;
    }
    if !rest.starts_with('/') {
        return None;
    }
    let path = rest.split(['?', '#']).next()?.trim_end_matches('/');
    let segments: Vec<&str> = path.get(1..)?.split('/').collect();
    let valid = |segment: &&str| {
        !segment.is_empty()
            && !segment.starts_with('.')
            && segment.matches('{').count() == segment.matches('}').count()
            && segment.chars().all(|c| {
                c.is_ascii_alphanumeric()
                    || matches!(c, '-' | '_' | '.' | '~' | '%' | '{' | '}' | ':')
            })
    };
    if path.is_empty() || !segments.iter().all(valid) {
        return None;
    }
    let literal =
        |segment: &&str| !is_parameter(segment) && segment.chars().any(|c| c.is_ascii_alphabetic());
    let first = segments[0];
    let last = segments[segments.len() - 1];
    let has_extension = last.rsplit_once('.').is_some_and(|(_, ext)| {
        (1..=5).contains(&ext.len())
            && ext.chars().all(|c| c.is_ascii_alphanumeric())
            && ext.chars().any(|c| c.is_ascii_alphabetic())
    });
    // `https://host/v1` is the address of an API, not one of its endpoints.
    let only_a_root = segments.len() == 1 && is_api_root(first);
    let plausible = literal(&first)
        && !has_extension
        && !only_a_root
        && !FILESYSTEM_ROOTS.contains(&first.to_ascii_lowercase().as_str())
        // `/c` and `/S` are command-line switches.
        && (segments.len() > 1 || first.len() > 1 || after_placeholder);
    plausible.then(|| path.to_string())
}

/// True for the segment an API is usually mounted under: `api`, `v1`, `v2`.
fn is_api_root(segment: &str) -> bool {
    segment == "api"
        || segment
            .strip_prefix('v')
            .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// True when the literal at `open` is the first argument of a string method
/// that tests or cuts text.
fn is_text_pattern(s: &Scanned, open: usize) -> bool {
    let before = s.code[..open].trim_end_matches(|c: char| c.is_whitespace() || c == '&');
    let Some(callee) = before.strip_suffix('(') else {
        return false;
    };
    let name_at = callee
        .rfind(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .map_or(0, |i| i + 1);
    TEXT_METHODS.contains(&&callee[name_at..])
}

fn is_parameter(segment: &str) -> bool {
    segment.contains('{') || segment.starts_with([':', '<', '*'])
}

/// A path with every parameter segment replaced by `{}`.
pub fn normalize(path: &str) -> String {
    path.split('/')
        .map(|segment| if is_parameter(segment) { "{}" } else { segment })
        .collect::<Vec<_>>()
        .join("/")
}

/// True when two normalised paths have the same shape: the same number of
/// segments, each equal or a parameter on either side.
fn same_shape(a: &str, b: &str) -> bool {
    let (a, b) = (a.trim_end_matches('/'), b.trim_end_matches('/'));
    a.split('/').count() == b.split('/').count()
        && a.split('/')
            .zip(b.split('/'))
            .all(|(x, y)| x == y || x == "{}" || y == "{}")
}

/// Contracts of one Rust source file. `path` is repo-relative.
pub fn scan_source(path: &str, source: &str) -> RepoContracts {
    if is_test_path(path) {
        return RepoContracts::default();
    }
    let s = Scanned::new(source);
    let mut defining = Vec::new();
    let mut routes = router_routes(&s, &mut defining);
    routes.extend(attribute_routes(&s, &mut defining));
    let nests = nests(&s, &mut defining);

    let mut provided = Vec::new();
    for route in routes.iter().filter(|r| !s.in_test(r.offset)) {
        let prefix = prefix_at(&nests, route.offset, &mut Vec::new());
        let full = match route.path.as_str() {
            "/" if !prefix.is_empty() => prefix,
            written => prefix + written,
        };
        for method in &route.methods {
            provided.push(Provided {
                method: method.clone(),
                path: full.clone(),
                file: path.to_string(),
                line: s.line_of(route.offset),
            });
        }
    }
    provided.sort_by(|a, b| a.line.cmp(&b.line).then(a.method.cmp(&b.method)));

    let consumed = s
        .literals
        .iter()
        .filter(|l| !defining.contains(&l.open) && !s.in_test(l.open))
        .filter(|l| !is_text_pattern(&s, l.open))
        .filter_map(|l| {
            Some(Consumed {
                path: url_path(&l.value)?,
                file: path.to_string(),
                line: s.line_of(l.open),
            })
        })
        .collect();
    RepoContracts { provided, consumed }
}

/// Contracts across every indexed Rust file, by file and line.
pub fn scan(root: &Path, index: &Index) -> Result<RepoContracts> {
    let mut out = RepoContracts::default();
    let mut files = index.files()?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    for file in files {
        if file.language != Language::Rust || is_test_path(&file.path) {
            continue;
        }
        // A file removed since the last index has nothing left to report.
        if let Ok(text) = std::fs::read_to_string(root.join(&file.path)) {
            let found = scan_source(&file.path, &text);
            out.provided.extend(found.provided);
            out.consumed.extend(found.consumed);
        }
    }
    Ok(out)
}

/// Match what each repository consumes against what every repository,
/// itself included, provides. `repos` pairs a repository name with its
/// contracts.
pub fn match_workspace(repos: &[(String, RepoContracts)]) -> WorkspaceContracts {
    let routes: Vec<(RouteRef, String)> = repos
        .iter()
        .flat_map(|(repo, contracts)| {
            contracts.provided.iter().map(move |p| {
                let route = RouteRef {
                    repo: repo.clone(),
                    method: p.method.clone(),
                    path: p.path.clone(),
                    file: p.file.clone(),
                    line: p.line,
                };
                (route, normalize(&p.path))
            })
        })
        .collect();
    let mut used = vec![false; routes.len()];
    let mut out = WorkspaceContracts::default();
    for (repo, contracts) in repos {
        out.repos.push(RepoCount {
            repo: repo.clone(),
            provided: contracts.provided.len(),
            consumed: contracts.consumed.len(),
        });
        for call in &contracts.consumed {
            let wanted = normalize(&call.path);
            let mut providers = Vec::new();
            for (i, (route, shape)) in routes.iter().enumerate() {
                if same_shape(&wanted, shape) {
                    used[i] = true;
                    providers.push(ProviderRef {
                        repo: route.repo.clone(),
                        method: route.method.clone(),
                        path: route.path.clone(),
                    });
                }
            }
            if providers.is_empty() {
                out.unprovided.push(CallRef {
                    repo: repo.clone(),
                    path: call.path.clone(),
                    file: call.file.clone(),
                    line: call.line,
                });
            } else {
                out.matched.push(Link {
                    repo: repo.clone(),
                    path: call.path.clone(),
                    file: call.file.clone(),
                    line: call.line,
                    providers,
                });
            }
        }
    }
    out.unconsumed = routes
        .into_iter()
        .zip(used)
        .filter(|(_, used)| !used)
        .map(|((route, _), _)| route)
        .collect();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provided(source: &str) -> Vec<(String, String, u32)> {
        scan_source("src/api.rs", source)
            .provided
            .into_iter()
            .map(|p| (p.method, p.path, p.line))
            .collect()
    }

    fn consumed(source: &str) -> Vec<String> {
        scan_source("src/client.rs", source)
            .consumed
            .into_iter()
            .map(|c| c.path)
            .collect()
    }

    fn row(method: &str, path: &str, line: u32) -> (String, String, u32) {
        (method.to_string(), path.to_string(), line)
    }

    #[test]
    fn finds_axum_routes_with_their_methods() {
        let found = provided(
            "fn app() -> Router {\n    Router::new()\n        .route(\"/health\", get(health))\n        .route(\"/items/{id}\", get(read).put(write).delete(remove))\n        .route(\n            \"/upload\",\n            post(upload).layer(limit(get_limit())),\n        )\n        .route(\"/any\", handler)\n        .route(\"/both\", on(MethodFilter::GET.or(MethodFilter::POST), both))\n        .route_service(\"/files\", get_service(serve_dir))\n}\n",
        );
        assert_eq!(
            found,
            [
                row("GET", "/health", 3),
                row("DELETE", "/items/{id}", 4),
                row("GET", "/items/{id}", 4),
                row("PUT", "/items/{id}", 4),
                row("POST", "/upload", 5),
                row("ANY", "/any", 9),
                row("GET", "/both", 10),
                row("POST", "/both", 10),
                row("GET", "/files", 11),
            ]
        );
    }

    #[test]
    fn applies_nesting_prefixes_from_the_same_file() {
        // A `let` binding, a function, an inline router, and a nest in a nest.
        let found = provided(
            "pub fn routes(state: AppState) -> Router {\n    let items = Router::new().route(\"/\", get(list)).route(\"/{id}\", get(read));\n    let protected = Router::new()\n        .route(\"/repos\", get(repos))\n        .nest(\"/items\", items)\n        .layer(auth());\n    Router::new()\n        .route(\"/v1/health\", get(health))\n        .nest(\"/v1\", protected)\n        .nest(\"/admin/\", admin::routes(state))\n        .nest(\"/inline\", Router::new().route(\"/ping\", get(ping)))\n        .nest(\"/elsewhere\", other_crate::router())\n}\n\nfn admin_routes() -> Router {\n    Router::new().route(\"/users\", post(add_user))\n}\n\nmod admin {\n    pub fn routes(state: AppState) -> Router {\n        Router::new().route(\"/stats\", get(stats))\n    }\n}\n",
        );
        assert_eq!(
            found,
            [
                row("GET", "/v1/items", 2),
                row("GET", "/v1/items/{id}", 2),
                row("GET", "/v1/repos", 4),
                row("GET", "/v1/health", 8),
                row("GET", "/inline/ping", 11),
                row("POST", "/users", 16),
                row("GET", "/admin/stats", 21),
            ]
        );
    }

    #[test]
    fn finds_attribute_routes() {
        let found = provided(
            "#[get(\"/hello/<name>?<lang>\")]\nfn hello(name: &str) {}\n\n#[actix_web::post(\"/users/{id}\")]\nasync fn update() {}\n\n#[derive(Debug)]\n#[serde(rename(\"/not/a/route\"))]\nstruct S;\n",
        );
        assert_eq!(
            found,
            [
                row("GET", "/hello/<name>", 1),
                row("POST", "/users/{id}", 4)
            ]
        );
    }

    #[test]
    fn finds_consumed_paths_and_skips_provider_definitions() {
        let found = consumed(
            "fn calls(b: &Backend, url: &str, id: &str) {\n    b.get(\"/v1/repos\");\n    b.get(&format!(\"/v1/repos/{id}/context?target={}\", 1));\n    agent.get(&format!(\"{url}/v1/health\")).call();\n    fetch(\"https://api.example.com/v2/items/42?page=1\");\n    fetch(\"http://localhost:8080\");\n    Router::new().route(\"/v1/served\", get(h)).nest(\"/v9\", inner);\n    let dir = stem.strip_suffix(\"/v1/index\").unwrap_or(stem);\n    if path.starts_with( \"/v1/admin\") {}\n}\n",
        );
        assert_eq!(
            found,
            [
                "/v1/repos",
                "/v1/repos/{id}/context",
                "/v1/health",
                "/v2/items/42"
            ]
        );
    }

    #[test]
    fn rejects_text_that_is_not_a_url_path() {
        for text in [
            "/",
            "//",
            "/c",
            "relative/path",
            "/tmp/cache",
            "/usr/bin/env",
            "/dev/null",
            "/src/main.rs",
            "/assets/logo.png",
            "/two words/here",
            "/a//b",
            "/../etc",
            "/.git/config",
            "/{}/{}",
            "/2024/01",
            "{dir}/src/lib",
            "{}/notes",
            "/// A doc comment",
            "/path/with\\backslash",
            "/open{brace",
            "/v1",
            "https://api.example.com/v1",
            "{base}/index",
        ] {
            assert_eq!(url_path(text), None, "{text}");
        }
        for (text, path) in [
            ("/health", "/health"),
            ("/v1/repos/", "/v1/repos"),
            ("/api/v1.0", "/api/v1.0"),
            ("/users/:id/posts", "/users/:id/posts"),
            ("{base_url}/items", "/items"),
            ("{}/v1/x", "/v1/x"),
        ] {
            assert_eq!(
                url_path(text),
                (!path.is_empty()).then(|| path.to_string()),
                "{text}"
            );
        }
    }

    #[test]
    fn test_code_is_skipped_on_both_sides() {
        let source = "fn live() { call(\"/v1/live\"); }\n\n#[cfg(test)]\nmod tests {\n    fn app() -> Router { Router::new().route(\"/v1/fake\", get(h)) }\n    #[test]\n    fn t() { call(\"/v1/only-in-tests\"); }\n}\n";
        let found = scan_source("src/lib.rs", source);
        assert!(found.provided.is_empty(), "{found:?}");
        assert_eq!(found.consumed.len(), 1);
        assert_eq!(found.consumed[0].path, "/v1/live");
        assert_eq!(
            scan_source("tests/api.rs", source),
            RepoContracts::default()
        );
    }

    #[test]
    fn normalises_parameters_and_compares_shapes() {
        assert_eq!(normalize("/v1/repos/{id}/notes"), "/v1/repos/{}/notes");
        assert_eq!(normalize("/users/:id/<rest>/{}"), "/users/{}/{}/{}");
        assert_eq!(normalize("/v{version}/x"), "/{}/x");
        assert!(same_shape("/v1/repos/{}/notes", "/v1/repos/{}/notes"));
        assert!(same_shape("/v1/repos/abc/notes", "/v1/repos/{}/notes"));
        assert!(same_shape("/v1/repos/{}/{}", "/v1/repos/{}/notes"));
        assert!(!same_shape("/v1/repos/{}", "/v1/repos/{}/notes"));
        assert!(!same_shape("/v1/repos/{}/files", "/v1/repos/{}/notes"));
    }

    #[test]
    fn matches_consumers_to_providers_across_repositories() {
        let server = scan_source(
            "src/api.rs",
            "fn r() -> Router { Router::new().route(\"/v1/items/{id}\", get(a).put(b)).route(\"/v1/unused\", get(c)) }\n",
        );
        let client = scan_source(
            "src/main.rs",
            "fn c(id: u8) { get(&format!(\"/v1/items/{id}\")); get(\"/v1/missing\"); }\n",
        );
        let all = match_workspace(&[("server".into(), server), ("client".into(), client)]);
        assert_eq!(all.repos[0].provided, 3);
        assert_eq!(all.repos[1].consumed, 2);
        assert_eq!(all.matched.len(), 1);
        assert_eq!(all.matched[0].repo, "client");
        let providers: Vec<(&str, &str)> = all.matched[0]
            .providers
            .iter()
            .map(|p| (p.repo.as_str(), p.method.as_str()))
            .collect();
        assert_eq!(providers, [("server", "GET"), ("server", "PUT")]);
        assert_eq!(all.unprovided.len(), 1);
        assert_eq!(all.unprovided[0].path, "/v1/missing");
        assert_eq!(all.unconsumed.len(), 1);
        assert_eq!(all.unconsumed[0].path, "/v1/unused");
    }
}
