//! MCP stdio bridge between coding agents and the CodeWiki backend.
//!
//! Standard output carries the MCP protocol, so nothing else may be printed
//! there; diagnostics go to standard error.

mod guard;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use cw_client::{Backend, encode};
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock};
use rmcp::service::RequestContext;
use rmcp::transport::stdio;
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler, ServiceExt, schemars, tool, tool_handler,
    tool_router,
};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Clone, Default)]
struct Bridge {
    /// Connected lazily so listing tools never starts a backend.
    backend: Arc<Mutex<Option<Backend>>>,
}

/// The backend in `slot`, connecting on first use. Only a success is kept:
/// after a failed start the next tool call tries again, where a kept error
/// disabled the bridge until the agent restarted it. A kept backend that no
/// longer answers is dropped and a new one found: the app stops the backend
/// when it installs an update, and the bridge outlives that. The lock is held
/// while connecting, so two calls at once never start two backends.
fn connected(slot: &Mutex<Option<Backend>>) -> Result<Backend, String> {
    let mut slot = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(backend) = slot.as_ref() {
        if backend.answers() {
            return Ok(backend.clone());
        }
        log::info!(
            "the backend at {} stopped answering; connecting again",
            backend.url()
        );
        *slot = None;
    }
    // Named, so that the backend can tell the app when an agent last called.
    let backend = Backend::connect()?.with_client(cw_client::MCP_CLIENT);
    *slot = Some(backend.clone());
    Ok(backend)
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
    /// Also return a `How it works` that is only the rule-made walk-through
    /// of which function calls which. Left out by default: the code shows it.
    #[serde(default)]
    full: Option<bool>,
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

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct WaitingParams {
    /// Only notes in this folder of the wiki, for example `01-Core/cw-parse`.
    #[serde(default)]
    folder: Option<String>,
    /// Only notes about one of these repo-relative files, for example the files you just changed.
    #[serde(default)]
    files: Option<Vec<String>>,
    /// How many notes to list, 1 to 20. Defaults to 5.
    #[serde(default)]
    limit: Option<u32>,
    /// Repository root path. Defaults to the directory the agent runs in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct BriefParams {
    /// Note name (`Scan Module`) or vault path (`01-Core/cw-parse/Scan Module.md`).
    note: String,
    /// Repository root path. Defaults to the directory the agent runs in.
    #[serde(default)]
    repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct WrittenSection {
    /// One of the headings the brief lists under "Your sections", exactly as written there.
    heading: String,
    /// The section's text in Markdown, without its heading. Prose, lists and `###` headings; no `#` or `##` heading.
    text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct WrittenPicture {
    /// Heading of the section the picture stands in. One of your sections that the note has.
    section: String,
    /// The picture as a short program: `root = Picture([...])` with a `Flow` of `Step`s and `Callout`s. The brief lists the components and the limits.
    source: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SubmitParams {
    /// Note name or vault path, as in the brief.
    note: String,
    /// The `base` value of the brief you wrote from.
    base: String,
    /// The sections you wrote. A section you leave out stays as it is.
    sections: Vec<WrittenSection>,
    /// At most one picture for a section, and only where a picture explains more than the text. A section sent without a picture loses the one it had.
    #[serde(default)]
    pictures: Option<Vec<WrittenPicture>>,
    /// Repo-relative files you read beside the note's sources. A later change to one of them marks the explanation as behind.
    #[serde(default)]
    read: Option<Vec<String>>,
    /// Your name as an agent, for example `claude-code`. Shown with the note.
    #[serde(default)]
    model: Option<String>,
    /// Repository root path. Defaults to the directory the agent runs in.
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
            let backend = connected(&slot)?;
            let cwd = std::env::current_dir()
                .map_err(|e| format!("cannot read the working directory: {e}"))?;
            // The backend gets what `screen` read: a registered repository's
            // id, or an absolute path against this session's folder, never a
            // word or relative path it would read against its own folder.
            let arg = repo.map(|r| guard::repo_arg(&r, &cwd));
            let listing = match &arg {
                Some(guard::RepoArg::IdOrName(_)) => backend.get("/v1/repos").ok(),
                _ => None,
            };
            let target = match &arg {
                Some(arg) => guard::target(arg, listing.as_ref(), &cwd),
                None => guard::Target::Path(cwd),
            };
            let id = match target {
                guard::Target::Id(id) => id,
                guard::Target::Path(path) => backend.repo_id(&path.to_string_lossy())?,
            };
            work(&backend, &id)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("tool task failed: {e}"), None))?;
        outcome.map_err(failed)
    }

    /// [`Bridge::call`] for a request that concerns no single repository.
    async fn call_global<F>(&self, work: F) -> Result<Value, McpError>
    where
        F: FnOnce(&Backend) -> Result<Value, String> + Send + 'static,
    {
        let slot = self.backend.clone();
        tokio::task::spawn_blocking(move || work(&connected(&slot)?))
            .await
            .map_err(|e| McpError::internal_error(format!("tool task failed: {e}"), None))?
            .map_err(failed)
    }
}

impl Bridge {
    /// The refusal text when this call may not go ahead.
    ///
    /// The list is read on every call. The session folders and a path `repo`
    /// argument are checked without a backend; only an id or name `repo`
    /// argument connects, for one `GET /v1/repos`, and when that fails the
    /// session folders alone decide.
    fn screen(&self, arguments: Option<&serde_json::Map<String, Value>>) -> Option<String> {
        let data = cw_core::paths::data_dir();
        let list = cw_core::agents_off::load(&data);
        if matches!(&list, Ok(entries) if entries.is_empty()) {
            return None;
        }
        let Ok(cwd) = std::env::current_dir() else {
            return Some("CodeWiki cannot read the working directory, so it cannot tell whether this project is kept from coding agents. CodeWiki's tools are off for now.".to_string());
        };
        let project = std::env::var_os("CLAUDE_PROJECT_DIR").filter(|d| !d.is_empty());
        let folders = guard::session_folders(&cwd, project.as_deref().map(Path::new));
        if let guard::Verdict::Refused(text) =
            guard::verdict(&list, &cw_core::agents_off::file(&data), &folders)
        {
            return Some(text);
        }
        let entries = list.ok()?;
        let value = arguments?.get("repo")?.as_str().filter(|v| !v.is_empty())?;
        let arg = guard::repo_arg(value, &cwd);
        // As a folder, without a backend: a path as written, a word under the
        // current folder (what it means when no repository answers to it).
        let direct = guard::direct_path(&arg, &cwd);
        if let Some(entry) = cw_core::agents_off::covering(&entries, &direct) {
            return Some(guard::refusal(&entry));
        }
        // The backend falls back to a repository's name when it cannot add a
        // path, so a missing path is looked up by its last component too.
        let name = guard::lookup_name(&arg, |path| path.is_dir())?;
        let listing = connected(&self.backend)
            .and_then(|backend| backend.get("/v1/repos"))
            .ok()?;
        let path = guard::resolve_id(&listing, &name)?;
        cw_core::agents_off::covering(&entries, &path).map(|entry| guard::refusal(&entry))
    }
}

/// The folders on the list; empty when it cannot be read, which `screen`
/// already refused.
fn listed_folders() -> Vec<PathBuf> {
    cw_core::agents_off::load(&cw_core::paths::data_dir()).unwrap_or_default()
}

/// The `/v1/repos` listing without the repositories on the list, and their
/// names.
fn visible_repos(backend: &Backend) -> Result<(Value, HashSet<String>), String> {
    let mut listing = backend.get("/v1/repos")?;
    let hidden = guard::hide_repos(&mut listing, &listed_folders());
    Ok((listing, hidden))
}

/// Fetch `uri` and drop the repositories on the list from it with `hide`.
/// With an empty list the answer is returned as it is.
fn get_visible(
    backend: &Backend,
    uri: &str,
    hide: fn(&mut Value, &HashSet<String>),
) -> Result<Value, String> {
    let mut view = backend.get(uri)?;
    if !listed_folders().is_empty() {
        hide(&mut view, &visible_repos(backend)?.1);
    }
    Ok(view)
}

/// A tool call that failed: logged, and returned to the agent as an error.
fn failed(message: String) -> McpError {
    log::warn!("tool call failed: {message}");
    McpError::internal_error(message, None)
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
        pretty(
            &self
                .call_global(|b| visible_repos(b).map(|(listing, _)| listing))
                .await?,
        )
    }

    #[tool(
        description = "Every repository the backend manages at a glance: files, symbols, notes, pending proposals, whether the wiki is stale, and the top hotspots of each, plus the external packages two or more repositories use. Use to orient across several repositories."
    )]
    async fn get_workspace(&self) -> Result<CallToolResult, McpError> {
        let view = self
            .call_global(|b| get_visible(b, "/v1/workspace", guard::hide_in_workspace))
            .await?;
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
                .call_global(|b| {
                    get_visible(b, "/v1/workspace/contracts", guard::hide_in_contracts)
                })
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
        description = "Security pattern findings in Rust source, test code excluded: hard-coded secrets, shell commands and SQL built with `format!`, disabled TLS verification, `std::env::set_var`, world-writable file modes, plain-HTTP URLs to remote hosts, and `unsafe` blocks with no `// SAFETY:` comment. Each finding has a rule id, level, file and line. `level` sets the lowest level returned (`note`, `warning` or `error`)."
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

    #[tool(
        description = "Read one wiki note, by note name or vault path: what the thing is for, where its source is, what each of its items does, its gotchas and what it depends on. While `How it works` is only the walk-through of calls that CodeWiki makes by rule, it is left out, since the code shows it; pass `full: true` to get it too. Once an agent explained that section, it is always included."
    )]
    async fn read_note(
        &self,
        Parameters(p): Parameters<NoteParams>,
    ) -> Result<CallToolResult, McpError> {
        let note = p.note;
        // The backend cuts the note: one rule for every client that asks.
        let view = if p.full.unwrap_or(false) {
            ""
        } else {
            "&view=agent"
        };
        let content = self
            .call(p.repo, move |b, id| {
                b.get(&format!("/v1/repos/{id}/note?path={}{view}", encode(&note)))
            })
            .await?;
        text(note_text(&content))
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
        text(sync_text(&report))
    }

    #[tool(
        description = "List the wiki notes that wait for you to explain them. CodeWiki writes each note's facts from the code; what the thing is for and how it works is yours to write. Notes whose explanation may be behind the code come first, then notes never explained. Narrow the list with `folder` or `files`. Then call get_note_brief for a note."
    )]
    async fn list_waiting_notes(
        &self,
        Parameters(p): Parameters<WaitingParams>,
    ) -> Result<CallToolResult, McpError> {
        let mut query: Vec<String> = Vec::new();
        if let Some(folder) = p.folder.as_deref().filter(|f| !f.trim().is_empty()) {
            query.push(format!("folder={}", encode(folder)));
        }
        let files = p.files.unwrap_or_default().join(",");
        if !files.is_empty() {
            query.push(format!("files={}", encode(&files)));
        }
        if let Some(limit) = p.limit {
            query.push(format!("limit={limit}"));
        }
        let queue = self
            .call(p.repo, move |b, id| {
                b.get(&format!("/v1/repos/{id}/explanations?{}", query.join("&")))
            })
            .await?;
        text(waiting_text(&queue))
    }

    #[tool(
        description = "Get what you need to explain one wiki note: the note as it is, which sections are yours to write, the source files to read first, and the limits. Read every listed file before you write, then call submit_note with the `base` value from this brief."
    )]
    async fn get_note_brief(
        &self,
        Parameters(p): Parameters<BriefParams>,
    ) -> Result<CallToolResult, McpError> {
        let note = p.note;
        let brief = self
            .call(p.repo, move |b, id| {
                b.get(&format!(
                    "/v1/repos/{id}/explanation?path={}",
                    encode(&note)
                ))
            })
            .await?;
        text(brief_text(&brief))
    }

    #[tool(
        description = "Send the sections you wrote for one wiki note, with pictures where one explains more than text. Send only your sections; CodeWiki puts them into the note and keeps its facts. The answer is `saved`, or a list of faults to fix and send again, or a request to read the brief again. A note its owner edited is not written: your text is queued for the owner to review."
    )]
    async fn submit_note(
        &self,
        Parameters(p): Parameters<SubmitParams>,
    ) -> Result<CallToolResult, McpError> {
        let note = p.note;
        let body = json!({
            "base": p.base,
            "sections": p
                .sections
                .iter()
                .map(|s| json!({ "heading": s.heading, "text": s.text }))
                .collect::<Vec<_>>(),
            "pictures": p
                .pictures
                .unwrap_or_default()
                .iter()
                .map(|x| json!({ "section": x.section, "source": x.source }))
                .collect::<Vec<_>>(),
            "read": p.read.unwrap_or_default(),
            "model": p.model,
        });
        let outcome = self
            .call(p.repo, move |b, id| {
                b.put(
                    &format!("/v1/repos/{id}/explanation?path={}", encode(&note)),
                    &body,
                )
            })
            .await?;
        text(outcome_text(&outcome))
    }
}

