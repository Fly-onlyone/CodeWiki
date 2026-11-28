//! A coding agent explains notes: the queue, the brief and the submission.

use std::fs;
use std::path::Path;

use cw_illustrate::Picture;
use cw_parse::scan_repo;
use cw_vault::{NoteEntry, NoteKind, ProposalKind, ProposalStatus, VaultMeta, proposal};
use cw_wiki::explanations::{
    self, Brief, Counts, DEFAULT_LIMIT, MAX_SECTION_WORDS, Outcome, Scope, Submission, Why, Written,
};
use cw_wiki::{Report, analyze, generate};

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap()
}

/// A workspace of two crates, synced once.
fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let r = dir.path();
    write(r, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
    write(
        r,
        "crates/engine/Cargo.toml",
        "[package]\nname = \"engine\"\ndescription = \"Order matching engine\"\n",
    );
    write(
        r,
        "crates/engine/src/lib.rs",
        "//! Order matching engine.\npub mod book;\npub mod pricing;\n",
    );
    write(
        r,
        "crates/engine/src/book.rs",
        "//! The order book. Orders rest here until matched.\nuse crate::pricing::Price;\n\n/// Holds resting orders, best price first.\npub struct OrderBook { best: Price }\n\nimpl OrderBook {\n    /// Create an empty book.\n    pub fn new() -> Self { Self { best: Price::zero() } }\n    /// Insert an order at `price`.\n    pub fn insert(&mut self, price: Price) { self.best = price; }\n    /// Best resting price.\n    pub fn best(&self) -> Price { self.best }\n}\n",
    );
    write(
        r,
        "crates/engine/src/pricing.rs",
        "//! Price arithmetic.\n\n/// A price in integer ticks.\n#[derive(Clone, Copy)]\npub struct Price(pub u64);\n\nimpl Price {\n    /// The zero price.\n    pub fn zero() -> Self { Price(0) }\n}\n",
    );
    write(
        r,
        "crates/cli/Cargo.toml",
        "[package]\nname = \"cli\"\n[dependencies]\nengine = { path = \"../engine\" }\n",
    );
    write(
        r,
        "crates/cli/src/main.rs",
        "use engine::book::OrderBook;\nuse engine::pricing::Price;\n\nfn main() {\n    let mut book = OrderBook::new();\n    book.insert(Price::zero());\n    let _ = book.best();\n}\n",
    );
    sync(r);
    dir
}

fn sync(root: &Path) -> Report {
    let facts = scan_repo(root).unwrap();
    generate(root, &facts, &analyze(&facts), None).unwrap()
}

fn entry(root: &Path, note: &str) -> NoteEntry {
    VaultMeta::load(&root.join("obsidian"))
        .unwrap()
        .by_path(note)
        .unwrap()
        .clone()
}

const PRICE: &str = "01-Core/engine/Price.md";
const PRICE_PICTURES: &str = "01-Core/engine/Price.openui";
const BOOK: &str = "01-Core/engine/OrderBook.md";
const FLOW: &str = "00-Architecture/Crate Dependency Flow.md";
const WORKS: &str = "A price counts ticks. One tick is the smallest step a price can move, so two prices compare as two whole numbers and no rounding is needed.";
const GOTCHA: &str = "- A price is never negative: the type holds an unsigned number.";
const FLOW_PICTURE: &str = "root = Picture([f])\nf = Flow([Step(\"A number of ticks\"), Step(\"One price\", \"Compared as a whole number.\")])\n";
const CALLOUT: &str = "root = Picture([c])\nc = Callout(\"Never negative.\", \"warning\")\n";

fn written(heading: &str, text: &str) -> Written {
    Written {
        heading: heading.into(),
        text: text.into(),
    }
}

fn picture(section: &str, source: &str) -> Picture {
    Picture {
        section: section.into(),
        source: source.into(),
    }
}

/// A submission for `note`, written from its brief as it is now.
fn submission(root: &Path, note: &str, sections: &[(&str, &str)]) -> Submission {
    Submission {
        base: explanations::brief(root, note).unwrap().base,
        sections: sections.iter().map(|(h, t)| written(h, t)).collect(),
        model: Some("test-agent".into()),
        ..Submission::default()
    }
}

fn submit(root: &Path, note: &str, submission: &Submission) -> Outcome {
    explanations::submit(root, note, submission).unwrap()
}

/// Explain the note about `Price`, with a picture in each of two sections.
fn explain_price(root: &Path) -> Outcome {
    let mut sent = submission(root, PRICE, &[("How it works", WORKS), ("Gotchas", GOTCHA)]);
    sent.pictures = vec![
        picture("Gotchas", CALLOUT),
        picture("How it works", FLOW_PICTURE),
    ];
    submit(root, PRICE, &sent)
}

