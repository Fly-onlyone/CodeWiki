//! What a generated page says, apart from how it is drawn.
//!
//! A page function turns backend data into a [`Doc`]: a title, and blocks
//! such as tables, cards and rows. `doc_view` draws a `Doc` with the `mui`
//! components. Keeping the two apart lets a test read a page without a
//! window.

use crate::icons::Glyph;
use crate::motion::MotionPref;
use crate::mui::feedback::Tone;

/// What following a link or pressing a row does.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Open a note, by path or by name.
    Note(String),
    /// Open the page of a symbol, by qualified name.
    Symbol(String),
    /// Open the page of a source file.
    File(String),
    /// Apply a theme.
    Theme(String),
    /// Open the theme picker.
    Themes,
    /// Choose how animations behave.
    Motion(MotionPref),
    /// Try to reach the backend again.
    Reconnect,
    /// Ask for a folder and open it as a repository.
    OpenFolder,
}

/// A stretch of text within a line.
#[derive(Debug, Clone, PartialEq)]
pub enum Span {
    Text(String),
    Strong(String),
    /// Set in the code font.
    Code(String),
    Link {
        label: String,
        /// Set the label in the code font, as for a path or a symbol.
        code: bool,
        action: Action,
    },
}

/// A line of text made of spans.
pub type Line = Vec<Span>;

pub fn text(text: impl Into<String>) -> Span {
    Span::Text(text.into())
}

pub fn strong(text: impl Into<String>) -> Span {
    Span::Strong(text.into())
}

pub fn code(text: impl Into<String>) -> Span {
    Span::Code(text.into())
}

/// Text from the backend, in which backticks mark code: `like this`.
pub fn rich(source: &str) -> Line {
    source
        .split('`')
        .enumerate()
        .filter(|(_, part)| !part.is_empty())
        // Every second part lies between two backticks.
        .map(|(index, part)| {
            if index % 2 == 1 {
                code(part)
            } else {
                text(part)
            }
        })
        .collect()
}

/// A link to a source file, labelled with its path.
pub fn file(path: &str) -> Span {
    Span::Link {
        label: path.to_string(),
        code: true,
        action: Action::File(path.to_string()),
    }
}

/// A link to a symbol, labelled with the last part of its name.
pub fn symbol(qualified: &str) -> Span {
    let label = qualified.rsplit("::").next().unwrap_or(qualified);
    Span::Link {
        label: label.to_string(),
        code: true,
        action: Action::Symbol(qualified.to_string()),
    }
}

/// A link to a note, labelled `label`.
pub fn note(label: impl Into<String>, target: impl Into<String>) -> Span {
    Span::Link {
        label: label.into(),
        code: false,
        action: Action::Note(target.into()),
    }
}

/// The text of a line with its styling dropped, for tests.
#[cfg(test)]
pub fn plain(line: &[Span]) -> String {
    line.iter()
        .map(|span| match span {
            Span::Text(text) | Span::Strong(text) | Span::Code(text) => text.as_str(),
            Span::Link { label, .. } => label.as_str(),
        })
        .collect()
}

/// One headline number.
#[derive(Debug, Clone, PartialEq)]
pub struct Stat {
    pub label: String,
    pub value: String,
    pub hint: Option<String>,
    pub tone: Tone,
}

impl Stat {
    pub fn new(label: impl Into<String>, value: impl ToString) -> Self {
        Self {
            label: label.into(),
            value: value.to_string(),
            hint: None,
            tone: Tone::Neutral,
        }
    }

    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

/// A column of a table.
#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub label: String,
    /// Numbers are set to the right.
    pub numeric: bool,
    /// Share of the table's width, relative to the other columns.
    pub weight: f32,
}

impl Column {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            numeric: false,
            weight: 1.0,
        }
    }

    /// A column of numbers: narrow, and set to the right.
    pub fn numeric(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            numeric: true,
            weight: 0.5,
        }
    }

    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }
}

/// What a table cell holds.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Line(Line),
    Chip {
        label: String,
        tone: Tone,
    },
    /// A share from 0 to 1, drawn as a bar beside its label.
    Share {
        label: String,
        share: f32,
        tone: Tone,
    },
}

impl Cell {
    pub fn text(value: impl ToString) -> Self {
        Self::Line(vec![text(value.to_string())])
    }

    pub fn chip(label: impl Into<String>, tone: Tone) -> Self {
        Self::Chip {
            label: label.into(),
            tone,
        }
    }

    /// What the cell reads as, for tests.
    #[cfg(test)]
    pub fn plain(&self) -> String {
        match self {
            Self::Line(line) => plain(line),
            Self::Chip { label, .. } | Self::Share { label, .. } => label.clone(),
        }
    }
}

