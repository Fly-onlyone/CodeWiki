//! Markdown for the report pages: decisions, dead code, health, drift, settings.

use serde_json::Value;

use crate::markdown::{NOTE_SCHEME, SETTING_SCHEME, THEME_SCHEME, encode, file_link, symbol_link};
use crate::motion::MotionPref;

fn strings<'a>(value: &'a Value, key: &str) -> Vec<&'a str> {
    value[key]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

/// Render the decision records with their status and staleness.
pub fn decisions_page(rows: &Value) -> String {
    let rows: Vec<&Value> = rows.as_array().into_iter().flatten().collect();
    let mut out = String::from(
        "# Decisions\n\nRecorded decisions and the code they cite. A decision is stale when that code changed after the record was last updated.\n",
    );
    if rows.is_empty() {
        out.push_str(
            "\nNo decision records found. Add ADR files, or notes under `07-Decisions`.\n",
        );
        return out;
    }
    for row in rows {
        let stale = if row["stale"] == true {
            " — **stale**"
        } else {
            ""
        };
        out.push_str(&format!(
            "\n## {}\n\n{} · `{}`{stale}\n",
            row["title"].as_str().unwrap_or("Untitled"),
            row["status"].as_str().unwrap_or("unknown"),
            row["path"].as_str().unwrap_or("")
        ));
        if let Some(summary) = row["summary"].as_str().filter(|s| !s.is_empty()) {
            out.push_str(&format!("\n{summary}\n"));
        }
        let cites = strings(row, "cites");
        if !cites.is_empty() {
            let links: Vec<String> = cites.iter().map(|c| file_link(c)).collect();
            out.push_str(&format!("\nCites: {}\n", links.join(", ")));
        }
        let reasons = strings(row, "stale_reasons");
        if !reasons.is_empty() {
            out.push('\n');
            for reason in reasons {
                out.push_str(&format!("- {reason}\n"));
            }
        }
    }
    out
}

/// Render private symbols that nothing refers to.
pub fn dead_code_page(rows: &Value) -> String {
    let rows: Vec<&Value> = rows.as_array().into_iter().flatten().collect();
    let mut out = String::from(
        "# Dead code\n\nPrivate symbols that nothing in the repository refers to. Entries marked likely are candidates for removal; entries marked possible may be reached in ways the index cannot see.\n",
    );
    if rows.is_empty() {
        out.push_str("\nNothing found.\n");
        return out;
    }
    out.push_str("\n| Certainty | Symbol | Kind | File | Lines |\n|---|---|---|---|---:|\n");
    for row in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            row["certainty"].as_str().unwrap_or(""),
            symbol_link(row["qualified"].as_str().unwrap_or("")),
            row["kind"].as_str().unwrap_or(""),
            file_link(row["file"].as_str().unwrap_or("")),
            row["lines"]
        ));
    }
    out
}

/// Render file health scores, worst first. Files without findings are left out.
pub fn health_page(rows: &Value) -> String {
    let mut rows: Vec<&Value> = rows
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["findings"].as_array().is_some_and(|f| !f.is_empty()))
        .collect();
    rows.sort_by_key(|r| r["score"].as_u64().unwrap_or(10));
    let mut out = String::from(
        "# Code health\n\nEach file starts at 10 and loses points for findings such as long functions or high fan-out.\n",
    );
    if rows.is_empty() {
        out.push_str("\nNo findings.\n");
        return out;
    }
    out.push_str("\n| Score | File | Findings |\n|---:|---|---|\n");
    for row in rows {
        let findings: Vec<String> = row["findings"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f["message"].as_str())
            .map(|m| m.replace('|', "/"))
            .collect();
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            row["score"],
            file_link(row["path"].as_str().unwrap_or("")),
            findings.join("; ")
        ));
    }
    out
}

