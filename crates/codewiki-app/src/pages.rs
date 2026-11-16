//! The report pages: decisions, dead code, health, drift, settings and the
//! rest. Each turns what the backend sent into a [`Doc`].

use serde_json::Value;

use crate::doc::{
    Action, Block, Cell, Choice, Column, Doc, Line, Stat, Table, code, file, note, rich, symbol,
    text,
};
use crate::motion::MotionPref;
use crate::mui::feedback::Tone;
use crate::update;

fn strings<'a>(value: &'a Value, key: &str) -> Vec<&'a str> {
    value[key]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn list<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value[key].as_array().into_iter().flatten().collect()
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("")
}

/// A number as the backend sent it; a dash when it sent none.
fn number(value: &Value) -> String {
    match value {
        Value::Null => "—".to_string(),
        other => other.to_string(),
    }
}

/// A list of file links, one to a line.
fn file_list(paths: &[&str], limit: usize) -> Block {
    Block::List {
        items: paths
            .iter()
            .take(limit)
            .map(|path| vec![file(path)])
            .collect(),
        ordered: false,
    }
}

/// The decision records with their status and staleness.
pub fn decisions_page(rows: &Value) -> Doc {
    let rows: Vec<&Value> = rows.as_array().into_iter().flatten().collect();
    let mut doc = Doc::new("Decisions").lead(
        "Recorded decisions and the code they cite. A decision is stale when that code changed after the record was last updated.",
    );
    if rows.is_empty() {
        doc.push(Block::Empty(
            "No decision records found. Add ADR files, or notes under 07-Decisions.".into(),
        ));
        return doc;
    }
    for row in rows {
        let mut chips = vec![(
            row["status"].as_str().unwrap_or("unknown").to_string(),
            Tone::Tertiary,
        )];
        if row["stale"] == true {
            chips.push(("stale".into(), Tone::Warning));
        }
        let mut body = vec![Block::Text(vec![code(string(row, "path"))])];
        if let Some(summary) = row["summary"].as_str().filter(|s| !s.is_empty()) {
            body.push(Block::Text(vec![text(summary)]));
        }
        let cites = strings(row, "cites");
        if !cites.is_empty() {
            let mut line: Line = vec![text("Cites: ")];
            for (index, cite) in cites.iter().enumerate() {
                if index > 0 {
                    line.push(text(", "));
                }
                line.push(file(cite));
            }
            body.push(Block::Text(line));
        }
        let reasons = strings(row, "stale_reasons");
        if !reasons.is_empty() {
            body.push(Block::List {
                items: reasons.iter().map(|reason| rich(reason)).collect(),
                ordered: false,
            });
        }
        doc.push(Block::Card {
            title: row["title"].as_str().unwrap_or("Untitled").to_string(),
            chips,
            body,
        });
    }
    doc
}

/// Private symbols that nothing refers to.
pub fn dead_code_page(rows: &Value) -> Doc {
    let rows: Vec<&Value> = rows.as_array().into_iter().flatten().collect();
    let mut doc = Doc::new("Dead code").lead(
        "Private symbols that nothing in the repository refers to. Entries marked likely are candidates for removal; entries marked possible may be reached in ways the index cannot see.",
    );
    if rows.is_empty() {
        doc.push(Block::Empty("Nothing found.".into()));
        return doc;
    }
    let mut table = Table::new([
        Column::new("Certainty").weight(0.6),
        Column::new("Symbol"),
        Column::new("Kind").weight(0.6),
        Column::new("File").weight(1.6),
        Column::numeric("Lines"),
    ]);
    for row in rows {
        let certainty = string(row, "certainty");
        let tone = if certainty == "likely" {
            Tone::Warning
        } else {
            Tone::Neutral
        };
        table.row([
            Cell::chip(certainty, tone),
            symbol(string(row, "qualified")).into(),
            Cell::text(string(row, "kind")),
            file(string(row, "file")).into(),
            Cell::text(number(&row["lines"])),
        ]);
    }
    doc.push(Block::Table(table));
    doc
}

/// File health scores, worst first. Files without findings are left out.
pub fn health_page(rows: &Value) -> Doc {
    let mut rows: Vec<&Value> = rows
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["findings"].as_array().is_some_and(|f| !f.is_empty()))
        .collect();
    rows.sort_by_key(|r| r["score"].as_u64().unwrap_or(10));
    let mut doc = Doc::new("Code health").lead(
        "Each file starts at 10 and loses points for findings such as long functions or high fan-out.",
    );
    if rows.is_empty() {
        doc.push(Block::Empty("No findings.".into()));
        return doc;
    }
    let mut table = Table::new([
        Column::new("Score").weight(0.7),
        Column::new("File").weight(1.4),
        Column::new("Findings").weight(2.2),
    ]);
    for row in rows {
        let findings: Vec<&str> = list(row, "findings")
            .into_iter()
            .filter_map(|finding| finding["message"].as_str())
            .collect();
        let score = row["score"].as_u64().unwrap_or(10);
        let tone = match score {
            0..=4 => Tone::Error,
            5..=7 => Tone::Warning,
            _ => Tone::Success,
        };
        table.row([
            Cell::Share {
                label: score.to_string(),
                share: score as f32 / 10.0,
                tone,
            },
            file(string(row, "path")).into(),
            rich(&findings.join("; ")).into(),
        ]);
    }
    doc.push(Block::Table(table));
    doc
}