impl From<Span> for Cell {
    fn from(span: Span) -> Self {
        Self::Line(vec![span])
    }
}

impl From<Line> for Cell {
    fn from(line: Line) -> Self {
        Self::Line(line)
    }
}

/// A table: columns, and rows of one cell for each.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Table {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Cell>>,
}

impl Table {
    pub fn new(columns: impl IntoIterator<Item = Column>) -> Self {
        Self {
            columns: columns.into_iter().collect(),
            rows: Vec::new(),
        }
    }

    pub fn row(&mut self, cells: impl IntoIterator<Item = Cell>) {
        let cells: Vec<Cell> = cells.into_iter().collect();
        debug_assert_eq!(cells.len(), self.columns.len(), "a cell for each column");
        self.rows.push(cells);
    }

    /// Each row as the text of its cells, for tests.
    #[cfg(test)]
    pub fn plain(&self) -> Vec<Vec<String>> {
        self.rows
            .iter()
            .map(|row| row.iter().map(Cell::plain).collect())
            .collect()
    }
}

/// A row that can be pressed.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub icon: Glyph,
    pub label: String,
    /// Set the label in the code font.
    pub code: bool,
    /// A second, quieter text after the label.
    pub detail: Option<String>,
    pub action: Action,
}

impl Row {
    pub fn new(icon: Glyph, label: impl Into<String>, action: Action) -> Self {
        Self {
            icon,
            label: label.into(),
            code: false,
            detail: None,
            action,
        }
    }

    pub fn code(mut self) -> Self {
        self.code = true;
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// One choice among a few, shown as a group of toggle buttons.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub label: String,
    pub action: Action,
    pub selected: bool,
}

/// A theme in the theme picker.
#[derive(Debug, Clone, PartialEq)]
pub struct Swatch {
    pub name: String,
    pub active: bool,
}

/// A part of a page.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    /// The title of a section.
    Heading(String),
    /// A paragraph.
    Text(Line),
    Stats(Vec<Stat>),
    Table(Table),
    /// A titled group, with small labels beside the title.
    Card {
        title: String,
        chips: Vec<(String, Tone)>,
        body: Vec<Block>,
    },
    List {
        items: Vec<Line>,
        ordered: bool,
    },
    Rows(Vec<Row>),
    Code {
        language: String,
        text: String,
    },
    Alert {
        tone: Tone,
        text: String,
    },
    /// What a page says when it has nothing to show.
    Empty(String),
    /// A label, with one thing to press beside it.
    Setting {
        label: String,
        value: String,
        button: Option<(String, Action)>,
    },
    Choices {
        label: String,
        choices: Vec<Choice>,
    },
    /// Themes to pick from, under a heading for each mode.
    Swatches(Vec<Swatch>),
}

/// A generated page.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Doc {
    pub title: String,
    /// What the page is, under its title.
    pub lead: Option<String>,
    pub blocks: Vec<Block>,
}

impl Doc {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            lead: None,
            blocks: Vec::new(),
        }
    }

    pub fn lead(mut self, lead: impl Into<String>) -> Self {
        self.lead = Some(lead.into());
        self
    }

    pub fn push(&mut self, block: Block) {
        self.blocks.push(block);
    }

    pub fn heading(&mut self, title: impl Into<String>) {
        self.push(Block::Heading(title.into()));
    }

    pub fn text(&mut self, line: impl Into<String>) {
        self.push(Block::Text(vec![text(line)]));
    }
}