/// Render documentation drift and coverage.
pub fn drift_page(report: &Value) -> String {
    let mut out = format!(
        "# Documentation drift\n\n{:.0}% of source files are cited by a note ({} of {}).\n",
        report["coverage"].as_f64().unwrap_or(0.0) * 100.0,
        report["documented_files"],
        report["source_files"]
    );
    let drift: Vec<&Value> = report["drift"].as_array().into_iter().flatten().collect();
    if drift.is_empty() {
        out.push_str("\nNo note disagrees with the code.\n");
    } else {
        out.push_str("\n| Note | Problem | Detail |\n|---|---|---|\n");
        for item in drift {
            let note = item["note"].as_str().unwrap_or("");
            let name = note.rsplit('/').next().unwrap_or(note);
            out.push_str(&format!(
                "| [{}]({NOTE_SCHEME}{}) | {} | {} |\n",
                name.trim_end_matches(".md"),
                encode(note),
                item["kind"].as_str().unwrap_or("").replace('_', " "),
                item["detail"].as_str().unwrap_or("").replace('|', "/")
            ));
        }
    }
    let undocumented = strings(report, "undocumented");
    if !undocumented.is_empty() {
        out.push_str("\n## Files no note cites\n\n");
        for path in undocumented.iter().take(60) {
            out.push_str(&format!("- {}\n", file_link(path)));
        }
    }
    out
}

/// The settings page: appearance, where data comes from, and the keyboard
/// shortcuts. `motion` describes the current animation setting; `repos` holds
/// `(name, path)` pairs.
pub fn settings_page(
    backend: &str,
    theme: &str,
    motion: &str,
    repos: &[(String, String)],
) -> String {
    let choices: Vec<String> = MotionPref::ALL
        .into_iter()
        .map(|pref| format!("[{}]({SETTING_SCHEME}motion/{})", pref.label(), pref.key()))
        .collect();
    let choices = choices.join(" · ");
    let mut out = format!(
        "# Settings\n\n## Appearance\n\nTheme: **{theme}** — [choose a theme]({THEME_SCHEME})\n\nAnimations: **{motion}** — {choices}\n\n## Backend\n\nConnected to `{backend}`.\n\nSet `CODEWIKI_URL` and `CODEWIKI_TOKEN` before starting the app to use a backend in Docker or on another machine. Without them the app starts a local backend.\n\n## Repositories\n\n"
    );
    for (name, path) in repos {
        out.push_str(&format!("- **{name}** — `{path}`\n"));
    }
    out.push_str("\n## Keyboard\n\n| Keys | Action |\n|---|---|\n| Ctrl+K | Command palette |\n| Ctrl+T | New tab |\n| Ctrl+W | Close tab |\n| Ctrl+Tab | Next tab |\n| Alt+Left | Back |\n");
    out
}

/// Render the test map: where the tests are and which files have none.
pub fn tests_page(map: &Value) -> String {
    let mut out = format!(
        "# Tests\n\n{} tests in {} test files.\n",
        map["tests_total"],
        map["test_files"].as_array().map(Vec::len).unwrap_or(0)
    );
    let files: Vec<&Value> = map["test_files"].as_array().into_iter().flatten().collect();
    if !files.is_empty() {
        out.push_str("\n| Test file | Kind | Tests | Run with |\n|---|---|---:|---|\n");
        for file in files {
            out.push_str(&format!(
                "| {} | {} | {} | `{}` |\n",
                file_link(file["path"].as_str().unwrap_or("")),
                file["kind"].as_str().unwrap_or(""),
                file["tests"].as_array().map(Vec::len).unwrap_or(0),
                file["command"].as_str().unwrap_or("")
            ));
        }
    }
    let untested = strings(map, "untested");
    if !untested.is_empty() {
        out.push_str("\n## Files no test reaches\n\n");
        for path in untested.iter().take(80) {
            out.push_str(&format!("- {}\n", file_link(path)));
        }
    }
    out
}

