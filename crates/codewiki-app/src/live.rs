//! The live pages: one symbol, one file, search results, hotspots, the theme
//! picker, a proposal, the result of a sync, and Home, the page the app
//! starts on. Like the reports in `pages`,
//! each turns what the backend sent into a [`Doc`]. These show code that the
//! vault deliberately does not hold as notes.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::doc::{
    Action, Block, Cell, Column, Doc, Line, Press, Row, Stat, Table, code, file, rich, strong, text,
};
use crate::icons;
use crate::mui::feedback::Tone;
use crate::settings::Category;

fn list(value: &Value) -> Vec<&Value> {
    value.as_array().into_iter().flatten().collect()
}

fn string<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("")
}

fn number(value: &Value) -> String {
    match value {
        Value::Null => "—".to_string(),
        other => other.to_string(),
    }
}

/// A row that opens a symbol, with its signature or relation under its name.
fn symbol_row(qualified: &str, detail: impl Into<String>) -> Row {
    let name = qualified.rsplit("::").next().unwrap_or(qualified);
    let row = Row::new(icons::SYMBOL, name, Action::Symbol(qualified.to_string())).code();
    let detail = detail.into();
    if detail.is_empty() {
        row
    } else {
        row.detail(detail)
    }
}

/// A row that opens a file.
fn file_row(path: &str) -> Row {
    Row::new(icons::FILE, path, Action::File(path.to_string())).code()
}

/// Rows that open notes, named by the note's file name.
fn note_rows(paths: &Value) -> Vec<Row> {
    list(paths)
        .into_iter()
        .filter_map(Value::as_str)
        .map(|path| {
            let target = path.trim_end_matches(".md");
            let stem = target.rsplit('/').next().unwrap_or(target);
            Row::new(icons::NOTE, stem, Action::Note(target.to_string()))
        })
        .collect()
}

/// A heading over rows, or nothing when there are no rows.
fn section(doc: &mut Doc, title: &str, rows: Vec<Row>) {
    if !rows.is_empty() {
        doc.heading(title);
        doc.push(Block::Rows(rows));
    }
}

fn related(items: &Value) -> Vec<Row> {
    list(items)
        .into_iter()
        .take(40)
        .map(|item| {
            symbol_row(
                string(item, "qualified"),
                format!(
                    "{} {} in {}",
                    string(item, "relation"),
                    string(item, "qualified"),
                    string(item, "file")
                ),
            )
        })
        .collect()
}

/// One symbol: what it is, where it is, and what it touches.
pub fn symbol_page(detail: &Value) -> Doc {
    let symbol = &detail["symbol"];
    let mut doc = Doc::new(string(symbol, "name"));
    let mut body = vec![Block::Code {
        language: "rust".into(),
        text: string(symbol, "signature").to_string(),
    }];
    let comment = string(symbol, "doc");
    if !comment.is_empty() {
        body.push(Block::Text(vec![text(comment)]));
    }
    body.push(Block::Text(vec![
        file(string(symbol, "file")),
        text(format!(
            " — lines {} to {}",
            number(&symbol["start_line"]),
            number(&symbol["end_line"])
        )),
    ]));
    doc.push(Block::Card {
        title: string(symbol, "qualified").to_string(),
        chips: vec![
            (string(symbol, "kind").replace('_', " "), Tone::Tertiary),
            (string(symbol, "visibility").to_string(), Tone::Neutral),
        ],
        body,
    });
    let members = list(&detail["members"])
        .into_iter()
        .map(|member| symbol_row(string(member, "qualified"), string(member, "signature")))
        .collect();
    section(&mut doc, "Members", members);
    section(&mut doc, "Depends on", related(&detail["depends_on"]));
    section(&mut doc, "Used by", related(&detail["used_by"]));
    section(&mut doc, "Notes", note_rows(&detail["notes"]));
    doc
}

/// Search results, grouped by what was found.
pub fn search_page(query: &str, hits: &Value) -> Doc {
    let mut doc = Doc::new(format!("Search: {query}"));
    let hits = list(hits);
    if hits.is_empty() {
        doc.push(Block::Empty("No results.".into()));
        return doc;
    }
    for (kind, heading) in [("note", "Notes"), ("symbol", "Symbols"), ("file", "Files")] {
        let rows = hits
            .iter()
            .filter(|hit| hit["kind"] == kind)
            .map(|hit| {
                let key = string(hit, "key");
                match kind {
                    "note" => {
                        let title = hit["title"].as_str().unwrap_or(key);
                        let target = key.trim_end_matches(".md").to_string();
                        let row = Row::new(icons::NOTE, title, Action::Note(target));
                        let snippet = string(hit, "snippet").replace('\n', " ");
                        if snippet.is_empty() {
                            row
                        } else {
                            row.detail(snippet)
                        }
                    }
                    "symbol" => symbol_row(key, key),
                    _ => file_row(key),
                }
            })
            .collect();
        section(&mut doc, heading, rows);
    }
    doc
}