fn issues(outcome: Outcome) -> Vec<String> {
    match outcome {
        Outcome::Issues { issues } => issues,
        other => panic!("expected issues, got {other:?}"),
    }
}

#[test]
fn the_queue_lists_what_waits_and_counts_all_of_it() {
    let dir = repo();
    let root = dir.path();
    let meta = VaultMeta::load(&root.join("obsidian")).unwrap();
    let explainable = meta.notes.values().filter(|e| e.kind.explainable()).count();
    assert!(explainable > DEFAULT_LIMIT, "{explainable}");

    let queue = explanations::queue(root, &Scope::default()).unwrap();
    assert_eq!(
        queue.waiting,
        Counts {
            behind: 0,
            unexplained: explainable
        }
    );
    assert_eq!(explanations::waiting_counts(root).unwrap(), queue.waiting);
    assert_eq!(queue.matching, explainable);
    assert_eq!(queue.notes.len(), DEFAULT_LIMIT);
    assert!(queue.notes.iter().all(|n| n.why == Why::Unexplained));
    // By path, so that two calls list the same notes.
    let paths: Vec<&str> = queue.notes.iter().map(|n| n.path.as_str()).collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted);
    // No map of the vault waits: nothing in one is an agent's to write.
    assert!(
        !paths
            .iter()
            .any(|p| p.ends_with("_index.md") || p.ends_with("_HOME.md"))
    );

    // The number asked for is kept within bounds.
    let all = explanations::queue(
        root,
        &Scope {
            limit: Some(500),
            ..Scope::default()
        },
    )
    .unwrap();
    assert_eq!(all.notes.len(), explainable.min(explanations::MAX_LIMIT));
    let one = explanations::queue(
        root,
        &Scope {
            limit: Some(0),
            ..Scope::default()
        },
    )
    .unwrap();
    assert_eq!(one.notes.len(), 1);

    // A folder of the vault, however it is written.
    for folder in ["01-Core/engine", "01-core\\engine\\", "./01-Core/engine/"] {
        let queue = explanations::queue(
            root,
            &Scope {
                folder: Some(folder.into()),
                limit: Some(20),
                ..Scope::default()
            },
        )
        .unwrap();
        assert!(
            queue.matching > 1 && queue.matching < explainable,
            "{folder}"
        );
        assert!(
            queue
                .notes
                .iter()
                .all(|n| n.path.starts_with("01-Core/engine/")),
            "{folder}"
        );
        assert_eq!(queue.waiting.unexplained, explainable);
    }

    // The notes about a file.
    let queue = explanations::queue(
        root,
        &Scope {
            files: vec!["crates\\engine\\src\\pricing.rs".into()],
            limit: Some(20),
            ..Scope::default()
        },
    )
    .unwrap();
    let paths: Vec<&str> = queue.notes.iter().map(|n| n.path.as_str()).collect();
    assert!(paths.contains(&PRICE), "{paths:?}");
    assert!(!paths.contains(&BOOK), "{paths:?}");
    let price = queue.notes.iter().find(|n| n.path == PRICE).unwrap();
    assert_eq!(price.title, "Price");
    assert_eq!(price.kind, NoteKind::Concept);
    assert_eq!(price.sources, ["crates/engine/src/pricing.rs"]);
}

#[test]
fn a_brief_says_which_sections_are_the_agents() {
    let dir = repo();
    let root = dir.path();
    let brief: Brief = explanations::brief(root, PRICE).unwrap();
    assert_eq!(brief.path, PRICE);
    assert_eq!(brief.title, "Price");
    assert_eq!(brief.text, read(root, &format!("obsidian/{PRICE}")));
    assert_eq!(brief.base, cw_vault::body_hash(&brief.text));
    let yours: Vec<&str> = brief.yours.iter().map(|y| y.heading.as_str()).collect();
    assert_eq!(yours, ["What it does", "How it works", "Gotchas"]);
    assert!(
        brief
            .yours
            .iter()
            .all(|y| y.written_by != "agent" && !y.prose_only)
    );
    let gotchas = &brief.yours[2];
    assert_eq!(
        (gotchas.written_by.as_str(), gotchas.text.as_str()),
        ("nobody", "")
    );
    assert_eq!(
        brief.rules,
        ["Source", "Items", "Depends on", "Used by", "See also"]
    );
    assert_eq!(brief.sources.len(), 1);
    assert_eq!(brief.sources[0].path, "crates/engine/src/pricing.rs");
    assert!(!brief.sources[0].changed && !brief.sync_first && !brief.edited_by_hand);
    assert!(brief.behind.is_empty() && brief.pictures.is_empty());
    assert_eq!(brief.explained_by, None);
    assert_eq!(brief.limits.section_words, MAX_SECTION_WORDS);
    assert!(
        brief
            .limits
            .components
            .contains("Flow(steps: array, title?: string)")
    );
    assert!(brief.limits.pictures.contains("2 to 7 steps"));

    // By name too, as a person would say it.
    assert_eq!(explanations::brief(root, "price").unwrap().path, PRICE);

    // In a flow note the diagram is a fact inside a section of the agent's.
    let flow = explanations::brief(root, FLOW).unwrap();
    let works = flow
        .yours
        .iter()
        .find(|y| y.heading == "How it works")
        .unwrap();
    assert!(works.prose_only);
    assert!(flow.text.contains("```mermaid"));
    assert!(!works.text.contains("```"), "{}", works.text);

    // A map of the vault is no note an agent explains.
    let error = explanations::brief(root, "_HOME.md")
        .unwrap_err()
        .to_string();
    assert!(error.contains("no agent explains"), "{error}");
    assert!(explanations::brief(root, "No Such Note").is_err());
    assert!(explanations::brief(root, "../../secret.md").is_err());
}