/// How many notes wait for an agent, in words; empty when none does.
fn waiting_line(waiting: &Value) -> String {
    let behind = waiting["behind"].as_u64().unwrap_or(0);
    let unexplained = waiting["unexplained"].as_u64().unwrap_or(0);
    if behind + unexplained == 0 {
        return String::new();
    }
    format!(
        "{} notes wait for an explanation: {behind} behind the code, {unexplained} never explained.",
        behind + unexplained
    )
}

/// What a sync did, and what it left for an agent.
fn sync_text(report: &Value) -> String {
    let count = |key: &str| report[key].as_array().map(Vec::len).unwrap_or(0);
    let mut summary = format!(
        "Wiki updated: {} notes written, {} unchanged, {} proposals queued for human review, {} removed, {} adopted. Total notes: {}.",
        count("written"),
        report["kept"],
        count("proposed"),
        count("removed"),
        count("adopted"),
        report["notes_total"]
    );
    if count("merged") > 0 {
        summary.push_str(&format!(
            " {} explained notes got new facts and kept their explanation.",
            count("merged")
        ));
    }
    let waiting = waiting_line(&json!({
        "behind": count("behind"),
        "unexplained": report["unexplained"].as_u64().unwrap_or(0),
    }));
    if !waiting.is_empty() {
        summary.push_str(&format!(" {waiting} Call list_waiting_notes to see them."));
    }
    summary
}