/// Render imported line coverage, least covered first.
pub fn coverage_page(report: &Value) -> String {
    let mut out = format!(
        "# Coverage\n\n{:.1}% of lines covered ({} of {}), from `{}`.\n",
        report["percent"].as_f64().unwrap_or(0.0),
        report["lines_hit"],
        report["lines_found"],
        report["source"].as_str().unwrap_or("")
    );
    let gaps: Vec<&Value> = report["least_covered"]
        .as_array()
        .into_iter()
        .flatten()
        .collect();
    if !gaps.is_empty() {
        out.push_str("\n## Where more tests pay off most\n\nRanked by missing coverage, change frequency and how much depends on the file.\n\n| File | Covered | Uncovered lines |\n|---|---:|---:|\n");
        for gap in gaps {
            out.push_str(&format!(
                "| {} | {:.0}% | {} |\n",
                file_link(gap["path"].as_str().unwrap_or("")),
                gap["percent"].as_f64().unwrap_or(0.0),
                gap["uncovered_lines"]
            ));
        }
    }
    let missing = strings(report, "not_in_report");
    if !missing.is_empty() {
        out.push_str("\n## Files the report does not mention\n\n");
        for path in missing.iter().take(60) {
            out.push_str(&format!("- {}\n", file_link(path)));
        }
    }
    out
}

/// Render security findings, most severe first.
pub fn security_page(rows: &Value) -> String {
    let mut rows: Vec<&Value> = rows.as_array().into_iter().flatten().collect();
    let order = |level: &str| match level {
        "error" => 0,
        "warning" => 1,
        _ => 2,
    };
    rows.sort_by_key(|r| order(r["level"].as_str().unwrap_or("")));
    let mut out = String::from(
        "# Security\n\nPatterns in the source that deserve a second look. The scan reads text, not data flow, so each finding is a prompt to check, not a verdict.\n",
    );
    if rows.is_empty() {
        out.push_str("\nNo findings.\n");
        return out;
    }
    out.push_str("\n| Level | Rule | File | Line | Finding |\n|---|---|---|---:|---|\n");
    for row in rows {
        out.push_str(&format!(
            "| {} | `{}` | {} | {} | {} |\n",
            row["level"].as_str().unwrap_or(""),
            row["rule"].as_str().unwrap_or(""),
            file_link(row["file"].as_str().unwrap_or("")),
            row["line"],
            row["message"].as_str().unwrap_or("").replace('|', "/")
        ));
    }
    out
}

/// Render refactoring suggestions with their evidence and steps.
pub fn refactor_page(rows: &Value) -> String {
    let rows: Vec<&Value> = rows.as_array().into_iter().flatten().collect();
    let mut out = String::from(
        "# Refactoring plan\n\nSuggestions derived from file health, change history and dependency cycles, highest priority first.\n",
    );
    if rows.is_empty() {
        out.push_str("\nNothing to suggest.\n");
        return out;
    }
    for row in rows {
        out.push_str(&format!(
            "\n## {}\n\n{} · {}\n",
            row["title"].as_str().unwrap_or("Suggestion"),
            row["kind"].as_str().unwrap_or("").replace('_', " "),
            file_link(row["file"].as_str().unwrap_or(""))
        ));
        let evidence = strings(row, "evidence");
        if !evidence.is_empty() {
            out.push_str("\nWhy:\n\n");
            for item in evidence {
                out.push_str(&format!("- {item}\n"));
            }
        }
        let steps = strings(row, "steps");
        if !steps.is_empty() {
            out.push_str("\nSteps:\n\n");
            for (number, step) in steps.iter().enumerate() {
                out.push_str(&format!("{}. {step}\n", number + 1));
            }
        }
    }
    out
}

fn history_files(history: &Value) -> Vec<&Value> {
    history["files"]
        .as_object()
        .into_iter()
        .flat_map(|files| files.values())
        .collect()
}

