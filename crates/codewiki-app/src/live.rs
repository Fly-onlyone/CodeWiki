//! The live pages: one symbol, one file, search results, hotspots, the theme
//! picker, a proposal, the result of a sync, the notes that wait for a coding
//! agent, and Home, the page the app starts on. Like the reports in `pages`,
//! each turns what the backend sent into a [`Doc`]. These show code that the
//! vault deliberately does not hold as notes.

use std::path::{Path, PathBuf};

use cw_core::auto_explain::Count;
use serde_json::Value;

use crate::doc::{
    Action, Aside, Block, Cell, Column, Doc, Line, Press, Row, Stat, Table, code, file, rich,
    strong, text,
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

/// The file name of a note's vault path, without `.md`.
fn note_stem(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".md").unwrap_or(name)
}

/// Rows that open notes, named by the note's file name. The row opens the
/// note by its whole vault path: the backend finds a note by that or by its
/// bare name, and a folder path without `.md` is neither.
fn note_rows(paths: &Value) -> Vec<Row> {
    list(paths)
        .into_iter()
        .filter_map(Value::as_str)
        .map(|path| Row::new(icons::NOTE, note_stem(path), Action::Note(path.to_string())))
        .collect()
}

/// The kinds of search hit, in the order they are listed, with the heading
/// over each.
const HIT_GROUPS: [(&str, &str); 3] = [("note", "Notes"), ("symbol", "Symbols"), ("file", "Files")];

/// The hits of `GET /search` that the app opens, under the heading of their
/// kind, in the order of [`HIT_GROUPS`] and then of the backend's ranking. A
/// kind with no hit has no group.
pub fn hit_groups(hits: &Value) -> Vec<(&'static str, Vec<HitTarget>)> {
    let hits = list(hits);
    HIT_GROUPS
        .into_iter()
        .filter_map(|(kind, heading)| {
            let targets: Vec<HitTarget> = hits
                .iter()
                .filter(|hit| hit["kind"] == kind)
                .filter_map(|hit| hit_target(hit))
                .collect();
            (!targets.is_empty()).then_some((heading, targets))
        })
        .collect()
}

/// What a search hit opens, and how a list names it.
#[derive(Debug, Clone, PartialEq)]
pub struct HitTarget {
    pub icon: icons::Glyph,
    pub label: String,
    /// A line under the label: the note's text around the match, or the
    /// symbol's qualified name. Empty when there is none.
    pub detail: String,
    /// The label is code: a symbol or a path.
    pub code: bool,
    pub action: Action,
}

/// What `hit`, one result of `GET /search`, opens; `None` for a kind of hit
/// the app does not open.
pub fn hit_target(hit: &Value) -> Option<HitTarget> {
    let key = string(hit, "key");
    let (icon, label, detail, code, action) = match string(hit, "kind") {
        "note" => (
            icons::NOTE,
            hit["title"]
                .as_str()
                .filter(|title| !title.is_empty())
                .unwrap_or(note_stem(key))
                .to_string(),
            plain_snippet(string(hit, "snippet")),
            false,
            Action::Note(key.to_string()),
        ),
        "symbol" => (
            icons::SYMBOL,
            key.rsplit("::").next().unwrap_or(key).to_string(),
            key.to_string(),
            true,
            Action::Symbol(key.to_string()),
        ),
        "file" => (
            icons::FILE,
            key.to_string(),
            String::new(),
            true,
            Action::File(key.to_string()),
        ),
        _ => return None,
    };
    Some(HitTarget {
        icon,
        label,
        detail,
        code,
        action,
    })
}

/// The Markdown of a note's search snippet as plain text on one line: no
/// frontmatter, heading or quote marks, list bullets, rules, table pipes,
/// link brackets, code ticks or `**` emphasis. A snippet is a window of the note, with its lines
/// kept (a text match) or flattened (the opening of a note only the embedder
/// found), so a block or link it cuts off keeps its text.
pub fn plain_snippet(snippet: &str) -> String {
    let mut text = snippet.trim_start();
    if let Some(rest) = text.strip_prefix("---") {
        // Frontmatter with its closing fence: none of it is prose.
        text = match rest.find("---") {
            Some(end) => &rest[end + 3..],
            None => rest,
        };
    }
    let unlinked = unlink(text);
    let marks = |c: char| matches!(c, '#' | '>' | '-' | '*' | '+' | '|' | ':');
    unlinked
        .split_whitespace()
        .filter(|word| !word.chars().all(marks))
        .collect::<Vec<_>>()
        .join(" ")
        .replace('`', "")
        .replace("**", "")
}