/// The queue of notes that wait, for an agent to read.
fn waiting_text(queue: &Value) -> String {
    let notes = rows(queue, "notes");
    let waiting = waiting_line(&queue["waiting"]);
    if waiting.is_empty() {
        return "No note waits for an explanation.\n".to_string();
    }
    let mut body = format!("{waiting}\n");
    if notes.is_empty() {
        body.push_str("None of them fits what you asked for.\n");
        return body;
    }
    body.push_str(&format!(
        "{} fit what you asked for; the first {}:\n\n",
        queue["matching"].as_u64().unwrap_or(0),
        notes.len()
    ));
    for (index, note) in notes.iter().enumerate() {
        let why = match field(note, "why") {
            "behind" => "behind the code",
            _ => "never explained",
        };
        body.push_str(&format!(
            "{}. {}  ({why})\n",
            index + 1,
            field(note, "path")
        ));
        for reason in rows(note, "behind") {
            body.push_str(&format!("   - {}\n", reason.as_str().unwrap_or("")));
        }
        let sources: Vec<&str> = rows(note, "sources")
            .into_iter()
            .filter_map(Value::as_str)
            .collect();
        if !sources.is_empty() {
            body.push_str(&format!("   about: {}\n", sources.join(", ")));
        }
    }
    body.push_str(
        "\nFor each note: call get_note_brief, read every file it lists, then call submit_note.\n",
    );
    body
}