/// Render who knows the code: authors by the files they lead, and files
/// only one person has worked on.
pub fn ownership_page(history: &Value) -> String {
    let files = history_files(history);
    let mut out = format!(
        "# Ownership\n\nFrom {} commits. The lead author of a file is whoever made most of its commits.\n",
        history["commits_scanned"]
    );
    if files.is_empty() {
        out.push_str("\nNo commit history is indexed yet. Commit your work, then press Sync.\n");
        return out;
    }
    // name -> (files led, files led alone)
    let mut authors: std::collections::BTreeMap<&str, (usize, usize)> = Default::default();
    let mut solo: Vec<(&str, &str, u64)> = Vec::new();
    for file in &files {
        let Some(lead) = file["owners"][0]["name"].as_str() else {
            continue;
        };
        let entry = authors.entry(lead).or_default();
        entry.0 += 1;
        if file["bus_factor"].as_u64() == Some(1) {
            entry.1 += 1;
            solo.push((
                file["path"].as_str().unwrap_or(""),
                lead,
                file["commits"].as_u64().unwrap_or(0),
            ));
        }
    }
    let mut ranked: Vec<(&str, (usize, usize))> = authors.into_iter().collect();
    ranked.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(b.0)));
    out.push_str(
        "\n| Author | Files led | Of those, known by this author alone |\n|---|---:|---:|\n",
    );
    for (name, (led, alone)) in ranked {
        out.push_str(&format!("| {name} | {led} | {alone} |\n"));
    }
    solo.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(b.0)));
    if !solo.is_empty() {
        out.push_str("\n## Most-changed files one person holds\n\nIf that person is away, nobody else has changed these.\n\n| File | Author | Commits |\n|---|---|---:|\n");
        for (path, lead, commits) in solo.iter().take(40) {
            out.push_str(&format!("| {} | {lead} | {commits} |\n", file_link(path)));
        }
    }
    out
}

/// Render files that tend to change in the same commit.
pub fn coupling_page(history: &Value) -> String {
    let mut pairs: Vec<(&str, &str, u64, f64)> = Vec::new();
    for file in history_files(history) {
        let path = file["path"].as_str().unwrap_or("");
        for partner in file["partners"].as_array().into_iter().flatten() {
            let other = partner["path"].as_str().unwrap_or("");
            // Each pair is stored on both files; keep one direction.
            if path < other {
                pairs.push((
                    path,
                    other,
                    partner["together"].as_u64().unwrap_or(0),
                    partner["strength"].as_f64().unwrap_or(0.0),
                ));
            }
        }
    }
    let mut out = String::from(
        "# Change coupling\n\nFiles that change in the same commits. A pair with no import between them is a hidden dependency: changing one usually means changing the other.\n",
    );
    if pairs.is_empty() {
        out.push_str("\nNo pairs found. This needs commit history with files changed together several times.\n");
        return out;
    }
    pairs.sort_by(|a, b| {
        b.3.total_cmp(&a.3)
            .then(b.2.cmp(&a.2))
            .then(a.0.cmp(b.0))
            .then(a.1.cmp(b.1))
    });
    out.push_str("\n| File | Changes with | Commits together | Strength |\n|---|---|---:|---:|\n");
    for (path, other, together, strength) in pairs.iter().take(80) {
        out.push_str(&format!(
            "| {} | {} | {together} | {:.0}% |\n",
            file_link(path),
            file_link(other),
            strength * 100.0
        ));
    }
    out.push_str("\nStrength is the share of the rarer file's commits that included both files.\n");
    out
}

