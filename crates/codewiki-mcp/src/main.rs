//! MCP stdio bridge between coding agents and the CodeWiki backend.
//!
//! Standard output carries the MCP protocol, so nothing else may be printed
//! there; diagnostics go to standard error.

mod guard;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use cw_client::{Backend, encode};
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorCode,
};
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
    /// The agent id each note claimed through this bridge was listed under.
    claims: Arc<ClaimIds>,
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
    /// True when you will write the listed notes now: they are kept for you for 30 minutes and other agents get other notes; submitting one frees it.
    #[serde(default)]
    claim: Option<bool>,
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
    ///
    /// With folders on the list, the repository the call reaches is checked
    /// once its id is known, and the call is refused when its folder is kept
    /// from coding agents or holds a kept folder. `screen` decided on a lookup
    /// of its own, or on none when the backend was not up yet, and `repo_id`
    /// falls back to a repository's name when it cannot add a path; so only
    /// the id the work is given is trusted. One listing serves the lookup of
    /// an id or name and that check.
    async fn call<F>(&self, repo: Option<String>, work: F) -> Result<Value, McpError>
    where
        F: FnOnce(&Backend, &str) -> Result<Value, String> + Send + 'static,
    {
        let slot = self.backend.clone();
        tokio::task::spawn_blocking(move || {
            let entries = listed_folders()?;
            let backend = connected(&slot).map_err(failed)?;
            let cwd = std::env::current_dir()
                .map_err(|e| failed(format!("cannot read the working directory: {e}")))?;
            // The backend gets what `screen` read: a registered repository's
            // id, or an absolute path against this session's folder, never a
            // word or relative path it would read against its own folder.
            let arg = repo.map(|r| guard::repo_arg(&r, &cwd));
            let listing = if !entries.is_empty() {
                Some(
                    backend
                        .get("/v1/repos")
                        .map_err(|e| failed(unchecked(&e)))?,
                )
            } else if matches!(arg, Some(guard::RepoArg::IdOrName(_))) {
                backend.get("/v1/repos").ok()
            } else {
                None
            };
            let target = match &arg {
                Some(arg) => guard::target(arg, listing.as_ref(), &cwd),
                None => guard::Target::Path(cwd),
            };
            let id = match target {
                guard::Target::Id(id) => id,
                guard::Target::Path(path) => {
                    backend.repo_id(&path.to_string_lossy()).map_err(failed)?
                }
            };
            if !entries.is_empty() {
                let path = match listing.as_ref().and_then(|l| guard::path_of(l, &id)) {
                    Some(path) => path,
                    // Registered by this call, or since the listing.
                    None => backend
                        .get(&format!("/v1/repos/{id}"))
                        .and_then(|repo| {
                            repo["path"]
                                .as_str()
                                .map(PathBuf::from)
                                .ok_or_else(|| format!("repository {id} has no path"))
                        })
                        .map_err(|e| failed(unchecked(&e)))?,
                };
                if cw_core::agents_off::blocking(&entries, &path).is_some() {
                    return Err(refused(cw_core::agents_off::repo_refusal()));
                }
            }
            work(&backend, &id).map_err(failed)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("tool task failed: {e}"), None))?
    }

    /// [`Bridge::call`] for a request that concerns no single repository.
    /// `work` gets the folders on the list, to leave out what they keep.
    async fn call_global<F>(&self, work: F) -> Result<Value, McpError>
    where
        F: FnOnce(&Backend, &[PathBuf]) -> Result<Value, String> + Send + 'static,
    {
        let slot = self.backend.clone();
        tokio::task::spawn_blocking(move || {
            let entries = listed_folders()?;
            let backend = connected(&slot).map_err(failed)?;
            work(&backend, &entries).map_err(failed)
        })
        .await
        .map_err(|e| McpError::internal_error(format!("tool task failed: {e}"), None))?
    }
}

/// Why a call that must check the repository it reaches cannot go ahead.
fn unchecked(error: &str) -> String {
    format!(
        "CodeWiki cannot check this repository against the folders kept from coding agents: {error}"
    )
}

/// Marks an error that is a refusal, found once the backend has answered:
/// [`answered`] gives the agent its text as a normal result.
const REFUSED: &str = "codewiki_refused";

/// A refusal that a tool's work returns as an error, to stop it there.
fn refused(text: impl Into<String>) -> McpError {
    McpError::new(
        ErrorCode::INVALID_REQUEST,
        text.into(),
        Some(json!({ REFUSED: true })),
    )
}

/// A tool call's response, with a refusal made the text an agent reads, as
/// the refusals of `screen` are.
fn answered(response: Result<CallToolResponse, McpError>) -> Result<CallToolResponse, McpError> {
    match response {
        Err(error) if error.data.as_ref().is_some_and(|d| d[REFUSED] == true) => {
            Ok(CallToolResult::success(vec![ContentBlock::text(error.message)]).into())
        }
        other => other,
    }
}