#[test]
fn a_submission_is_woven_into_the_note_and_recorded() {
    let dir = repo();
    let root = dir.path();
    let vault = root.join("obsidian");
    let before = read(&vault, PRICE);
    let waiting = explanations::waiting_counts(root).unwrap();

    let outcome = explain_price(root);
    assert_eq!(
        outcome,
        Outcome::Saved {
            path: PRICE.into(),
            sections: vec!["How it works".into(), "Gotchas".into()],
            pictures: 2,
            waiting: Counts {
                behind: 0,
                unexplained: waiting.unexplained - 1
            },
        }
    );
    let note = read(&vault, PRICE);
    assert!(note.contains(&format!("## How it works\n\n{WORKS}\n")));
    assert!(note.contains(&format!("## Gotchas\n\n{GOTCHA}\n\n## See also")));
    // Every fact is as rules wrote it.
    let (head, sections) = cw_vault::note::split(&note);
    let (old_head, old_sections) = cw_vault::note::split(&before);
    assert_eq!(head, old_head);
    for old in old_sections.iter().filter(|s| s.heading != "How it works") {
        assert!(sections.contains(old), "{}", old.heading);
    }
    // The pictures stand in the order of the note's sections.
    assert_eq!(
        read(&vault, PRICE_PICTURES),
        format!("# section: How it works\n{FLOW_PICTURE}\n# section: Gotchas\n{CALLOUT}")
    );

    let saved = entry(root, PRICE);
    assert_eq!(saved.model.as_deref(), Some("test-agent"));
    assert_eq!(saved.illustration.as_deref(), Some(PRICE_PICTURES));
    assert_eq!(saved.body_hash, cw_vault::body_hash(&note));
    let explained = saved.explained.clone().unwrap();
    assert_eq!(explained.sections, ["How it works", "Gotchas"]);
    assert_eq!(explained.sources, saved.sources);
    assert!(saved.behind.is_empty());

    // To a reader of the vault the note is explained, and has its pictures.
    let content = cw_wiki::query::read_note(root, PRICE).unwrap();
    assert_eq!(
        content.info.explanation,
        cw_wiki::query::Explanation::Explained
    );
    assert_eq!(content.info.explained_by.as_deref(), Some("test-agent"));
    assert_eq!(content.pictures.len(), 2);
    assert_eq!(content.pictures[0].section, "How it works");
    let waits = cw_wiki::query::read_note(root, BOOK).unwrap();
    assert_eq!(waits.info.explanation, cw_wiki::query::Explanation::Waiting);
    assert!(waits.pictures.is_empty());

    // The brief shows the agent its own text.
    let brief = explanations::brief(root, PRICE).unwrap();
    let by: Vec<&str> = brief.yours.iter().map(|y| y.written_by.as_str()).collect();
    assert_eq!(&by[1..], ["agent", "agent"]);
    assert_eq!(brief.yours[1].text, WORKS);
    assert_eq!(brief.explained_by.as_deref(), Some("test-agent"));
    assert_eq!(brief.pictures.len(), 2);

    // A sync leaves it alone: it is no note a person edited, and it no
    // longer waits.
    let report = sync(root);
    assert!(
        report.written.is_empty() && report.proposed.is_empty(),
        "{report:?}"
    );
    assert!(
        report.merged.is_empty() && report.behind.is_empty(),
        "{report:?}"
    );
    assert_eq!(read(&vault, PRICE), note);
    let queue = explanations::queue(
        root,
        &Scope {
            limit: Some(20),
            ..Scope::default()
        },
    )
    .unwrap();
    assert!(!queue.notes.iter().any(|n| n.path == PRICE));
}