/// Render every registered repository side by side.
pub fn workspace_page(workspace: &Value) -> String {
    let repos: Vec<&Value> = workspace["repos"]
        .as_array()
        .into_iter()
        .flatten()
        .collect();
    let mut out = format!("# Workspace\n\n{} repositories.\n", repos.len());
    if !repos.is_empty() {
        out.push_str("\n| Repository | Files | Symbols | Notes | To review | State |\n|---|---:|---:|---:|---:|---|\n");
        for repo in &repos {
            let state = match repo["error"].as_str() {
                Some(error) => format!("error: {}", error.replace('|', "/")),
                None if repo["stale"] == true => "wiki behind the code".to_string(),
                None => "up to date".to_string(),
            };
            out.push_str(&format!(
                "| **{}** | {} | {} | {} | {} | {state} |\n",
                repo["name"].as_str().unwrap_or(""),
                repo["files"],
                repo["symbols"],
                repo["notes"],
                repo["pending_proposals"]
            ));
        }
    }
    let shared: Vec<&Value> = workspace["shared_dependencies"]
        .as_array()
        .into_iter()
        .flatten()
        .collect();
    if !shared.is_empty() {
        out.push_str(
            "\n## Dependencies shared by repositories\n\n| Package | Used by |\n|---|---|\n",
        );
        for item in shared {
            out.push_str(&format!(
                "| `{}` | {} |\n",
                item["name"].as_str().unwrap_or(""),
                strings(item, "repos").join(", ")
            ));
        }
    }
    out
}