impl Bridge {
    /// The refusal text when this call may not go ahead.
    ///
    /// The list is read on every call. The session folders and a path `repo`
    /// argument are checked without a backend; only an id or name `repo`
    /// argument connects, for one `GET /v1/repos`. When that fails the
    /// session folders alone decide here, and [`Bridge::call`] checks the
    /// repository the call reaches. A session folder that is kept, or holds
    /// a kept folder, refuses with the entry's path; a `repo` argument that
    /// is or reaches such a folder refuses with no path, for that repository
    /// alone.
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
        let repo_refusal = || cw_core::agents_off::repo_refusal().to_string();
        if cw_core::agents_off::blocking(&entries, &direct).is_some() {
            return Some(repo_refusal());
        }
        // The backend falls back to a repository's name when it cannot add a
        // path, so a missing path is looked up by its last component too.
        let name = guard::lookup_name(&arg, |path| path.is_dir())?;
        let listing = connected(&self.backend)
            .and_then(|backend| backend.get("/v1/repos"))
            .ok()?;
        let path = guard::resolve_id(&listing, &name)?;
        cw_core::agents_off::blocking(&entries, &path).map(|_| repo_refusal())
    }
}

/// The folders on the list. A list that cannot be read refuses the call, as
/// `screen` does: it never passes for an empty one.
fn listed_folders() -> Result<Vec<PathBuf>, McpError> {
    let data = cw_core::paths::data_dir();
    cw_core::agents_off::load(&data).map_err(|_| {
        refused(cw_core::agents_off::unreadable(&cw_core::agents_off::file(
            &data,
        )))
    })
}

/// The `/v1/repos` listing without the repositories whose folders `entries`
/// keep or that hold one of them, and their names.
fn visible_repos(
    backend: &Backend,
    entries: &[PathBuf],
) -> Result<(Value, HashSet<String>), String> {
    let mut listing = backend.get("/v1/repos")?;
    let hidden = guard::hide_repos(&mut listing, entries);
    Ok((listing, hidden))
}

/// Fetch `uri` and drop the repositories `entries` keep from it with `hide`.
/// With an empty list the answer is returned as it is.
fn get_visible(
    backend: &Backend,
    entries: &[PathBuf],
    uri: &str,
    hide: fn(&mut Value, &HashSet<String>),
) -> Result<Value, String> {
    let mut view = backend.get(uri)?;
    if !entries.is_empty() {
        hide(&mut view, &visible_repos(backend, entries)?.1);
    }
    Ok(view)
}

/// A tool call that failed: logged, and returned to the agent as an error.
fn failed(message: String) -> McpError {
    log::warn!("tool call failed: {message}");
    McpError::internal_error(message, None)
}

/// Whether a call failed because the wiki has no note of the name given.
fn is_no_such_note(error: &McpError) -> bool {
    error.message.contains("not found: note '")
}

