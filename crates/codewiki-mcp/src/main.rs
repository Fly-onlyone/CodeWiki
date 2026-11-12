//! MCP stdio bridge between coding agents and the CodeWiki backend.
//!
//! Standard output carries the MCP protocol, so nothing else may be printed
//! there; diagnostics go to standard error.

use std::sync::{Arc, OnceLock};

use cw_client::{Backend, encode};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::transport::stdio;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt, schemars, tool, tool_handler, tool_router,
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct Bridge {
    /// Connected lazily so listing tools never starts a backend.
    backend: Arc<OnceLock<Result<Backend, String>>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct RepoParam {
    /// Repository root path. Defaults to the directory the agent runs in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ContextParams {
    /// A repo-relative file path (`src/lib.rs`) or a symbol name (`Parser` or `my_crate::Parser`).
    target: String,
    /// Repository root path. Defaults to the directory the agent runs in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SymbolParams {
    /// Bare or qualified symbol name.
    name: String,
    /// Repository root path. Defaults to the directory the agent runs in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// Words to search for.
    query: String,
    /// Restrict results to `note`, `symbol` or `file`.
    #[serde(default)]
    kind: Option<String>,
    /// Maximum results, 1 to 50. Defaults to 10.
    #[serde(default)]
    limit: Option<u32>,
    /// Repository root path. Defaults to the directory the agent runs in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct AnswerParams {
    /// A question about the codebase in plain language.
    question: String,
    /// Repository root path. Defaults to the directory the agent runs in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PathParams {
    /// Symbol the chain starts at.
    from: String,
    /// Symbol the chain should reach.
    to: String,
    /// Repository root path. Defaults to the directory the agent runs in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct NoteParams {
    /// Note name (`Scan Module`) or vault path (`01-Core/cw-parse/Scan Module.md`).
    note: String,
    /// Repository root path. Defaults to the directory the agent runs in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FileParams {
    /// Repo-relative file path, for example `src/lib.rs`.
    path: String,
    /// Repository root path. Defaults to the directory the agent runs in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ChangeParams {
    /// Repo-relative paths of the changed files. Omit to use the working tree's uncommitted changes.
    #[serde(default)]
    files: Option<Vec<String>>,
    /// Repository root path. Defaults to the directory the agent runs in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CoverageParams {
    /// Path of an LCOV or Cobertura report inside the repository, for example `lcov.info`. Omit to use the last imported report.
    #[serde(default)]
    report: Option<String>,
    /// Repository root path. Defaults to the directory the agent runs in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SecurityParams {
    /// Lowest level to return: `note`, `warning` or `error`. Defaults to `note`.
    #[serde(default)]
    level: Option<String>,
    /// Repository root path. Defaults to the directory the agent runs in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct PlanParams {
    /// Maximum suggestions, 1 to 50. Defaults to 10.
    #[serde(default)]
    limit: Option<u32>,
    /// Repository root path. Defaults to the directory the agent runs in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ContractsParams {
    /// Set to true to match the contracts of every registered repository against each other. Defaults to false: the contracts of one repository.
    #[serde(default)]
    workspace: Option<bool>,
    /// Repository root path. Defaults to the directory the agent runs in. Ignored when `workspace` is true.
    #[serde(default)]
    repo: Option<String>,
}

fn text(body: impl Into<String>) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![ContentBlock::text(body)]))
}

fn pretty(value: &Value) -> Result<CallToolResult, McpError> {
    text(serde_json::to_string_pretty(value).unwrap_or_default())
}

impl Bridge {
    /// Run blocking backend work for one tool call. Failures are returned to
    /// the agent as tool errors it can read and act on.
    async fn call<F>(&self, repo: Option<String>, work: F) -> Result<Value, McpError>
    where
        F: FnOnce(&Backend, &str) -> Result<Value, String> + Send + 'static,
    {
        let slot = self.backend.clone();
        let outcome = tokio::task::spawn_blocking(move || {
            let backend = slot
                .get_or_init(Backend::connect)
                .as_ref()
                .map_err(String::clone)?;
            let path = match repo {
                Some(path) => path,
                None => std::env::current_dir()
                    .map_err(|e| format!("cannot read the working directory: {e}"))?
                    .to_string_lossy()
                    .to_string(),
            };
            let id = backend.repo_id(&path)?;
            work(backend, &id)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("tool task failed: {e}"), None))?;
        outcome.map_err(|message| McpError::internal_error(message, None))
    }

    /// [`Bridge::call`] for a request that concerns no single repository.
    async fn call_global<F>(&self, work: F) -> Result<Value, McpError>
    where
        F: FnOnce(&Backend) -> Result<Value, String> + Send + 'static,
    {
        let slot = self.backend.clone();
        tokio::task::spawn_blocking(move || {
            work(
                slot.get_or_init(Backend::connect)
                    .as_ref()
                    .map_err(String::clone)?,
            )
        })
        .await
        .map_err(|e| McpError::internal_error(format!("tool task failed: {e}"), None))?
        .map_err(|message| McpError::internal_error(message, None))
    }
}