/// Documentation drift and coverage.
pub fn drift_page(report: &Value) -> Doc {
    let coverage = report["coverage"].as_f64().unwrap_or(0.0) * 100.0;
    let mut doc = Doc::new("Documentation drift").lead(format!(
        "{coverage:.0}% of source files are cited by a note ({} of {}).",
        number(&report["documented_files"]),
        number(&report["source_files"])
    ));
    let drift = list(report, "drift");
    doc.push(Block::Stats(vec![
        Stat::new("Files cited by a note", format!("{coverage:.0}%")),
        Stat::new("Notes that disagree", drift.len()).tone(if drift.is_empty() {
            Tone::Success
        } else {
            Tone::Warning
        }),
    ]));
    if drift.is_empty() {
        doc.push(Block::Alert {
            tone: Tone::Success,
            text: "No note disagrees with the code.".into(),
            button: None,
        });
    } else {
        let mut table = Table::new([
            Column::new("Note"),
            Column::new("Problem").weight(0.7),
            Column::new("Detail").weight(1.6),
        ]);
        for item in drift {
            let path = string(item, "note");
            let name = path.rsplit('/').next().unwrap_or(path);
            table.row([
                note(name.trim_end_matches(".md"), path).into(),
                Cell::chip(string(item, "kind").replace('_', " "), Tone::Warning),
                rich(string(item, "detail")).into(),
            ]);
        }
        doc.push(Block::Table(table));
    }
    let undocumented = strings(report, "undocumented");
    if !undocumented.is_empty() {
        doc.heading("Files no note cites");
        doc.push(file_list(&undocumented, 60));
    }
    doc
}

/// The settings page: appearance, where data comes from, updates and the
/// keyboard shortcuts. `animations_on` is what the choice in `motion` comes
/// to; `repos` holds `(name, path)` pairs; `auto_update` is whether the app
/// looks for updates by itself; `autostart` is whether Windows starts the app
/// at sign-in, `None` for a copy that is not installed.
#[allow(clippy::too_many_arguments)]
pub fn settings_page(
    backend: &str,
    theme: &str,
    motion: MotionPref,
    animations_on: bool,
    repos: &[(String, String)],
    update: &update::State,
    auto_update: bool,
    autostart: Option<bool>,
) -> Doc {
    let mut doc = Doc::new("Settings");
    doc.push(Block::Card {
        title: "Appearance".into(),
        chips: Vec::new(),
        body: vec![
            Block::Setting {
                label: "Theme".into(),
                value: theme.into(),
                button: Some(("Choose a theme".into(), Action::Themes)),
            },
            Block::Text(vec![
                text("Your own themes load from the "),
                code("themes"),
                text(" folder in "),
                code("%APPDATA%\\CodeWiki"),
                text(", in the same JSON format."),
            ]),
            Block::Choices {
                label: "Animations".into(),
                choices: MotionPref::ALL
                    .into_iter()
                    .map(|pref| Choice {
                        label: pref.label().into(),
                        action: Action::Motion(pref),
                        selected: pref == motion,
                    })
                    .collect(),
            },
            Block::Text(vec![text(format!(
                "Animations are {}.",
                if animations_on { "on" } else { "off" }
            ))]),
        ],
    });
    doc.push(Block::Card {
        title: "Backend".into(),
        chips: Vec::new(),
        body: vec![
            Block::Text(vec![text("Connected to "), code(backend), text(".")]),
            Block::Text(vec![
                text("Set "),
                code("CODEWIKI_URL"),
                text(" and "),
                code("CODEWIKI_TOKEN"),
                text(
                    " before starting the app to use a backend in Docker or on another machine. Without them the app starts a local backend.",
                ),
            ]),
        ],
    });
    doc.push(Block::Card {
        title: "Logs".into(),
        chips: Vec::new(),
        body: vec![
            Block::Setting {
                label: "Folder".into(),
                value: cw_core::logs::dir().display().to_string(),
                button: Some(("Open logs folder".into(), Action::OpenLogs)),
            },
            Block::Text(vec![
                text("Attach "),
                code("app.log"),
                text(" and "),
                code("backend.log"),
                text(" to a bug report; they name the folders you open. Set "),
                code("CODEWIKI_LOG=debug"),
                text(" before starting the app to keep more."),
            ]),
        ],
    });
    let mut repositories = Table::new([
        Column::new("Repository").weight(0.6),
        Column::new("Folder").weight(1.6),
    ]);
    for (name, path) in repos {
        repositories.row([Cell::text(name), code(path).into()]);
    }
    doc.push(Block::Card {
        title: "Repositories".into(),
        chips: Vec::new(),
        body: vec![Block::Table(repositories)],
    });
    doc.push(startup_card(autostart));
    doc.push(updates_card(update, auto_update));
    let mut keys = Table::new([
        Column::new("Keys").weight(0.6),
        Column::new("Action").weight(1.6),
    ]);
    for (chord, action) in [
        ("Ctrl+K", "Command palette"),
        ("Ctrl+T", "New tab"),
        ("Ctrl+W", "Close tab"),
        ("Ctrl+Tab", "Next tab"),
        ("Alt+Left", "Back"),
    ] {
        keys.row([Cell::chip(chord, Tone::Neutral), Cell::text(action)]);
    }
    doc.push(Block::Card {
        title: "Keyboard".into(),
        chips: Vec::new(),
        body: vec![Block::Table(keys)],
    });
    doc
}

