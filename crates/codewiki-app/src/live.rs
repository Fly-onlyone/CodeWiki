//! The live pages: one symbol, one file, search results, hotspots, the theme
//! picker, a proposal and the result of a sync. Like the reports in `pages`,
//! each turns what the backend sent into a [`Doc`]. These show code that the
//! vault deliberately does not hold as notes.

use serde_json::Value;

use crate::doc::{
    Action, Block, Cell, Column, Doc, Line, Row, Stat, Swatch, Table, code, file, rich, strong,
    text,
};
use crate::icons;
use crate::mui::feedback::Tone;

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
            (string(symbol, "kind").replace('_', " "), Tone::Primary),
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

/// The theme picker: every theme as a card, grouped by mode. `themes` holds
/// each name and whether it is dark.
pub fn themes_page(themes: &[(String, bool)], active: &str) -> Doc {
    let mut doc = Doc::new("Themes").lead("Select a theme to apply it. The choice is remembered.");
    for (dark, heading) in [(true, "Dark"), (false, "Light")] {
        let swatches: Vec<Swatch> = themes
            .iter()
            .filter(|(_, is_dark)| *is_dark == dark)
            .map(|(name, _)| Swatch {
                name: name.clone(),
                active: name == active,
            })
            .collect();
        if !swatches.is_empty() {
            doc.heading(heading);
            doc.push(Block::Swatches(swatches));
        }
    }
    doc.push(Block::Text(vec![
        text("Your own themes load from the "),
        code("themes"),
        text(" folder in "),
        code("%APPDATA%\\CodeWiki"),
        text(", in the same JSON format."),
    ]));
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
    doc.push(Block::Alert {
        tone: Tone::Info,
        text: "This note was edited by hand, so the change waits for your decision. Nothing is written until you approve.".into(),
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
    });
    doc.push(Block::Stats(vec![
        Stat::new("Written", count("written")),
        Stat::new("Unchanged", number(&report["kept"])),
        Stat::new("Queued for review", count("proposed")).tone(if count("proposed") > 0 {
            Tone::Warning
        } else {
            Tone::Neutral
        }),
    ]));
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
    doc
}

/// A page that only says something, such as that there is no wiki yet.
pub fn message_page(message: &str) -> Doc {
    let mut doc = Doc::new("CodeWiki");
    if !message.is_empty() {
        doc.push(Block::Text(vec![strong(message)]));
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
    fn renders_the_theme_picker() {
        let themes = vec![("Nord".to_string(), true), ("One Light".to_string(), false)];
        let page = themes_page(&themes, "Nord");
        assert_eq!(page.headings(), ["Dark", "Light"]);
        let picked: Vec<(&str, bool)> = page
            .blocks
            .iter()
            .filter_map(|block| match block {
                Block::Swatches(swatches) => Some(swatches),
                _ => None,
            })
            .flatten()
            .map(|swatch| (swatch.name.as_str(), swatch.active))
            .collect();
        assert_eq!(picked, [("Nord", true), ("One Light", false)]);
        assert_eq!(
            page.actions(),
            [
                Action::Theme("Nord".into()),
                Action::Theme("One Light".into())
            ]
        );
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