/// The string under `key`, or an empty string.
fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("")
}

fn rows<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value[key].as_array().into_iter().flatten().collect()
}

/// Text for the contracts of one repository.
fn repo_contracts_text(found: &Value) -> String {
    let provided = rows(found, "provided");
    let consumed = rows(found, "consumed");
    let mut body = format!("Provided endpoints ({}):\n", provided.len());
    for p in &provided {
        body.push_str(&format!(
            "  {:<7} {}  {}:{}\n",
            field(p, "method"),
            field(p, "path"),
            field(p, "file"),
            p["line"]
        ));
    }
    body.push_str(&format!("\nConsumed URL paths ({}):\n", consumed.len()));
    for c in &consumed {
        body.push_str(&format!(
            "  {}  {}:{}\n",
            field(c, "path"),
            field(c, "file"),
            c["line"]
        ));
    }
    body
}

/// Text for contracts matched across the workspace.
fn workspace_contracts_text(all: &Value) -> String {
    let mut body = String::new();
    for repo in rows(all, "repos") {
        body.push_str(&format!(
            "{}: {} provided, {} consumed\n",
            field(repo, "repo"),
            repo["provided"],
            repo["consumed"]
        ));
    }
    for failed in rows(all, "errors") {
        body.push_str(&format!(
            "{}: not read: {}\n",
            field(failed, "repo"),
            field(failed, "error")
        ));
    }
    let matched = rows(all, "matched");
    body.push_str(&format!("\nMatched calls ({}):\n", matched.len()));
    for link in matched.iter().take(80) {
        let providers = rows(link, "providers");
        let mut servers: Vec<&str> = providers.iter().map(|p| field(p, "repo")).collect();
        servers.dedup();
        body.push_str(&format!(
            "  {}  {}:{}:{}  <- {} ({} routes)\n",
            field(link, "path"),
            field(link, "repo"),
            field(link, "file"),
            link["line"],
            servers.join(", "),
            providers.len()
        ));
    }
    if matched.len() > 80 {
        body.push_str(&format!("  … and {} more\n", matched.len() - 80));
    }
    let unprovided = rows(all, "unprovided");
    body.push_str(&format!(
        "\nConsumed, provided by no registered repository ({}):\n",
        unprovided.len()
    ));
    for call in &unprovided {
        body.push_str(&format!(
            "  {}  {}:{}:{}\n",
            field(call, "path"),
            field(call, "repo"),
            field(call, "file"),
            call["line"]
        ));
    }
    let unconsumed = rows(all, "unconsumed");
    body.push_str(&format!(
        "\nProvided, consumed by nothing ({}):\n",
        unconsumed.len()
    ));
    for route in &unconsumed {
        body.push_str(&format!(
            "  {:<7} {}  {}:{}:{}\n",
            field(route, "method"),
            field(route, "path"),
            field(route, "repo"),
            field(route, "file"),
            route["line"]
        ));
    }
    body
}