/// Whether Windows starts the app at sign-in, and what closing the window
/// does: the two belong together, since an app started that way is seen only
/// as its tray icon.
fn startup_card(autostart: Option<bool>) -> Block {
    let setting = match autostart {
        Some(on) => Block::Setting {
            // Short: the label column is narrow, and a longer one wraps.
            label: "Start with Windows".into(),
            value: if on {
                "On, in the tray, with no window".into()
            } else {
                "Off".into()
            },
            button: Some((
                if on { "Turn off" } else { "Turn on" }.into(),
                Action::Autostart(!on),
            )),
        },
        None => Block::Setting {
            label: "Start with Windows".into(),
            value: "Only an installed copy can start with Windows".into(),
            button: None,
        },
    };
    Block::Card {
        title: "Startup".into(),
        chips: Vec::new(),
        body: vec![
            setting,
            Block::Text(vec![text(
                "Closing the window keeps CodeWiki running in the tray, so that agents find its backend at once. Click the tray icon to open the window again; right-click it to quit.",
            )]),
        ],
    }
}

/// The version, what the last check found, and whether the app checks by
/// itself.
fn updates_card(update: &update::State, auto_update: bool) -> Block {
    use update::State;
    let button = match update {
        State::Available(_) => Some(("Install and restart".to_string(), Action::InstallUpdate)),
        State::Idle | State::UpToDate | State::Unavailable(_) => {
            Some(("Check now".to_string(), Action::CheckUpdates))
        }
        State::NotInstalled | State::Checking | State::Installing(_) | State::Installed(_) => None,
    };
    let mut body = vec![Block::Setting {
        label: "Version".into(),
        value: update.describe(),
        button,
    }];
    if *update != State::NotInstalled {
        body.push(Block::Setting {
            // Short: the label column is narrow, and a longer one wraps.
            label: "Daily check".into(),
            value: if auto_update {
                "On, once a day while CodeWiki runs".into()
            } else {
                "Off".into()
            },
            button: Some((
                if auto_update { "Turn off" } else { "Turn on" }.into(),
                Action::AutoUpdate(!auto_update),
            )),
        });
    }
    Block::Card {
        title: "Updates".into(),
        chips: Vec::new(),
        body,
    }
}

/// The test map: where the tests are and which files have none.
pub fn tests_page(map: &Value) -> Doc {
    let files = list(map, "test_files");
    let untested = strings(map, "untested");
    let mut doc = Doc::new("Tests").lead(format!(
        "{} tests in {} test files.",
        number(&map["tests_total"]),
        files.len()
    ));
    doc.push(Block::Stats(vec![
        Stat::new("Tests", number(&map["tests_total"])),
        Stat::new("Test files", files.len()),
        Stat::new("Files no test reaches", untested.len()).tone(if untested.is_empty() {
            Tone::Success
        } else {
            Tone::Warning
        }),
    ]));
    if !files.is_empty() {
        let mut table = Table::new([
            Column::new("Test file").weight(1.4),
            Column::new("Kind").weight(0.6),
            Column::numeric("Tests"),
            Column::new("Run with").weight(1.6),
        ]);
        for file_row in files {
            table.row([
                file(string(file_row, "path")).into(),
                Cell::chip(string(file_row, "kind"), Tone::Neutral),
                Cell::text(list(file_row, "tests").len()),
                code(string(file_row, "command")).into(),
            ]);
        }
        doc.push(Block::Table(table));
    }
    if !untested.is_empty() {
        doc.heading("Files no test reaches");
        doc.push(file_list(&untested, 80));
    }
    doc
}