#[test]
fn a_note_that_is_behind_comes_first_and_is_cleared_by_writing_again() {
    let dir = repo();
    let root = dir.path();
    let mut sent = submission(root, PRICE, &[("Gotchas", GOTCHA)]);
    // The agent read a second file to write this.
    sent.read = vec![
        "crates\\engine\\src\\book.rs".into(),
        "crates/engine/src/pricing.rs".into(),
        "no/such/file.rs".into(),
        "../outside.rs".into(),
    ];
    assert!(matches!(submit(root, PRICE, &sent), Outcome::Saved { .. }));
    let from: Vec<String> = entry(root, PRICE)
        .explained
        .unwrap()
        .sources
        .into_iter()
        .map(|s| s.path)
        .collect();
    assert_eq!(
        from,
        ["crates/engine/src/pricing.rs", "crates/engine/src/book.rs"]
    );

    // That file changes. The note's own facts do not.
    let book = read(root, "crates/engine/src/book.rs")
        .replace("self.best = price;", "self.best = price; let _ = 0;");
    write(root, "crates/engine/src/book.rs", &book);
    let report = sync(root);
    assert_eq!(report.behind, [PRICE], "{report:?}");
    assert_eq!(
        cw_wiki::query::read_note(root, PRICE)
            .unwrap()
            .info
            .explanation,
        cw_wiki::query::Explanation::Behind
    );

    let queue = explanations::queue(root, &Scope::default()).unwrap();
    assert_eq!(queue.waiting.behind, 1);
    assert_eq!(queue.notes[0].path, PRICE);
    assert_eq!(queue.notes[0].why, Why::Behind);
    assert_eq!(queue.notes[0].behind, ["crates/engine/src/book.rs changed"]);
    // It is found by the file that changed, though the note is not about it.
    let by_file = explanations::queue(
        root,
        &Scope {
            files: vec!["crates/engine/src/book.rs".into()],
            limit: Some(20),
            ..Scope::default()
        },
    )
    .unwrap();
    assert!(by_file.notes.iter().any(|n| n.path == PRICE));

    let brief = explanations::brief(root, PRICE).unwrap();
    assert_eq!(brief.behind, ["crates/engine/src/book.rs changed"]);
    let changed: Vec<(&str, bool)> = brief
        .sources
        .iter()
        .map(|s| (s.path.as_str(), s.changed))
        .collect();
    assert_eq!(
        changed,
        [
            ("crates/engine/src/pricing.rs", false),
            ("crates/engine/src/book.rs", true)
        ]
    );
    assert!(!brief.sync_first);

    // The agent reads the files again and writes: the note is up to date.
    let mut again = submission(root, PRICE, &[("Gotchas", GOTCHA)]);
    again.read = vec!["crates/engine/src/book.rs".into()];
    assert!(matches!(submit(root, PRICE, &again), Outcome::Saved { .. }));
    assert!(entry(root, PRICE).behind.is_empty());
    assert_eq!(explanations::waiting_counts(root).unwrap().behind, 0);
    assert!(sync(root).behind.is_empty());
}

#[test]
fn a_submission_with_faults_writes_nothing_and_names_each() {
    let dir = repo();
    let root = dir.path();
    let vault = root.join("obsidian");
    let note = read(&vault, PRICE);
    let recorded = entry(root, PRICE);
    let long = "word ".repeat(MAX_SECTION_WORDS + 1);
    let mut sent = submission(
        root,
        PRICE,
        &[
            ("Items", "- `Price` is mine now."),
            (
                "Gotchas",
                "See [[No Such Note]] and [the docs](https://example.com).",
            ),
            ("Gotchas", "Twice."),
            ("How it works", "Before.\n\n## Source\n\nA new place."),
            ("What it does", &long),
        ],
    );
    sent.pictures = vec![
        picture("Items", CALLOUT),
        picture(
            "Gotchas",
            "root = Picture([c])\nc = Stat(\"Ticks\", \"64\")\n",
        ),
        picture("Gotchas", CALLOUT),
    ];
    let found = issues(submit(root, PRICE, &sent));
    for expected in [
        "`Items` is not a section an agent writes in this kind of note; yours are: What it does, How it works, Gotchas",
        "`Gotchas` was sent twice",
        "`How it works`: a section's text holds no `#` or `##` heading",
        "`What it does`: 301 words; at most 300",
        "[link-unresolved] link `[[No Such Note]]` does not resolve",
        "[wikilinks-only] use `[[wikilinks]]`, not Markdown links",
        "a picture names the section `Items`",
        "picture for `Gotchas`: unknown-component",
        "two pictures for `Gotchas`",
    ] {
        assert!(
            found.iter().any(|issue| issue.contains(expected)),
            "missing `{expected}` in {found:#?}"
        );
    }
    assert_eq!(read(&vault, PRICE), note);
    assert_eq!(entry(root, PRICE), recorded);
    assert!(!vault.join(PRICE_PICTURES).exists());

    // Nothing sent, an empty text, and a code block where the blocks are
    // the rules'.
    let found = issues(submit(root, PRICE, &submission(root, PRICE, &[])));
    assert!(found[0].starts_with("no section was sent"), "{found:?}");
    let found = issues(submit(
        root,
        PRICE,
        &submission(root, PRICE, &[("Gotchas", "  ")]),
    ));
    assert!(found[0].contains("the text is empty"), "{found:?}");
    let flow = read(&vault, FLOW);
    let found = issues(submit(
        root,
        FLOW,
        &submission(
            root,
            FLOW,
            &[(
                "How it works",
                "My own.\n\n```mermaid\nflowchart LR\n  a --> b\n```",
            )],
        ),
    ));
    assert!(
        found.iter().any(|i| i.contains("send prose only")),
        "{found:?}"
    );
    assert_eq!(read(&vault, FLOW), flow);

    // Five pictures are one too many, and a picture needs its section.
    let mut sent = submission(root, PRICE, &[("Gotchas", GOTCHA)]);
    sent.pictures = vec![
        picture("How it works", CALLOUT),
        picture("What it does", CALLOUT),
    ];
    let found = issues(submit(root, PRICE, &sent));
    let missing: Vec<&String> = found
        .iter()
        .filter(|i| i.contains("a picture names"))
        .collect();
    let (_, sections) = cw_vault::note::split(&note);
    let lacking = ["How it works", "What it does"]
        .iter()
        .filter(|h| !sections.iter().any(|s| s.heading == **h))
        .count();
    assert_eq!(missing.len(), lacking, "{found:?}");
}