/// The hotspot ranking.
pub fn hotspots_page(rows: &Value) -> Doc {
    let rows = list(rows);
    let mut doc = Doc::new("Hotspots").lead(
        "Files that are both large and frequently changed. Defects concentrate here, and refactoring here pays off most.",
    );
    if rows.is_empty() {
        doc.push(Block::Empty(
            "No commit history is indexed yet. Commit your work, then press Sync.".into(),
        ));
        return doc;
    }
    let mut table = Table::new([
        Column::new("Score").weight(0.9),
        Column::new("File").weight(2.0),
        Column::numeric("Commits"),
        Column::numeric("Lines"),
        Column::new("Main author"),
        Column::numeric("Bus factor"),
    ]);
    for row in rows {
        let score = row["score"].as_f64().unwrap_or(0.0);
        table.row([
            Cell::Share {
                label: format!("{score:.1}"),
                share: score as f32 / 100.0,
                tone: Tone::Neutral,
            },
            file(string(row, "path")).into(),
            Cell::text(number(&row["commits"])),
            Cell::text(number(&row["lines"])),
            Cell::text(row["top_owner"].as_str().unwrap_or("—")),
            Cell::text(number(&row["bus_factor"])),
        ]);
    }
    doc.push(Block::Table(table));
    doc.text(
        "Bus factor is the smallest number of authors who together made over half of a file's commits. A value of 1 means one person holds the knowledge.",
    );
    doc
}

/// One file: risk, symbols, relations and notes.
pub fn file_page(context: &Value, risk: &Value, radius: &Value) -> Doc {
    let path = string(context, "path");
    let mut doc = Doc::new(path.rsplit('/').next().unwrap_or(path));
    let level = risk["level"].as_str().unwrap_or("unknown");
    let tone = match level {
        "high" => Tone::Error,
        "medium" => Tone::Warning,
        "low" => Tone::Success,
        _ => Tone::Neutral,
    };
    doc.push(Block::Stats(vec![
        Stat::new("Risk", level)
            .hint(format!("{} of 100", number(&risk["score"])))
            .tone(tone),
        Stat::new("Lines", number(&context["lines"])),
        Stat::new("Language", string(context, "language")),
    ]));
    doc.push(Block::Text(vec![code(path)]));
    let reasons: Vec<Line> = list(&risk["reasons"])
        .into_iter()
        .filter_map(Value::as_str)
        .map(rich)
        .collect();
    if !reasons.is_empty() {
        doc.heading("Risk");
        doc.push(Block::List {
            items: reasons,
            ordered: false,
        });
    }
    let owners = list(&risk["history"]["owners"]);
    if !owners.is_empty() {
        doc.heading("Ownership");
        let mut table = Table::new([
            Column::new("Author").weight(1.4),
            Column::numeric("Commits"),
            Column::new("Share"),
        ]);
        for owner in owners.into_iter().take(5) {
            let share = owner["share"].as_f64().unwrap_or(0.0);
            table.row([
                Cell::text(string(owner, "name")),
                Cell::text(number(&owner["commits"])),
                Cell::Share {
                    label: format!("{:.0}%", share * 100.0),
                    share: share as f32,
                    tone: Tone::Neutral,
                },
            ]);
        }
        doc.push(Block::Table(table));
    }
    let partners = list(&risk["history"]["partners"])
        .into_iter()
        .map(|partner| {
            file_row(string(partner, "path"))
                .detail(format!("together {} times", number(&partner["together"])))
        })
        .collect();
    section(&mut doc, "Usually changes with", partners);

    let files = |value: &Value| -> Vec<Row> {
        list(value)
            .into_iter()
            .filter_map(Value::as_str)
            .map(file_row)
            .collect()
    };
    let direct = files(&radius["direct"]);
    if !direct.is_empty() {
        doc.heading("Blast radius");
        doc.text("Direct dependents:");
        doc.push(Block::Rows(direct));
        let transitive = files(&radius["transitive"]);
        if !transitive.is_empty() {
            doc.text(format!(
                "Reached through further hops ({}):",
                transitive.len()
            ));
            doc.push(Block::Rows(transitive.into_iter().take(30).collect()));
        }
    }
    let symbols = list(&context["symbols"])
        .into_iter()
        .filter(|symbol| symbol["kind"] != "module")
        .take(60)
        .map(|symbol| symbol_row(string(symbol, "qualified"), string(symbol, "signature")))
        .collect();
    section(&mut doc, "Symbols", symbols);
    section(&mut doc, "Imports from", files(&context["imports"]));
    section(&mut doc, "Notes", note_rows(&context["notes"]));
    doc
}

/// One proposal for review.
pub fn proposal_page(detail: &Value) -> Doc {
    let proposal = &detail["proposal"];
    let mut doc = Doc::new("Proposed change");
    doc.push(Block::Text(vec![
        code(string(proposal, "note_path")),
        text(" — "),
        text(string(proposal, "reason")),
    ]));
    // Two things propose: a sync, when the code under the note changed, and
    // a coding agent that explained the note.
    let from_agent = proposal["kind"] == "agent";
    let pictures = detail["pictures"].as_array().map_or(0, Vec::len);
    let mut said = if from_agent {
        "A coding agent wrote an explanation for this note. You edited the note by hand, so the agent's text waits for your decision. Nothing is written until you approve.".to_string()
    } else {
        "This note was edited by hand, so the change waits for your decision. Nothing is written until you approve.".to_string()
    };
    match pictures {
        0 => {}
        1 => said.push_str(" Approving also gives the note 1 picture."),
        n => said.push_str(&format!(" Approving also gives the note {n} pictures.")),
    }
    doc.push(Block::Alert {
        tone: Tone::Info,
        text: said,
        button: None,
    });
    doc.push(Block::Code {
        language: "diff".into(),
        text: string(detail, "patch").trim_end().to_string(),
    });
    doc
}