/// What an agent is told of a note that is not in the wiki. It is no
/// failure of the agent's: a sync removes a note when what it describes is
/// gone from the repository, and a list of waiting notes may be older than
/// that sync.
fn no_such_note(note: &str) -> String {
    format!(
        "No such note: {note}. A sync removes a note when what it describes is gone from the repository. Skip it and go on."
    )
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
                .call_global(|b, entries| visible_repos(b, entries).map(|(listing, _)| listing))
                .await?,
        )
    }

    #[tool(
        description = "Every repository the backend manages at a glance: files, symbols, notes, pending proposals, whether the wiki is stale, and the top hotspots of each, plus the external packages two or more repositories use. Use to orient across several repositories."
    )]
    async fn get_workspace(&self) -> Result<CallToolResult, McpError> {
        let view = self
            .call_global(|b, entries| {
                get_visible(b, entries, "/v1/workspace", guard::hide_in_workspace)
            })
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
                .call_global(|b, entries| {
                    get_visible(
                        b,
                        entries,
                        "/v1/workspace/contracts",
                        guard::hide_in_contracts,
                    )
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
        description = "Tests to run after a change. With no `files`, uses the uncommitted changes in the working tree. Lists the test files that can reach the changed files through the dependency graph, nearest first, with the test names and the commands that run them (`cargo test` for Rust, `dotnet test` for C#, `go test` for Go), plus changed files no test reaches. Call after editing and before reporting work as done."
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
        description = "List the wiki notes that wait for you to explain them. CodeWiki writes each note's facts from the code; what the thing is for and how it works is yours to write. Notes whose explanation may be behind the code come first, then notes never explained. Narrow the list with `folder` or `files`. Pass `claim: true` when you will write the listed notes now: they are kept for you for 30 minutes and other agents get other notes. Then call get_note_brief for a note."
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
        let claim = p.claim == Some(true);
        let listing = claim.then(|| self.claims.next());
        if let Some(agent) = &listing {
            query.push(format!("agent={}", encode(agent)));
        }
        let claims = self.claims.clone();
        let queue = self
            .call(p.repo, move |b, id| {
                let queue = b.get(&format!("/v1/repos/{id}/explanations?{}", query.join("&")))?;
                if let Some(agent) = &listing {
                    claims.hold(id, agent, &queue);
                }
                Ok(queue)
            })
            .await?;
        text(waiting_text(&queue, claim))
    }

    #[tool(
        description = "Get what you need to explain one wiki note: the note as it is, which sections are yours to write, the source files to read first, and the limits. Read every listed file before you write, then call submit_note with the `base` value from this brief."
    )]
    async fn get_note_brief(
        &self,
        Parameters(p): Parameters<BriefParams>,
    ) -> Result<CallToolResult, McpError> {
        let (note, named) = (p.note.clone(), p.note);
        let brief = self
            .call(p.repo, move |b, id| {
                b.get(&format!(
                    "/v1/repos/{id}/explanation?path={}",
                    encode(&note)
                ))
            })
            .await;
        match brief {
            Err(error) if is_no_such_note(&error) => text(no_such_note(&named)),
            brief => text(brief_text(&brief?)),
        }
    }

    #[tool(
        description = "Send the sections you wrote for one wiki note, with pictures where one explains more than text. Send only your sections; CodeWiki puts them into the note and keeps its facts. The answer is `saved`, or a list of faults to fix and send again, or a request to read the brief again. A note its owner edited is not written: your text is queued for the owner to review."
    )]
    async fn submit_note(
        &self,
        Parameters(p): Parameters<SubmitParams>,
    ) -> Result<CallToolResult, McpError> {
        let (note, named) = (p.note.clone(), p.note);
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
        let claims = self.claims.clone();
        let outcome = self
            .call(p.repo, move |b, id| {
                // The id the note was listed under, so that the backend
                // renews that listing's other notes.
                let agent = claims.agent_for(id, &note);
                let outcome = b.put(
                    &format!(
                        "/v1/repos/{id}/explanation?path={}&agent={}",
                        encode(&note),
                        encode(&agent)
                    ),
                    &body,
                )?;
                claims.settle(id, &note, &outcome);
                Ok(outcome)
            })
            .await;
        match outcome {
            Err(error) if is_no_such_note(&error) => text(no_such_note(&named)),
            outcome => text(outcome_text(&outcome?)),
        }
    }
}

/// The agent ids this bridge lists notes under. Every listing that claims
/// gets an id of its own: the main agent, its background note writer and a
/// note writer launched after `/clear` all run through one bridge, and with
/// one id for all of them a second batch was handed the notes the first
/// still held. A submission sends the id its note was listed under.
#[derive(Default)]
struct ClaimIds {
    /// How many ids were made.
    made: AtomicU64,
    /// (repository id, note path in the vault) to the id the note was
    /// listed under.
    held: Mutex<HashMap<(String, String), String>>,
}

impl ClaimIds {
    /// A new id for one listing: `<pid>-<start nanos>-<count>`, some 50
    /// characters at the most, within the backend's 64.
    fn next(&self) -> String {
        let count = self.made.fetch_add(1, Ordering::Relaxed) + 1;
        format!("{}-{count}", agent_id())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<(String, String), String>> {
        self.held
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Note that the notes `queue` lists in `repo` were listed under `agent`.
    fn hold(&self, repo: &str, agent: &str, queue: &Value) {
        let mut held = self.lock();
        for note in rows(queue, "notes") {
            let path = field(note, "path");
            if !path.is_empty() {
                held.insert((repo.to_string(), path.to_string()), agent.to_string());
            }
        }
    }

    /// The id to submit `note` under: the id of the listing that claimed it,
    /// or the bridge's own when it was not claimed through this bridge.
    fn agent_for(&self, repo: &str, note: &str) -> String {
        let held = self.lock();
        held_key(&held, repo, note)
            .and_then(|key| held.get(&key).cloned())
            .unwrap_or_else(|| agent_id().to_string())
    }

    /// Forget the note an answer saved or proposed: the backend freed it.
    fn settle(&self, repo: &str, note: &str, outcome: &Value) {
        if !matches!(field(outcome, "outcome"), "saved" | "proposed") {
            return;
        }
        let mut held = self.lock();
        held.remove(&(repo.to_string(), field(outcome, "path").to_string()));
        if let Some(key) = held_key(&held, repo, note) {
            held.remove(&key);
        }
    }
}

/// The key of the note that `note` names in `repo`, read as the backend
/// reads it: a vault path, or a note's name, which is its file name without
/// `.md` in any case.
fn held_key(
    held: &HashMap<(String, String), String>,
    repo: &str,
    note: &str,
) -> Option<(String, String)> {
    let exact = (repo.to_string(), note.to_string());
    if held.contains_key(&exact) {
        return Some(exact);
    }
    let wanted = note.trim_end_matches(".md").to_lowercase();
    held.keys()
        .find(|(r, path)| {
            let name = path.rsplit('/').next().unwrap_or(path);
            r == repo && name.trim_end_matches(".md").to_lowercase() == wanted
        })
        .cloned()
}

/// Names this bridge process to the backend, so it can keep notes for it. Made
/// once. A listing that claims adds a count to it ([`ClaimIds::next`]); a
/// note not claimed through this bridge is submitted under it alone.
fn agent_id() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("{}-{nanos}", std::process::id())
    })
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
    // What became of notes whose subject is gone or moved, in the words
    // the backend chose. An older backend sends none.
    for line in report["summary"].as_array().into_iter().flatten() {
        if let Some(line) = line.as_str() {
            summary.push(' ');
            summary.push_str(line);
        }
    }
    summary
}