#[test]
fn a_submission_against_an_older_note_or_older_facts_is_turned_back() {
    let dir = repo();
    let root = dir.path();
    let vault = root.join("obsidian");
    let note = read(&vault, PRICE);

    let mut old = submission(root, PRICE, &[("Gotchas", GOTCHA)]);
    old.base = "the hash of an older note".into();
    match submit(root, PRICE, &old) {
        Outcome::Changed { message } => {
            assert!(message.contains("read the brief again"), "{message}")
        }
        other => panic!("{other:?}"),
    }

    // The code changes and nobody syncs: the facts of the note are old.
    let sent = submission(root, PRICE, &[("Gotchas", GOTCHA)]);
    let pricing = read(root, "crates/engine/src/pricing.rs")
        .replace("integer ticks", "integer ticks of one cent");
    write(root, "crates/engine/src/pricing.rs", &pricing);
    assert!(explanations::brief(root, PRICE).unwrap().sync_first);
    match submit(root, PRICE, &sent) {
        Outcome::SyncFirst { message } => {
            assert!(
                message.contains("crates/engine/src/pricing.rs"),
                "{message}"
            )
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(read(&vault, PRICE), note);
    assert_eq!(entry(root, PRICE).explained, None);

    // After the sync the older submission is against an older note.
    sync(root);
    assert!(matches!(
        submit(root, PRICE, &sent),
        Outcome::Changed { .. }
    ));
    let fresh = submission(root, PRICE, &[("Gotchas", GOTCHA)]);
    assert!(matches!(submit(root, PRICE, &fresh), Outcome::Saved { .. }));
    assert!(read(&vault, PRICE).contains("of one cent"));

    // A note whose file is gone waits for a sync.
    fs::remove_file(vault.join(BOOK)).unwrap();
    let gone = Submission {
        base: "any".into(),
        sections: vec![written("Gotchas", GOTCHA)],
        ..Submission::default()
    };
    assert!(matches!(
        submit(root, BOOK, &gone),
        Outcome::SyncFirst { .. }
    ));
}

#[test]
fn a_sync_that_changes_no_fact_still_lets_the_note_be_written() {
    let dir = repo();
    let root = dir.path();
    // A function's body: no fact of the note changes.
    let pricing =
        read(root, "crates/engine/src/pricing.rs").replace("{ Price(0) }", "{ Price(0u64) }");
    write(root, "crates/engine/src/pricing.rs", &pricing);
    assert!(explanations::brief(root, PRICE).unwrap().sync_first);

    let report = sync(root);
    assert!(
        report.written.is_empty() && report.merged.is_empty(),
        "{report:?}"
    );
    // The sync read the file as it is now, so the brief does not stop.
    assert!(!explanations::brief(root, PRICE).unwrap().sync_first);
    let sent = submission(root, PRICE, &[("Gotchas", GOTCHA)]);
    assert!(matches!(submit(root, PRICE, &sent), Outcome::Saved { .. }));
}

/// Give the repository file `rel` CRLF line endings, as git hands it out on
/// Windows with `core.autocrlf`.
fn to_crlf(root: &Path, rel: &str) {
    let text = read(root, rel).replace('\n', "\r\n");
    write(root, rel, &text);
}

/// Explain `Price` from a checkout whose files have CRLF line endings, the
/// agent having read the file of `OrderBook` too.
fn explain_price_from_crlf(root: &Path) {
    to_crlf(root, "crates/engine/src/pricing.rs");
    to_crlf(root, "crates/engine/src/book.rs");
    sync(root);
    // The facts were made from the files as they are, so nothing waits for
    // a sync.
    assert!(!explanations::brief(root, PRICE).unwrap().sync_first);
    let mut sent = submission(root, PRICE, &[("Gotchas", GOTCHA)]);
    sent.read = vec!["crates/engine/src/book.rs".into()];
    assert!(matches!(submit(root, PRICE, &sent), Outcome::Saved { .. }));
}

#[test]
fn a_file_with_crlf_line_endings_holds_up_no_note() {
    let dir = repo();
    let root = dir.path();
    explain_price_from_crlf(root);
    // What the agent read was recorded as a scan hashes it: nothing is
    // behind, and no file reads as changed.
    assert!(sync(root).behind.is_empty());
    let brief = explanations::brief(root, PRICE).unwrap();
    assert!(!brief.sync_first);
    assert!(brief.sources.iter().all(|s| !s.changed));
}

#[test]
fn an_explanation_recorded_from_raw_bytes_is_behind_only_when_they_change() {
    let dir = repo();
    let root = dir.path();
    explain_price_from_crlf(root);
    // Before CRLF was read as LF, a hash was of the file's raw bytes, and
    // the wiki kept it so.
    let vault = root.join("obsidian");
    let mut meta = VaultMeta::load(&vault).unwrap();
    let id = meta.by_path(PRICE).unwrap().id.clone();
    let explained = meta.notes.get_mut(&id).unwrap().explained.as_mut().unwrap();
    for source in &mut explained.sources {
        source.hash = cw_core::sha256_hex(fs::read(root.join(&source.path)).unwrap());
    }
    meta.save(&vault).unwrap();

    // The bytes are the same, so the explanation is not behind.
    assert!(sync(root).behind.is_empty());
    let brief = explanations::brief(root, PRICE).unwrap();
    assert!(brief.sources.iter().all(|s| !s.changed));

    // A change to them still is.
    let book = read(root, "crates/engine/src/book.rs")
        .replace("self.best = price;", "self.best = price; let _ = 0;");
    write(root, "crates/engine/src/book.rs", &book);
    assert_eq!(sync(root).behind, [PRICE]);
}

#[test]
fn a_sync_renews_a_raw_byte_hash_so_another_checkout_agrees() {
    let dir = repo();
    let root = dir.path();
    explain_price_from_crlf(root);
    // The wiki kept hashes of raw bytes, taken from files with CRLF.
    let vault = root.join("obsidian");
    let mut meta = VaultMeta::load(&vault).unwrap();
    let id = meta.by_path(PRICE).unwrap().id.clone();
    let explained = meta.notes.get_mut(&id).unwrap().explained.as_mut().unwrap();
    for source in &mut explained.sources {
        source.hash = cw_core::sha256_hex(fs::read(root.join(&source.path)).unwrap());
    }
    meta.save(&vault).unwrap();

    // A sync where the files still have those bytes records them as a scan
    // hashes them.
    assert!(sync(root).behind.is_empty());
    let meta = VaultMeta::load(&vault).unwrap();
    let explained = meta.notes[&id].explained.as_ref().unwrap();
    assert!(!explained.sources.is_empty());
    for source in &explained.sources {
        let bytes = fs::read(root.join(&source.path)).unwrap();
        assert_eq!(source.hash, cw_core::source_hash(&bytes), "{}", source.path);
    }

    // The same commit checked out with LF line endings, as on a CI runner:
    // nothing changed, so nothing is behind.
    for rel in ["crates/engine/src/pricing.rs", "crates/engine/src/book.rs"] {
        let text = read(root, rel).replace("\r\n", "\n");
        write(root, rel, &text);
    }
    assert!(sync(root).behind.is_empty());
    let brief = explanations::brief(root, PRICE).unwrap();
    assert!(!brief.sync_first);
    assert!(brief.sources.iter().all(|s| !s.changed));
}

#[test]
fn pictures_follow_the_sections_that_are_sent() {
    let dir = repo();
    let root = dir.path();
    let vault = root.join("obsidian");
    explain_price(root);

    // One section again, without a picture: its picture explained the old
    // text and goes. The other section was not sent; its picture stays.
    let sent = submission(root, PRICE, &[("Gotchas", "- Ticks are whole numbers.")]);
    match submit(root, PRICE, &sent) {
        Outcome::Saved {
            pictures, sections, ..
        } => {
            assert_eq!(pictures, 1);
            assert_eq!(sections, ["How it works", "Gotchas"]);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        read(&vault, PRICE_PICTURES),
        format!("# section: How it works\n{FLOW_PICTURE}")
    );
    let note = read(&vault, PRICE);
    assert!(note.contains(WORKS) && note.contains("- Ticks are whole numbers."));
    assert!(!note.contains(GOTCHA));

    // The other section, without a picture: the note has none left, and
    // no file is kept for none.
    let sent = submission(root, PRICE, &[("How it works", WORKS)]);
    assert!(matches!(
        submit(root, PRICE, &sent),
        Outcome::Saved { pictures: 0, .. }
    ));
    assert!(!vault.join(PRICE_PICTURES).exists());
    assert_eq!(entry(root, PRICE).illustration, None);
}

#[test]
fn the_diagram_of_a_flow_stays_above_the_agents_words() {
    let dir = repo();
    let root = dir.path();
    let vault = root.join("obsidian");
    let before = read(&vault, FLOW);
    let diagram: String = before
        .split("```")
        .nth(1)
        .map(|block| format!("```{block}```"))
        .unwrap();
    let words = "The command line program depends on the engine and on nothing else.";
    let sent = submission(root, FLOW, &[("How it works", words)]);
    assert!(matches!(submit(root, FLOW, &sent), Outcome::Saved { .. }));
    let note = read(&vault, FLOW);
    assert!(
        note.contains(&format!("## How it works\n\n{diagram}\n\n{words}\n")),
        "{note}"
    );
    // The agent's part of that section is the prose alone.
    let brief = explanations::brief(root, FLOW).unwrap();
    let works = brief
        .yours
        .iter()
        .find(|y| y.heading == "How it works")
        .unwrap();
    assert_eq!(
        (works.text.as_str(), works.written_by.as_str()),
        (words, "agent")
    );
    assert!(sync(root).written.is_empty());
}

#[test]
fn a_note_its_owner_edited_gets_a_proposal_and_is_not_written() {
    let dir = repo();
    let root = dir.path();
    let vault = root.join("obsidian");
    let edited = read(&vault, PRICE).replace(
        "## See also",
        "## Gotchas\n\n- My own warning.\n\n## See also",
    );
    write(&vault, PRICE, &edited);
    // It left the queue, and the brief says what will happen.
    assert!(explanations::brief(root, PRICE).unwrap().edited_by_hand);
    let listed = explanations::queue(
        root,
        &Scope {
            limit: Some(20),
            ..Scope::default()
        },
    )
    .unwrap();
    assert!(!listed.notes.iter().any(|n| n.path == PRICE));

    let mut sent = submission(root, PRICE, &[("How it works", WORKS)]);
    sent.pictures = vec![picture("How it works", FLOW_PICTURE)];
    let id = match submit(root, PRICE, &sent) {
        Outcome::Proposed {
            path,
            proposal,
            message,
        } => {
            assert_eq!(path, PRICE);
            assert!(message.contains("review queue"), "{message}");
            proposal
        }
        other => panic!("{other:?}"),
    };
    assert_eq!(read(&vault, PRICE), edited, "the note is untouched");
    assert_eq!(entry(root, PRICE).explained, None);
    assert!(!vault.join(PRICE_PICTURES).exists());

    let filed = proposal::get(&vault, &id).unwrap();
    assert_eq!(filed.kind, ProposalKind::Agent);
    assert_eq!(filed.status, ProposalStatus::Pending);
    assert_eq!(filed.reason, "test-agent explained this note");
    let proposed = proposal::proposed_text(&vault, &id).unwrap();
    assert!(proposed.contains(WORKS) && proposed.contains("- My own warning."));

    // Rejected: nothing changes, and an agent may offer another.
    explanations::reject(root, &id).unwrap();
    assert_eq!(read(&vault, PRICE), edited);
    assert!(entry(root, PRICE).rejected_inputs.is_empty());
    let id = match submit(root, PRICE, &sent) {
        Outcome::Proposed { proposal, .. } => proposal,
        other => panic!("{other:?}"),
    };

    // Approved: the note, its pictures and who explained it.
    let done = explanations::approve(root, &id).unwrap();
    assert_eq!(done.status, ProposalStatus::Approved);
    assert_eq!(read(&vault, PRICE), proposed);
    assert_eq!(
        read(&vault, PRICE_PICTURES),
        format!("# section: How it works\n{FLOW_PICTURE}")
    );
    let saved = entry(root, PRICE);
    assert_eq!(saved.model.as_deref(), Some("test-agent"));
    assert_eq!(saved.explained.clone().unwrap().sections, ["How it works"]);
    // It stays its owner's note: the next change of its facts is proposed,
    // and the proposal keeps both the owner's line and the agent's words.
    assert_ne!(saved.body_hash, cw_vault::body_hash(&proposed));
    let pricing = read(root, "crates/engine/src/pricing.rs")
        .replace("integer ticks", "integer ticks of one cent");
    write(root, "crates/engine/src/pricing.rs", &pricing);
    let report = sync(root);
    assert_eq!(report.proposed, [PRICE], "{report:?}");
    assert_eq!(read(&vault, PRICE), proposed);
    assert!(vault.join(PRICE_PICTURES).exists());
    let pending: Vec<_> = proposal::list(&vault)
        .unwrap()
        .into_iter()
        .filter(|p| p.status == ProposalStatus::Pending)
        .collect();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, ProposalKind::Sync);
    let text = proposal::proposed_text(&vault, &pending[0].id).unwrap();
    assert!(
        text.contains("of one cent") && text.contains(WORKS) && text.contains("- My own warning.")
    );

    // A sync's proposal is approved as before: that input counts as seen.
    let done = explanations::approve(root, &pending[0].id).unwrap();
    assert_eq!(entry(root, PRICE).input_hash, done.input_hash);
    assert!(sync(root).proposed.is_empty());
}

#[test]
fn a_fault_the_owner_wrote_is_not_held_against_the_agent() {
    let dir = repo();
    let root = dir.path();
    let vault = root.join("obsidian");
    // The owner's own section and a link that leads nowhere.
    let edited = read(&vault, PRICE).replace(
        "## See also",
        "## History\n\nSee [[A Note I Plan]].\n\n## See also",
    );
    write(&vault, PRICE, &edited);
    let sent = submission(root, PRICE, &[("Gotchas", GOTCHA)]);
    let id = match submit(root, PRICE, &sent) {
        Outcome::Proposed { proposal, .. } => proposal,
        other => panic!("{other:?}"),
    };
    let proposed = proposal::proposed_text(&vault, &id).unwrap();
    assert!(proposed.contains("## History\n\nSee [[A Note I Plan]].\n"));
    assert!(proposed.contains(GOTCHA));
    // A fault of the agent's own is still one.
    let sent = submission(root, PRICE, &[("Gotchas", "See [[Another Missing Note]].")]);
    let found = issues(submit(root, PRICE, &sent));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("Another Missing Note"));
}

#[test]
fn claimed_notes_are_left_out_of_the_queue_and_counted() {
    let dir = repo();
    let root = dir.path();
    let all = explanations::queue(
        root,
        &Scope {
            limit: Some(explanations::MAX_LIMIT),
            ..Scope::default()
        },
    )
    .unwrap();
    let first = all.notes[0].path.clone();
    let second = all.notes[1].path.clone();

    let queue = explanations::queue(
        root,
        &Scope {
            claimed: vec![first.clone(), second.clone()],
            limit: Some(explanations::MAX_LIMIT),
            ..Scope::default()
        },
    )
    .unwrap();
    assert_eq!(queue.claimed, 2);
    assert_eq!(queue.matching, all.matching);
    assert_eq!(queue.waiting, all.waiting);
    assert_eq!(queue.notes.len(), all.notes.len() - 2);
    assert!(
        queue
            .notes
            .iter()
            .all(|n| n.path != first && n.path != second)
    );

    // The limit applies after the held notes are out.
    let limited = explanations::queue(
        root,
        &Scope {
            claimed: vec![first.clone()],
            limit: Some(2),
            ..Scope::default()
        },
    )
    .unwrap();
    let paths: Vec<&str> = limited.notes.iter().map(|n| n.path.as_str()).collect();
    assert_eq!(paths, [second.as_str(), all.notes[2].path.as_str()]);
    assert_eq!(limited.claimed, 1);

    // A path no note fits is not counted.
    let none = explanations::queue(
        root,
        &Scope {
            claimed: vec!["nothing.md".into()],
            ..Scope::default()
        },
    )
    .unwrap();
    assert_eq!(none.claimed, 0);
}

#[test]
fn every_lists_all_waiting_notes_and_marks_the_claimed() {
    let dir = repo();
    let root = dir.path();
    let counts = explanations::waiting_counts(root).unwrap();
    let plain = explanations::every(root, &[]).unwrap();
    assert_eq!(plain.notes.len(), counts.total());
    assert_eq!(plain.waiting, counts);
    assert_eq!(plain.claimed, 0);
    assert!(plain.notes.iter().all(|n| !n.claimed));

    let held = plain.notes[1].path.clone();
    let marked = explanations::every(root, std::slice::from_ref(&held)).unwrap();
    assert_eq!(marked.notes.len(), counts.total());
    assert_eq!(marked.claimed, 1);
    let paths =
        |q: &explanations::Queue| q.notes.iter().map(|n| n.path.clone()).collect::<Vec<_>>();
    assert_eq!(paths(&marked), paths(&plain));
    for note in &marked.notes {
        assert_eq!(note.claimed, note.path == held);
    }
    // Behind first, then by path.
    assert!(
        plain
            .notes
            .windows(2)
            .all(|w| (w[0].why, &w[0].path) <= (w[1].why, &w[1].path))
    );
}