/// Imported line coverage, least covered first.
pub fn coverage_page(report: &Value) -> Doc {
    let percent = report["percent"].as_f64().unwrap_or(0.0);
    let mut doc = Doc::new("Coverage").lead(format!(
        "{percent:.1}% of lines covered ({} of {}), from {}.",
        number(&report["lines_hit"]),
        number(&report["lines_found"]),
        string(report, "source")
    ));
    doc.push(Block::Stats(vec![
        Stat::new("Lines covered", format!("{percent:.1}%")).tone(match percent {
            p if p >= 80.0 => Tone::Success,
            p if p >= 50.0 => Tone::Warning,
            _ => Tone::Error,
        }),
        Stat::new("Lines hit", number(&report["lines_hit"])),
        Stat::new("Lines found", number(&report["lines_found"])),
    ]));
    let gaps = list(report, "least_covered");
    if !gaps.is_empty() {
        doc.heading("Where more tests pay off most");
        doc.text("Ranked by missing coverage, change frequency and how much depends on the file.");
        let mut table = Table::new([
            Column::new("File").weight(2.0),
            Column::new("Covered"),
            Column::numeric("Uncovered lines").weight(0.8),
        ]);
        for gap in gaps {
            let covered = gap["percent"].as_f64().unwrap_or(0.0);
            table.row([
                file(string(gap, "path")).into(),
                Cell::Share {
                    label: format!("{covered:.0}%"),
                    share: covered as f32 / 100.0,
                    tone: Tone::Neutral,
                },
                Cell::text(number(&gap["uncovered_lines"])),
            ]);
        }
        doc.push(Block::Table(table));
    }
    let missing = strings(report, "not_in_report");
    if !missing.is_empty() {
        doc.heading("Files the report does not mention");
        doc.push(file_list(&missing, 60));
    }
    doc
}

/// Security findings, most severe first.
pub fn security_page(rows: &Value) -> Doc {
    let mut rows: Vec<&Value> = rows.as_array().into_iter().flatten().collect();
    let order = |level: &str| match level {
        "error" => 0,
        "warning" => 1,
        _ => 2,
    };
    rows.sort_by_key(|r| order(string(r, "level")));
    let mut doc = Doc::new("Security").lead(
        "Patterns in the source that deserve a second look. The scan reads text, not data flow, so each finding is a prompt to check, not a verdict.",
    );
    if rows.is_empty() {
        doc.push(Block::Empty("No findings.".into()));
        return doc;
    }
    let mut table = Table::new([
        Column::new("Level").weight(0.6),
        Column::new("Rule").weight(1.2),
        Column::new("File").weight(1.2),
        Column::numeric("Line").weight(0.4),
        Column::new("Finding").weight(1.8),
    ]);
    for row in rows {
        let level = string(row, "level");
        let tone = match level {
            "error" => Tone::Error,
            "warning" => Tone::Warning,
            _ => Tone::Info,
        };
        table.row([
            Cell::chip(level, tone),
            code(string(row, "rule")).into(),
            file(string(row, "file")).into(),
            Cell::text(number(&row["line"])),
            rich(string(row, "message")).into(),
        ]);
    }
    doc.push(Block::Table(table));
    doc
}

/// Refactoring suggestions with their evidence and steps.
pub fn refactor_page(rows: &Value) -> Doc {
    let rows: Vec<&Value> = rows.as_array().into_iter().flatten().collect();
    let mut doc = Doc::new("Refactoring plan").lead(
        "Suggestions derived from file health, change history and dependency cycles, highest priority first.",
    );
    if rows.is_empty() {
        doc.push(Block::Empty("Nothing to suggest.".into()));
        return doc;
    }
    for row in rows {
        let mut body = vec![Block::Text(vec![file(string(row, "file"))])];
        let evidence = strings(row, "evidence");
        if !evidence.is_empty() {
            body.push(Block::Heading("Why".into()));
            body.push(Block::List {
                items: evidence.iter().map(|item| rich(item)).collect(),
                ordered: false,
            });
        }
        let steps = strings(row, "steps");
        if !steps.is_empty() {
            body.push(Block::Heading("Steps".into()));
            body.push(Block::List {
                items: steps.iter().map(|step| rich(step)).collect(),
                ordered: true,
            });
        }
        doc.push(Block::Card {
            title: row["title"].as_str().unwrap_or("Suggestion").to_string(),
            chips: vec![(string(row, "kind").replace('_', " "), Tone::Tertiary)],
            body,
        });
    }
    doc
}

fn history_files(history: &Value) -> Vec<&Value> {
    history["files"]
        .as_object()
        .into_iter()
        .flat_map(|files| files.values())
        .collect()
}