#[tool_router]
impl Bridge {
    #[tool(
        description = "List the repositories the CodeWiki backend manages, with their ids and paths."
    )]
    async fn list_repos(&self) -> Result<CallToolResult, McpError> {
        pretty(&self.call_global(|b| b.get("/v1/repos")).await?)
    }

    #[tool(
        description = "Every repository the backend manages at a glance: files, symbols, notes, pending proposals, whether the wiki is stale, and the top hotspots of each, plus the external packages two or more repositories use. Use to orient across several repositories."
    )]
    async fn get_workspace(&self) -> Result<CallToolResult, McpError> {
        let view = self.call_global(|b| b.get("/v1/workspace")).await?;
        let repos = rows(&view, "repos");
        if repos.is_empty() {
            return text("No repository is registered with the backend.");
        }
        let mut body = String::new();
        for repo in repos {
            body.push_str(&format!(
                "{} ({})\n  {}\n",
                field(repo, "name"),
                field(repo, "id"),
                field(repo, "path")
            ));
            if let Some(error) = repo["error"].as_str() {
                body.push_str(&format!("  could not be read: {error}\n"));
                continue;
            }
            body.push_str(&format!(
                "  {} files, {} symbols, {} notes, {} pending proposals, stale: {}\n",
                repo["files"],
                repo["symbols"],
                repo["notes"],
                repo["pending_proposals"],
                repo["stale"]
            ));
            for hotspot in rows(repo, "hotspots") {
                body.push_str(&format!(
                    "  hotspot {} ({} commits, {} lines)\n",
                    field(hotspot, "path"),
                    hotspot["commits"],
                    hotspot["lines"]
                ));
            }
        }
        let shared = rows(&view, "shared_dependencies");
        if !shared.is_empty() {
            body.push_str("\nExternal packages used by two or more repositories:\n");
        }
        for dep in shared {
            let users: Vec<&str> = rows(dep, "repos")
                .into_iter()
                .filter_map(Value::as_str)
                .collect();
            body.push_str(&format!(
                "  {} — {}\n",
                field(dep, "name"),
                users.join(", ")
            ));
        }
        text(body)
    }

    #[tool(
        description = "HTTP contracts found in Rust source: the endpoints a repository provides (axum `.route` calls with nesting prefixes, `#[get(\"/path\")]`-style attributes) and the URL paths its code calls, each with file and line. With `workspace: true`, matches every registered repository's calls against every repository's routes and lists calls no repository answers and routes nothing calls. Lexical: a path built at run time is not seen. Call before changing or removing an endpoint."
    )]
    async fn get_contracts(
        &self,
        Parameters(p): Parameters<ContractsParams>,
    ) -> Result<CallToolResult, McpError> {
        if p.workspace.unwrap_or(false) {
            let all = self
                .call_global(|b| b.get("/v1/workspace/contracts"))
                .await?;
            return text(workspace_contracts_text(&all));
        }
        let found = self
            .call(p.repo, |b, id| b.get(&format!("/v1/repos/{id}/contracts")))
            .await?;
        text(repo_contracts_text(&found))
    }

    #[tool(
        description = "Start here. Returns the wiki home note for the repository (architecture flows, crates, reading paths) plus index status. Use before exploring an unfamiliar codebase."
    )]
    async fn get_overview(
        &self,
        Parameters(p): Parameters<RepoParam>,
    ) -> Result<CallToolResult, McpError> {
        let overview = self
            .call(p.repo, |b, id| b.get(&format!("/v1/repos/{id}/overview")))
            .await?;
        let status = &overview["status"];
        let home = overview["home"].as_str().unwrap_or("");
        let body = if home.is_empty() {
            "The wiki has not been generated yet. Call `sync_wiki` first.".to_string()
        } else {
            home.to_string()
        };
        text(format!(
            "{body}\n\n---\nIndexed: {} files, {} symbols, {} notes. Pending proposals: {}. Stale: {}.",
            status["files"],
            status["symbols"],
            status["notes"],
            status["pending_proposals"],
            status["stale"]
        ))
    }

    #[tool(
        description = "What to know before touching a file or symbol: its symbols or members, what it depends on, what uses it, and which wiki notes describe it. `target` is a repo-relative file path or a symbol name."
    )]
    async fn get_context(
        &self,
        Parameters(p): Parameters<ContextParams>,
    ) -> Result<CallToolResult, McpError> {
        let target = p.target;
        pretty(
            &self
                .call(p.repo, move |b, id| {
                    b.get(&format!(
                        "/v1/repos/{id}/context?target={}",
                        encode(&target)
                    ))
                })
                .await?,
        )
    }

    #[tool(
        description = "Definition, signature, documentation, members, dependencies and users of one symbol, by bare or qualified name."
    )]
    async fn get_symbol(
        &self,
        Parameters(p): Parameters<SymbolParams>,
    ) -> Result<CallToolResult, McpError> {
        let name = p.name;
        pretty(
            &self
                .call(p.repo, move |b, id| {
                    b.get(&format!("/v1/repos/{id}/symbol?name={}", encode(&name)))
                })
                .await?,
        )
    }

    #[tool(
        description = "Full-text search across wiki notes, symbols and file paths. Returns ranked hits with snippets. Use `kind` to restrict to `note`, `symbol` or `file`."
    )]
    async fn search_codebase(
        &self,
        Parameters(p): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = p.limit.unwrap_or(10).clamp(1, 50);
        let mut uri = format!("search?q={}&limit={limit}", encode(&p.query));
        if let Some(kind) = p.kind.filter(|k| !k.is_empty()) {
            uri.push_str(&format!("&kind={}", encode(&kind)));
        }
        pretty(
            &self
                .call(p.repo, move |b, id| b.get(&format!("/v1/repos/{id}/{uri}")))
                .await?,
        )
    }

    #[tool(
        description = "Answer a question about the codebase from the wiki: returns the best-matching notes' summaries with their sources. Follow up with `read_note` or `get_context` for detail."
    )]
    async fn get_answer(
        &self,
        Parameters(p): Parameters<AnswerParams>,
    ) -> Result<CallToolResult, McpError> {
        let question = p.question;
        let answer = self
            .call(p.repo, move |b, id| {
                b.post(
                    &format!("/v1/repos/{id}/ask"),
                    &json!({ "question": question }),
                )
            })
            .await?;
        let mut body = answer["answer"].as_str().unwrap_or("").to_string();
        body.push_str("\n\nSources:\n");
        for hit in answer["sources"].as_array().into_iter().flatten() {
            body.push_str(&format!(
                "- [{}] {}\n",
                hit["kind"].as_str().unwrap_or(""),
                hit["key"].as_str().unwrap_or("")
            ));
        }
        text(body)
    }

    #[tool(
        description = "Shortest dependency chain from one symbol to another (calls, type references, trait implementations). Empty when `from` does not depend on `to`."
    )]
    async fn get_dependency_path(
        &self,
        Parameters(p): Parameters<PathParams>,
    ) -> Result<CallToolResult, McpError> {
        let (from, to) = (p.from, p.to);
        let chain = self
            .call(p.repo, move |b, id| {
                b.get(&format!(
                    "/v1/repos/{id}/path?from={}&to={}",
                    encode(&from),
                    encode(&to)
                ))
            })
            .await?;
        let steps: Vec<&str> = chain
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if steps.is_empty() {
            text("No dependency path found.")
        } else {
            text(steps.join("\n  -> "))
        }
    }

    #[tool(
        description = "The repository's architecture notes in full: crate dependency flow and startup call sequences, with Mermaid diagrams."
    )]
    async fn get_architecture(
        &self,
        Parameters(p): Parameters<RepoParam>,
    ) -> Result<CallToolResult, McpError> {
        let notes = self
            .call(p.repo, |b, id| {
                let list = b.get(&format!("/v1/repos/{id}/notes"))?;
                let mut out = Vec::new();
                for note in list
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|n| n["kind"] == "flow")
                    .take(6)
                {
                    let path = note["path"].as_str().unwrap_or("");
                    let content = b.get(&format!("/v1/repos/{id}/note?path={}", encode(path)))?;
                    out.push(content["text"].clone());
                }
                Ok(Value::Array(out))
            })
            .await?;
        let texts: Vec<&str> = notes
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if texts.is_empty() {
            text("No architecture notes yet. Call `sync_wiki` to generate the wiki.")
        } else {
            text(texts.join("\n\n---\n\n"))
        }
    }

    #[tool(
        description = "Risk of changing one file, 0 to 100, with the reasons: how many files depend on it, how often it changes, whether one author holds the knowledge, and which files usually change with it. Call before editing a file you do not know."
    )]
    async fn get_risk(
        &self,
        Parameters(p): Parameters<FileParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = p.path;
        let risk = self
            .call(p.repo, move |b, id| {
                b.get(&format!("/v1/repos/{id}/risk?path={}", encode(&path)))
            })
            .await?;
        let mut body = format!(
            "{} — risk {} ({}/100)\n",
            risk["path"].as_str().unwrap_or(""),
            risk["level"].as_str().unwrap_or(""),
            risk["score"]
        );
        for reason in risk["reasons"].as_array().into_iter().flatten() {
            body.push_str(&format!("- {}\n", reason.as_str().unwrap_or("")));
        }
        text(body)
    }

    #[tool(
        description = "Risk of a set of changed files taken together. With no `files`, uses the uncommitted changes in the working tree. Reports the riskiest files, files that usually change with them but are missing from the change, and files outside the change that depend on it. Call before finishing a change."
    )]
    async fn get_change_risk(
        &self,
        Parameters(p): Parameters<ChangeParams>,
    ) -> Result<CallToolResult, McpError> {
        let files = p.files.unwrap_or_default();
        let risk = self
            .call(p.repo, move |b, id| {
                b.post(
                    &format!("/v1/repos/{id}/change-risk"),
                    &json!({ "files": files }),
                )
            })
            .await?;
        let mut body = format!(
            "Change risk {} ({}/100)\n",
            risk["level"].as_str().unwrap_or(""),
            risk["score"]
        );
        for file in risk["files"].as_array().into_iter().flatten().take(10) {
            body.push_str(&format!(
                "\n{} — {}/100\n",
                file["path"].as_str().unwrap_or(""),
                file["score"]
            ));
            for reason in file["reasons"].as_array().into_iter().flatten() {
                body.push_str(&format!("  - {}\n", reason.as_str().unwrap_or("")));
            }
        }
        let missing: Vec<String> = risk["missing_partners"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|p| {
                format!(
                    "- {} (changed together {} times)",
                    p["path"].as_str().unwrap_or(""),
                    p["together"]
                )
            })
            .collect();
        if !missing.is_empty() {
            body.push_str(&format!(
                "\nUsually changed together but not in this change:\n{}\n",
                missing.join("\n")
            ));
        }
        let affected: Vec<&str> = risk["affected"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if !affected.is_empty() {
            body.push_str(&format!(
                "\n{} files outside the change depend on it:\n",
                affected.len()
            ));
            for path in affected.iter().take(25) {
                body.push_str(&format!("- {path}\n"));
            }
        }
        text(body)
    }

    #[tool(
        description = "Files affected if a file changes its behavior: direct dependents, then those reached through further hops."
    )]
    async fn get_blast_radius(
        &self,
        Parameters(p): Parameters<FileParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = p.path;
        pretty(
            &self
                .call(p.repo, move |b, id| {
                    b.get(&format!(
                        "/v1/repos/{id}/blast-radius?path={}",
                        encode(&path)
                    ))
                })
                .await?,
        )
    }

    #[tool(
        description = "Hotspots: files that are both large and frequently changed, highest first, with main author and bus factor. Defects concentrate here."
    )]
    async fn get_hotspots(
        &self,
        Parameters(p): Parameters<RepoParam>,
    ) -> Result<CallToolResult, McpError> {
        let rows = self
            .call(p.repo, |b, id| {
                b.get(&format!("/v1/repos/{id}/hotspots?limit=20"))
            })
            .await?;
        let mut body = String::from("score  commits  lines  bus  owner  path\n");
        for row in rows.as_array().into_iter().flatten() {
            body.push_str(&format!(
                "{:>5}  {:>7}  {:>5}  {:>3}  {}  {}\n",
                row["score"],
                row["commits"],
                row["lines"],
                row["bus_factor"],
                row["top_owner"].as_str().unwrap_or("-"),
                row["path"].as_str().unwrap_or("")
            ));
        }
        text(body)
    }

    #[tool(
        description = "Symbols nothing refers to. `likely` means private, unreferenced and carrying no attribute that could register it; `possible` means it could be reached in ways the index cannot see (attribute macros, trait dispatch, external callers). Verify before deleting."
    )]
    async fn get_dead_code(
        &self,
        Parameters(p): Parameters<RepoParam>,
    ) -> Result<CallToolResult, McpError> {
        let rows = self
            .call(p.repo, |b, id| b.get(&format!("/v1/repos/{id}/dead-code")))
            .await?;
        let rows: Vec<&Value> = rows.as_array().into_iter().flatten().collect();
        if rows.is_empty() {
            return text("No unreferenced symbols found.");
        }
        let mut body = String::new();
        for row in rows.iter().take(60) {
            body.push_str(&format!(
                "{}  {}:{}  {} ({}, {} lines) — {}\n",
                row["certainty"].as_str().unwrap_or(""),
                row["file"].as_str().unwrap_or(""),
                row["line"],
                row["qualified"].as_str().unwrap_or(""),
                row["kind"].as_str().unwrap_or(""),
                row["lines"],
                row["reason"].as_str().unwrap_or("")
            ));
        }
        if rows.len() > 60 {
            body.push_str(&format!("… and {} more\n", rows.len() - 60));
        }
        text(body)
    }

    #[tool(
        description = "Code health: files scoring below 10 with the findings that lowered the score (long files and functions, large types, high fan-out, undocumented public API, dead code), worst first, plus documentation coverage and drift (notes citing missing or changed sources, broken links)."
    )]
    async fn get_health(
        &self,
        Parameters(p): Parameters<RepoParam>,
    ) -> Result<CallToolResult, McpError> {
        let report = self
            .call(p.repo, |b, id| {
                Ok(json!({
                    "files": b.get(&format!("/v1/repos/{id}/health"))?,
                    "docs": b.get(&format!("/v1/repos/{id}/doc-drift"))?,
                }))
            })
            .await?;
        let mut body = String::from("File health (10 is best):\n");
        let mut listed = 0;
        for file in report["files"].as_array().into_iter().flatten() {
            let findings: Vec<&Value> = file["findings"].as_array().into_iter().flatten().collect();
            if findings.is_empty() || listed >= 25 {
                continue;
            }
            listed += 1;
            body.push_str(&format!(
                "{}/10  {}\n",
                file["score"],
                file["path"].as_str().unwrap_or("")
            ));
            for finding in findings {
                body.push_str(&format!(
                    "    [{}] {}\n",
                    finding["detector"].as_str().unwrap_or(""),
                    finding["message"].as_str().unwrap_or("")
                ));
            }
        }
        if listed == 0 {
            body.push_str("  every file scores 10\n");
        }
        let docs = &report["docs"];
        body.push_str(&format!(
            "\nDocumentation: {:.0}% of source files are cited by a note ({} of {}).\n",
            docs["coverage"].as_f64().unwrap_or(0.0) * 100.0,
            docs["documented_files"],
            docs["source_files"]
        ));
        for drift in docs["drift"].as_array().into_iter().flatten().take(30) {
            body.push_str(&format!(
                "  {}: {} — {}\n",
                drift["kind"].as_str().unwrap_or(""),
                drift["note"].as_str().unwrap_or(""),
                drift["detail"].as_str().unwrap_or("")
            ));
        }
        text(body)
    }

    #[tool(
        description = "Why a file or symbol is the way it is: decision records that cite it (with status and whether they are stale), rationale recorded in wiki notes (Why, Gotchas, Don't), and the most recent commits touching it. Call before changing behavior that looks deliberate."
    )]
    async fn get_why(
        &self,
        Parameters(p): Parameters<ContextParams>,
    ) -> Result<CallToolResult, McpError> {
        let target = p.target;
        let why = self
            .call(p.repo, move |b, id| {
                b.get(&format!("/v1/repos/{id}/why?target={}", encode(&target)))
            })
            .await?;
        let mut body = format!("{}\n", why["file"].as_str().unwrap_or(""));
        let decisions: Vec<&Value> = why["decisions"].as_array().into_iter().flatten().collect();
        if !decisions.is_empty() {
            body.push_str("\nDecisions:\n");
            for d in decisions {
                body.push_str(&format!(
                    "- [{}{}] {} — {} ({})\n",
                    d["status"].as_str().unwrap_or(""),
                    if d["stale"] == true { ", stale" } else { "" },
                    d["title"].as_str().unwrap_or(""),
                    d["summary"].as_str().unwrap_or(""),
                    d["path"].as_str().unwrap_or("")
                ));
            }
        }
        for r in why["rationale"].as_array().into_iter().flatten() {
            body.push_str(&format!(
                "\n{} (from {}):\n{}\n",
                r["section"].as_str().unwrap_or(""),
                r["note"].as_str().unwrap_or(""),
                r["text"].as_str().unwrap_or("")
            ));
        }
        let commits: Vec<&Value> = why["commits"].as_array().into_iter().flatten().collect();
        if !commits.is_empty() {
            body.push_str("\nRecent commits:\n");
            for c in commits {
                body.push_str(&format!(
                    "- {} {} {} — {}\n",
                    c["hash"].as_str().unwrap_or(""),
                    c["date"].as_str().unwrap_or(""),
                    c["author"].as_str().unwrap_or(""),
                    c["subject"].as_str().unwrap_or("")
                ));
            }
        }
        if body.lines().count() == 1 {
            body.push_str(
                "\nNo decision, recorded rationale or commit history found for this file.\n",
            );
        }
        text(body)
    }

    #[tool(
        description = "List decision records (ADR files and decision notes) with status, and which are stale because the code they cite changed afterwards."
    )]
    async fn list_decisions(
        &self,
        Parameters(p): Parameters<RepoParam>,
    ) -> Result<CallToolResult, McpError> {
        let rows = self
            .call(p.repo, |b, id| b.get(&format!("/v1/repos/{id}/decisions")))
            .await?;
        let rows: Vec<&Value> = rows.as_array().into_iter().flatten().collect();
        if rows.is_empty() {
            return text("No decision records found.");
        }
        let mut body = String::new();
        for d in rows {
            body.push_str(&format!(
                "[{}{}] {} — {}\n",
                d["status"].as_str().unwrap_or(""),
                if d["stale"] == true { ", stale" } else { "" },
                d["path"].as_str().unwrap_or(""),
                d["title"].as_str().unwrap_or("")
            ));
            for reason in d["stale_reasons"].as_array().into_iter().flatten() {
                body.push_str(&format!("    {}\n", reason.as_str().unwrap_or("")));
            }
        }
        text(body)
    }

    #[tool(
        description = "Tests to run after a change. With no `files`, uses the uncommitted changes in the working tree. Lists the test files that can reach the changed files through the dependency graph, nearest first, with the test names and the `cargo test` commands that run them, plus changed files no test reaches. Call after editing and before reporting work as done."
    )]
    async fn get_impacted_tests(
        &self,
        Parameters(p): Parameters<ChangeParams>,
    ) -> Result<CallToolResult, McpError> {
        let files = p.files.unwrap_or_default();
        let impact = self
            .call(p.repo, move |b, id| {
                b.post(
                    &format!("/v1/repos/{id}/impacted-tests"),
                    &json!({ "files": files }),
                )
            })
            .await?;
        let files: Vec<&Value> = impact["test_files"]
            .as_array()
            .into_iter()
            .flatten()
            .collect();
        let mut body = format!(
            "{} changed files reach {} tests in {} files.\n",
            impact["changed"].as_array().map(Vec::len).unwrap_or(0),
            impact["tests_total"],
            files.len()
        );
        let commands: Vec<&str> = impact["commands"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if !commands.is_empty() {
            body.push_str(&format!("\nRun:\n{}\n", commands.join("\n")));
        }
        for file in files.iter().take(40) {
            let tests: Vec<&str> = file["tests"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            let shown: Vec<&str> = tests.iter().take(12).copied().collect();
            body.push_str(&format!(
                "\n{} ({}, {} hops): {}{}\n",
                file["path"].as_str().unwrap_or(""),
                file["kind"].as_str().unwrap_or(""),
                file["distance"],
                shown.join(", "),
                if tests.len() > shown.len() {
                    format!(" and {} more", tests.len() - shown.len())
                } else {
                    String::new()
                }
            ));
        }
        if files.len() > 40 {
            body.push_str(&format!("\n… and {} more test files\n", files.len() - 40));
        }
        let untested: Vec<&str> = impact["untested_changes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if !untested.is_empty() {
            body.push_str(&format!(
                "\nNo test reaches:\n- {}\n",
                untested.join("\n- ")
            ));
        }
        text(body)
    }

    #[tool(
        description = "Line coverage from an LCOV (`lcov.info`) or Cobertura XML report: repository total, and the least covered files ranked by how often they change (hotspot) and how much depends on them (rank). Pass `report` (a path inside the repository) to import a new report; without it the last imported report is used, or one found at a conventional path such as `lcov.info` or `coverage.xml`."
    )]
    async fn get_coverage(
        &self,
        Parameters(p): Parameters<CoverageParams>,
    ) -> Result<CallToolResult, McpError> {
        let report = p.report.filter(|r| !r.is_empty());
        let coverage = self
            .call(p.repo, move |b, id| match report {
                Some(report) => b.post(
                    &format!("/v1/repos/{id}/coverage"),
                    &json!({ "report": report, "limit": 20 }),
                ),
                None => b.get(&format!("/v1/repos/{id}/coverage?limit=20")),
            })
            .await?;
        let mut body = format!(
            "Line coverage {}% ({} of {} lines, {} files, {} report {}).\n",
            coverage["percent"],
            coverage["lines_hit"],
            coverage["lines_found"],
            coverage["files"].as_array().map(Vec::len).unwrap_or(0),
            coverage["format"].as_str().unwrap_or(""),
            coverage["source"].as_str().unwrap_or("")
        );
        let gaps: Vec<&Value> = coverage["least_covered"]
            .as_array()
            .into_iter()
            .flatten()
            .collect();
        if !gaps.is_empty() {
            body.push_str(
                "\nGaps worth closing first:\npriority  covered  uncovered lines  hotspot  path\n",
            );
        }
        for gap in gaps {
            body.push_str(&format!(
                "{:>8}  {:>6}%  {:>15}  {:>7}  {}\n",
                gap["score"],
                gap["percent"],
                gap["uncovered_lines"],
                gap["hotspot"],
                gap["path"].as_str().unwrap_or("")
            ));
        }
        let absent: Vec<&str> = coverage["not_in_report"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if !absent.is_empty() {
            body.push_str(&format!(
                "\nSource files the report does not mention:\n- {}\n",
                absent.join("\n- ")
            ));
        }
        text(body)
    }

    #[tool(
        description = "Security pattern findings in Rust source, test code excluded: hard-coded secrets, shell commands and SQL built with `format!`, disabled TLS verification, `std::env::set_var`, world-writable file modes, plain-HTTP URLs to remote hosts, and `unsafe` blocks. Each finding has a rule id, level, file and line. `level` sets the lowest level returned (`note`, `warning` or `error`)."
    )]
    async fn get_security_findings(
        &self,
        Parameters(p): Parameters<SecurityParams>,
    ) -> Result<CallToolResult, McpError> {
        let level = p.level.filter(|l| !l.is_empty());
        let rows = self
            .call(p.repo, move |b, id| match level {
                Some(level) => b.get(&format!("/v1/repos/{id}/security?level={}", encode(&level))),
                None => b.get(&format!("/v1/repos/{id}/security")),
            })
            .await?;
        let rows: Vec<&Value> = rows.as_array().into_iter().flatten().collect();
        if rows.is_empty() {
            return text("No security findings.");
        }
        let mut body = String::new();
        for row in rows.iter().take(80) {
            body.push_str(&format!(
                "{}: {}:{}: [{}] {}\n",
                row["level"].as_str().unwrap_or(""),
                row["file"].as_str().unwrap_or(""),
                row["line"],
                row["rule"].as_str().unwrap_or(""),
                row["message"].as_str().unwrap_or("")
            ));
        }
        if rows.len() > 80 {
            body.push_str(&format!("… and {} more\n", rows.len() - 80));
        }
        text(body)
    }

    #[tool(
        description = "Refactoring suggestions derived from the index, highest priority first: split a long or far-reaching file, extract a type from one with many methods, break a dependency loop between files, move a function to the file it works with. Each has the evidence behind it and ordered steps. Deterministic; no model is involved."
    )]
    async fn get_refactoring_plan(
        &self,
        Parameters(p): Parameters<PlanParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = p.limit.unwrap_or(10).clamp(1, 50);
        let plan = self
            .call(p.repo, move |b, id| {
                b.get(&format!("/v1/repos/{id}/refactor?limit={limit}"))
            })
            .await?;
        let plan: Vec<&Value> = plan.as_array().into_iter().flatten().collect();
        if plan.is_empty() {
            return text("Nothing to suggest.");
        }
        let mut body = String::new();
        for (number, suggestion) in plan.iter().enumerate() {
            body.push_str(&format!(
                "{}. {} (priority {})\n",
                number + 1,
                suggestion["title"].as_str().unwrap_or(""),
                suggestion["priority"]
            ));
            for line in suggestion["evidence"].as_array().into_iter().flatten() {
                body.push_str(&format!("   - {}\n", line.as_str().unwrap_or("")));
            }
            for (step, line) in suggestion["steps"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
            {
                body.push_str(&format!(
                    "   {}. {}\n",
                    step + 1,
                    line.as_str().unwrap_or("")
                ));
            }
            body.push('\n');
        }
        text(body)
    }

    #[tool(description = "Read one wiki note in full, by note name or vault path.")]
    async fn read_note(
        &self,
        Parameters(p): Parameters<NoteParams>,
    ) -> Result<CallToolResult, McpError> {
        let note = p.note;
        let content = self
            .call(p.repo, move |b, id| {
                b.get(&format!("/v1/repos/{id}/note?path={}", encode(&note)))
            })
            .await?;
        text(content["text"].as_str().unwrap_or("").to_string())
    }

    #[tool(
        description = "Re-index the repository and bring the wiki up to date with the working tree. Notes a human edited are never overwritten; changes to them are queued as proposals for review in the CodeWiki app."
    )]
    async fn sync_wiki(
        &self,
        Parameters(p): Parameters<RepoParam>,
    ) -> Result<CallToolResult, McpError> {
        let report = self
            .call(p.repo, |b, id| {
                b.post(&format!("/v1/repos/{id}/update"), &json!({}))
            })
            .await?;
        let count = |key: &str| report[key].as_array().map(Vec::len).unwrap_or(0);
        text(format!(
            "Wiki updated: {} notes written, {} unchanged, {} proposals queued for human review, {} removed, {} adopted. Total notes: {}.",
            count("written"),
            report["kept"],
            count("proposed"),
            count("removed"),
            count("adopted"),
            report["notes_total"]
        ))
    }
}