/// The brief of one note, for an agent to read.
fn brief_text(brief: &Value) -> String {
    let mut body = format!(
        "Note: {}\nbase: {}  (send this value back in submit_note)\n",
        field(brief, "path"),
        field(brief, "base")
    );
    if brief["sync_first"] == true {
        body.push_str(
            "\nSTOP: the code under this note changed since the last sync, so the facts below are old. Call sync_wiki, then get this brief again.\n",
        );
    }
    if brief["edited_by_hand"] == true {
        body.push_str(
            "\nThe owner edited this note by hand. What you submit is not written into it: it is queued for the owner to review.\n",
        );
    }
    let behind = rows(brief, "behind");
    if !behind.is_empty() {
        body.push_str("\nThe explanation may be behind the code:\n");
        for reason in behind {
            body.push_str(&format!("- {}\n", reason.as_str().unwrap_or("")));
        }
    }
    body.push_str("\nYour sections (write these, and only these):\n");
    for section in rows(brief, "yours") {
        let state = match field(section, "written_by") {
            "agent" => "an agent wrote it; rewrite it only if it is wrong or behind",
            "rules" => "holds text made by rule from the code; replace it with an explanation",
            _ => "not in the note yet",
        };
        body.push_str(&format!("- {}: {state}", field(section, "heading")));
        if section["prose_only"] == true {
            body.push_str(
                ". Its code block is a fact CodeWiki writes: send prose only, it goes under the block",
            );
        }
        body.push('\n');
    }
    let rules: Vec<&str> = rows(brief, "rules")
        .into_iter()
        .filter_map(Value::as_str)
        .collect();
    body.push_str(&format!(
        "\nCodeWiki writes these sections from the code, and you cannot change them: {}.\n",
        rules.join(", ")
    ));
    body.push_str("\nRead these files before you write:\n");
    for source in rows(brief, "sources") {
        body.push_str(&format!(
            "- {}{}\n",
            field(source, "path"),
            if source["changed"] == true {
                "  (changed since the note was explained)"
            } else {
                ""
            }
        ));
    }
    let limits = &brief["limits"];
    body.push_str(&format!(
        "\nText: at most {} words for a section. No `#` or `##` heading inside a text. Link to another note as [[Note Name]]; the note must exist. No Markdown links, no callouts, no checklists, no file names with line numbers.\n",
        limits["section_words"]
    ));
    body.push_str(&format!(
        "\nPictures: {}\nComponents:\n{}\n",
        field(limits, "pictures"),
        field(limits, "components")
    ));
    let pictures = rows(brief, "pictures");
    if !pictures.is_empty() {
        body.push_str("\nThe note's pictures now:\n");
        for picture in pictures {
            body.push_str(&format!(
                "# section: {}\n{}\n",
                field(picture, "section"),
                field(picture, "source").trim_end()
            ));
        }
    }
    body.push_str(&format!(
        "\n----- The note as it is -----\n{}",
        field(brief, "text")
    ));
    body
}