/// Who knows the code: authors by the files they lead, and files only one
/// person has worked on.
pub fn ownership_page(history: &Value) -> Doc {
    let files = history_files(history);
    let mut doc = Doc::new("Ownership").lead(format!(
        "From {} commits. The lead author of a file is whoever made most of its commits.",
        number(&history["commits_scanned"])
    ));
    if files.is_empty() {
        doc.push(Block::Empty(
            "No commit history is indexed yet. Commit your work, then press Sync.".into(),
        ));
        return doc;
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
                string(file, "path"),
                lead,
                file["commits"].as_u64().unwrap_or(0),
            ));
        }
    }
    let mut ranked: Vec<(&str, (usize, usize))> = authors.into_iter().collect();
    ranked.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(b.0)));
    let mut table = Table::new([
        Column::new("Author").weight(1.4),
        Column::numeric("Files led"),
        Column::numeric("Of those, known by this author alone").weight(1.2),
    ]);
    for (name, (led, alone)) in ranked {
        table.row([Cell::text(name), Cell::text(led), Cell::text(alone)]);
    }
    doc.push(Block::Table(table));
    solo.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(b.0)));
    if !solo.is_empty() {
        doc.heading("Most-changed files one person holds");
        doc.text("If that person is away, nobody else has changed these.");
        let mut table = Table::new([
            Column::new("File").weight(2.0),
            Column::new("Author"),
            Column::numeric("Commits"),
        ]);
        for (path, lead, commits) in solo.iter().take(40) {
            table.row([file(path).into(), Cell::text(lead), Cell::text(commits)]);
        }
        doc.push(Block::Table(table));
    }
    doc
}

