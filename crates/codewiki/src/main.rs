//! CodeWiki command line.

mod serve;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use cw_core::{Result, VAULT_DIR};
use cw_vault::{VaultMeta, proposal};
use cw_wiki::service;

#[derive(Parser)]
#[command(
    name = "codewiki",
    version,
    about = "Generate and maintain a codebase wiki"
)]
struct Cli {
    /// Repository root. Defaults to the current directory.
    #[arg(long, global = true, default_value = ".")]
    repo: PathBuf,
    /// Print machine-readable JSON instead of text.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Index the repository and generate the vault for the first time.
    Init,
    /// Re-index and regenerate notes affected by source changes.
    Update,
    /// Rebuild the index without touching notes.
    Reindex,
    /// Show index and vault state.
    Status,
    /// Full-text search across notes, symbols and files.
    Search {
        query: String,
        /// Restrict to `note`, `symbol` or `file`.
        #[arg(long)]
        kind: Option<String>,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// Look up symbols by name.
    Symbol { name: String },
    /// List decision records, or create one with `--new <title>`.
    Decisions {
        /// Create a decision note from the template with this title.
        #[arg(long)]
        new: Option<String>,
    },
    /// Why a file or symbol is the way it is: decisions, recorded rationale, recent commits.
    Why { target: String },
    /// Write a short project briefing into CLAUDE.md and AGENTS.md, between
    /// marker comments. Text outside the markers is never changed.
    AgentDocs,
    /// Run every analysis and report findings for CI. Exits with status 1
    /// when a finding is at or above `--fail-on`.
    Check {
        /// Output format: `text`, `sarif` or `junit`.
        #[arg(long, default_value = "text")]
        format: String,
        /// Lowest level that fails the check: `note`, `warning`, `error` or `never`.
        #[arg(long, default_value = "error")]
        fail_on: String,
        /// Write the report to this file instead of standard output.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Symbols nothing refers to, most certain first.
    DeadCode,
    /// Where notes no longer match the code, and documentation coverage.
    DocDrift,
    /// Health score per source file, worst first.
    Health,
    /// Files that are both large and frequently changed.
    Hotspots,
    /// Risk of changing a file, or of the uncommitted changes when no file is given.
    Risk { file: Option<String> },
    /// Tests per file, and source files no test reaches.
    Tests,
    /// Tests to run after changing the given files, or after the uncommitted
    /// changes when no file is given.
    ImpactedTests { files: Vec<String> },
    /// Import an LCOV or Cobertura coverage report and rank the least
    /// covered files by how often they change and how much depends on them.
    Coverage {
        /// Report file. Defaults to `lcov.info`, `coverage.xml` and similar
        /// names in the repository, then to the last imported report.
        #[arg(long)]
        report: Option<PathBuf>,
        /// Number of gaps to list.
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Security pattern findings in Rust source, test code excluded.
    Security {
        /// Lowest level to show: `note`, `warning` or `error`.
        #[arg(long, default_value = "note")]
        level: String,
    },
    /// Refactoring suggestions with evidence and ordered steps.
    Refactor {
        /// Number of suggestions to show.
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    /// HTTP endpoints this repository provides and the URL paths it calls.
    Contracts {
        /// Match the contracts of every registered repository against each
        /// other instead of listing this repository's.
        #[arg(long)]
        workspace: bool,
    },
    /// Every registered repository at a glance, and the external packages
    /// two or more of them use.
    Workspace,
    /// Print context for a coding agent's lifecycle hook. Prints nothing and
    /// still exits with status 0 when anything goes wrong.
    Hook {
        #[command(subcommand)]
        event: HookEvent,
    },
    /// List proposals waiting for review.
    Proposals,
    /// Apply a proposal to its note.
    Approve { id: String },
    /// Discard a proposal; the same change is not proposed again.
    Reject { id: String },
    /// Send a test error report to Sentry, or with `--panic` panic, to check
    /// that reports arrive. Reports must be on: `SENTRY_DSN` set, or a build
    /// that carries a DSN with "Send error reports" turned on.
    #[command(hide = true)]
    SentryTest {
        /// Panic instead of logging an error.
        #[arg(long)]
        panic: bool,
    },
    /// Run the backend HTTP server used by the desktop app and agents.
    Serve {
        /// Address to listen on. Anything but loopback requires CODEWIKI_TOKEN.
        #[arg(long, default_value = "127.0.0.1")]
        host: std::net::IpAddr,
        /// Port to listen on; 0 picks a free port.
        #[arg(long, default_value_t = 0)]
        port: u16,
    },
    /// Keep coding agents out of chosen folders: CodeWiki's hook and tools stay
    /// silent there.
    Agents {
        #[command(subcommand)]
        action: AgentsAction,
    },
}

#[derive(Subcommand)]
enum AgentsAction {
    /// Keep coding agents out of a folder (default: the current folder).
    Off { folder: Option<PathBuf> },
    /// Let coding agents into a folder again (default: the current folder).
    On { folder: Option<PathBuf> },
    /// List the folders kept from coding agents.
    List,
    /// Say whether a folder is kept from coding agents (default: the current
    /// folder).
    Status { folder: Option<PathBuf> },
}

/// Run `codewiki agents`. A list that cannot be read is reported, never
/// overwritten, and every action then fails.
fn agents(action: AgentsAction, json: bool) -> ExitCode {
    use cw_core::agents_off::{self, Added, Block, Removed};
    let data = cw_core::paths::data_dir();
    let broken = |_: cw_core::Error| {
        eprintln!("{}", agents_off::unreadable(&agents_off::file(&data)));
        ExitCode::FAILURE
    };
    // The list was read; writing it failed.
    let unwritten = |e: cw_core::Error| {
        eprintln!("could not change the list of folders kept from coding agents: {e}");
        ExitCode::FAILURE
    };
    let here = || std::env::current_dir().unwrap_or_default();
    let target = |folder: Option<PathBuf>| match folder {
        Some(folder) => agents_off::normalize(&folder, &here()),
        None => agents_off::normalize(&here(), &here()),
    };
    // A change reads the list first, so that only a list that cannot be read
    // is reported as one.
    if matches!(action, AgentsAction::Off { .. } | AgentsAction::On { .. })
        && let Err(e) = agents_off::load(&data)
    {
        return broken(e);
    }
    match action {
        AgentsAction::Off { folder } => match agents_off::add(&data, &target(folder)) {
            Ok(Added::Added(folder)) => {
                println!("kept from coding agents: {}", folder.display());
                ExitCode::SUCCESS
            }
            Ok(Added::CoveredBy(entry)) => {
                println!("already kept from coding agents by {}", entry.display());
                ExitCode::SUCCESS
            }
            Err(e) => unwritten(e),
        },
        AgentsAction::On { folder } => match agents_off::remove(&data, &target(folder.clone())) {
            Ok(Removed::Removed(entry)) => {
                // A parent entry may cover the folder too.
                let rest = agents_off::load(&data).unwrap_or_default();
                if let Some(parent) = agents_off::covering(&rest, &target(folder)) {
                    println!("still kept from coding agents by {}", parent.display());
                    return ExitCode::FAILURE;
                }
                println!("coding agents allowed again: {}", entry.display());
                ExitCode::SUCCESS
            }
            Ok(Removed::NotListed) => {
                println!("not listed: {}", target(folder).display());
                ExitCode::SUCCESS
            }
            Ok(Removed::CoveredBy(entry)) => {
                println!(
                    "kept from coding agents by {}; remove that entry to let agents in",
                    entry.display()
                );
                ExitCode::FAILURE
            }
            Err(e) => unwritten(e),
        },
        AgentsAction::List => match agents_off::load(&data) {
            Ok(entries) => {
                if json {
                    println!("{}", serde_json::json!({ "folders": entries }));
                } else if entries.is_empty() {
                    println!("no folder is kept from coding agents");
                } else {
                    for entry in entries {
                        println!("{}", entry.display());
                    }
                }
                ExitCode::SUCCESS
            }
            Err(e) => broken(e),
        },
        AgentsAction::Status { folder } => match agents_off::load(&data) {
            Ok(entries) => {
                let block = agents_off::blocking(&entries, &target(folder));
                if json {
                    let (covered_by, contains) = match &block {
                        Some(Block::Covered(entry)) => (Some(entry), None),
                        Some(Block::Contains(entry)) => (None, Some(entry)),
                        None => (None, None),
                    };
                    println!(
                        "{}",
                        serde_json::json!({ "covered_by": covered_by, "contains": contains })
                    );
                } else {
                    match &block {
                        Some(Block::Covered(entry)) => {
                            println!("kept from coding agents by {}", entry.display());
                        }
                        Some(Block::Contains(entry)) => {
                            println!("kept from coding agents: contains {}", entry.display());
                        }
                        None => println!("open to coding agents"),
                    }
                }
                ExitCode::SUCCESS
            }
            Err(e) => broken(e),
        },
    }
}

#[derive(Subcommand)]
enum HookEvent {
    /// Briefing for an agent starting a session in this repository: whether
    /// a wiki exists, whether it is stale, and which tools read it.
    SessionStart {
        /// The agent host that runs the hook.
        #[arg(long, value_enum, default_value = "claude-code")]
        agent: HookAgent,
    },
}

/// The agent host that runs a hook.
#[derive(Clone, Copy, clap::ValueEnum)]
enum HookAgent {
    ClaudeCode,
    Codex,
}

/// The JSON an agent host pipes to a hook, or `None` when there is none:
/// stdin is a terminal, empty, not JSON, or does not arrive within a second.
/// The read runs on a helper thread so a host that keeps stdin open cannot
/// hold the session start.
fn hook_input() -> Option<serde_json::Value> {
    use std::io::{IsTerminal, Read};
    if std::io::stdin().is_terminal() {
        return None;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = std::io::stdin().take(64 * 1024).read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    let buf = rx.recv_timeout(std::time::Duration::from_secs(1)).ok()?;
    serde_json::from_slice(&buf).ok()
}

/// Print the session-start briefing for the repository at `repo`. Every
/// failure is swallowed: a hook must never break the agent's session.
fn session_start_hook(repo: &std::path::Path, agent: HookAgent) {
    // The agent host runs hooks from wherever the session started; it names
    // the project folder in the environment.
    let start = if repo == std::path::Path::new(".") {
        std::env::var_os("CLAUDE_PROJECT_DIR")
            .filter(|dir| !dir.is_empty())
            .map_or_else(|| repo.to_path_buf(), PathBuf::from)
    } else {
        repo.to_path_buf()
    };
    let Ok(start) = start.canonicalize() else {
        return;
    };
    let root = cw_wiki::briefing::vault_root(&start).unwrap_or_else(|| start.clone());
    // A folder closed to coding agents gets no briefing, nor one that holds
    // a closed folder, and neither does any run where the list cannot be
    // read: when in doubt, say nothing.
    let data = cw_core::paths::data_dir();
    match cw_core::agents_off::load(&data) {
        Err(_) => return,
        Ok(entries) => {
            if cw_core::agents_off::blocking(&entries, &start).is_some()
                || cw_core::agents_off::blocking(&entries, &root).is_some()
            {
                return;
            }
        }
    }
    use cw_wiki::briefing::{Explain, Host, explain_for};
    let host = match agent {
        HookAgent::ClaudeCode => Host::ClaudeCode,
        HookAgent::Codex => Host::Codex,
    };
    let auto_on = cw_core::auto_explain::load(&data);
    // What the host pipes in only decides whether to order a batch: with
    // auto explaining off, stdin is not read at all.
    let input = if auto_on {
        hook_input().unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::Value::Null
    };
    let source = input["source"].as_str();
    // Claude Code may leave `permission_mode` out; the order itself tells the
    // agent to skip it in plan mode.
    let plan = input["permission_mode"] == "plan";
    // Only a session that would be ordered needs the git call. The session's
    // own folder is asked first: a worktree inside the main checkout, with
    // no vault of its own, finds the main checkout's vault as its root.
    let count = cw_core::auto_explain::load_count(&data);
    let mut explain = explain_for(host, auto_on, count, source, plan, false);
    if explain != Explain::Facts
        && (cw_wiki::git::is_linked_worktree(&start)
            || (root != start && cw_wiki::git::is_linked_worktree(&root)))
    {
        explain = Explain::Facts;
    }
    if let Ok(text) = cw_wiki::briefing::session_start(&root, explain) {
        print!("{text}");
    }
}

fn print_report(report: &cw_wiki::Report, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(report).unwrap_or_default()
        );
        return Ok(());
    }
    println!("notes: {} total", report.notes_total);
    println!("written: {}", report.written.len());
    println!("kept: {}", report.kept);
    println!("proposed: {}", report.proposed.len());
    println!("removed: {}", report.removed.len());
    println!("adopted: {}", report.adopted.len());
    println!("lint warnings: {}", report.lint_warnings);
    println!("links resolved: {:.1}%", report.link_rate * 100.0);
    // CodeWiki writes a note's facts; a coding agent explains it.
    println!("facts refreshed, explanation kept: {}", report.merged.len());
    println!("explanation may be behind: {}", report.behind.len());
    println!("not explained yet: {}", report.unexplained);
    for path in &report.proposed {
        println!("  proposal queued for {path}");
    }
    for path in &report.behind {
        println!("  explanation may be behind in {path}");
    }
    if !report.behind.is_empty() || report.unexplained > 0 {
        println!(
            "A coding agent writes the explanations: in Claude Code with the CodeWiki plugin, run /codewiki:write-notes."
        );
    }
    Ok(())
}

/// Print a result encoded with `serde_json::to_string_pretty`.
fn print_json(encoded: serde_json::Result<String>) -> Result<()> {
    let text = encoded.map_err(|e| cw_core::Error::external("encoding the result", e))?;
    println!("{text}");
    Ok(())
}

fn run(cli: Cli) -> Result<()> {
    if let Command::Serve { host, port } = cli.command {
        return serve::run(host, port);
    }
    let root = cli
        .repo
        .canonicalize()
        .map_err(|e| cw_core::Error::io(&cli.repo, e))?;
    let vault = root.join(VAULT_DIR);
    // The settings the backend keeps apply here too, under the environment:
    // an embedder chosen in the app is the one this program uses.
    let settings = cw_core::settings::Resolved::load(&cw_core::paths::data_dir());
    cw_wiki::semantic::configure(&settings);
    match cli.command {
        Command::Init | Command::Update => {
            let report = service::update(&root)?;
            print_report(&report, cli.json)
        }
        Command::Reindex => {
            let mut index = service::open_index(&root)?;
            let facts = service::reindex(&root, &mut index)?;
            println!(
                "indexed {} files, {} symbols, {} edges",
                facts.files.len(),
                facts.symbols.len(),
                facts.edges.len()
            );
            if !facts.files_with_errors.is_empty() {
                println!("{} files had syntax errors", facts.files_with_errors.len());
            }
            Ok(())
        }
        Command::Status => {
            let index = service::open_index(&root)?;
            let meta = VaultMeta::load(&vault)?;
            let pending = proposal::list(&vault)?
                .iter()
                .filter(|p| p.status == proposal::ProposalStatus::Pending)
                .count();
            // Canonical Windows paths carry a `\\?\` prefix nobody types.
            let shown = root.display().to_string();
            println!(
                "repository: {}",
                shown.strip_prefix(r"\\?\").unwrap_or(&shown)
            );
            println!("files indexed: {}", index.files()?.len());
            println!("symbols indexed: {}", index.symbols()?.len());
            println!("notes: {}", meta.notes.len());
            println!("pending proposals: {pending}");
            println!(
                "vault commit: {}",
                meta.manifest
                    .last_indexed_commit
                    .as_deref()
                    .unwrap_or("none")
            );
            println!("stale: {}", service::is_stale(&root)?);
            Ok(())
        }
        Command::Search { query, kind, limit } => {
            let index = service::open_index(&root)?;
            let hits = index.search(&query, kind.as_deref(), limit)?;
            if cli.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&hits).unwrap_or_default()
                );
            } else {
                for hit in hits {
                    println!(
                        "[{}] {} — {}",
                        hit.kind,
                        hit.key,
                        hit.snippet.replace('\n', " ")
                    );
                }
            }
            Ok(())
        }
        Command::Symbol { name } => {
            let index = service::open_index(&root)?;
            for s in index.find_symbols(&name, 20)? {
                println!(
                    "{} ({}) {}:{}",
                    s.qualified,
                    s.kind.as_str(),
                    s.file,
                    s.start_line
                );
                println!("    {}", s.signature);
            }
            Ok(())
        }
        Command::Decisions { new } => {
            if let Some(title) = new {
                let path = cw_wiki::decisions::create(&root, &title)?;
                println!("created {VAULT_DIR}/{path}");
                return Ok(());
            }
            let index = service::open_index(&root)?;
            for d in cw_wiki::decisions::list(&root, &index)? {
                let flag = if d.stale { "  STALE" } else { "" };
                println!("[{}]{flag}  {} — {}", d.status, d.path, d.title);
                for reason in &d.stale_reasons {
                    println!("      {reason}");
                }
            }
            Ok(())
        }
        Command::Why { target } => {
            let index = service::open_index(&root)?;
            let why = cw_wiki::decisions::why(&root, &index, &target)?;
            println!("{}", why.file);
            for d in &why.decisions {
                println!("  decision [{}] {} — {}", d.status, d.path, d.title);
            }
            for r in &why.rationale {
                println!("  {} ({}):", r.section, r.note);
                for line in r.text.lines() {
                    println!("    {line}");
                }
            }
            for c in &why.commits {
                println!("  {} {} {} — {}", c.hash, c.date, c.author, c.subject);
            }
            Ok(())
        }
        Command::AgentDocs => {
            let index = service::open_index(&root)?;
            let changed = cw_wiki::agentdocs::apply(&root, &index)?;
            if changed.is_empty() {
                println!("briefing already up to date");
            }
            for file in changed {
                println!("updated {file}");
            }
            Ok(())
        }
        Command::Check {
            format,
            fail_on,
            output,
        } => {
            let threshold = match fail_on.as_str() {
                "never" => None,
                other => Some(cw_wiki::ci::Level::parse(other).ok_or_else(|| {
                    cw_core::Error::invalid(format!(
                        "unknown --fail-on value '{other}'; use note, warning, error or never"
                    ))
                })?),
            };
            let index = service::open_index(&root)?;
            let findings = cw_wiki::ci::check(&root, &index)?;
            let gate = threshold.unwrap_or(cw_wiki::ci::Level::Error);
            let report = match format.as_str() {
                "text" => cw_wiki::ci::to_text(&findings),
                "sarif" => serde_json::to_string_pretty(&cw_wiki::ci::to_sarif(&findings))
                    .unwrap_or_default(),
                "junit" => cw_wiki::ci::to_junit(&findings, gate),
                other => {
                    return Err(cw_core::Error::invalid(format!(
                        "unknown --format value '{other}'; use text, sarif or junit"
                    )));
                }
            };
            match output {
                Some(path) => {
                    std::fs::write(&path, report).map_err(|e| cw_core::Error::io(&path, e))?
                }
                None => print!("{report}"),
            }
            if threshold.is_some_and(|level| cw_wiki::ci::fails(&findings, level)) {
                return Err(cw_core::Error::invalid(format!(
                    "check failed: findings at or above '{fail_on}'"
                )));
            }
            Ok(())
        }
        Command::DeadCode => {
            let index = service::open_index(&root)?;
            for d in cw_wiki::health::dead_code(&index)? {
                println!(
                    "{:?}  {}:{}  {} ({}, {} lines) — {}",
                    d.certainty, d.file, d.line, d.qualified, d.kind, d.lines, d.reason
                );
            }
            Ok(())
        }
        Command::DocDrift => {
            let index = service::open_index(&root)?;
            let report = cw_wiki::health::doc_drift(&root, &index)?;
            println!(
                "coverage: {:.1}% ({} of {} source files cited by a note)",
                report.coverage * 100.0,
                report.documented_files,
                report.source_files
            );
            for d in &report.drift {
                println!("{:?}  {} — {}", d.kind, d.note, d.detail);
            }
            for path in &report.undocumented {
                println!("undocumented  {path}");
            }
            Ok(())
        }
        Command::Health => {
            let index = service::open_index(&root)?;
            for h in cw_wiki::health::file_health(&index)? {
                if h.findings.is_empty() {
                    continue;
                }
                println!("{:>2}/10  {}", h.score, h.path);
                for f in &h.findings {
                    println!("        [{}] {}", f.detector, f.message);
                }
            }
            Ok(())
        }
        Command::Hotspots => {
            let index = service::open_index(&root)?;
            let intel = cw_wiki::gitintel::load(&index)?;
            for h in cw_wiki::gitintel::hotspots(&index, &intel, 25)? {
                println!(
                    "{:>5.1}  {:>4} commits  {:>5} lines  bus {}  {}",
                    h.score, h.commits, h.lines, h.bus_factor, h.path
                );
            }
            Ok(())
        }
        Command::Risk { file } => {
            let index = service::open_index(&root)?;
            let intel = cw_wiki::gitintel::load(&index)?;
            match file {
                Some(file) => {
                    let risk =
                        cw_wiki::gitintel::file_risk(&index, &intel, &file.replace('\\', "/"))?;
                    println!("{} — {:?} ({}/100)", risk.path, risk.level, risk.score);
                    for reason in &risk.reasons {
                        println!("  - {reason}");
                    }
                }
                None => {
                    let changed = cw_wiki::gitintel::working_tree_changes(&root);
                    let risk = cw_wiki::gitintel::change_risk(&index, &intel, &changed)?;
                    println!(
                        "{} changed files — {:?} ({}/100)",
                        changed.len(),
                        risk.level,
                        risk.score
                    );
                    for partner in &risk.missing_partners {
                        println!(
                            "  usually changed together, not in this change: {} ({} times)",
                            partner.path, partner.together
                        );
                    }
                    println!(
                        "  {} files outside the change depend on it",
                        risk.affected.len()
                    );
                }
            }
            Ok(())
        }
        Command::Tests => {
            let index = service::open_index(&root)?;
            let map = cw_wiki::testmap::test_map(&root, &index)?;
            if cli.json {
                return print_json(serde_json::to_string_pretty(&map));
            }
            println!(
                "{} tests in {} files",
                map.tests_total,
                map.test_files.len()
            );
            for file in &map.files {
                let reached = file.direct.len() + file.indirect.len();
                if file.unit_tests + reached > 0 {
                    println!(
                        "{:>3} unit  {:>3} direct  {:>3} indirect  {}",
                        file.unit_tests,
                        file.direct.len(),
                        file.indirect.len(),
                        file.path
                    );
                }
            }
            for path in &map.untested {
                println!("untested  {path}");
            }
            Ok(())
        }
        Command::ImpactedTests { files } => {
            let index = service::open_index(&root)?;
            let changed = if files.is_empty() {
                cw_wiki::gitintel::working_tree_changes(&root)
            } else {
                files
            };
            let impact = cw_wiki::testmap::impacted_tests(&root, &index, &changed)?;
            if cli.json {
                return print_json(serde_json::to_string_pretty(&impact));
            }
            println!(
                "{} changed files reach {} tests in {} files",
                impact.changed.len(),
                impact.tests_total,
                impact.test_files.len()
            );
            for file in &impact.test_files {
                println!(
                    "  {} hops  {} ({} tests)",
                    file.distance,
                    file.path,
                    file.tests.len()
                );
            }
            for path in &impact.untested_changes {
                println!("  no test reaches {path}");
            }
            for command in &impact.commands {
                println!("{command}");
            }
            Ok(())
        }
        Command::Coverage { report, limit } => {
            let index = service::open_index(&root)?;
            let fresh = match &report {
                Some(path) => {
                    // A path is relative to where the command runs, not to `--repo`.
                    let path = std::path::absolute(path).unwrap_or_else(|_| path.clone());
                    Some(cw_wiki::coverage::import(&root, &index, Some(&path))?)
                }
                None => match cw_wiki::coverage::import(&root, &index, None) {
                    Ok(data) => Some(data),
                    Err(cw_core::Error::NotFound(_)) => None,
                    Err(e) => return Err(e),
                },
            };
            let data = match fresh {
                Some(data) => {
                    cw_wiki::coverage::save(&index, &data)?;
                    data
                }
                None => cw_wiki::coverage::load(&index)?.ok_or_else(|| {
                    cw_core::Error::not_found(
                        "no coverage report; pass --report <lcov.info or cobertura.xml>",
                    )
                })?,
            };
            let summary = cw_wiki::coverage::report(&index, &data, limit)?;
            if cli.json {
                return print_json(serde_json::to_string_pretty(&summary));
            }
            println!(
                "coverage: {:.1}% ({} of {} lines in {} files, from {})",
                summary.percent,
                summary.lines_hit,
                summary.lines_found,
                summary.files.len(),
                summary.source
            );
            for gap in &summary.least_covered {
                println!(
                    "{:>5.1}  {:>5.1}% covered  {:>5} lines uncovered  {}",
                    gap.score, gap.percent, gap.uncovered_lines, gap.path
                );
            }
            for path in &summary.not_in_report {
                println!("not in report  {path}");
            }
            Ok(())
        }
        Command::Security { level } => {
            let floor = cw_wiki::ci::Level::parse(&level).ok_or_else(|| {
                cw_core::Error::invalid(format!(
                    "unknown --level value '{level}'; use note, warning or error"
                ))
            })?;
            let index = service::open_index(&root)?;
            let mut findings = cw_wiki::security::scan(&root, &index)?;
            findings.retain(|f| f.level >= floor);
            if cli.json {
                return print_json(serde_json::to_string_pretty(&findings));
            }
            for f in &findings {
                println!(
                    "{}: {}:{}: [{}] {}",
                    f.level.as_str(),
                    f.file,
                    f.line,
                    f.rule,
                    f.message
                );
            }
            println!("{} findings", findings.len());
            Ok(())
        }
        Command::Refactor { limit } => {
            let index = service::open_index(&root)?;
            let mut plan = cw_wiki::refactor::plan(&index)?;
            plan.truncate(limit);
            if cli.json {
                return print_json(serde_json::to_string_pretty(&plan));
            }
            for (number, s) in plan.iter().enumerate() {
                println!("{}. {} (priority {})", number + 1, s.title, s.priority);
                for line in &s.evidence {
                    println!("     - {line}");
                }
                for (step, line) in s.steps.iter().enumerate() {
                    println!("     {}. {line}", step + 1);
                }
            }
            if plan.is_empty() {
                println!("nothing to suggest");
            }
            Ok(())
        }
        Command::Contracts { workspace: true } => {
            let all = cw_server::workspace::workspace_contracts(&cw_core::paths::data_dir())?;
            if cli.json {
                return print_json(serde_json::to_string_pretty(&all));
            }
            for repo in &all.contracts.repos {
                println!(
                    "{}: {} provided, {} consumed",
                    repo.repo, repo.provided, repo.consumed
                );
            }
            for failed in &all.errors {
                println!("{}: not read: {}", failed.repo, failed.error);
            }
            println!("\nmatched ({})", all.contracts.matched.len());
            for link in &all.contracts.matched {
                let mut servers: Vec<&str> =
                    link.providers.iter().map(|p| p.repo.as_str()).collect();
                servers.dedup();
                println!(
                    "  {}  {}:{}:{}  <- {} ({} routes)",
                    link.path,
                    link.repo,
                    link.file,
                    link.line,
                    servers.join(", "),
                    link.providers.len()
                );
            }
            println!(
                "\nconsumed, provided by no repository ({})",
                all.contracts.unprovided.len()
            );
            for call in &all.contracts.unprovided {
                println!("  {}  {}:{}:{}", call.path, call.repo, call.file, call.line);
            }
            println!(
                "\nprovided, consumed by nothing ({})",
                all.contracts.unconsumed.len()
            );
            for route in &all.contracts.unconsumed {
                println!(
                    "  {:<7} {}  {}:{}:{}",
                    route.method, route.path, route.repo, route.file, route.line
                );
            }
            Ok(())
        }
        Command::Contracts { workspace: false } => {
            let index = service::open_index(&root)?;
            let found = cw_wiki::contracts::scan(&root, &index)?;
            if cli.json {
                return print_json(serde_json::to_string_pretty(&found));
            }
            println!("provided ({})", found.provided.len());
            for p in &found.provided {
                println!("  {:<7} {}  {}:{}", p.method, p.path, p.file, p.line);
            }
            println!("consumed ({})", found.consumed.len());
            for c in &found.consumed {
                println!("  {}  {}:{}", c.path, c.file, c.line);
            }
            Ok(())
        }
        Command::Workspace => {
            let view = cw_server::workspace::workspace(&cw_core::paths::data_dir())?;
            if cli.json {
                return print_json(serde_json::to_string_pretty(&view));
            }
            for repo in &view.repos {
                println!("{}  {}", repo.name, repo.path);
                if let Some(error) = &repo.error {
                    println!("  error: {error}");
                    continue;
                }
                println!(
                    "  {} files, {} symbols, {} notes, {} pending proposals{}",
                    repo.files,
                    repo.symbols,
                    repo.notes,
                    repo.pending_proposals,
                    if repo.stale { ", stale" } else { "" }
                );
                for h in &repo.hotspots {
                    println!(
                        "  hotspot {:>5.1}  {} ({} commits, {} lines)",
                        h.score, h.path, h.commits, h.lines
                    );
                }
            }
            if view.repos.is_empty() {
                println!("no repository is registered with the backend");
            }
            if !view.shared_dependencies.is_empty() {
                println!("\nshared dependencies");
            }
            for dep in &view.shared_dependencies {
                println!("  {}  {}", dep.name, dep.repos.join(", "));
            }
            Ok(())
        }
        Command::Hook { .. } => unreachable!("handled before the repository is resolved"),
        Command::Proposals => {
            for p in proposal::list(&vault)? {
                println!("{} [{:?}] {} — {}", p.id, p.status, p.note_path, p.reason);
            }
            Ok(())
        }
        Command::Approve { id } => {
            let p = proposal::approve(&vault, &id)?;
            let mut meta = VaultMeta::load(&vault)?;
            if let Some(entry) = meta.notes.get_mut(&p.note_id) {
                entry.input_hash = p.input_hash.clone();
                meta.save(&vault)?;
            }
            println!("applied {} to {}", p.id, p.note_path);
            Ok(())
        }
        Command::Reject { id } => {
            let p = proposal::reject(&vault, &id)?;
            let mut meta = VaultMeta::load(&vault)?;
            if let Some(entry) = meta.notes.get_mut(&p.note_id) {
                entry.rejected_inputs.push(p.input_hash.clone());
                meta.save(&vault)?;
            }
            println!("rejected {}", p.id);
            Ok(())
        }
        Command::Serve { .. } | Command::SentryTest { .. } | Command::Agents { .. } => {
            unreachable!("handled before the repository is resolved")
        }
    }
}

/// `codewiki sentry-test`: one report through the same path an error or a
/// panic of the backend takes.
fn sentry_test(panic: bool) -> ExitCode {
    let _telemetry = cw_telemetry::init("sentry-test", log::LevelFilter::Info);
    if !cw_telemetry::sending() {
        eprintln!(
            "error: error reports are off; set {} or turn on \"Send error reports\" in a build that carries a DSN",
            cw_telemetry::DSN_ENV
        );
        return ExitCode::FAILURE;
    }
    log::info!(
        "a breadcrumb before the test report, from {}",
        cw_core::paths::data_dir().display()
    );
    if panic {
        panic!("test panic, sent on request");
    }
    cw_telemetry::send_test_report("codewiki sentry-test");
    println!("sent a test report");
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        // A hook stays silent and succeeds even when its arguments are wrong;
        // `--help` and `--version` still print.
        Err(e) if e.use_stderr() && std::env::args().any(|arg| arg == "hook") => {
            return ExitCode::SUCCESS;
        }
        Err(e) => e.exit(),
    };
    if let Command::Hook {
        event: HookEvent::SessionStart { agent },
    } = cli.command
    {
        // A panic must not reach the agent either, as text or as a status.
        std::panic::set_hook(Box::new(|_| {}));
        let _ = std::panic::catch_unwind(|| session_start_hook(&cli.repo, agent));
        return ExitCode::SUCCESS;
    }
    if let Command::SentryTest { panic } = cli.command {
        return sentry_test(panic);
    }
    if let Command::Agents { action } = cli.command {
        return agents(action, cli.json);
    }
    // The backend has no console when the app or an agent starts it: it keeps
    // `backend.log`, and prints the same lines when run from a terminal. A
    // command prints only warnings and errors, as it always has. Only the
    // backend sends error reports.
    let serving = matches!(cli.command, Command::Serve { .. });
    let _telemetry = if serving {
        Some(cw_telemetry::init("backend", log::LevelFilter::Info))
    } else {
        cw_core::logs::init_console(log::LevelFilter::Warn);
        None
    };
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if serving => {
            log::error!("the backend stopped: {e}");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