/// What became of a submission, for an agent to act on.
fn outcome_text(outcome: &Value) -> String {
    match field(outcome, "outcome") {
        "saved" => {
            let sections: Vec<&str> = rows(outcome, "sections")
                .into_iter()
                .filter_map(Value::as_str)
                .collect();
            let waiting = waiting_line(&outcome["waiting"]);
            format!(
                "Saved {}. An agent's text is in: {}. Pictures: {}. {}\n",
                field(outcome, "path"),
                sections.join(", "),
                outcome["pictures"],
                if waiting.is_empty() {
                    "No note waits any more.".to_string()
                } else {
                    waiting
                }
            )
        }
        "issues" => {
            let mut body =
                "Not saved. Fix each of these, then call submit_note again with the same base:\n"
                    .to_string();
            for issue in rows(outcome, "issues") {
                body.push_str(&format!("- {}\n", issue.as_str().unwrap_or("")));
            }
            body
        }
        "proposed" => format!(
            "Not written: {} Proposal {}.\n",
            field(outcome, "message"),
            field(outcome, "proposal")
        ),
        "sync_first" => format!(
            "Not saved: {}. Call sync_wiki, then get_note_brief again.\n",
            field(outcome, "message")
        ),
        "changed" => format!(
            "Not saved: {}. Call get_note_brief again.\n",
            field(outcome, "message")
        ),
        other => format!("Unknown answer '{other}': {outcome}\n"),
    }
}

/// The text of a note as the backend answered, with a last line that names
/// what was left out and how to get it.
fn note_text(content: &Value) -> String {
    let mut body = content["text"].as_str().unwrap_or("").to_string();
    let left_out: Vec<String> = content["left_out"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|heading| heading.as_str())
        .map(|heading| format!("`{heading}`"))
        .collect();
    if !left_out.is_empty() {
        if !body.ends_with('\n') {
            body.push('\n');
        }
        body.push_str(&format!(
            "\n(Left out: {}. Call read_note with `full: true` to read it.)\n",
            left_out.join(", ")
        ));
    }
    body
}