/// `text` with each `[[target]]` written as its target, and each
/// `[[target|alias]]` as its alias.
fn unlink(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        out.push_str(&rest[..start]);
        let link = &rest[start + 2..];
        match link.find("]]") {
            Some(end) => {
                let inner = &link[..end];
                out.push_str(inner.rsplit('|').next().unwrap_or(inner));
                rest = &link[end + 2..];
            }
            None => {
                out.push_str(link);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
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
    if list(hits).is_empty() {
        doc.push(Block::Empty("No results.".into()));
        return doc;
    }
    for (heading, targets) in hit_groups(hits) {
        let rows = targets
            .into_iter()
            .map(|target| {
                let row = Row::new(target.icon, target.label, target.action);
                let row = if target.code { row.code() } else { row };
                if target.detail.is_empty() {
                    row
                } else {
                    row.detail(target.detail)
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
    if proposal["kind"] == "retire" {
        return retire_page(proposal);
    }
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

/// Paths of a retired note's that the page names before it counts the rest.
const RETIRE_PATHS_SHOWN: usize = 3;

/// A retire proposal: the notes their owner wrote that name only files that
/// are gone, each with those files. The check boxes that leave a note out
/// stand above this page (`review_view.rs`).
fn retire_page(proposal: &Value) -> Doc {
    let mut doc = Doc::new("Notes about code that is gone");
    doc.push(Block::Text(vec![text(string(proposal, "reason"))]));
    doc.push(Block::Alert {
        tone: Tone::Warning,
        text: "Each of these notes names only files that are no longer in the repository. Approving deletes the ticked notes from the wiki. Git keeps every one of them as it is now. A note you keep stays as history and is not proposed again.".into(),
        button: None,
    });
    let items: Vec<Line> = proposal["retire"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|note| {
            let gone: Vec<&str> = note["cited"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            let mut line = vec![strong(string(note, "path")), text(" — ")];
            for (index, path) in gone.iter().take(RETIRE_PATHS_SHOWN).enumerate() {
                if index > 0 {
                    line.push(text(", "));
                }
                line.push(code(*path));
            }
            line.push(text(match gone.len().saturating_sub(RETIRE_PATHS_SHOWN) {
                0 if gone.len() == 1 => " is gone".to_string(),
                0 => " are gone".to_string(),
                more => format!(" and {more} more are gone"),
            }));
            line
        })
        .collect();
    doc.push(Block::List {
        items,
        ordered: false,
    });
    doc
}

/// The review page when no proposal waits.
pub fn review_empty_page() -> Doc {
    let mut doc = Doc::new("Review");
    doc.push(Block::Text(vec![strong("Nothing waits for review.")]));
    doc.push(Block::Alert {
        tone: Tone::Info,
        text: "A sync proposes a change to a note you edited by hand, and a coding agent proposes its explanation of such a note. Each waits here until you approve or reject it.".into(),
        button: None,
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
    // What became of notes whose subject is gone or moved, in the backend's
    // words; an older backend sends none.
    let gone: Vec<crate::doc::Line> = report["summary"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(crate::doc::rich)
        .collect();
    if !gone.is_empty() {
        doc.push(Block::Heading("Notes of code that is gone or moved".into()));
        doc.push(Block::List {
            items: gone,
            ordered: false,
        });
    }
    doc
}

/// How many notes wait for a coding agent, from the queue of
/// `GET /explanations?all=true`: those behind the code and those never
/// explained. 0 until the queue has been read.
pub fn waiting_count(queue: Option<&Value>) -> u64 {
    queue.map_or(0, |queue| {
        let counts = &queue["waiting"];
        counts["behind"].as_u64().unwrap_or(0) + counts["unexplained"].as_u64().unwrap_or(0)
    })
}

/// The notes that wait for a coding agent to explain them, from the queue of
/// `GET /explanations?all=true`, in the backend's order: those behind the
/// code first. `None` while the queue is being read. `auto` says how many of
/// them agents explain by themselves at the start of a session, or `None`
/// while auto explaining is off.
pub fn waiting_page(queue: Option<&Value>, auto: Option<Count>) -> Doc {
    let mut doc = Doc::new("Waiting notes").lead(
        "These notes wait for a coding agent to explain what they are for and how they work.",
    );
    doc.push(Block::Alert {
        tone: Tone::Info,
        text: match auto {
            Some(Count::All) => "Auto explaining is on: coding agents with the CodeWiki plugin \
                 explain all of these at the start of a session."
                .to_string(),
            Some(Count::Notes(n)) => format!(
                "Auto explaining is on: coding agents with the CodeWiki plugin explain up to {n} \
                 of these at the start of a session. Run /codewiki:write-notes to explain more."
            ),
            None => "Auto explaining is off. Run /codewiki:write-notes in Claude Code to explain \
                 these notes."
                .to_string(),
        },
        button: Some((
            "Settings".into(),
            Action::Settings(Some(Category::Repositories)),
        )),
    });
    let Some(queue) = queue else {
        doc.push(Block::Empty("Reading the waiting notes…".into()));
        return doc;
    };
    let counts = &queue["waiting"];
    let behind = counts["behind"].as_u64().unwrap_or(0);
    doc.push(Block::Stats(vec![
        Stat::new("Behind the code", behind).tone(if behind > 0 {
            Tone::Warning
        } else {
            Tone::Neutral
        }),
        Stat::new(
            "Never explained",
            counts["unexplained"].as_u64().unwrap_or(0),
        ),
        // The backend leaves `claimed` out when no agent holds a note.
        Stat::new("Being written", queue["claimed"].as_u64().unwrap_or(0)),
    ]));
    let rows: Vec<Row> = list(&queue["notes"])
        .into_iter()
        .filter_map(|note| {
            let path = note["path"].as_str()?;
            let title = note["title"]
                .as_str()
                .filter(|title| !title.is_empty())
                .unwrap_or(note_stem(path));
            let mut detail = if note["why"] == "behind" {
                "behind the code"
            } else {
                "never explained"
            }
            .to_string();
            if note["claimed"].as_bool().unwrap_or(false) {
                detail.push_str(" · an agent is writing it");
            }
            Some(Row::new(icons::NOTE, title, Action::Note(path.to_string())).detail(detail))
        })
        .collect();
    if rows.is_empty() {
        doc.push(Block::Empty("Every note is explained.".into()));
    } else {
        doc.push(Block::Rows(rows));
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
    /// How many linked git worktrees of the repository are registered and
    /// there. They are no projects and have no rows of their own.
    pub worktrees: usize,
    /// For a linked worktree, which is listed only while it is the open
    /// repository: the name of the repository it belongs to.
    pub worktree_of: Option<&'a str>,
}

/// Whether Home shows its search field. The search finds the projects opened
/// before, so it needs one: a folder of `paths`, the projects Home lists,
/// that is in `recent`. The app asks too, before it gives the field the
/// focus: a field that is not drawn would take the keys from everything.
pub fn home_has_search<'a>(mut paths: impl Iterator<Item = &'a str>, recent: &[String]) -> bool {
    paths.any(|path| recent.iter().any(|opened| opened == path))
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
    if home_has_search(
        home.projects.iter().map(|project| project.path),
        home.recent,
    ) {
        doc.push(Block::Search);
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
                    if let Some(main) = project.worktree_of {
                        detail.push_str(&format!("Worktree of {main} · "));
                    }
                    match project.worktrees {
                        0 => {}
                        1 => detail.push_str("1 worktree · "),
                        n => detail.push_str(&format!("{n} worktrees · ")),
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
                    // The shield is filled while the project itself is kept.
                    let settings = Action::Settings(Some(Category::Repositories));
                    let agents = match project.kept {
                        Kept::Open => Aside::new(
                            icons::KEEP_OUT,
                            "Keep agents out",
                            Action::KeepAgentsOut(project.path.to_string()),
                        ),
                        Kept::Own => Aside::new(
                            icons::KEPT,
                            "Let agents in",
                            Action::LetAgentsIn(project.path.to_string()),
                        )
                        .on(true),
                        Kept::Under(_) => Aside::new(
                            icons::KEPT,
                            "Kept by a folder above. Change it in Settings",
                            settings,
                        )
                        .on(true),
                        Kept::Contains(_) => Aside::new(
                            icons::KEEP_OUT,
                            "A folder inside is kept. Change it in Settings",
                            settings,
                        ),
                    };
                    // Takes the project off the list, the open one too, which
                    // then closes. Nothing on disk is deleted.
                    row.aside(agents).aside(Aside::new(
                        icons::CLOSE,
                        "Remove from list",
                        Action::ForgetRepo(project.id.to_string()),
                    ))
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

/// How many projects Home's search suggests.
pub const RECENT_MATCHES: usize = 8;

/// What Home's search suggests for `query`, ignoring case: the projects of
/// `recent` whose name holds it, then those whose folder matches it
/// ([`path_matches`]), each newest first. A folder that is not among
/// `projects`, the registered ones that are there, is left out, and so is
/// every project never opened in the app.
pub fn recent_matches(projects: &[Project], recent: &[String], query: &str) -> Vec<HitTarget> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let (mut by_name, mut by_path) = (Vec::new(), Vec::new());
    for project in recent
        .iter()
        .filter_map(|path| projects.iter().find(|project| project.path == path))
    {
        if project.name.to_lowercase().contains(&query) {
            by_name.push(project);
        } else if path_matches(project.path, &query) {
            by_path.push(project);
        }
    }
    by_name
        .into_iter()
        .chain(by_path)
        .take(RECENT_MATCHES)
        .map(|project| HitTarget {
            icon: icons::REPOSITORY,
            label: project.name.to_string(),
            detail: project.path.to_string(),
            code: false,
            action: Action::OpenRepo(project.id.to_string()),
        })
        .collect()
}

/// Where a word of a folder path starts: after one of these.
const WORD_BREAKS: [char; 6] = ['\\', '/', '-', '_', '.', ' '];

/// Whether the folder `path` matches `query`, which is lowercase. Only
/// where a folder name, or a word in one, starts with it: the folders every
/// path goes through, such as `AppData\Local`, would otherwise match a
/// query as short as "al". A query that holds a separator, such as
/// `src\api`, names a piece of the path and matches anywhere in it.
fn path_matches(path: &str, query: &str) -> bool {
    let path = path.to_lowercase();
    if query.contains(['\\', '/']) {
        return path.contains(query);
    }
    path.match_indices(query)
        .any(|(at, _)| at == 0 || path[..at].ends_with(WORD_BREAKS))
}

/// The page shown when no backend could be reached: that the app keeps
/// looking for one, where its log is, and a button that tries again at once.
///
/// `remote` is the address of the backend saved in the settings, when that
/// is the one that did not answer. The app does not go back to a backend on
/// this machine by itself, which would show other wikis or none; the page
/// offers it.
pub fn offline_page(log: &str, logs: &str, remote: Option<&str>) -> Doc {
    let mut doc = message_page("The backend is not reachable.");
    doc.push(Block::Text(vec![text("CodeWiki keeps trying to connect.")]));
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
        // The app looks for the backend by itself; the button need not be
        // pressed.
        assert!(
            page.words().contains("CodeWiki keeps trying to connect."),
            "{}",
            page.words()
        );
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
        worktrees: 0,
        worktree_of: None,
    };
    const BETA: Project = Project {
        id: "b2",
        name: "Beta",
        path: r"C:\code\beta",
        kept: &Kept::Open,
        worktrees: 0,
        worktree_of: None,
    };
    const GAMMA: Project = Project {
        id: "g3",
        name: "gamma",
        path: r"D:\gamma",
        kept: &Kept::Open,
        worktrees: 0,
        worktree_of: None,
    };

    #[test]
    fn a_project_says_how_many_worktrees_it_has_and_an_open_worktree_whose_it_is() {
        let main = Project {
            worktrees: 2,
            ..ALPHA
        };
        let side = Project {
            id: "w7",
            name: "ui-port",
            path: r"C:\code\alpha\.claude\worktrees\ui-port",
            worktree_of: Some("alpha"),
            ..ALPHA
        };
        let page = home_page(&HomeInput {
            current: Some("w7"),
            ..home(&[main, side], &[])
        });
        let words = page.words();
        assert!(
            words.contains(r"alpha — 2 worktrees · C:\code\alpha"),
            "{words}"
        );
        assert!(
            words.contains(
                r"ui-port — Open now · Worktree of alpha · C:\code\alpha\.claude\worktrees\ui-port"
            ),
            "{words}"
        );
        let one = Project {
            worktrees: 1,
            ..ALPHA
        };
        assert!(
            home_page(&home(&[one], &[]))
                .words()
                .contains(r"alpha — 1 worktree · C:\code\alpha")
        );
    }

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
                Action::ForgetRepo("g3".into()),
                Action::OpenRepo("a1".into()),
                Action::KeepAgentsOut(r"C:\code\alpha".into()),
                Action::ForgetRepo("a1".into()),
                Action::OpenRepo("b2".into()),
                Action::KeepAgentsOut(r"C:\code\beta".into()),
                Action::ForgetRepo("b2".into()),
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

    #[test]
    fn home_search_sits_above_open_folder_once_a_project_was_opened() {
        let recent = [BETA.path.to_string()];
        let page = home_page(&home(&[ALPHA, BETA], &recent));
        let search = page.blocks.iter().position(|b| *b == Block::Search);
        let folder = page.blocks.iter().position(|b| matches!(b, Block::Rows(_)));
        assert_eq!(search, Some(1), "under the logo: {page:?}");
        assert_eq!(folder, Some(2));
        // Never opened, or opened but no longer registered: nothing to find.
        for recent in [vec![], vec![r"C:\gone".to_string()]] {
            let page = home_page(&home(&[ALPHA, BETA], &recent));
            assert!(!page.blocks.contains(&Block::Search), "{page:?}");
        }
        assert!(!home_page(&home(&[], &[])).blocks.contains(&Block::Search));
    }

    #[test]
    fn home_has_its_search_only_with_a_listed_project_that_was_opened() {
        let recent = [BETA.path.to_string(), r"C:\gone".to_string()];
        assert!(home_has_search(
            [ALPHA.path, BETA.path].into_iter(),
            &recent
        ));
        // Opened before, but not among the projects Home lists.
        assert!(!home_has_search([ALPHA.path].into_iter(), &recent));
        assert!(!home_has_search([ALPHA.path, BETA.path].into_iter(), &[]));
        assert!(!home_has_search(std::iter::empty(), &recent));
    }

    fn labels(hits: &[HitTarget]) -> Vec<&str> {
        hits.iter().map(|hit| hit.label.as_str()).collect()
    }

    #[test]
    fn recent_matches_finds_name_or_folder_ignoring_case_newest_first() {
        let recent = [
            GAMMA.path.to_string(),
            BETA.path.to_string(),
            ALPHA.path.to_string(),
        ];
        let projects = [ALPHA, BETA, GAMMA];
        assert_eq!(labels(&recent_matches(&projects, &recent, "BET")), ["Beta"]);
        // `code` is in the folders of alpha and Beta only.
        assert_eq!(
            labels(&recent_matches(&projects, &recent, " code ")),
            ["Beta", "alpha"]
        );
        assert_eq!(
            labels(&recent_matches(&projects, &recent, "a")),
            ["gamma", "Beta", "alpha"]
        );
        let hit = &recent_matches(&projects, &recent, "gam")[0];
        assert_eq!(hit.detail, r"D:\gamma");
        assert_eq!(hit.action, Action::OpenRepo("g3".into()));
        assert!(recent_matches(&projects, &recent, "zzz").is_empty());
        assert!(recent_matches(&projects, &recent, "  ").is_empty());
    }

    #[test]
    fn recent_matches_finds_a_folder_only_where_a_word_starts() {
        const BASE: &str = r"C:\Users\PC\AppData\Local\Temp\work";
        let paths =
            ["alpha-tools", "beta-service", "wiki-demo"].map(|name| format!(r"{BASE}\{name}"));
        let projects: Vec<Project> = paths
            .iter()
            .zip(["alpha-tools", "beta-service", "wiki-demo"])
            .map(|(path, name)| Project {
                id: name,
                name,
                path,
                kept: &Kept::Open,
                worktrees: 0,
                worktree_of: None,
            })
            .collect();
        let find = |query: &str| labels(&recent_matches(&projects, &paths, query)).join(",");
        // "Local" holds "al" mid-word; only the name alpha-tools starts it.
        assert_eq!(find("al"), "alpha-tools");
        assert_eq!(find("ppdata"), "", "not where a folder name starts");
        // Folders every path goes through still match from their start.
        assert_eq!(find("local"), "alpha-tools,beta-service,wiki-demo");
        // A word starts after '-', '_', '.' or a space too.
        assert!(path_matches(r"d:\src\beta-service", "serv"));
        assert!(path_matches(r"d:\src\my_app.core", "core"));
        assert!(!path_matches(r"d:\src\beta-service", "ervice"));
        // A separator names a piece of the path, matched anywhere.
        assert_eq!(find(r"rk\wiki"), "wiki-demo");
        assert_eq!(find("temp/"), "", "the path has no '/'");
    }

    #[test]
    fn recent_matches_puts_name_hits_before_folder_hits() {
        let recent = [ALPHA.path.to_string(), BETA.path.to_string()];
        let renamed = Project {
            name: "code-review",
            ..BETA
        };
        // alpha is newer, but only its folder holds "code"; the name of the
        // other project does.
        let hits = recent_matches(&[ALPHA, renamed], &recent, "code");
        assert_eq!(labels(&hits), ["code-review", "alpha"]);
    }

    #[test]
    fn recent_matches_leaves_out_projects_never_opened_or_gone() {
        let recent = [BETA.path.to_string(), r"C:\code\gone".to_string()];
        let hits = recent_matches(&[ALPHA, BETA, GAMMA], &recent, "code");
        assert_eq!(labels(&hits), ["Beta"]);
    }

    #[test]
    fn recent_matches_shows_at_most_eight() {
        let paths: Vec<String> = (0..12).map(|n| format!(r"C:\p\project{n}")).collect();
        let ids: Vec<String> = (0..12).map(|n| format!("id{n}")).collect();
        let projects: Vec<Project> = paths
            .iter()
            .zip(&ids)
            .map(|(path, id)| Project {
                id,
                name: "project",
                path,
                kept: &Kept::Open,
                worktrees: 0,
                worktree_of: None,
            })
            .collect();
        let hits = recent_matches(&projects, &paths, "project");
        assert_eq!(hits.len(), RECENT_MATCHES);
        assert_eq!(hits[0].detail, r"C:\p\project0");
    }

    fn asides_of<'a>(page: &'a Doc, label: &str) -> Option<&'a [Aside]> {
        page.blocks.iter().find_map(|block| match block {
            Block::Rows(rows) => rows
                .iter()
                .find(|row| row.label == label)
                .map(|row| row.asides.as_slice()),
            _ => None,
        })
    }

    /// The first button at the end of the row `label`: its tooltip, its
    /// action, and whether it is drawn switched on.
    fn aside_of<'a>(page: &'a Doc, label: &str) -> Option<(&'a str, &'a Action, bool)> {
        asides_of(page, label)
            .and_then(<[_]>::first)
            .map(|aside| (aside.tooltip.as_str(), &aside.action, aside.on))
    }

    #[test]
    fn home_offers_remove_on_every_row_the_open_one_too() {
        let parent = Kept::Under(PathBuf::from(r"C:\code"));
        let under = Project {
            kept: &parent,
            ..BETA
        };
        let page = home_page(&HomeInput {
            current: Some("a1"),
            missing: &[GAMMA],
            ..home(&[ALPHA, under], &[])
        });
        for (label, id) in [("alpha", "a1"), ("Beta", "b2")] {
            let asides = asides_of(&page, label).unwrap();
            // After the button about agents, which stays first.
            assert_eq!(asides.len(), 2, "{label}");
            assert_eq!(
                asides.last(),
                Some(&Aside::new(
                    icons::CLOSE,
                    "Remove from list",
                    Action::ForgetRepo(id.into())
                )),
                "{label}"
            );
        }
        // A project whose folder is gone keeps its one Remove.
        assert_eq!(
            page.actions()
                .iter()
                .filter(|a| **a == Action::ForgetRepo("g3".into()))
                .count(),
            1
        );
    }

    #[test]
    fn home_offers_keep_agents_out() {
        let page = home_page(&home(&[ALPHA], &[]));
        assert_eq!(
            aside_of(&page, "alpha"),
            Some((
                "Keep agents out",
                &Action::KeepAgentsOut(r"C:\code\alpha".into()),
                false
            ))
        );
        assert_eq!(asides_of(&page, "alpha").unwrap()[0].icon, icons::KEEP_OUT);
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
            Some((
                "Let agents in",
                &Action::LetAgentsIn(r"C:\code\alpha".into()),
                true
            ))
        );
        assert_eq!(asides_of(&page, "alpha").unwrap()[0].icon, icons::KEPT);
        // Another project's row is its own.
        assert_eq!(
            aside_of(&page, "Beta").map(|a| (a.0, a.2)),
            Some(("Keep agents out", false))
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
            // No button that changes the list of kept folders; only a way
            // to Settings.
            // The shield is filled: the project is kept.
            assert_eq!(
                aside_of(&page, "alpha"),
                Some((
                    "Kept by a folder above. Change it in Settings",
                    &Action::Settings(Some(Category::Repositories)),
                    true
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
        // The shield stays outlined: the project itself is not kept.
        assert_eq!(
            aside_of(&page, "alpha"),
            Some((
                "A folder inside is kept. Change it in Settings",
                &Action::Settings(Some(Category::Repositories)),
                false
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
                Action::ForgetRepo("a1".into()),
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
        assert!(actions.contains(&Action::Note("01-Core/demo/Core Module.md".into())));
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
                Action::Note("01-Core/demo/Math Module.md".into())
            ]
        );
        assert!(
            words.contains("Math Module"),
            "a note row is named by its file name"
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
                Action::Note("a/B.md".into()),
                Action::Symbol("demo::run".into()),
                Action::File("src/lib.rs".into())
            ]
        );
        assert!(search_page("q", &json!([])).words().contains("No results."));
    }

    #[test]
    fn a_note_hit_opens_the_note_by_its_vault_path() {
        // The backend finds a note by its whole vault path or by its bare
        // name; a folder path without `.md` is neither.
        let hits = json!([{"kind": "note", "key": "01-Backend/exchange/OKX REST Client.md",
            "title": "OKX REST Client", "snippet": ""}]);
        assert_eq!(
            search_page("okx", &hits).actions(),
            [Action::Note(
                "01-Backend/exchange/OKX REST Client.md".into()
            )]
        );
        let target = hit_target(&hits[0]).unwrap();
        assert_eq!(target.label, "OKX REST Client");
        assert_eq!(
            target.action,
            Action::Note("01-Backend/exchange/OKX REST Client.md".into())
        );
    }

    #[test]
    fn hit_targets_name_what_each_hit_opens() {
        let symbol = hit_target(&json!({"kind": "symbol", "key": "app::okx::OKXClient"})).unwrap();
        assert_eq!(
            (symbol.label.as_str(), symbol.detail.as_str(), symbol.code),
            ("OKXClient", "app::okx::OKXClient", true)
        );
        assert_eq!(symbol.action, Action::Symbol("app::okx::OKXClient".into()));
        let file = hit_target(&json!({"kind": "file", "key": "src/okx.py"})).unwrap();
        assert_eq!((file.label.as_str(), file.code), ("src/okx.py", true));
        assert_eq!(file.action, Action::File("src/okx.py".into()));
        let untitled = hit_target(&json!({"kind": "note", "key": "a/Plain Note.md"})).unwrap();
        assert_eq!(untitled.label, "Plain Note");
        assert!(hit_target(&json!({"kind": "commit", "key": "abc"})).is_none());
    }

    #[test]
    fn a_snippet_reads_as_plain_text() {
        assert_eq!(
            plain_snippet(
                "---\ntags: [integrations]\n---\n# python-okx SDK\n> Official SDK for [[OKX Exchange]] — `REST` **clients**.\n## Used for\n- Placing orders"
            ),
            "python-okx SDK Official SDK for OKX Exchange — REST clients. Used for Placing orders"
        );
        // A hit that only the embedder found carries the note's opening with
        // its whitespace flattened.
        assert_eq!(
            plain_snippet(
                "--- tags: [core, demo] --- # Math Module > Arithmetic helpers. ## Source - `src/math.rs` — module `demo::math` ---"
            ),
            "Math Module Arithmetic helpers. Source src/math.rs — module demo::math"
        );
        assert_eq!(
            plain_snippet("see [[01-Core/Parser|the parser]] and [[Lexer"),
            "see the parser and Lexer",
            "an alias names a link; a link cut off by the snippet keeps its text"
        );
        assert_eq!(
            plain_snippet("---\ntags: [a]"),
            "tags: [a]",
            "frontmatter cut off by the snippet keeps its text"
        );
        assert_eq!(plain_snippet(" … a **b**\n\n  c … "), "… a b c …");
        assert_eq!(
            plain_snippet("root. | Task | Command | |---|---| | Re"),
            "root. Task Command Re",
            "a table keeps its cells"
        );
        assert_eq!(
            plain_snippet("calls `__init__` once"),
            "calls __init__ once",
            "underscores are code more often than emphasis"
        );
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
    fn an_empty_review_says_that_nothing_waits_and_what_would() {
        let page = review_empty_page();
        let words = page.words();
        assert!(words.contains("Nothing waits for review"), "{words}");
        assert!(words.contains("A sync proposes a change to a note you edited by hand"));
        assert!(words.contains("a coding agent proposes its explanation"));
        // Nothing on it to press.
        assert!(
            page.blocks
                .iter()
                .all(|block| !matches!(block, Block::Buttons(_)))
        );
    }

    #[test]
    fn a_retire_proposal_lists_each_note_with_the_files_that_are_gone() {
        let page = proposal_page(&json!({
            "proposal": {
                "id": "retire-1", "kind": "retire", "note_path": "",
                "reason": "2 notes describe code that is gone",
                "retire": [
                    {"path": "06-Notes/Old Service.md", "cited": ["legacy/service.cfg"]},
                    {"path": "06-Notes/Big.md",
                     "cited": ["legacy/a.cfg", "legacy/b.cfg", "legacy/c.cfg", "legacy/d.cfg", "legacy/e.cfg"]},
                ]
            },
            "patch": "- 06-Notes/Old Service.md\n- 06-Notes/Big.md\n"
        }));
        assert_eq!(page.title, "Notes about code that is gone");
        let words = page.words();
        assert!(
            words.contains("2 notes describe code that is gone"),
            "{words}"
        );
        assert!(words.contains("Approving deletes the ticked notes from the wiki."));
        assert!(words.contains("Git keeps every one of them"));
        assert!(words.contains("06-Notes/Old Service.md — legacy/service.cfg is gone"));
        assert!(words.contains(
            "06-Notes/Big.md — legacy/a.cfg, legacy/b.cfg, legacy/c.cfg and 2 more are gone"
        ));
        // It is no change to a note that someone edited.
        assert!(!words.contains("edited by hand"), "{words}");
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
        // Nor about notes of code that is gone when the run met none.
        assert!(!words.contains("gone or moved"));

        let page = sync_page(&json!({
            "written": [], "kept": 8, "proposed": [], "removed": ["06-Notes/Old.md"],
            "adopted": [], "notes_total": 8, "link_rate": 1.0,
            "summary": [
                "Removed 1 note whose subject is gone: `06-Notes/Old.md`.",
                "2 notes written by hand name only files that are gone."
            ]
        }));
        let words = page.words();
        assert!(
            words.contains("Notes of code that is gone or moved"),
            "{words}"
        );
        assert!(words.contains("Removed 1 note whose subject is gone: 06-Notes/Old.md."));
        assert!(words.contains("2 notes written by hand name only files that are gone."));

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

    /// A queue as `GET /explanations?all=true` answers it: behind first, a
    /// note an agent holds marked, `claimed` left out when it is 0.
    fn queue() -> Value {
        json!({
            "notes": [
                { "path": "01-Core/engine/Engine.md", "title": "Engine", "kind": "module",
                  "why": "behind", "behind": ["src/engine.rs changed"],
                  "sources": ["src/engine.rs"] },
                { "path": "01-Core/Parser.md", "title": "Parser", "kind": "module",
                  "why": "unexplained", "sources": ["src/parse.rs"], "claimed": true },
                { "path": "02-Api/Routes.md", "title": "", "kind": "module",
                  "why": "unexplained", "sources": ["src/api.rs"] }
            ],
            "matching": 3,
            "claimed": 1,
            "waiting": { "behind": 1, "unexplained": 2 }
        })
    }

    fn rows(page: &Doc) -> Vec<&Row> {
        page.blocks
            .iter()
            .filter_map(|block| match block {
                Block::Rows(rows) => Some(rows),
                _ => None,
            })
            .flatten()
            .collect()
    }

    #[test]
    fn the_waiting_page_says_whether_agents_explain_by_themselves() {
        let alert = |auto: Option<Count>| {
            waiting_page(Some(&queue()), auto)
                .blocks
                .into_iter()
                .find_map(|block| match block {
                    Block::Alert { text, button, .. } => Some((text, button)),
                    _ => None,
                })
                .expect("the page has an alert")
        };
        let settings = Some((
            "Settings".to_string(),
            Action::Settings(Some(Category::Repositories)),
        ));
        assert_eq!(
            alert(Some(Count::Notes(50))),
            (
                "Auto explaining is on: coding agents with the CodeWiki plugin explain up to 50 \
                 of these at the start of a session. Run /codewiki:write-notes to explain more."
                    .to_string(),
                settings.clone()
            )
        );
        assert_eq!(
            alert(Some(Count::All)),
            (
                "Auto explaining is on: coding agents with the CodeWiki plugin explain all of \
                 these at the start of a session."
                    .to_string(),
                settings.clone()
            )
        );
        assert_eq!(
            alert(None),
            (
                "Auto explaining is off. Run /codewiki:write-notes in Claude Code to explain \
                 these notes."
                    .to_string(),
                settings
            )
        );
    }

    #[test]
    fn the_waiting_page_lists_the_notes_in_the_backends_order() {
        let page = waiting_page(Some(&queue()), Some(Count::Notes(20)));
        assert_eq!(page.title, "Waiting notes");
        assert_eq!(
            page.lead.as_deref(),
            Some(
                "These notes wait for a coding agent to explain what they are for and how they work."
            )
        );
        let words = page.words();
        assert!(words.contains("Behind the code: 1"), "{words}");
        assert!(words.contains("Never explained: 2"), "{words}");
        assert!(words.contains("Being written: 1"), "{words}");
        let rows = rows(&page);
        let listed: Vec<(&str, Option<&str>, &Action)> = rows
            .iter()
            .map(|row| (row.label.as_str(), row.detail.as_deref(), &row.action))
            .collect();
        assert_eq!(
            listed,
            [
                (
                    "Engine",
                    Some("behind the code"),
                    &Action::Note("01-Core/engine/Engine.md".into())
                ),
                (
                    "Parser",
                    Some("never explained · an agent is writing it"),
                    &Action::Note("01-Core/Parser.md".into())
                ),
                // A note with no title is named by its file.
                (
                    "Routes",
                    Some("never explained"),
                    &Action::Note("02-Api/Routes.md".into())
                ),
            ]
        );
        assert!(!words.contains("Every note is explained."));
    }

    #[test]
    fn the_waiting_page_says_when_nothing_waits() {
        let page = waiting_page(
            Some(&json!({
                "notes": [], "matching": 0,
                "waiting": { "behind": 0, "unexplained": 0 }
            })),
            None,
        );
        assert!(
            page.blocks
                .contains(&Block::Empty("Every note is explained.".into()))
        );
        assert!(rows(&page).is_empty());
        // No agent holds a note: the backend leaves `claimed` out.
        assert!(page.words().contains("Being written: 0"));
        assert_eq!(waiting_count(Some(&json!({ "notes": [] }))), 0);
    }

    #[test]
    fn the_waiting_page_shows_that_it_is_reading() {
        let page = waiting_page(None, Some(Count::All));
        assert_eq!(page.title, "Waiting notes");
        assert!(
            page.blocks
                .contains(&Block::Empty("Reading the waiting notes…".into()))
        );
        assert!(rows(&page).is_empty());
        assert!(!page.words().contains("Every note is explained."));
        assert!(!page.words().contains("Behind the code"));
    }

    #[test]
    fn the_notes_to_explain_are_those_behind_and_those_never_explained() {
        assert_eq!(waiting_count(Some(&queue())), 3);
        assert_eq!(waiting_count(None), 0);
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