/// The queue of notes that wait, for an agent to read. `claimed` tells that
/// the listing asked to keep the notes for this agent.
fn waiting_text(queue: &Value, claimed: bool) -> String {
    let notes = rows(queue, "notes");
    let waiting = waiting_line(&queue["waiting"]);
    if waiting.is_empty() {
        return "No note waits for an explanation.\n".to_string();
    }
    let mut body = format!("{waiting}\n");
    let others = queue["claimed"].as_u64().unwrap_or(0);
    if notes.is_empty() {
        if others > 0 {
            body.push_str(&format!(
                "None is free now: another agent is writing the {others} that fit what you asked for.\n"
            ));
        } else {
            body.push_str("None of them fits what you asked for.\n");
        }
        return body;
    }
    if others > 0 {
        body.push_str(&format!(
            "{others} more are being written by another agent.\n"
        ));
    }
    body.push_str(&format!(
        "{} fit what you asked for; the first {}:\n\n",
        queue["matching"].as_u64().unwrap_or(0),
        notes.len()
    ));
    if claimed {
        body.push_str(&format!(
            "These notes are kept for you for {} minutes.\n\n",
            cw_core::CLAIM_MINUTES
        ));
    }
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
    // A note kept after what it describes went must not be read as a note
    // about the code as it is.
    if let Some(why) = content["info"]["gone"].as_str() {
        let reason = match why {
            "pending" => "it waits for its owner to retire it",
            "history" => "its owner keeps it as history",
            "uncommitted" => "the deletion is not committed yet",
            _ => "git does not hold the note as it is, so it was not deleted",
        };
        body = format!(
            "(What this note describes is gone from the repository; {reason}. Do not take it for the code as it is now.)\n\n{body}"
        );
    }
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
        answered(Self::tool_router().call(call).await)
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

    /// The variables a session sets, and puts back when it ends.
    const SESSION_VARS: [&str; 4] = [
        "CODEWIKI_DATA_DIR",
        "CLAUDE_PROJECT_DIR",
        "CODEWIKI_URL",
        "CODEWIKI_TOKEN",
    ];

    struct Session {
        _lock: std::sync::MutexGuard<'static, ()>,
        folder: PathBuf,
        saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
    }

    impl Session {
        /// Work in `folder`, with `data` as the data folder and `project` as
        /// `CLAUDE_PROJECT_DIR`, and no backend named.
        fn start(folder: &Path, data: &Path, project: Option<&str>) -> Session {
            Session::start_with(folder, data, project, None)
        }

        /// [`Session::start`], with `backend` as `CODEWIKI_URL`: the bridge
        /// then talks to it and never starts a backend of its own.
        #[allow(unsafe_code)]
        fn start_with(
            folder: &Path,
            data: &Path,
            project: Option<&str>,
            backend: Option<&str>,
        ) -> Session {
            let lock = PROCESS_STATE.lock().unwrap_or_else(|e| e.into_inner());
            let session = Session {
                _lock: lock,
                folder: std::env::current_dir().unwrap(),
                saved: SESSION_VARS
                    .iter()
                    .map(|key| (*key, std::env::var_os(key)))
                    .collect(),
            };
            std::env::set_current_dir(folder).unwrap();
            // SAFETY: every test that touches the environment holds PROCESS_STATE.
            unsafe {
                std::env::set_var("CODEWIKI_DATA_DIR", data);
                match project {
                    Some(dir) => std::env::set_var("CLAUDE_PROJECT_DIR", dir),
                    None => std::env::remove_var("CLAUDE_PROJECT_DIR"),
                }
                match backend {
                    Some(url) => {
                        std::env::set_var("CODEWIKI_URL", url);
                        std::env::set_var("CODEWIKI_TOKEN", "stand-in-token");
                    }
                    None => {
                        std::env::remove_var("CODEWIKI_URL");
                        std::env::remove_var("CODEWIKI_TOKEN");
                    }
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
                for (key, old) in &self.saved {
                    match old {
                        Some(value) => std::env::set_var(key, value),
                        None => std::env::remove_var(key),
                    }
                }
            }
        }
    }

    /// A backend on a local port that answers from `answer`, given a request
    /// as `"GET /v1/repos"` and how many such requests came before it. It
    /// answers `/v1/health` itself; it keeps every other request it is sent.
    struct StandIn {
        url: String,
        seen: Arc<Mutex<Vec<String>>>,
    }

    impl StandIn {
        fn start(answer: impl Fn(&str, usize) -> (u16, Value) + Send + 'static) -> StandIn {
            use std::io::{BufRead, BufReader, Read, Write};
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}", listener.local_addr().unwrap());
            let seen = Arc::new(Mutex::new(Vec::new()));
            let kept = seen.clone();
            std::thread::spawn(move || {
                let mut counts = std::collections::HashMap::<String, usize>::new();
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { return };
                    let mut reader = BufReader::new(stream);
                    let mut line = String::new();
                    if reader.read_line(&mut line).is_err() {
                        continue;
                    }
                    let mut length = 0;
                    loop {
                        let mut header = String::new();
                        if reader.read_line(&mut header).is_err() {
                            break;
                        }
                        let header = header.trim_end();
                        if header.is_empty() {
                            break;
                        }
                        if let Some((name, value)) = header.split_once(':')
                            && name.eq_ignore_ascii_case("content-length")
                        {
                            length = value.trim().parse().unwrap_or(0);
                        }
                    }
                    // Closing on a body nobody read resets the connection.
                    let _ = reader.read_exact(&mut vec![0; length]);
                    let request = line
                        .trim_end()
                        .rsplit_once(' ')
                        .map_or(line.trim_end(), |(request, _)| request)
                        .to_string();
                    let before = counts.entry(request.clone()).or_default();
                    let count = *before;
                    *before += 1;
                    let (status, body) = if request == "GET /v1/health" {
                        (200, json!({ "status": "ok", "version": "0.0.1" }))
                    } else {
                        kept.lock().unwrap().push(request.clone());
                        answer(&request, count)
                    };
                    let body = body.to_string();
                    let mut stream = reader.into_inner();
                    let _ = write!(
                        stream,
                        "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.flush();
                }
            });
            StandIn { url, seen }
        }

        /// The requests it was sent, `/v1/health` left out.
        fn seen(&self) -> Vec<String> {
            self.seen.lock().unwrap().clone()
        }
    }

    /// The `/v1/repos` listing of a scene: `closed` is on the list, `holder`
    /// (the scene's root) holds it, `open` is neither.
    fn repos(root: &Path, closed: &Path, open: &Path) -> Value {
        json!([
            { "id": "closed-1", "name": "closed", "path": closed },
            { "id": "holder-1", "name": "holder", "path": root },
            { "id": "open-1", "name": "open", "path": open },
        ])
    }

    fn run<T>(future: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    /// What a tool call says to the agent: its text, or its error.
    fn said(result: Result<CallToolResult, McpError>) -> String {
        match result {
            Ok(result) => result
                .content
                .iter()
                .filter_map(|block| block.as_text())
                .map(|text| text.text.clone())
                .collect(),
            Err(error) => error.message.into_owned(),
        }
    }

    fn overview_of(repo: &str) -> Parameters<RepoParam> {
        Parameters(RepoParam {
            repo: Some(repo.to_string()),
        })
    }

    fn repo_refusal() -> String {
        cw_core::agents_off::repo_refusal().to_string()
    }

    #[test]
    fn an_id_or_name_naming_a_kept_repo_is_refused_after_one_lookup() {
        let (root, data, closed, open) = scene("by-id");
        let listing = repos(&root, &closed, &open);
        let backend = StandIn::start(move |request, _| match request {
            "GET /v1/repos" => (200, listing.clone()),
            _ => (404, json!({ "error": "not here" })),
        });
        let session = Session::start_with(&open, &data, None, Some(&backend.url));
        let bridge = Bridge::default();
        // By id, by name in any case, and a repository that holds the entry.
        for repo in ["closed-1", "CLOSED", "holder-1", "holder"] {
            let args = json!({ "repo": repo });
            assert_eq!(
                bridge.screen(args.as_object()),
                Some(repo_refusal()),
                "{repo}"
            );
        }
        // One lookup each, and nothing else: no registration, no tool.
        assert_eq!(backend.seen(), vec!["GET /v1/repos"; 4]);
        let fine = json!({ "repo": "open-1" });
        assert_eq!(bridge.screen(fine.as_object()), None);
        drop(session);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn call_refuses_a_kept_repo_that_screen_could_not_look_up() {
        let (root, data, closed, open) = scene("late");
        let listing = repos(&root, &closed, &open);
        // The backend is still starting when `screen` asks, then answers.
        let backend = StandIn::start(move |request, count| match request {
            "GET /v1/repos" if count == 0 => (500, json!({ "error": "starting" })),
            "GET /v1/repos" => (200, listing.clone()),
            _ => (200, json!({ "home": "# Home", "status": {} })),
        });
        let session = Session::start_with(&open, &data, None, Some(&backend.url));
        let bridge = Bridge::default();
        let args = json!({ "repo": "closed-1" });
        assert_eq!(bridge.screen(args.as_object()), None);
        let answer = said(run(bridge.get_overview(overview_of("closed-1"))));
        assert_eq!(answer, repo_refusal());
        assert!(
            backend.seen().iter().all(|r| !r.contains("overview")),
            "{:?}",
            backend.seen()
        );
        // An open repository is not held up by the check.
        let answer = said(run(bridge.get_overview(overview_of("open-1"))));
        assert!(answer.starts_with("# Home"), "{answer}");
        drop(session);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_refusal_from_the_work_reaches_the_agent_as_a_normal_result() {
        let Ok(CallToolResponse::Complete(result)) = answered(Err(refused(repo_refusal()))) else {
            panic!("a refusal is not a result");
        };
        assert_ne!(result.is_error, Some(true));
        assert_eq!(said(Ok(result)), repo_refusal());
        // Any other error stays an error.
        let other = answered(Err(failed("backend request failed".to_string())));
        assert!(matches!(other, Err(e) if e.message == "backend request failed"));
    }

    #[test]
    fn call_refuses_a_kept_repo_reached_by_the_name_fallback() {
        let (root, data, closed, open) = scene("fallback");
        // An existing folder, not kept, with the kept repository's name.
        let namesake = root.join("other").join("closed");
        std::fs::create_dir_all(&namesake).unwrap();
        let listing = repos(&root, &closed, &open);
        let backend = StandIn::start(move |request, _| match request {
            "POST /v1/repos" => (500, json!({ "error": "cannot save repos.json" })),
            "GET /v1/repos" => (200, listing.clone()),
            _ => (200, json!({ "home": "# Home", "status": {} })),
        });
        let session = Session::start_with(&open, &data, None, Some(&backend.url));
        let bridge = Bridge::default();
        let path = namesake.to_string_lossy().into_owned();
        let args = json!({ "repo": path });
        assert_eq!(bridge.screen(args.as_object()), None);
        let answer = said(run(bridge.get_overview(overview_of(&path))));
        assert_eq!(answer, repo_refusal());
        assert!(
            backend.seen().iter().all(|r| !r.contains("overview")),
            "{:?}",
            backend.seen()
        );
        drop(session);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn call_checks_a_repository_registered_by_the_call() {
        let (root, data, closed, open) = scene("fresh");
        let listing = repos(&root, &closed, &open);
        let fresh = open.clone();
        let backend = StandIn::start(move |request, _| match request {
            "GET /v1/repos" => (200, listing.clone()),
            "POST /v1/repos" => (
                200,
                json!({ "id": "fresh-1", "name": "open", "path": fresh }),
            ),
            "GET /v1/repos/fresh-1" => (200, json!({ "id": "fresh-1", "path": fresh })),
            _ => (200, json!({ "home": "# Home", "status": {} })),
        });
        let session = Session::start_with(&open, &data, None, Some(&backend.url));
        let bridge = Bridge::default();
        let answer = said(run(
            bridge.get_overview(Parameters(RepoParam { repo: None }))
        ));
        assert!(answer.starts_with("# Home"), "{answer}");
        assert_eq!(
            backend.seen(),
            [
                "GET /v1/repos",
                "POST /v1/repos",
                "GET /v1/repos/fresh-1",
                "GET /v1/repos/fresh-1/overview"
            ]
        );
        drop(session);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cross_project_tools_leave_out_kept_and_holding_repos() {
        let (root, data, closed, open) = scene("hide");
        let listing = repos(&root, &closed, &open);
        let backend = StandIn::start(move |request, _| match request {
            "GET /v1/repos" => (200, listing.clone()),
            "GET /v1/workspace" => (
                200,
                json!({
                    "repos": [
                        { "name": "closed", "id": "closed-1", "path": "c", "files": 1 },
                        { "name": "holder", "id": "holder-1", "path": "h", "files": 2 },
                        { "name": "open", "id": "open-1", "path": "o", "files": 3 },
                    ],
                    "shared_dependencies": [
                        { "name": "serde", "repos": ["closed", "holder", "open"] },
                    ],
                }),
            ),
            "GET /v1/workspace/contracts" => (
                200,
                json!({
                    "repos": [
                        { "repo": "closed", "provided": 1, "consumed": 0 },
                        { "repo": "holder", "provided": 2, "consumed": 0 },
                        { "repo": "open", "provided": 3, "consumed": 0 },
                    ],
                    "errors": [], "matched": [], "unprovided": [], "unconsumed": [],
                }),
            ),
            _ => (404, json!({ "error": "not here" })),
        });
        let session = Session::start_with(&open, &data, None, Some(&backend.url));
        let bridge = Bridge::default();

        let listed: Value = serde_json::from_str(&said(run(bridge.list_repos()))).unwrap();
        let names: Vec<&str> = listed
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|r| r["name"].as_str())
            .collect();
        assert_eq!(names, ["open"]);

        let workspace = said(run(bridge.get_workspace()));
        assert!(workspace.starts_with("open (open-1)\n"), "{workspace}");
        assert!(
            !workspace.contains("closed") && !workspace.contains("holder"),
            "{workspace}"
        );
        assert!(!workspace.contains("serde"), "{workspace}");

        let contracts = said(run(bridge.get_contracts(Parameters(ContractsParams {
            workspace: Some(true),
            repo: None,
        }))));
        assert!(contracts.starts_with("open: 3 provided"), "{contracts}");
        assert!(
            !contracts.contains("closed") && !contracts.contains("holder"),
            "{contracts}"
        );
        drop(session);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cross_project_tools_refuse_when_the_list_is_broken() {
        let (root, data, _, open) = scene("hide-broken");
        let file = cw_core::agents_off::file(&data);
        std::fs::write(&file, "{").unwrap();
        let backend = StandIn::start(|_, _| (200, json!([])));
        let session = Session::start_with(&open, &data, None, Some(&backend.url));
        let bridge = Bridge::default();
        let unreadable = cw_core::agents_off::unreadable(&file);
        assert_eq!(said(run(bridge.list_repos())), unreadable);
        assert_eq!(said(run(bridge.get_workspace())), unreadable);
        assert!(backend.seen().is_empty(), "{:?}", backend.seen());
        drop(session);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_session_in_a_folder_that_holds_a_kept_one_is_refused() {
        let (root, data, closed, _) = scene("holder");
        let session = Session::start(&root, &data, None);
        let bridge = Bridge::default();
        assert_eq!(
            bridge.screen(None),
            Some(cw_core::agents_off::refusal(&closed))
        );
        assert!(bridge.backend.lock().unwrap().is_none());
        drop(session);
        let _ = std::fs::remove_dir_all(root);
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
        assert_eq!(
            bridge.screen(None),
            Some(cw_core::agents_off::refusal(&closed))
        );
        assert!(bridge.backend.lock().unwrap().is_none());
        drop(session);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn screen_refuses_a_covered_repo_path_without_a_backend() {
        let (root, data, closed, open) = scene("repo");
        let session = Session::start(&open, &data, None);
        let bridge = Bridge::default();
        // Below the entry, and a folder that holds it: refused for that
        // repository alone, with no path named.
        for repo in [closed.join("sub"), root.clone()] {
            let args = json!({ "repo": repo.to_string_lossy() });
            assert_eq!(
                bridge.screen(args.as_object()),
                Some(repo_refusal()),
                "{}",
                repo.display()
            );
        }
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
        assert_eq!(
            bridge.screen(None),
            Some(cw_core::agents_off::refusal(&closed))
        );
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
    fn a_note_of_something_gone_says_so_first() {
        let kept = note_text(&json!({
            "info": { "state": "gone", "gone": "history" },
            "text": "# Old Service\n\n> Mine.\n",
        }));
        assert_eq!(
            kept,
            "(What this note describes is gone from the repository; its owner keeps it as history. Do not take it for the code as it is now.)\n\n# Old Service\n\n> Mine.\n"
        );
        let live = note_text(&json!({
            "info": { "state": "generated" },
            "text": "# Scan Module\n",
        }));
        assert_eq!(live, "# Scan Module\n");
    }

    #[test]
    fn a_sync_says_what_became_of_notes_of_code_that_is_gone() {
        let text = sync_text(&json!({
            "written": [], "kept": 4, "proposed": [], "removed": ["01-Core/a/Old.md"],
            "adopted": [], "notes_total": 4,
            "summary": [
                "Removed 1 note whose subject is gone: `01-Core/a/Old.md`.",
                "2 notes written by hand name only files that are gone."
            ]
        }));
        assert!(
            text.ends_with(
                "Total notes: 4. Removed 1 note whose subject is gone: `01-Core/a/Old.md`. 2 notes written by hand name only files that are gone."
            ),
            "{text}"
        );
        // A backend from before the lines existed.
        let old = sync_text(&json!({
            "written": [], "kept": 4, "proposed": [], "removed": [], "adopted": [], "notes_total": 4
        }));
        assert!(old.ends_with("Total notes: 4."), "{old}");
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
        let body = waiting_text(
            &json!({
                "waiting": { "behind": 1, "unexplained": 7 },
                "matching": 3,
                "notes": [
                    { "path": "01-Core/app/Tray Module.md", "why": "behind",
                      "behind": ["src/tray.rs changed"], "sources": ["src/tray.rs"] },
                    { "path": "01-Core/app/Autostart Module.md", "why": "unexplained",
                      "sources": ["src/autostart.rs", "src/main.rs"] },
                ],
            }),
            false,
        );
        assert!(
            body.starts_with(
                "8 notes wait for an explanation: 1 behind the code, 7 never explained.\n3 fit what you asked for; the first 2:\n\n1. 01-Core/app/Tray Module.md  (behind the code)\n   - src/tray.rs changed\n   about: src/tray.rs\n2. 01-Core/app/Autostart Module.md  (never explained)\n   about: src/autostart.rs, src/main.rs\n"
            ),
            "{body}"
        );
        assert!(body.contains("call get_note_brief"), "{body}");
        assert_eq!(
            waiting_text(
                &json!({ "waiting": { "behind": 0, "unexplained": 0 }, "notes": [] }),
                false
            ),
            "No note waits for an explanation.\n"
        );
        let none = waiting_text(
            &json!({
                "waiting": { "behind": 0, "unexplained": 2 }, "matching": 0, "notes": [],
            }),
            false,
        );
        assert!(
            none.ends_with("None of them fits what you asked for.\n"),
            "{none}"
        );
    }

    #[test]
    fn the_queue_tells_about_claimed_notes() {
        let queue = json!({
            "waiting": { "behind": 0, "unexplained": 4 },
            "matching": 2,
            "claimed": 2,
            "notes": [
                { "path": "01-Core/a.md", "why": "unexplained", "sources": ["a.rs"] },
            ],
        });
        let plain = waiting_text(&queue, false);
        assert!(
            plain.contains("2 more are being written by another agent.\n"),
            "{plain}"
        );
        assert!(!plain.contains("kept for you"), "{plain}");
        let kept = waiting_text(&queue, true);
        let minutes = format!(
            "These notes are kept for you for {} minutes.\n",
            cw_core::CLAIM_MINUTES
        );
        assert!(kept.contains(&minutes), "{kept}");
        let none = waiting_text(
            &json!({ "waiting": { "behind": 0, "unexplained": 4 }, "notes": [] }),
            false,
        );
        assert!(!none.contains("another agent"), "{none}");
        // Every note that fits is held: none is free, though some fit.
        let busy = waiting_text(
            &json!({
                "waiting": { "behind": 0, "unexplained": 4 }, "matching": 3,
                "claimed": 3, "notes": [],
            }),
            true,
        );
        assert!(
            busy.ends_with(
                "None is free now: another agent is writing the 3 that fit what you asked for.\n"
            ),
            "{busy}"
        );
        assert!(!busy.contains("None of them fits"), "{busy}");
        assert!(!busy.contains("kept for you"), "{busy}");
    }

    #[test]
    fn the_agent_id_is_made_once() {
        let id = agent_id();
        assert!(!id.is_empty());
        assert!(id.starts_with(&format!("{}-", std::process::id())));
        assert_eq!(id, agent_id());
    }

    #[test]
    fn each_claiming_listing_has_an_id_of_its_own() {
        let claims = ClaimIds::default();
        let (first, second) = (claims.next(), claims.next());
        assert_ne!(first, second);
        for id in [&first, &second] {
            assert!(id.starts_with(&format!("{}-", agent_id())), "{id}");
            assert!(id.chars().count() <= 64, "{id}");
        }
        let tray = "01-Core/app/Tray Module.md";
        let autostart = "01-Core/app/Autostart Module.md";
        claims.hold("r1", &first, &json!({ "notes": [{ "path": tray }] }));
        claims.hold("r1", &second, &json!({ "notes": [{ "path": autostart }] }));
        // A listed note goes with its listing's id, named by path or by name.
        assert_eq!(claims.agent_for("r1", tray), first);
        assert_eq!(claims.agent_for("r1", "Tray Module"), first);
        assert_eq!(claims.agent_for("r1", "autostart module.md"), second);
        // A note not claimed here, or in another repository, goes with the
        // bridge's own id.
        assert_eq!(claims.agent_for("r1", "01-Core/Other.md"), agent_id());
        assert_eq!(claims.agent_for("r2", tray), agent_id());
    }

    #[test]
    fn a_saved_or_proposed_note_is_forgotten_and_a_fault_keeps_it() {
        let claims = ClaimIds::default();
        let id = claims.next();
        let tray = "01-Core/app/Tray Module.md";
        let autostart = "01-Core/app/Autostart Module.md";
        claims.hold(
            "r1",
            &id,
            &json!({ "notes": [{ "path": tray }, { "path": autostart }] }),
        );
        claims.settle(
            "r1",
            "Tray Module",
            &json!({ "outcome": "issues", "issues": ["Items is not yours"] }),
        );
        assert_eq!(claims.agent_for("r1", tray), id);
        claims.settle(
            "r1",
            "Tray Module",
            &json!({ "outcome": "saved", "path": tray }),
        );
        assert_eq!(claims.agent_for("r1", tray), agent_id());
        claims.settle(
            "r1",
            autostart,
            &json!({ "outcome": "proposed", "path": autostart }),
        );
        assert_eq!(claims.agent_for("r1", autostart), agent_id());
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