/// What a sync did.
pub fn sync_page(report: &Value) -> Doc {
    let count = |key: &str| report[key].as_array().map(Vec::len).unwrap_or(0);
    let mut doc = Doc::new("Sync complete");
    doc.push(Block::Alert {
        tone: Tone::Success,
        text: format!(
            "{} notes written, {} unchanged.",
            count("written"),
            number(&report["kept"])
        ),
        button: None,
    });
    let first = vec![
        Stat::new("Written", count("written")),
        Stat::new("Unchanged", number(&report["kept"])),
        Stat::new("Queued for review", count("proposed")).tone(if count("proposed") > 0 {
            Tone::Warning
        } else {
            Tone::Neutral
        }),
    ];
    doc.push(Block::Stats(first));
    doc.push(Block::Stats(vec![
        Stat::new("Removed", count("removed")),
        Stat::new("Adopted", count("adopted")),
        Stat::new("Notes in all", number(&report["notes_total"])),
        Stat::new(
            "Links that resolve",
            format!(
                "{:.1}%",
                report["link_rate"].as_f64().unwrap_or(0.0) * 100.0
            ),
        ),
    ]));
    // CodeWiki writes each note's facts; a coding agent explains it.
    let unexplained = report["unexplained"].as_u64().unwrap_or(0) as usize;
    let waiting = count("behind") + unexplained;
    if waiting > 0 || count("merged") > 0 {
        doc.push(Block::Stats(vec![
            Stat::new("Facts refreshed, explanation kept", count("merged")),
            Stat::new("Explanation may be behind", count("behind")).tone(if count("behind") > 0 {
                Tone::Warning
            } else {
                Tone::Neutral
            }),
            Stat::new("Not explained yet", unexplained),
        ]));
    }
    if waiting > 0 {
        doc.push(Block::Alert {
            tone: Tone::Info,
            text: format!(
                "{waiting} notes wait for a coding agent to explain them. In Claude Code with the CodeWiki plugin, run /codewiki:write-notes."
            ),
            button: None,
        });
    }
    doc
}

/// The line above a note in the agent view: what the view is, and what it
/// leaves out of the note.
pub fn agent_view_line(left_out: &[String]) -> String {
    let start = "Agent view: the note as a coding agent reads it.";
    if left_out.is_empty() {
        format!("{start} The text is whole; the pictures are left out.")
    } else {
        format!(
            "{start} Left out: {}, which is a walk-through made by rule that the code shows, and the pictures.",
            left_out.join(", ")
        )
    }
}

/// What the details pane says about a note's explanation, a line for each
/// string: who wrote it, and why it may be behind. `explanation` is the
/// backend's word for where it stands. Nothing for a note no agent explains.
pub fn explanation_lines(explanation: &str, by: &str, behind: &[String]) -> Vec<String> {
    let who = if by.is_empty() { "a coding agent" } else { by };
    match explanation {
        "waiting" => vec!["Facts written by rules. Waits for a coding agent to explain it.".into()],
        "explained" => vec![format!("Explained by {who}.")],
        "behind" => {
            let mut lines = vec![
                format!("Explained by {who}."),
                "The explanation may be behind the code:".to_string(),
            ];
            lines.extend(behind.iter().map(|reason| format!("• {reason}")));
            lines
        }
        _ => Vec::new(),
    }
}

/// A page that only says something, such as that there is no wiki yet.
pub fn message_page(message: &str) -> Doc {
    let mut doc = Doc::new("CodeWiki");
    if !message.is_empty() {
        doc.push(Block::Text(vec![strong(message)]));
    }
    doc
}

/// Whether coding agents are kept out of a project's registered folder.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Kept {
    /// Nothing in the list covers the folder.
    #[default]
    Open,
    /// An entry for exactly this folder covers it.
    Own,
    /// This entry, a parent folder, covers it: alone, or beside an entry for
    /// exactly this folder, whose removal would not let agents in.
    Under(PathBuf),
    /// Nothing covers the folder, but this entry lies inside it, so agents
    /// get no CodeWiki for the project.
    Contains(PathBuf),
}

/// The state of a project that has none of its own.
pub static OPEN: Kept = Kept::Open;

/// How `entries` keep agents out of `folder`. A parent entry wins over the
/// folder's own: removing the own entry would leave the folder kept. It
/// looks at the file system (a link is followed), so the app works it out
/// when Home's data is refreshed, never while a page is built.
pub fn kept_by(entries: &[PathBuf], folder: &str) -> Kept {
    use cw_core::agents_off::{self, Block};
    let folder = Path::new(folder);
    if !folder.is_absolute() {
        return Kept::Open;
    }
    let lower = |path: &Path| {
        agents_off::normalize(path, Path::new(""))
            .to_string_lossy()
            .to_lowercase()
    };
    let covers =
        |entry: &&PathBuf| agents_off::covering(std::slice::from_ref(*entry), folder).is_some();
    let (own, parents): (Vec<&PathBuf>, Vec<&PathBuf>) = entries
        .iter()
        .filter(covers)
        .partition(|entry| lower(entry) == lower(folder));
    if let Some(parent) = parents.first() {
        return Kept::Under((*parent).clone());
    }
    if !own.is_empty() {
        return Kept::Own;
    }
    match agents_off::blocking(entries, folder) {
        Some(Block::Contains(entry)) => Kept::Contains(entry),
        _ => Kept::Open,
    }
}