#[tool_handler(
    name = "codewiki",
    instructions = "CodeWiki serves a maintained wiki and a structural index of the repository. Call get_overview first in an unfamiliar codebase. Before editing a file or symbol, call get_context on it. Use search_codebase to locate things and read_note for full notes. After editing, call get_impacted_tests to learn which tests to run. Call sync_wiki after substantial code changes."
)]
impl ServerHandler for Bridge {}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let service = match Bridge::default().serve(stdio()).await {
        Ok(service) => service,
        Err(e) => {
            eprintln!("codewiki-mcp: failed to start: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if let Err(e) = service.waiting().await {
        eprintln!("codewiki-mcp: stopped with an error: {e}");
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_the_contracts_of_one_repository() {
        let body = repo_contracts_text(&json!({
            "provided": [
                { "method": "GET", "path": "/v1/health", "file": "src/api.rs", "line": 7 }
            ],
            "consumed": [
                { "path": "/v1/repos/{id}/notes", "file": "src/main.rs", "line": 12 }
            ],
        }));
        assert!(body.starts_with("Provided endpoints (1):\n"), "{body}");
        assert!(
            body.contains("  GET     /v1/health  src/api.rs:7\n"),
            "{body}"
        );
        assert!(body.contains("Consumed URL paths (1):\n"), "{body}");
        assert!(
            body.contains("  /v1/repos/{id}/notes  src/main.rs:12\n"),
            "{body}"
        );
    }

    #[test]
    fn formats_workspace_contracts() {
        let body = workspace_contracts_text(&json!({
            "repos": [
                { "repo": "server", "provided": 2, "consumed": 0 },
                { "repo": "client", "provided": 0, "consumed": 2 }
            ],
            "errors": [{ "repo": "gone", "error": "not found: folder 'x'" }],
            "matched": [{
                "repo": "client", "path": "/v1/items", "file": "src/main.rs", "line": 3,
                "providers": [
                    { "repo": "server", "method": "GET", "path": "/v1/items" },
                    { "repo": "server", "method": "POST", "path": "/v1/items" }
                ]
            }],
            "unprovided": [
                { "repo": "client", "path": "/v1/missing", "file": "src/main.rs", "line": 4 }
            ],
            "unconsumed": [],
        }));
        assert!(body.contains("server: 2 provided, 0 consumed\n"), "{body}");
        assert!(body.contains("gone: not read: not found: folder 'x'\n"));
        assert!(
            body.contains("  /v1/items  client:src/main.rs:3  <- server (2 routes)\n"),
            "{body}"
        );
        assert!(body.contains("provided by no registered repository (1):\n"));
        assert!(body.contains("  /v1/missing  client:src/main.rs:4\n"));
        assert!(
            body.ends_with("Provided, consumed by nothing (0):\n"),
            "{body}"
        );
    }
}