/// Helpers for tests that read a page.
#[cfg(test)]
impl Doc {
    /// Every block, with the bodies of cards opened out.
    fn flat(&self) -> Vec<&Block> {
        fn walk<'a>(blocks: &'a [Block], out: &mut Vec<&'a Block>) {
            for block in blocks {
                out.push(block);
                if let Block::Card { body, .. } = block {
                    walk(body, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.blocks, &mut out);
        out
    }

    /// The tables of the page, in order.
    pub fn tables(&self) -> Vec<&Table> {
        self.flat()
            .into_iter()
            .filter_map(|block| match block {
                Block::Table(table) => Some(table),
                _ => None,
            })
            .collect()
    }

    /// Section headings and card titles, in order.
    pub fn headings(&self) -> Vec<&str> {
        self.flat()
            .into_iter()
            .filter_map(|block| match block {
                Block::Heading(title) | Block::Card { title, .. } => Some(title.as_str()),
                _ => None,
            })
            .collect()
    }

    /// All the words of the page, one block to a line.
    pub fn words(&self) -> String {
        let mut out = vec![self.title.clone()];
        out.extend(self.lead.clone());
        for block in self.flat() {
            match block {
                Block::Heading(title) => out.push(title.clone()),
                Block::Text(line) => out.push(plain(line)),
                Block::Stats(stats) => {
                    out.extend(stats.iter().map(|s| format!("{}: {}", s.label, s.value)))
                }
                Block::Table(table) => out.extend(table.plain().into_iter().map(|r| r.join(" | "))),
                Block::Card { title, chips, .. } => {
                    let chips: Vec<&str> = chips.iter().map(|(label, _)| label.as_str()).collect();
                    out.push(format!("{title} [{}]", chips.join(", ")));
                }
                Block::List { items, .. } => out.extend(items.iter().map(|line| plain(line))),
                Block::Rows(rows) => out.extend(rows.iter().map(|row| match &row.detail {
                    Some(detail) => format!("{} — {detail}", row.label),
                    None => row.label.clone(),
                })),
                Block::Code { text, .. } => out.push(text.clone()),
                Block::Alert { text, .. } | Block::Empty(text) => out.push(text.clone()),
                Block::Setting { label, value, .. } => out.push(format!("{label}: {value}")),
                Block::Choices { label, choices } => {
                    let names: Vec<&str> = choices.iter().map(|c| c.label.as_str()).collect();
                    out.push(format!("{label}: {}", names.join(" / ")));
                }
                Block::Swatches(swatches) => out.extend(swatches.iter().map(|s| s.name.clone())),
            }
        }
        out.join("\n")
    }

    /// Every action a link, row or button of the page carries.
    pub fn actions(&self) -> Vec<Action> {
        fn of_line(line: &[Span], out: &mut Vec<Action>) {
            for span in line {
                if let Span::Link { action, .. } = span {
                    out.push(action.clone());
                }
            }
        }
        let mut out = Vec::new();
        for block in self.flat() {
            match block {
                Block::Text(line) => of_line(line, &mut out),
                Block::Table(table) => {
                    for cell in table.rows.iter().flatten() {
                        if let Cell::Line(line) = cell {
                            of_line(line, &mut out);
                        }
                    }
                }
                Block::List { items, .. } => items.iter().for_each(|line| of_line(line, &mut out)),
                Block::Rows(rows) => out.extend(rows.iter().map(|row| row.action.clone())),
                Block::Setting { button, .. } => {
                    out.extend(button.iter().map(|(_, action)| action.clone()))
                }
                Block::Choices { choices, .. } => {
                    out.extend(choices.iter().map(|choice| choice.action.clone()))
                }
                Block::Swatches(swatches) => {
                    out.extend(swatches.iter().map(|s| Action::Theme(s.name.clone())))
                }
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_symbol_link_shows_the_last_part_of_the_name() {
        assert_eq!(plain(&[symbol("demo::core::compute")]), "compute");
        assert_eq!(plain(&[symbol("main")]), "main");
        let Span::Link { action, code, .. } = symbol("demo::run") else {
            panic!("not a link");
        };
        assert_eq!(action, Action::Symbol("demo::run".into()));
        assert!(code);
    }

    #[test]
    fn backticks_in_backend_text_mark_code() {
        assert_eq!(
            rich("`run` is 550 lines; depends on `a.rs`"),
            [
                code("run"),
                text(" is 550 lines; depends on "),
                code("a.rs")
            ]
        );
        assert_eq!(rich("plain"), [text("plain")]);
        assert!(rich("").is_empty());
        // An unclosed backtick still shows its text.
        assert_eq!(plain(&rich("a `b")), "a b");
    }

    #[test]
    fn a_line_reads_without_its_styling() {
        let line = vec![text("in "), file("src/a.rs"), text(", "), strong("twice")];
        assert_eq!(plain(&line), "in src/a.rs, twice");
    }

    #[test]
    fn a_page_gives_up_its_words_and_actions() {
        let mut doc = Doc::new("Report").lead("What it is.");
        doc.heading("Files");
        let mut table = Table::new([Column::new("File"), Column::numeric("Lines")]);
        table.row([file("src/a.rs").into(), Cell::text(12)]);
        doc.push(Block::Table(table));
        doc.push(Block::Card {
            title: "Inside".into(),
            chips: vec![("stale".into(), Tone::Warning)],
            body: vec![Block::Text(vec![note("Parser", "01-Core/Parser")])],
        });
        assert_eq!(doc.headings(), ["Files", "Inside"]);
        assert_eq!(doc.tables()[0].plain(), [["src/a.rs", "12"]]);
        let words = doc.words();
        assert!(words.starts_with("Report\nWhat it is.\nFiles\n"));
        assert!(words.contains("src/a.rs | 12"));
        assert!(words.contains("Inside [stale]"));
        assert_eq!(
            doc.actions(),
            [
                Action::File("src/a.rs".into()),
                Action::Note("01-Core/Parser".into())
            ]
        );
    }
}