/// Render HTTP endpoints across the workspace: who provides what each
/// repository calls, and what nothing provides or calls.
pub fn contracts_page(contracts: &Value) -> String {
    let rows =
        |key: &str| -> Vec<&Value> { contracts[key].as_array().into_iter().flatten().collect() };
    let matched = rows("matched");
    let unprovided = rows("unprovided");
    let unconsumed = rows("unconsumed");
    let mut out = format!(
        "# API contracts\n\nHTTP endpoints found in source text across all repositories: {} calls matched to a provider, {} calls to endpoints no repository provides, {} routes nothing calls.\n",
        matched.len(),
        unprovided.len(),
        unconsumed.len()
    );
    if !unprovided.is_empty() {
        out.push_str("\n## Called but not provided here\n\nCalls to outside services belong in this list; a typo in a path shows up here too.\n\n| Path | Called from | Repository |\n|---|---|---|\n");
        for row in unprovided {
            out.push_str(&format!(
                "| `{}` | `{}` line {} | {} |\n",
                row["path"].as_str().unwrap_or(""),
                row["file"].as_str().unwrap_or(""),
                row["line"],
                row["repo"].as_str().unwrap_or("")
            ));
        }
    }
    if !unconsumed.is_empty() {
        out.push_str("\n## Provided but never called\n\n| Route | Defined in | Repository |\n|---|---|---|\n");
        for row in unconsumed {
            out.push_str(&format!(
                "| `{} {}` | `{}` line {} | {} |\n",
                row["method"].as_str().unwrap_or(""),
                row["path"].as_str().unwrap_or(""),
                row["file"].as_str().unwrap_or(""),
                row["line"],
                row["repo"].as_str().unwrap_or("")
            ));
        }
    }
    if !matched.is_empty() {
        out.push_str("\n## Matched calls\n\n| Path | Called from | Provided by |\n|---|---|---|\n");
        for row in matched.iter().take(200) {
            let providers: Vec<String> = row["providers"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|p| {
                    format!(
                        "{} `{}`",
                        p["repo"].as_str().unwrap_or(""),
                        p["method"].as_str().unwrap_or("")
                    )
                })
                .collect();
            out.push_str(&format!(
                "| `{}` | {} `{}` | {} |\n",
                row["path"].as_str().unwrap_or(""),
                row["repo"].as_str().unwrap_or(""),
                row["file"].as_str().unwrap_or(""),
                providers.join(", ")
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decisions_show_staleness_and_citations() {
        let page = decisions_page(&json!([{
            "title": "Use SQLite", "status": "accepted", "path": "docs/adr/0001.md",
            "summary": "One file, no server.", "cites": ["src/store.rs"],
            "stale": true, "stale_reasons": ["src/store.rs changed"]
        }]));
        assert!(page.contains("## Use SQLite"));
        assert!(page.contains("accepted · `docs/adr/0001.md` — **stale**"));
        assert!(page.contains("Cites: [`src/store.rs`](codewiki://file/"));
        assert!(page.contains("- src/store.rs changed"));
        assert!(decisions_page(&json!([])).contains("No decision records found"));
    }

    #[test]
    fn health_lists_only_files_with_findings_worst_first() {
        let page = health_page(&json!([
            {"path": "a.rs", "score": 10, "findings": []},
            {"path": "b.rs", "score": 7, "findings": [{"message": "long | file"}]},
            {"path": "c.rs", "score": 4, "findings": [{"message": "x"}, {"message": "y"}]},
        ]));
        assert!(!page.contains("a.rs"));
        assert!(page.find("c.rs").unwrap() < page.find("b.rs").unwrap());
        assert!(page.contains("long / file"));
        assert!(page.contains("x; y"));
    }

    #[test]
    fn drift_links_notes_and_reports_coverage() {
        let page = drift_page(&json!({
            "coverage": 0.5, "documented_files": 1, "source_files": 2,
            "drift": [{"note": "01-Core/Parser.md", "kind": "broken_link", "detail": "[[Gone]]"}],
            "undocumented": ["src/x.rs"]
        }));
        assert!(page.contains("50% of source files are cited by a note (1 of 2)"));
        assert!(page.contains("| [Parser](codewiki://note/"));
        assert!(page.contains("broken link"));
        assert!(page.contains("## Files no note cites"));
    }

    #[test]
    fn ownership_and_coupling_render_from_history() {
        let history = json!({
            "commits_scanned": 12,
            "files": {
                "src/a.rs": {"path": "src/a.rs", "commits": 9, "bus_factor": 1,
                    "owners": [{"name": "Ana", "commits": 9, "share": 1.0}],
                    "partners": [{"path": "src/b.rs", "together": 6, "strength": 0.75}]},
                "src/b.rs": {"path": "src/b.rs", "commits": 8, "bus_factor": 2,
                    "owners": [{"name": "Ana", "commits": 5, "share": 0.6},
                               {"name": "Bo", "commits": 3, "share": 0.4}],
                    "partners": [{"path": "src/a.rs", "together": 6, "strength": 0.75}]},
                "src/c.rs": {"path": "src/c.rs", "commits": 2, "bus_factor": 1,
                    "owners": [{"name": "Bo", "commits": 2, "share": 1.0}], "partners": []},
            }
        });
        let ownership = ownership_page(&history);
        assert!(ownership.contains("From 12 commits."));
        assert!(ownership.contains("| Ana | 2 | 1 |"));
        assert!(ownership.contains("| Bo | 1 | 1 |"));
        // Solo files, most-changed first.
        assert!(ownership.find("src/a.rs").unwrap() < ownership.find("src/c.rs").unwrap());

        let coupling = coupling_page(&history);
        // The pair is listed once although both files record it.
        assert_eq!(coupling.matches("| 6 | 75% |").count(), 1);
        let empty = json!({"commits_scanned": 0, "files": {}});
        assert!(ownership_page(&empty).contains("No commit history"));
        assert!(coupling_page(&empty).contains("No pairs found"));
    }

    #[test]
    fn workspace_pages_render() {
        let page = workspace_page(&json!({
            "repos": [
                {"name": "api", "files": 10, "symbols": 80, "notes": 12,
                 "pending_proposals": 1, "stale": true},
                {"name": "gone", "files": 0, "symbols": 0, "notes": 0,
                 "pending_proposals": 0, "stale": false, "error": "not found"},
            ],
            "shared_dependencies": [{"name": "serde", "repos": ["api", "web"]}]
        }));
        assert!(page.contains("2 repositories."));
        assert!(page.contains("| **api** | 10 | 80 | 12 | 1 | wiki behind the code |"));
        assert!(page.contains("error: not found"));
        assert!(page.contains("| `serde` | api, web |"));

        let contracts = contracts_page(&json!({
            "matched": [{"repo": "app", "path": "/v1/repos", "file": "src/main.rs", "line": 3,
                "providers": [{"repo": "api", "method": "GET", "path": "/v1/repos"}]}],
            "unprovided": [{"repo": "app", "path": "/api/chat", "file": "src/llm.rs", "line": 9}],
            "unconsumed": [{"repo": "api", "method": "POST", "path": "/v1/hooks/git",
                "file": "src/api.rs", "line": 40}]
        }));
        assert!(contracts.contains("1 calls matched to a provider, 1 calls to endpoints"));
        assert!(contracts.contains("| `/api/chat` | `src/llm.rs` line 9 | app |"));
        assert!(contracts.contains("| `POST /v1/hooks/git` | `src/api.rs` line 40 | api |"));
        assert!(contracts.contains("| `/v1/repos` | app `src/main.rs` | api `GET` |"));
    }

    #[test]
    fn analysis_pages_render() {
        let tests = tests_page(&json!({
            "tests_total": 3,
            "test_files": [{"path": "tests/api.rs", "kind": "integration",
                "tests": [{"name": "a", "line": 1}], "command": "cargo test --test api"}],
            "untested": ["src/x.rs"]
        }));
        assert!(tests.contains("3 tests in 1 test files."));
        assert!(tests.contains("| integration | 1 | `cargo test --test api` |"));
        assert!(tests.contains("## Files no test reaches"));

        let coverage = coverage_page(&json!({
            "percent": 62.5, "lines_hit": 5, "lines_found": 8, "source": "lcov.info",
            "least_covered": [{"path": "src/a.rs", "percent": 25.0, "uncovered_lines": 3}],
            "not_in_report": []
        }));
        assert!(coverage.contains("62.5% of lines covered (5 of 8), from `lcov.info`."));
        assert!(coverage.contains("| 25% | 3 |"));
        assert!(!coverage.contains("does not mention"));

        let security = security_page(&json!([
            {"level": "note", "rule": "security/unsafe-block", "file": "a.rs", "line": 2, "message": "m"},
            {"level": "error", "rule": "security/hardcoded-secret", "file": "b.rs", "line": 9, "message": "s"},
        ]));
        assert!(
            security.find("hardcoded-secret").unwrap() < security.find("unsafe-block").unwrap()
        );
        assert!(security_page(&json!([])).contains("No findings."));

        let plan = refactor_page(&json!([{
            "title": "Split api.rs", "kind": "split_file", "file": "src/api.rs",
            "evidence": ["1,200 lines"], "steps": ["Move handlers", "Re-export"]
        }]));
        assert!(plan.contains("## Split api.rs"));
        assert!(plan.contains("split file · [`src/api.rs`]"));
        assert!(plan.contains("1. Move handlers\n2. Re-export\n"));
    }

    #[test]
    fn dead_code_and_settings_render() {
        let page = dead_code_page(&json!([{
            "certainty": "likely", "qualified": "demo::old", "kind": "function",
            "file": "src/lib.rs", "lines": 4
        }]));
        assert!(page.contains("| likely | [`old`](codewiki://symbol/"));
        let settings = settings_page(
            "http://127.0.0.1:7411",
            "Tokyo Night",
            "follow Windows (animations are on)",
            &[("demo".into(), "C:/demo".into())],
        );
        assert!(settings.contains("Theme: **Tokyo Night**"));
        assert!(settings.contains("Animations: **follow Windows (animations are on)**"));
        // One link per choice, each naming the value it sets.
        for key in ["system", "reduced", "full"] {
            assert!(settings.contains(&format!("](codewiki://setting/motion/{key})")));
        }
        assert!(settings.contains("- **demo** — `C:/demo`"));
        assert!(settings.contains("| Ctrl+K | Command palette |"));
    }
}