#[tool_handler(
    name = "codewiki",
    instructions = "CodeWiki serves a maintained wiki and a structural index of the repository. Call get_overview first in an unfamiliar codebase. Before editing a file or symbol, call get_context on it. Use search_codebase to locate things and read_note for a note; read_note leaves out the walk-through of calls unless you pass full: true. After editing, call get_impacted_tests to learn which tests to run. Call sync_wiki after substantial code changes. CodeWiki writes each note's facts and you write its explanation: list_waiting_notes shows the notes that wait, get_note_brief gives one note with the sections that are yours, submit_note sends what you wrote."
)]
impl ServerHandler for Bridge {
    /// Refuse before the router sees the call, so a closed folder starts no
    /// backend. `#[tool_handler]` writes `call_tool` only when it is absent;
    /// this one is the macro's own body behind the screen.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let bridge = self.clone();
        let arguments = request.arguments.clone();
        let refused = tokio::task::spawn_blocking(move || bridge.screen(arguments.as_ref()))
            .await
            .map_err(|e| McpError::internal_error(format!("tool task failed: {e}"), None))?;
        if let Some(text) = refused {
            return Ok(CallToolResult::success(vec![ContentBlock::text(text)]).into());
        }
        let call = ToolCallContext::new(self, request, context);
        Self::tool_router().call(call).await
    }
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    // Standard output carries the protocol; the log goes to `mcp.log` and to
    // standard error, which the agent's host keeps.
    let _telemetry = cw_telemetry::init("mcp", log::LevelFilter::Info);
    let service = match Bridge::default().serve(stdio()).await {
        Ok(service) => service,
        Err(e) => {
            log::error!("codewiki-mcp: failed to start: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    if let Err(e) = service.waiting().await {
        log::error!("codewiki-mcp: stopped with an error: {e}");
        return std::process::ExitCode::FAILURE;
    }
    log::info!("codewiki-mcp: the agent closed the connection");
    std::process::ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The process folder and environment are global: tests that change them
    /// take this lock and put everything back.
    static PROCESS_STATE: Mutex<()> = Mutex::new(());

    struct Session {
        _lock: std::sync::MutexGuard<'static, ()>,
        folder: PathBuf,
        data: Option<std::ffi::OsString>,
        project: Option<std::ffi::OsString>,
    }

    impl Session {
        /// Work in `folder`, with `data` as the data folder and `project` as
        /// `CLAUDE_PROJECT_DIR`.
        #[allow(unsafe_code)]
        fn start(folder: &Path, data: &Path, project: Option<&str>) -> Session {
            let lock = PROCESS_STATE.lock().unwrap_or_else(|e| e.into_inner());
            let session = Session {
                _lock: lock,
                folder: std::env::current_dir().unwrap(),
                data: std::env::var_os("CODEWIKI_DATA_DIR"),
                project: std::env::var_os("CLAUDE_PROJECT_DIR"),
            };
            std::env::set_current_dir(folder).unwrap();
            // SAFETY: every test that touches the environment holds PROCESS_STATE.
            unsafe {
                std::env::set_var("CODEWIKI_DATA_DIR", data);
                match project {
                    Some(dir) => std::env::set_var("CLAUDE_PROJECT_DIR", dir),
                    None => std::env::remove_var("CLAUDE_PROJECT_DIR"),
                }
            }
            session
        }
    }

    impl Drop for Session {
        #[allow(unsafe_code)]
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.folder);
            // SAFETY: the lock held by this session is still held here.
            unsafe {
                for (key, old) in [
                    ("CODEWIKI_DATA_DIR", &self.data),
                    ("CLAUDE_PROJECT_DIR", &self.project),
                ] {
                    match old {
                        Some(value) => std::env::set_var(key, value),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }

    /// A fresh folder holding `data`, `closed` and `open`; `closed` is on the
    /// list.
    fn scene(name: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("cw-mcp-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (data, closed, open) = (root.join("data"), root.join("closed"), root.join("open"));
        for dir in [&data, &closed, &open] {
            std::fs::create_dir_all(dir).unwrap();
        }
        cw_core::agents_off::add(&data, &closed).unwrap();
        (root, data, closed, open)
    }

    #[test]
    fn screen_refuses_before_any_backend_starts() {
        let (root, data, closed, _) = scene("folder");
        let session = Session::start(&closed, &data, None);
        let bridge = Bridge::default();
        assert_eq!(bridge.screen(None), Some(guard::refusal(&closed)));
        assert!(bridge.backend.lock().unwrap().is_none());
        drop(session);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn screen_refuses_a_covered_repo_path_without_a_backend() {
        let (root, data, closed, open) = scene("repo");
        let session = Session::start(&open, &data, None);
        let bridge = Bridge::default();
        let args = json!({ "repo": closed.join("sub").to_string_lossy() });
        assert_eq!(
            bridge.screen(args.as_object()),
            Some(guard::refusal(&closed))
        );
        // The open folder, and a path outside the list, are fine.
        assert_eq!(bridge.screen(None), None);
        let other = json!({ "repo": open.to_string_lossy() });
        assert_eq!(bridge.screen(other.as_object()), None);
        assert!(bridge.backend.lock().unwrap().is_none());
        drop(session);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn screen_normalizes_a_relative_claude_project_dir() {
        let (root, data, closed, open) = scene("project");
        let session = Session::start(&open, &data, Some("../closed"));
        let bridge = Bridge::default();
        assert_eq!(bridge.screen(None), Some(guard::refusal(&closed)));
        drop(session);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn screen_refuses_when_the_list_is_broken() {
        let (root, data, _, open) = scene("broken");
        std::fs::write(cw_core::agents_off::file(&data), "{").unwrap();
        let session = Session::start(&open, &data, None);
        let refused = Bridge::default().screen(None).unwrap();
        assert!(refused.contains("cannot be read"), "{refused}");
        drop(session);
        let _ = std::fs::remove_dir_all(root);
    }

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
    fn a_cut_note_says_what_was_left_out() {
        let cut = note_text(&json!({
            "text": "# Scan Module\n\n## Items\n\n- `scan`\n",
            "left_out": ["How it works"],
        }));
        assert_eq!(
            cut,
            "# Scan Module\n\n## Items\n\n- `scan`\n\n(Left out: `How it works`. Call read_note with `full: true` to read it.)\n"
        );
        // A whole note is given as it is.
        let whole = note_text(&json!({ "text": "# Crate Dependency Flow\n" }));
        assert_eq!(whole, "# Crate Dependency Flow\n");
        assert_eq!(note_text(&json!({})), "");
    }

    #[test]
    fn a_sync_says_what_waits_for_an_agent() {
        let quiet = sync_text(&json!({
            "written": ["a.md"], "kept": 4, "proposed": [], "removed": [], "adopted": [],
            "notes_total": 5, "merged": [], "behind": [], "unexplained": 0,
        }));
        assert_eq!(
            quiet,
            "Wiki updated: 1 notes written, 4 unchanged, 0 proposals queued for human review, 0 removed, 0 adopted. Total notes: 5."
        );
        // A report of an older backend has no such fields.
        assert!(!sync_text(&json!({ "kept": 1, "notes_total": 1 })).contains("wait"));
        let busy = sync_text(&json!({
            "written": [], "kept": 4, "proposed": [], "removed": [], "adopted": [],
            "notes_total": 5, "merged": ["a.md"], "behind": ["a.md", "b.md"], "unexplained": 3,
        }));
        assert!(
            busy.ends_with(
                "Total notes: 5. 1 explained notes got new facts and kept their explanation. 5 notes wait for an explanation: 2 behind the code, 3 never explained. Call list_waiting_notes to see them."
            ),
            "{busy}"
        );
    }

    #[test]
    fn the_queue_reads_as_a_list_of_work() {
        let body = waiting_text(&json!({
            "waiting": { "behind": 1, "unexplained": 7 },
            "matching": 3,
            "notes": [
                { "path": "01-Core/app/Tray Module.md", "why": "behind",
                  "behind": ["src/tray.rs changed"], "sources": ["src/tray.rs"] },
                { "path": "01-Core/app/Autostart Module.md", "why": "unexplained",
                  "sources": ["src/autostart.rs", "src/main.rs"] },
            ],
        }));
        assert!(
            body.starts_with(
                "8 notes wait for an explanation: 1 behind the code, 7 never explained.\n3 fit what you asked for; the first 2:\n\n1. 01-Core/app/Tray Module.md  (behind the code)\n   - src/tray.rs changed\n   about: src/tray.rs\n2. 01-Core/app/Autostart Module.md  (never explained)\n   about: src/autostart.rs, src/main.rs\n"
            ),
            "{body}"
        );
        assert!(body.contains("call get_note_brief"), "{body}");
        assert_eq!(
            waiting_text(&json!({ "waiting": { "behind": 0, "unexplained": 0 }, "notes": [] })),
            "No note waits for an explanation.\n"
        );
        let none = waiting_text(&json!({
            "waiting": { "behind": 0, "unexplained": 2 }, "matching": 0, "notes": [],
        }));
        assert!(
            none.ends_with("None of them fits what you asked for.\n"),
            "{none}"
        );
    }

    #[test]
    fn a_brief_tells_an_agent_what_is_its_own() {
        let body = brief_text(&json!({
            "path": "00-Architecture/app Startup Flow.md",
            "base": "abc123",
            "text": "# app Startup Flow\n\n> Call sequence.\n",
            "yours": [
                { "heading": "How it works", "text": "", "written_by": "rules", "prose_only": true },
                { "heading": "Gotchas", "text": "", "written_by": "nobody" },
            ],
            "rules": ["Source", "See also"],
            "sources": [{ "path": "src/main.rs", "changed": true }, { "path": "src/app.rs" }],
            "behind": ["src/main.rs changed"],
            "pictures": [{ "section": "Gotchas", "source": "root = Picture([c])\nc = Callout(\"x\")\n" }],
            "limits": { "section_words": 300, "pictures": "At most 4 pictures.", "components": "Picture(children: array)" },
        }));
        for expected in [
            "Note: 00-Architecture/app Startup Flow.md\nbase: abc123  (send this value back in submit_note)\n",
            "The explanation may be behind the code:\n- src/main.rs changed\n",
            "- How it works: holds text made by rule from the code; replace it with an explanation. Its code block is a fact CodeWiki writes: send prose only, it goes under the block\n",
            "- Gotchas: not in the note yet\n",
            "you cannot change them: Source, See also.\n",
            "- src/main.rs  (changed since the note was explained)\n- src/app.rs\n",
            "at most 300 words for a section",
            "Pictures: At most 4 pictures.\nComponents:\nPicture(children: array)\n",
            "# section: Gotchas\nroot = Picture([c])\nc = Callout(\"x\")\n",
            "----- The note as it is -----\n# app Startup Flow\n\n> Call sequence.\n",
        ] {
            assert!(body.contains(expected), "missing `{expected}` in:\n{body}");
        }
        assert!(!body.contains("STOP") && !body.contains("by hand"));
        let warned = brief_text(&json!({ "sync_first": true, "edited_by_hand": true }));
        assert!(warned.contains("STOP: the code under this note changed"));
        assert!(warned.contains("queued for the owner to review"));
    }

    #[test]
    fn each_answer_to_a_submission_says_what_to_do_next() {
        assert_eq!(
            outcome_text(&json!({
                "outcome": "saved", "path": "01-Core/app/Tray Module.md",
                "sections": ["What it does", "Gotchas"], "pictures": 1,
                "waiting": { "behind": 0, "unexplained": 2 },
            })),
            "Saved 01-Core/app/Tray Module.md. An agent's text is in: What it does, Gotchas. Pictures: 1. 2 notes wait for an explanation: 0 behind the code, 2 never explained.\n"
        );
        let last = outcome_text(&json!({
            "outcome": "saved", "path": "a.md", "sections": ["Gotchas"], "pictures": 0,
            "waiting": { "behind": 0, "unexplained": 0 },
        }));
        assert!(
            last.ends_with("Pictures: 0. No note waits any more.\n"),
            "{last}"
        );
        assert_eq!(
            outcome_text(
                &json!({ "outcome": "issues", "issues": ["`Items` is not yours", "too long"] })
            ),
            "Not saved. Fix each of these, then call submit_note again with the same base:\n- `Items` is not yours\n- too long\n"
        );
        let proposed = outcome_text(&json!({
            "outcome": "proposed", "proposal": "note-agent-abc", "message": "it waits in the review queue.",
        }));
        assert_eq!(
            proposed,
            "Not written: it waits in the review queue. Proposal note-agent-abc.\n"
        );
        assert!(
            outcome_text(&json!({ "outcome": "sync_first", "message": "the code changed" }))
                .ends_with("Call sync_wiki, then get_note_brief again.\n")
        );
        assert!(
            outcome_text(&json!({ "outcome": "changed", "message": "the note changed" }))
                .ends_with("Call get_note_brief again.\n")
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