/// Files that tend to change in the same commit.
pub fn coupling_page(history: &Value) -> Doc {
    let mut pairs: Vec<(&str, &str, u64, f64)> = Vec::new();
    for file in history_files(history) {
        let path = string(file, "path");
        for partner in list(file, "partners") {
            let other = string(partner, "path");
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
    let mut doc = Doc::new("Change coupling").lead(
        "Files that change in the same commits. A pair with no import between them is a hidden dependency: changing one usually means changing the other.",
    );
    if pairs.is_empty() {
        doc.push(Block::Empty(
            "No pairs found. This needs commit history with files changed together several times."
                .into(),
        ));
        return doc;
    }
    pairs.sort_by(|a, b| {
        b.3.total_cmp(&a.3)
            .then(b.2.cmp(&a.2))
            .then(a.0.cmp(b.0))
            .then(a.1.cmp(b.1))
    });
    let mut table = Table::new([
        Column::new("File").weight(1.6),
        Column::new("Changes with").weight(1.6),
        Column::numeric("Commits together").weight(0.7),
        Column::new("Strength").weight(0.8),
    ]);
    for (path, other, together, strength) in pairs.iter().take(80) {
        table.row([
            file(path).into(),
            file(other).into(),
            Cell::text(together),
            Cell::Share {
                label: format!("{:.0}%", strength * 100.0),
                share: *strength as f32,
                tone: Tone::Neutral,
            },
        ]);
    }
    doc.push(Block::Table(table));
    doc.text("Strength is the share of the rarer file's commits that included both files.");
    doc
}

/// Every registered repository side by side.
pub fn workspace_page(workspace: &Value) -> Doc {
    let repos = list(workspace, "repos");
    let mut doc = Doc::new("Workspace").lead(format!("{} repositories.", repos.len()));
    if !repos.is_empty() {
        let mut table = Table::new([
            Column::new("Repository").weight(1.2),
            Column::numeric("Files"),
            Column::numeric("Symbols"),
            Column::numeric("Notes"),
            Column::numeric("To review"),
            Column::new("State").weight(1.4),
        ]);
        for repo in &repos {
            let state = match repo["error"].as_str() {
                Some(error) => Cell::chip(format!("error: {error}"), Tone::Error),
                None if repo["stale"] == true => Cell::chip("wiki behind the code", Tone::Warning),
                None => Cell::chip("up to date", Tone::Success),
            };
            table.row([
                vec![crate::doc::strong(string(repo, "name"))].into(),
                Cell::text(number(&repo["files"])),
                Cell::text(number(&repo["symbols"])),
                Cell::text(number(&repo["notes"])),
                Cell::text(number(&repo["pending_proposals"])),
                state,
            ]);
        }
        doc.push(Block::Table(table));
    }
    let shared = list(workspace, "shared_dependencies");
    if !shared.is_empty() {
        doc.heading("Dependencies shared by repositories");
        let mut table = Table::new([Column::new("Package"), Column::new("Used by").weight(2.0)]);
        for item in shared {
            table.row([
                code(string(item, "name")).into(),
                Cell::text(strings(item, "repos").join(", ")),
            ]);
        }
        doc.push(Block::Table(table));
    }
    doc
}

/// HTTP endpoints across the workspace: who provides what each repository
/// calls, and what nothing provides or calls.
pub fn contracts_page(contracts: &Value) -> Doc {
    let matched = list(contracts, "matched");
    let unprovided = list(contracts, "unprovided");
    let unconsumed = list(contracts, "unconsumed");
    let mut doc = Doc::new("API contracts").lead(format!(
        "HTTP endpoints found in source text across all repositories: {} calls matched to a provider, {} calls to endpoints no repository provides, {} routes nothing calls.",
        matched.len(),
        unprovided.len(),
        unconsumed.len()
    ));
    doc.push(Block::Stats(vec![
        Stat::new("Matched calls", matched.len()).tone(Tone::Success),
        Stat::new("Called, not provided", unprovided.len()),
        Stat::new("Provided, never called", unconsumed.len()),
    ]));
    let place = |row: &Value| -> Line {
        vec![
            code(string(row, "file")),
            text(format!(" line {}", number(&row["line"]))),
        ]
    };
    if !unprovided.is_empty() {
        doc.heading("Called but not provided here");
        doc.text(
            "Calls to outside services belong in this list; a typo in a path shows up here too.",
        );
        let mut table = Table::new([
            Column::new("Path").weight(1.4),
            Column::new("Called from").weight(1.4),
            Column::new("Repository").weight(0.7),
        ]);
        for row in unprovided {
            table.row([
                code(string(row, "path")).into(),
                place(row).into(),
                Cell::text(string(row, "repo")),
            ]);
        }
        doc.push(Block::Table(table));
    }
    if !unconsumed.is_empty() {
        doc.heading("Provided but never called");
        let mut table = Table::new([
            Column::new("Route").weight(1.4),
            Column::new("Defined in").weight(1.4),
            Column::new("Repository").weight(0.7),
        ]);
        for row in unconsumed {
            table.row([
                code(format!("{} {}", string(row, "method"), string(row, "path"))).into(),
                place(row).into(),
                Cell::text(string(row, "repo")),
            ]);
        }
        doc.push(Block::Table(table));
    }
    if !matched.is_empty() {
        doc.heading("Matched calls");
        let mut table = Table::new([
            Column::new("Path").weight(1.4),
            Column::new("Called from").weight(1.4),
            Column::new("Provided by"),
        ]);
        for row in matched.iter().take(200) {
            let mut providers: Line = Vec::new();
            for (index, provider) in list(row, "providers").into_iter().enumerate() {
                if index > 0 {
                    providers.push(text(", "));
                }
                providers.push(text(format!("{} ", string(provider, "repo"))));
                providers.push(code(string(provider, "method")));
            }
            table.row([
                code(string(row, "path")).into(),
                vec![
                    text(format!("{} ", string(row, "repo"))),
                    code(string(row, "file")),
                ]
                .into(),
                providers.into(),
            ]);
        }
        doc.push(Block::Table(table));
    }
    doc
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
        let words = page.words();
        assert!(words.contains("Use SQLite [accepted, stale]"), "{words}");
        assert!(words.contains("docs/adr/0001.md"));
        assert!(words.contains("One file, no server."));
        assert!(words.contains("Cites: src/store.rs"));
        assert!(words.contains("src/store.rs changed"));
        assert_eq!(page.actions(), [Action::File("src/store.rs".into())]);
        let empty = decisions_page(&json!([]));
        assert!(empty.words().contains("No decision records found"));
    }

    #[test]
    fn a_record_that_is_not_stale_is_not_marked() {
        let page = decisions_page(&json!([{"title": "T", "status": "proposed", "path": "a.md"}]));
        assert!(page.words().contains("T [proposed]"));
    }

    #[test]
    fn health_lists_only_files_with_findings_worst_first() {
        let page = health_page(&json!([
            {"path": "a.rs", "score": 10, "findings": []},
            {"path": "b.rs", "score": 7, "findings": [{"message": "long | file"}]},
            {"path": "c.rs", "score": 4, "findings": [{"message": "x"}, {"message": "y"}]},
        ]));
        assert_eq!(
            page.tables()[0].plain(),
            [["4", "c.rs", "x; y"], ["7", "b.rs", "long | file"]]
        );
        assert!(health_page(&json!([])).words().contains("No findings."));
    }

    #[test]
    fn drift_links_notes_and_reports_coverage() {
        let page = drift_page(&json!({
            "coverage": 0.5, "documented_files": 1, "source_files": 2,
            "drift": [{"note": "01-Core/Parser.md", "kind": "broken_link", "detail": "[[Gone]]"}],
            "undocumented": ["src/x.rs"]
        }));
        let words = page.words();
        assert!(words.contains("50% of source files are cited by a note (1 of 2)."));
        assert_eq!(
            page.tables()[0].plain(),
            [["Parser", "broken link", "[[Gone]]"]]
        );
        assert_eq!(page.headings(), ["Files no note cites"]);
        assert_eq!(
            page.actions(),
            [
                Action::Note("01-Core/Parser.md".into()),
                Action::File("src/x.rs".into())
            ]
        );
        let clean = drift_page(&json!({"coverage": 1.0, "drift": [], "undocumented": []}));
        assert!(clean.words().contains("No note disagrees with the code."));
        assert!(clean.tables().is_empty());
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
        assert!(ownership.words().contains("From 12 commits."));
        let tables = ownership.tables();
        assert_eq!(tables[0].plain(), [["Ana", "2", "1"], ["Bo", "1", "1"]]);
        // Solo files, most-changed first.
        assert_eq!(
            tables[1].plain(),
            [["src/a.rs", "Ana", "9"], ["src/c.rs", "Bo", "2"]]
        );

        let coupling = coupling_page(&history);
        // The pair is listed once although both files record it.
        assert_eq!(
            coupling.tables()[0].plain(),
            [["src/a.rs", "src/b.rs", "6", "75%"]]
        );
        let empty = json!({"commits_scanned": 0, "files": {}});
        assert!(ownership_page(&empty).words().contains("No commit history"));
        assert!(coupling_page(&empty).words().contains("No pairs found"));
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
        assert!(page.words().contains("2 repositories."));
        let tables = page.tables();
        assert_eq!(
            tables[0].plain(),
            [
                ["api", "10", "80", "12", "1", "wiki behind the code"],
                ["gone", "0", "0", "0", "0", "error: not found"]
            ]
        );
        assert_eq!(tables[1].plain(), [["serde", "api, web"]]);
        assert!(page.actions().is_empty());

        let contracts = contracts_page(&json!({
            "matched": [{"repo": "app", "path": "/v1/repos", "file": "src/main.rs", "line": 3,
                "providers": [{"repo": "api", "method": "GET", "path": "/v1/repos"}]}],
            "unprovided": [{"repo": "app", "path": "/api/chat", "file": "src/llm.rs", "line": 9}],
            "unconsumed": [{"repo": "api", "method": "POST", "path": "/v1/hooks/git",
                "file": "src/api.rs", "line": 40}]
        }));
        assert!(
            contracts
                .words()
                .contains("1 calls matched to a provider, 1 calls to endpoints")
        );
        assert_eq!(
            contracts.headings(),
            [
                "Called but not provided here",
                "Provided but never called",
                "Matched calls"
            ]
        );
        let tables = contracts.tables();
        assert_eq!(
            tables[0].plain(),
            [["/api/chat", "src/llm.rs line 9", "app"]]
        );
        assert_eq!(
            tables[1].plain(),
            [["POST /v1/hooks/git", "src/api.rs line 40", "api"]]
        );
        assert_eq!(
            tables[2].plain(),
            [["/v1/repos", "app src/main.rs", "api GET"]]
        );
    }

    #[test]
    fn analysis_pages_render() {
        let tests = tests_page(&json!({
            "tests_total": 3,
            "test_files": [{"path": "tests/api.rs", "kind": "integration",
                "tests": [{"name": "a", "line": 1}], "command": "cargo test --test api"}],
            "untested": ["src/x.rs"]
        }));
        assert!(tests.words().contains("3 tests in 1 test files."));
        assert_eq!(
            tests.tables()[0].plain(),
            [["tests/api.rs", "integration", "1", "cargo test --test api"]]
        );
        assert_eq!(tests.headings(), ["Files no test reaches"]);

        let coverage = coverage_page(&json!({
            "percent": 62.5, "lines_hit": 5, "lines_found": 8, "source": "lcov.info",
            "least_covered": [{"path": "src/a.rs", "percent": 25.0, "uncovered_lines": 3}],
            "not_in_report": []
        }));
        assert!(
            coverage
                .words()
                .contains("62.5% of lines covered (5 of 8), from lcov.info.")
        );
        assert_eq!(coverage.tables()[0].plain(), [["src/a.rs", "25%", "3"]]);
        assert_eq!(coverage.headings(), ["Where more tests pay off most"]);

        let security = security_page(&json!([
            {"level": "note", "rule": "security/unsafe-block", "file": "a.rs", "line": 2, "message": "m"},
            {"level": "error", "rule": "security/hardcoded-secret", "file": "b.rs", "line": 9, "message": "s"},
        ]));
        assert_eq!(
            security.tables()[0].plain(),
            [
                ["error", "security/hardcoded-secret", "b.rs", "9", "s"],
                ["note", "security/unsafe-block", "a.rs", "2", "m"]
            ]
        );
        assert!(security_page(&json!([])).words().contains("No findings."));

        let plan = refactor_page(&json!([{
            "title": "Split api.rs", "kind": "split_file", "file": "src/api.rs",
            "evidence": ["1,200 lines"], "steps": ["Move handlers", "Re-export"]
        }]));
        let words = plan.words();
        assert!(words.contains("Split api.rs [split file]"));
        assert!(words.contains("Why\n1,200 lines\nSteps\nMove handlers\nRe-export"));
        assert_eq!(plan.actions(), [Action::File("src/api.rs".into())]);
    }

    #[test]
    fn dead_code_and_settings_render() {
        let page = dead_code_page(&json!([{
            "certainty": "likely", "qualified": "demo::old", "kind": "function",
            "file": "src/lib.rs", "lines": 4
        }]));
        assert_eq!(
            page.tables()[0].plain(),
            [["likely", "old", "function", "src/lib.rs", "4"]]
        );
        assert_eq!(
            page.actions(),
            [
                Action::Symbol("demo::old".into()),
                Action::File("src/lib.rs".into())
            ]
        );
        let settings = settings_page(
            "http://127.0.0.1:7411",
            "Nord",
            MotionPref::Reduced,
            false,
            &[("demo".into(), "C:/demo".into())],
            &update::State::UpToDate,
            true,
            Some(false),
        );
        let words = settings.words();
        assert!(words.contains("Theme: Nord"));
        assert!(words.contains("Your own themes load from the themes folder"));
        assert!(words.contains("Animations: follow Windows / reduced / full"));
        assert!(words.contains("Animations are off."));
        assert!(words.contains("Connected to http://127.0.0.1:7411."));
        assert!(words.contains("demo | C:/demo"));
        assert!(words.contains("Ctrl+K | Command palette"));
        assert!(words.contains("Attach app.log and backend.log to a bug report"));
        // One choice per setting, each carrying the value it sets.
        let actions = settings.actions();
        assert_eq!(actions[0], Action::Themes);
        assert!(actions.contains(&Action::OpenLogs));
        for pref in MotionPref::ALL {
            assert!(actions.contains(&Action::Motion(pref)));
        }
        let Some(Block::Card { body, .. }) = settings.blocks.first() else {
            panic!("no appearance card");
        };
        let Some(Block::Choices { choices, .. }) = body.get(2) else {
            panic!("no animation choices");
        };
        let chosen: Vec<&str> = choices
            .iter()
            .filter(|choice| choice.selected)
            .map(|choice| choice.label.as_str())
            .collect();
        assert_eq!(chosen, ["reduced"]);
    }

    fn updates(settings: &Doc) -> Vec<Block> {
        settings
            .blocks
            .iter()
            .find_map(|block| match block {
                Block::Card { title, body, .. } if title == "Updates" => Some(body.clone()),
                _ => None,
            })
            .expect("no updates card")
    }

    fn settings_with(state: update::State, auto: bool) -> Doc {
        settings_page("", "", MotionPref::Full, true, &[], &state, auto, None)
    }

    #[test]
    fn settings_offer_the_start_with_windows_only_to_an_installed_copy() {
        let with = |autostart| {
            settings_page(
                "",
                "",
                MotionPref::Full,
                true,
                &[],
                &update::State::Idle,
                true,
                autostart,
            )
        };
        let off = with(Some(false));
        assert!(off.words().contains("Start with Windows: Off"));
        assert!(off.actions().contains(&Action::Autostart(true)));

        let on = with(Some(true));
        assert!(
            on.words()
                .contains("Start with Windows: On, in the tray, with no window")
        );
        assert!(on.actions().contains(&Action::Autostart(false)));

        // A build from the sources says why it has no switch.
        let dev = with(None);
        assert!(
            dev.words()
                .contains("Start with Windows: Only an installed copy can start with Windows")
        );
        assert!(
            dev.actions()
                .iter()
                .all(|a| !matches!(a, Action::Autostart(_)))
        );
        // Whoever closes the window is told where the app went.
        for page in [off, on, dev] {
            assert!(
                page.words()
                    .contains("Closing the window keeps CodeWiki running in the tray")
            );
        }
    }

    #[test]
    fn settings_offer_what_the_update_state_allows() {
        let version = cw_core::VERSION;

        let checked = settings_with(update::State::UpToDate, true);
        assert!(
            checked
                .words()
                .contains(&format!("Version: {version}, the latest version"))
        );
        assert!(
            checked
                .words()
                .contains("Daily check: On, once a day while CodeWiki runs")
        );
        assert!(checked.actions().contains(&Action::CheckUpdates));
        assert!(checked.actions().contains(&Action::AutoUpdate(false)));

        let release = cw_update::Release {
            version: cw_update::Version::new(9, 0, 0),
            zip_name: "CodeWiki-9.0.0-windows-x64.zip".into(),
            zip_url: String::new(),
            sha256_url: String::new(),
        };
        let available = settings_with(update::State::Available(release), false);
        assert!(
            available
                .words()
                .contains(&format!("Version: {version}; 9.0.0 is available"))
        );
        assert!(available.actions().contains(&Action::InstallUpdate));
        assert!(!available.actions().contains(&Action::CheckUpdates));
        assert!(available.actions().contains(&Action::AutoUpdate(true)));

        // Nothing to press while a check or an install runs.
        let busy = settings_with(update::State::Checking, true);
        assert!(
            busy.actions()
                .iter()
                .all(|a| !matches!(a, Action::CheckUpdates | Action::InstallUpdate))
        );

        // A development build says so, and offers neither a check nor the
        // daily setting.
        let dev = settings_with(update::State::NotInstalled, true);
        let rows = updates(&dev);
        assert_eq!(rows.len(), 1);
        assert!(dev.words().contains("development build"));
        assert!(
            dev.actions()
                .iter()
                .all(|a| !matches!(a, Action::CheckUpdates | Action::AutoUpdate(_)))
        );
    }
}