/// A registered repository, as Home lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Project<'a> {
    pub id: &'a str,
    pub name: &'a str,
    /// The repository's folder.
    pub path: &'a str,
    /// Whether coding agents are kept out of the folder.
    pub kept: &'a Kept,
}

/// What Home is made from.
pub struct HomeInput<'a> {
    /// The registered repositories whose folders are there.
    pub projects: &'a [Project<'a>],
    /// Folders of the repositories opened in the app, newest first.
    pub recent: &'a [String],
    /// Id of the open repository.
    pub current: Option<&'a str>,
    /// Name of the open repository, when it has no wiki yet.
    pub no_wiki: Option<&'a str>,
    /// The registered repositories whose folders are not found.
    pub missing: &'a [Project<'a>],
    /// The newer version that waits to be installed. Home has no toolbar,
    /// where the offer otherwise is.
    pub update: Option<&'a str>,
}

/// The page the app starts on: a way to open a folder, and every registered
/// repository, the ones opened most recently first.
pub fn home_page(home: &HomeInput) -> Doc {
    // Home fills the window, so it carries the app's name, as a welcome
    // screen does.
    let mut doc = Doc::new("CodeWiki").lead("Open a project, or pick up one you opened before.");
    doc.push(Block::Logo);
    if let Some(version) = home.update {
        doc.push(Block::Alert {
            tone: Tone::Info,
            text: format!("CodeWiki {version} is available."),
            button: Some((format!("Update to {version}"), Action::InstallUpdate)),
        });
    }
    if let Some(name) = home.no_wiki {
        doc.push(Block::Alert {
            tone: Tone::Info,
            text: format!("{name} has no wiki yet. Sync indexes its code and writes one."),
            button: Some(("Sync".into(), Action::Sync)),
        });
    }
    doc.push(Block::Rows(vec![
        Row::new(icons::FOLDER, "Open folder…", Action::OpenFolder).detail(
            "Choose a project. Sync then indexes its code and writes the wiki into an \
             obsidian folder inside it.",
        ),
    ]));
    if home.projects.is_empty() && home.missing.is_empty() {
        doc.push(Block::Empty(
            "No projects yet. Open a folder to add one.".into(),
        ));
        return doc;
    }
    if !home.projects.is_empty() {
        doc.heading("Recent projects");
        doc.push(Block::Rows(
            by_recency(home.projects, home.recent)
                .into_iter()
                .map(|project| {
                    let open = home.current == Some(project.id);
                    // The marks go first: a long path is cut short at its end.
                    let mut detail = String::new();
                    if open {
                        detail.push_str("Open now · ");
                    }
                    // Short, so that the project's own path stays readable;
                    // Settings lists the entry in full.
                    let name = |entry: &PathBuf| {
                        entry.file_name().map_or_else(
                            || entry.display().to_string(),
                            |n| n.to_string_lossy().into_owned(),
                        )
                    };
                    match project.kept {
                        Kept::Open => {}
                        Kept::Own => detail.push_str("Kept from agents · "),
                        Kept::Under(entry) => {
                            detail.push_str(&format!("Kept from agents by {} · ", name(entry)));
                        }
                        Kept::Contains(entry) => {
                            detail.push_str(&format!(
                                "Kept from agents (contains {}) · ",
                                name(entry)
                            ));
                        }
                    }
                    detail.push_str(project.path);
                    let row = Row::new(
                        icons::REPOSITORY,
                        project.name,
                        Action::OpenRepo(project.id.to_string()),
                    )
                    .detail(detail)
                    .selected(open);
                    // A parent entry is changed in Settings: removing it
                    // would let agents into other projects too. So is an
                    // entry inside the project, which is not the project's.
                    match project.kept {
                        Kept::Open => row.aside(
                            "Keep agents out",
                            Action::KeepAgentsOut(project.path.to_string()),
                        ),
                        Kept::Own => row.aside(
                            "Let agents in",
                            Action::LetAgentsIn(project.path.to_string()),
                        ),
                        Kept::Under(_) | Kept::Contains(_) => {
                            row.aside("Settings…", Action::Settings(Some(Category::Repositories)))
                        }
                    }
                })
                .collect(),
        ));
    }
    if !home.missing.is_empty() {
        // A folder can be away for a while: a drive that is not plugged in,
        // a share that is offline. So the project stays until the user
        // removes it.
        doc.heading("Folder not found");
        doc.push(Block::Text(vec![text(
            "These projects stay on the list and open again once their folders are back. \
             Remove takes a project off the list; nothing on disk is deleted.",
        )]));
        let mut missing = home.missing.to_vec();
        missing.sort_by_key(|project| project.name.to_lowercase());
        for project in missing {
            doc.push(Block::Setting {
                label: project.name.into(),
                value: project.path.into(),
                button: Some(("Remove".into(), Action::ForgetRepo(project.id.to_string()))),
            });
        }
    }
    doc
}

/// `projects` in the order of `recent`, then the ones never opened in the
/// app, by name.
fn by_recency<'a>(projects: &[Project<'a>], recent: &[String]) -> Vec<Project<'a>> {
    let mut ordered: Vec<Project> = recent
        .iter()
        .filter_map(|path| projects.iter().find(|project| project.path == path))
        .copied()
        .collect();
    let mut rest: Vec<Project> = projects
        .iter()
        .filter(|project| !recent.iter().any(|path| path == project.path))
        .copied()
        .collect();
    rest.sort_by_key(|project| project.name.to_lowercase());
    ordered.extend(rest);
    ordered
}

/// The page shown when no backend could be reached: where its log is, and a
/// button that tries again.
///
/// `remote` is the address of the backend saved in the settings, when that
/// is the one that did not answer. The app does not go back to a backend on
/// this machine by itself, which would show other wikis or none; the page
/// offers it.
pub fn offline_page(log: &str, logs: &str, remote: Option<&str>) -> Doc {
    let mut doc = message_page("The backend is not reachable.");
    if let Some(remote) = remote {
        doc.push(Block::Text(vec![
            text("The backend at "),
            code(remote),
            text(", chosen in Settings, does not answer."),
        ]));
        doc.push(Block::Buttons(vec![
            Press::new("Use local backend", Action::UseLocalBackend),
            Press::new("Open settings", Action::Settings(Some(Category::Backend))),
        ]));
    }
    doc.push(Block::Setting {
        label: "Backend log".into(),
        value: log.into(),
        button: Some(("Try again".into(), Action::Reconnect)),
    });
    doc.push(Block::Setting {
        label: "Logs folder".into(),
        value: logs.into(),
        button: Some(("Open logs folder".into(), Action::OpenLogs)),
    });
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_offline_page_names_the_log_and_offers_to_try_again() {
        let page = offline_page(r"C:\data\logs\backend.log", r"C:\data\logs", None);
        assert!(page.words().contains("not reachable"));
        assert!(page.words().contains(r"C:\data\logs\backend.log"));
        assert!(page.words().contains(r"Logs folder: C:\data\logs"));
        assert_eq!(page.actions(), [Action::Reconnect, Action::OpenLogs]);
    }

    #[test]
    fn a_saved_backend_that_does_not_answer_offers_the_local_one() {
        let page = offline_page(
            r"C:\data\logs\backend.log",
            r"C:\data\logs",
            Some("http://nas:7411"),
        );
        let words = page.words();
        assert!(
            words.contains("The backend at http://nas:7411, chosen in Settings, does not answer."),
            "{words}"
        );
        assert!(words.contains("Use local backend | Open settings"));
        assert_eq!(
            page.actions(),
            [
                Action::UseLocalBackend,
                Action::Settings(Some(Category::Backend)),
                Action::Reconnect,
                Action::OpenLogs
            ]
        );
    }

    fn home<'a>(projects: &'a [Project<'a>], recent: &'a [String]) -> HomeInput<'a> {
        HomeInput {
            projects,
            recent,
            current: None,
            no_wiki: None,
            missing: &[],
            update: None,
        }
    }

    const ALPHA: Project = Project {
        id: "a1",
        name: "alpha",
        path: r"C:\code\alpha",
        kept: &Kept::Open,
    };
    const BETA: Project = Project {
        id: "b2",
        name: "Beta",
        path: r"C:\code\beta",
        kept: &Kept::Open,
    };
    const GAMMA: Project = Project {
        id: "g3",
        name: "gamma",
        path: r"D:\gamma",
        kept: &Kept::Open,
    };

    #[test]
    fn home_without_projects_asks_for_a_folder() {
        let page = home_page(&home(&[], &[]));
        assert_eq!(page.title, "CodeWiki");
        assert_eq!(page.blocks.first(), Some(&Block::Logo));
        assert!(page.words().contains("Open folder…"));
        assert!(page.words().contains("No projects yet"));
        assert!(page.headings().is_empty());
        assert_eq!(page.actions(), [Action::OpenFolder]);
    }

    #[test]
    fn home_lists_recent_projects_first_then_the_rest_by_name() {
        // A folder in the recent list that is no longer registered is left
        // out.
        let recent = [GAMMA.path.to_string(), r"C:\gone".to_string()];
        let page = home_page(&home(&[ALPHA, BETA, GAMMA], &recent));
        assert_eq!(page.headings(), ["Recent projects"]);
        assert_eq!(
            page.actions(),
            [
                Action::OpenFolder,
                Action::OpenRepo("g3".into()),
                Action::KeepAgentsOut(r"D:\gamma".into()),
                Action::OpenRepo("a1".into()),
                Action::KeepAgentsOut(r"C:\code\alpha".into()),
                Action::OpenRepo("b2".into()),
                Action::KeepAgentsOut(r"C:\code\beta".into()),
            ]
        );
        assert!(page.words().contains(r"gamma — D:\gamma"));
    }

    #[test]
    fn home_marks_the_open_project() {
        let page = home_page(&HomeInput {
            current: Some("b2"),
            ..home(&[ALPHA, BETA], &[])
        });
        let Some(Block::Rows(rows)) = page.blocks.last() else {
            panic!("the projects are not the last block: {page:?}");
        };
        let selected: Vec<&str> = rows
            .iter()
            .filter(|row| row.selected)
            .map(|row| row.label.as_str())
            .collect();
        assert_eq!(selected, ["Beta"]);
        assert!(page.words().contains(r"Beta — Open now · C:\code\beta"));
        assert!(page.words().contains(r"alpha — C:\code\alpha"));
        assert_eq!(page.words().matches("Open now").count(), 1);
    }

    fn aside_of<'a>(page: &'a Doc, label: &str) -> Option<&'a (String, Action)> {
        page.blocks.iter().find_map(|block| match block {
            Block::Rows(rows) => rows
                .iter()
                .find(|row| row.label == label)
                .and_then(|row| row.aside.as_ref()),
            _ => None,
        })
    }

    #[test]
    fn home_offers_keep_agents_out() {
        let page = home_page(&home(&[ALPHA], &[]));
        assert_eq!(
            aside_of(&page, "alpha"),
            Some(&(
                "Keep agents out".to_string(),
                Action::KeepAgentsOut(r"C:\code\alpha".into())
            ))
        );
        assert!(!page.words().contains("ept from agents"));
    }

    #[test]
    fn home_offers_let_agents_in_for_an_exact_entry() {
        let own = Project {
            kept: &Kept::Own,
            ..ALPHA
        };
        let page = home_page(&home(&[own, BETA], &[]));
        assert_eq!(
            aside_of(&page, "alpha"),
            Some(&(
                "Let agents in".to_string(),
                Action::LetAgentsIn(r"C:\code\alpha".into())
            ))
        );
        // Another project's row is its own.
        assert_eq!(
            aside_of(&page, "Beta").map(|a| a.0.as_str()),
            Some("Keep agents out")
        );
    }

    #[test]
    fn home_names_the_parent_entry_and_offers_no_button() {
        let parent = Kept::Under(PathBuf::from(r"C:\code"));
        let under = Project {
            kept: &parent,
            ..ALPHA
        };
        for current in [None, Some("a1")] {
            let page = home_page(&HomeInput {
                current,
                ..home(&[under], &[])
            });
            // No button that changes the list; only a way to Settings.
            assert_eq!(
                aside_of(&page, "alpha"),
                Some(&(
                    "Settings…".to_string(),
                    Action::Settings(Some(Category::Repositories))
                ))
            );
            assert!(
                page.words()
                    .contains(r"Kept from agents by code · C:\code\alpha"),
                "{}",
                page.words()
            );
            assert!(!page.words().contains(r"by C:\code"));
            assert!(
                page.actions()
                    .iter()
                    .all(|a| !matches!(a, Action::KeepAgentsOut(_) | Action::LetAgentsIn(_)))
            );
        }
    }

    #[test]
    fn an_exact_entry_gets_a_short_marker() {
        let own = Project {
            kept: &Kept::Own,
            ..ALPHA
        };
        let page = home_page(&home(&[own], &[]));
        assert!(
            page.words()
                .contains(r"alpha — Kept from agents · C:\code\alpha"),
            "{}",
            page.words()
        );
    }

    #[test]
    fn kept_by_tells_an_exact_entry_from_a_parent_and_a_subfolder() {
        let entries = [
            PathBuf::from(r"C:\work\exact"),
            PathBuf::from(r"C:\parent"),
            PathBuf::from(r"C:\work\inside\sub"),
        ];
        assert_eq!(kept_by(&entries, r"C:\WORK\exact"), Kept::Own);
        assert_eq!(
            kept_by(&entries, r"C:\parent\child"),
            Kept::Under(PathBuf::from(r"C:\parent"))
        );
        assert_eq!(
            kept_by(&entries, r"C:\work\inside"),
            Kept::Contains(PathBuf::from(r"C:\work\inside\sub"))
        );
        assert_eq!(kept_by(&entries, r"C:\work\insid"), Kept::Open);
        assert_eq!(kept_by(&[], r"C:\work\exact"), Kept::Open);
    }

    #[test]
    fn an_exact_entry_under_a_parent_entry_is_kept_by_the_parent() {
        let child = PathBuf::from(r"C:\parent\child");
        let parent = PathBuf::from(r"C:\parent");
        // Either order in the file: removing the child's entry would not let
        // agents in, so Home offers no toggle.
        for entries in [
            [child.clone(), parent.clone()],
            [parent.clone(), child.clone()],
        ] {
            assert_eq!(
                kept_by(&entries, r"C:\parent\child"),
                Kept::Under(parent.clone())
            );
        }
        assert_eq!(kept_by(&[child, parent], r"C:\parent"), Kept::Own);
    }

    #[test]
    fn home_names_an_entry_inside_and_offers_no_button() {
        let inside = Kept::Contains(PathBuf::from(r"C:\code\alpha\vendor\acme"));
        let holder = Project {
            kept: &inside,
            ..ALPHA
        };
        let page = home_page(&home(&[holder], &[]));
        assert_eq!(
            aside_of(&page, "alpha"),
            Some(&(
                "Settings…".to_string(),
                Action::Settings(Some(Category::Repositories))
            ))
        );
        assert!(
            page.words()
                .contains(r"alpha — Kept from agents (contains acme) · C:\code\alpha"),
            "{}",
            page.words()
        );
        assert!(
            page.actions()
                .iter()
                .all(|a| !matches!(a, Action::KeepAgentsOut(_) | Action::LetAgentsIn(_)))
        );
    }

    #[test]
    fn home_offers_sync_when_the_open_project_has_no_wiki() {
        let page = home_page(&HomeInput {
            current: Some("a1"),
            no_wiki: Some("alpha"),
            ..home(&[ALPHA], &[])
        });
        assert!(page.words().contains("alpha has no wiki yet"));
        assert_eq!(page.actions()[0], Action::Sync);

        let page = home_page(&HomeInput {
            current: Some("a1"),
            ..home(&[ALPHA], &[])
        });
        assert!(!page.words().contains("no wiki yet"));
        assert!(!page.actions().contains(&Action::Sync));
    }

    #[test]
    fn home_keeps_a_project_whose_folder_is_not_found_and_offers_to_remove_it() {
        let page = home_page(&HomeInput {
            missing: &[GAMMA, BETA],
            ..home(&[ALPHA], &[])
        });
        assert_eq!(page.headings(), ["Recent projects", "Folder not found"]);
        assert!(page.words().contains(r"Beta: C:\code\beta"));
        assert!(page.words().contains(r"gamma: D:\gamma"));
        // It cannot be opened, only removed, and the list is by name.
        assert_eq!(
            page.actions(),
            [
                Action::OpenFolder,
                Action::OpenRepo("a1".into()),
                Action::KeepAgentsOut(r"C:\code\alpha".into()),
                Action::ForgetRepo("b2".into()),
                Action::ForgetRepo("g3".into()),
            ]
        );

        // With no folder there at all, the missing ones still show.
        let page = home_page(&HomeInput {
            missing: &[GAMMA],
            ..home(&[], &[])
        });
        assert_eq!(page.headings(), ["Folder not found"]);
        assert!(!page.words().contains("No projects yet"));

        let page = home_page(&home(&[ALPHA], &[]));
        assert!(!page.words().contains("not found"));
    }

    #[test]
    fn home_offers_a_waiting_update() {
        let page = home_page(&HomeInput {
            update: Some("0.2.0"),
            ..home(&[ALPHA], &[])
        });
        assert!(page.words().contains("CodeWiki 0.2.0 is available."));
        assert_eq!(page.actions()[0], Action::InstallUpdate);

        let page = home_page(&home(&[ALPHA], &[]));
        assert!(!page.actions().contains(&Action::InstallUpdate));
    }

    #[test]
    fn renders_hotspots_and_file_pages() {
        let rows = json!([{"score": 100.0, "path": "src/core.rs", "commits": 6, "lines": 40,
            "top_owner": "Ann", "bus_factor": 1}]);
        let page = hotspots_page(&rows);
        assert_eq!(
            page.tables()[0].plain(),
            [["100.0", "src/core.rs", "6", "40", "Ann", "1"]]
        );
        assert_eq!(page.actions(), [Action::File("src/core.rs".into())]);
        assert!(
            hotspots_page(&json!([]))
                .words()
                .contains("No commit history")
        );

        let context = json!({"path": "src/core.rs", "language": "rust", "lines": 40,
            "symbols": [
                {"qualified": "demo::core", "kind": "module", "signature": "mod core"},
                {"qualified": "demo::core::compute", "kind": "function", "signature": "pub fn compute(x: u64) -> u64"}],
            "imports": [], "notes": ["01-Core/demo/Core Module.md"]});
        let risk = json!({"level": "high", "score": 70, "reasons": ["2 files depend on it (1 directly)"],
            "history": {"owners": [{"name": "Ann", "commits": 6, "share": 1.0}],
                "partners": [{"path": "src/api.rs", "together": 6}]}});
        let radius = json!({"direct": ["src/api.rs"], "transitive": ["src/cli.rs"]});
        let page = file_page(&context, &risk, &radius);
        let words = page.words();
        assert!(words.starts_with("core.rs\nRisk: high\nLines: 40\nLanguage: rust\nsrc/core.rs\n"));
        assert!(words.contains("2 files depend on it (1 directly)"));
        assert_eq!(page.tables()[0].plain(), [["Ann", "6", "100%"]]);
        assert!(words.contains("src/api.rs — together 6 times"));
        assert!(words.contains("Reached through further hops (1):"));
        assert!(words.contains("compute — pub fn compute(x: u64) -> u64"));
        assert!(
            !words.contains("mod core"),
            "the module's own symbol is not listed"
        );
        assert_eq!(
            page.headings(),
            [
                "Risk",
                "Ownership",
                "Usually changes with",
                "Blast radius",
                "Symbols",
                "Notes"
            ],
            "empty sections are dropped"
        );
        let actions = page.actions();
        assert!(actions.contains(&Action::Symbol("demo::core::compute".into())));
        assert!(actions.contains(&Action::Note("01-Core/demo/Core Module".into())));
        assert!(actions.contains(&Action::File("src/cli.rs".into())));
    }

    #[test]
    fn renders_symbol_and_search_pages() {
        let detail = json!({
            "symbol": {"name": "double", "qualified": "demo::math::double", "kind": "function",
                "visibility": "public", "signature": "pub fn double(x: u64) -> u64",
                "doc": "Doubles a number.", "file": "src/math.rs", "start_line": 4, "end_line": 4},
            "members": [],
            "depends_on": [],
            "used_by": [{"qualified": "demo::run", "relation": "calls", "file": "src/lib.rs"}],
            "notes": ["01-Core/demo/Math Module.md"],
        });
        let page = symbol_page(&detail);
        let words = page.words();
        assert!(words.starts_with("double\ndemo::math::double [function, public]\n"));
        assert!(words.contains("pub fn double(x: u64) -> u64"));
        assert!(words.contains("Doubles a number."));
        assert!(words.contains("src/math.rs — lines 4 to 4"));
        assert_eq!(
            page.headings(),
            ["demo::math::double", "Used by", "Notes"],
            "empty sections are dropped"
        );
        assert!(words.contains("run — calls demo::run in src/lib.rs"));
        assert_eq!(
            page.actions(),
            [
                Action::File("src/math.rs".into()),
                Action::Symbol("demo::run".into()),
                Action::Note("01-Core/demo/Math Module".into())
            ]
        );

        let hits = json!([
            {"kind": "note", "key": "a/B.md", "title": "B", "snippet": "x\ny"},
            {"kind": "symbol", "key": "demo::run"},
            {"kind": "file", "key": "src/lib.rs"},
            {"kind": "commit", "key": "abc"}
        ]);
        let search = search_page("q", &hits);
        assert_eq!(search.title, "Search: q");
        assert_eq!(search.headings(), ["Notes", "Symbols", "Files"]);
        assert!(search.words().contains("B — x y"));
        assert_eq!(
            search.actions(),
            [
                Action::Note("a/B".into()),
                Action::Symbol("demo::run".into()),
                Action::File("src/lib.rs".into())
            ]
        );
        assert!(search_page("q", &json!([])).words().contains("No results."));
    }

    #[test]
    fn a_proposal_shows_its_patch_as_a_diff() {
        let page = proposal_page(&json!({
            "proposal": {"note_path": "01-Core/Parser.md", "reason": "source changed"},
            "patch": "@@ -1 +1 @@\n-old\n+new\n\n"
        }));
        let words = page.words();
        assert!(words.contains("01-Core/Parser.md — source changed"));
        assert!(words.contains("Nothing is written until you approve."));
        let diff = page.blocks.iter().find_map(|block| match block {
            Block::Code { language, text } if language == "diff" => Some(text.as_str()),
            _ => None,
        });
        assert_eq!(diff, Some("@@ -1 +1 @@\n-old\n+new"));
        assert!(!words.contains("picture"));
    }

    #[test]
    fn a_proposal_of_an_agent_says_who_proposes_and_what_comes_with_it() {
        let page = proposal_page(&json!({
            "proposal": {"note_path": "01-Core/Parser.md", "kind": "agent",
                         "reason": "claude-code explained this note"},
            "patch": "@@ -1 +1 @@\n+new\n",
            "pictures": [{"section": "How it works", "source": "root = Picture([])"},
                         {"section": "Gotchas", "source": "root = Picture([])"}],
        }));
        let words = page.words();
        assert!(words.contains("01-Core/Parser.md — claude-code explained this note"));
        assert!(words.contains("A coding agent wrote an explanation for this note."));
        assert!(words.contains("Nothing is written until you approve."));
        assert!(words.contains("Approving also gives the note 2 pictures."));
        let one = proposal_page(&json!({
            "proposal": {"kind": "agent"}, "pictures": [{"section": "Gotchas"}],
        }));
        assert!(one.words().contains("gives the note 1 picture."));
    }

    #[test]
    fn the_agent_view_says_what_it_leaves_out() {
        assert_eq!(
            agent_view_line(&["How it works".to_string()]),
            "Agent view: the note as a coding agent reads it. Left out: How it works, which is a walk-through made by rule that the code shows, and the pictures."
        );
        assert_eq!(
            agent_view_line(&[]),
            "Agent view: the note as a coding agent reads it. The text is whole; the pictures are left out."
        );
    }

    #[test]
    fn the_details_say_where_a_notes_explanation_stands() {
        assert_eq!(
            explanation_lines("waiting", "", &[]),
            ["Facts written by rules. Waits for a coding agent to explain it."]
        );
        assert_eq!(
            explanation_lines("explained", "claude-code", &[]),
            ["Explained by claude-code."]
        );
        assert_eq!(
            explanation_lines("explained", "", &[]),
            ["Explained by a coding agent."]
        );
        assert_eq!(
            explanation_lines("behind", "claude-code", &["src/a.rs changed".into()]),
            [
                "Explained by claude-code.",
                "The explanation may be behind the code:",
                "• src/a.rs changed"
            ]
        );
        // A map of the vault, or a backend that does not say.
        assert!(explanation_lines("none", "", &[]).is_empty());
        assert!(explanation_lines("", "", &[]).is_empty());
    }

    #[test]
    fn a_sync_reports_what_it_did() {
        let page = sync_page(&json!({
            "written": ["a", "b"], "kept": 5, "proposed": ["p"], "removed": [],
            "adopted": [], "notes_total": 8, "link_rate": 0.975
        }));
        let words = page.words();
        assert!(words.contains("2 notes written, 5 unchanged."));
        assert!(words.contains("Queued for review: 1"));
        assert!(words.contains("Notes in all: 8"));
        assert!(words.contains("Links that resolve: 97.5%"));
        // Nothing about explanations when nothing waits.
        assert!(!words.contains("wait for a coding agent"));
        assert!(!words.contains("Not explained yet"));

        let page = sync_page(&json!({
            "written": [], "kept": 8, "proposed": [], "removed": [], "adopted": [],
            "notes_total": 8, "link_rate": 1.0,
            "merged": ["a"], "behind": ["a", "b"], "unexplained": 3
        }));
        let words = page.words();
        assert!(
            words.contains("Facts refreshed, explanation kept: 1"),
            "{words}"
        );
        assert!(words.contains("Explanation may be behind: 2"));
        assert!(words.contains("Not explained yet: 3"));
        assert!(words.contains("5 notes wait for a coding agent to explain them."));
        assert!(words.contains("/codewiki:write-notes"));
    }

    #[test]
    fn a_message_page_says_one_thing() {
        assert!(
            message_page("The backend is not reachable.")
                .words()
                .contains("The backend is not reachable.")
        );
        assert!(message_page("").blocks.is_empty());
    }
}
