//! What a generated page says, apart from how it is drawn.
//!
//! A page function turns backend data into a [`Doc`]: a title, and blocks
//! such as tables, cards and rows. `doc_view` draws a `Doc` with the `mui`
//! components. Keeping the two apart lets a test read a page without a
//! window.

use crate::icons::Glyph;
use crate::motion::MotionPref;
use crate::mui::feedback::Tone;
use crate::settings::{Category, FieldId, Note};

/// What following a link or pressing a row does.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Open a note, by path or by name.
    Note(String),
    /// Open the page of a symbol, by qualified name.
    Symbol(String),
    /// Open the page of a source file.
    File(String),
    /// Choose how animations behave.
    Motion(MotionPref),
    /// Zoom the app to this level, in percent.
    Zoom(u16),
    /// Try to reach the backend again.
    Reconnect,
    /// Ask for a folder and open it as a repository.
    OpenFolder,
    /// Look for a newer release now.
    CheckUpdates,
    /// Download the newer release, install it and restart.
    InstallUpdate,
    /// Turn the daily check for updates on or off.
    AutoUpdate(bool),
    /// Start the app when the user signs in to Windows, or stop doing so.
    Autostart(bool),
    /// Let coding agents explain waiting notes by themselves at the start of a
    /// session, or stop them.
    AutoExplain(bool),
    /// Choose how many waiting notes a session explains by itself.
    AutoExplainCount(cw_core::auto_explain::Count),
    /// Start the app on the wiki map of the repository it opens, or on Home.
    OpenLast(bool),
    /// Draw the backdrop, its color and its sky of notes, or leave plain
    /// Mica behind the glass.
    Backdrop(bool),
    /// Show tabs, or keep one view that every note opens in.
    TabsOn(bool),
    /// Open every note the user opens in a tab of its own, or in the tab
    /// that shows.
    NotesInNewTab(bool),
    /// Send error reports to Sentry from the next start on, or stop at once.
    ErrorReports(bool),
    /// Open the registered repository with this id.
    OpenRepo(String),
    /// Take the repository with this id off the list. Its folder and the
    /// wiki in it are left alone.
    ForgetRepo(String),
    /// Keep coding agents out of this folder and everything below it.
    KeepAgentsOut(String),
    /// Take this folder off the list of those kept from coding agents.
    LetAgentsIn(String),
    /// Ask for a folder to keep coding agents out of.
    PickAgentsFolder,
    /// Replace an unreadable list of folders kept from coding agents with an
    /// empty one.
    ResetAgentsList,
    /// Bring the open repository's wiki up to date.
    Sync,
    /// Open the review page of the repository with this id, opening the
    /// repository first when another is open.
    OpenReview(String),
    /// Show the folder of the log files in Explorer.
    OpenLogs,
    /// Open the settings: at this category, or at the first.
    Settings(Option<Category>),
    /// Apply the theme with this name.
    Theme(String),
    /// Save a setting the backend keeps, named by its section and field in
    /// `GET /v1/settings`. No value puts it back to its default.
    SetSetting {
        section: String,
        field: String,
        value: Option<String>,
    },
    /// Keep this much in the app's log: `error`, `warn`, `info`, `debug` or
    /// `trace`.
    LogLevel(String),
    /// Show the input of a saved secret, to type another in its place.
    ReplaceSecret(FieldId),
    /// Leave the saved secret as it is after all.
    KeepSecret(FieldId),
    /// Forget the saved secret.
    RemoveSecret(FieldId),
    /// Send the chosen embedder of semantic search one small request and
    /// show whether it answered.
    TestEmbedder,
    /// Ask the chosen embedding server which models it has.
    RefreshEmbedModels,
    /// Check the backend typed into the fields and, when it answers, use it.
    ConnectRemote,
    /// Go back to the backend on this machine.
    UseLocalBackend,
    /// Forget the saved backend and its token.
    ForgetRemote,
    /// Show the data folder in Explorer.
    OpenDataFolder,
    /// Stop the job that embeds every repository's documents.
    CancelEmbedJob,
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
    /// Drawn as the current item of the list.
    pub selected: bool,
    pub action: Action,
    /// A small text button at the row's end. A click on it runs its own
    /// action, not the row's.
    pub aside: Option<(String, Action)>,
}

impl Row {
    pub fn new(icon: Glyph, label: impl Into<String>, action: Action) -> Self {
        Self {
            icon,
            label: label.into(),
            code: false,
            detail: None,
            selected: false,
            action,
            aside: None,
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

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// A text button at the row's end, with its own action.
    pub fn aside(mut self, label: impl Into<String>, action: Action) -> Self {
        self.aside = Some((label.into(), action));
        self
    }
}

/// One choice among a few, shown as a group of toggle buttons.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub label: String,
    pub action: Action,
    pub selected: bool,
    /// Drawn before the label, in a select and in its list.
    pub icon: Option<Glyph>,
}

/// Where a setting that is typed once and then hidden stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretState {
    /// Nothing is saved: the input shows.
    Empty,
    /// One is saved, and the input shows to type another in its place.
    Typing,
    /// One is saved; `hint` is its last characters, or empty for a short one.
    Saved { hint: String },
    /// One is saved that this user or this machine cannot read.
    Unreadable,
}

impl SecretState {
    /// What the page says in place of the input, when it shows none.
    pub fn text(&self) -> Option<String> {
        match self {
            Self::Empty | Self::Typing => None,
            Self::Saved { hint } if hint.is_empty() => Some("Saved".into()),
            Self::Saved { hint } => Some(format!("Saved, ends in {hint}")),
            Self::Unreadable => Some("Saved, but it cannot be read by this Windows user".into()),
        }
    }
}

/// One thing to press in a row of buttons.
#[derive(Debug, Clone, PartialEq)]
pub struct Press {
    pub label: String,
    pub action: Action,
    /// The main action of the row: drawn filled.
    pub primary: bool,
    pub disabled: bool,
}

impl Press {
    pub fn new(label: impl Into<String>, action: Action) -> Self {
        Self {
            label: label.into(),
            action,
            primary: false,
            disabled: false,
        }
    }

    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
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
        /// One thing to press at the alert's end.
        button: Option<(String, Action)>,
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
    /// A setting that is on or off. `action` is what a click asks for.
    /// `locked` says why it cannot be changed here, when it cannot.
    Switch {
        label: String,
        help: Option<String>,
        on: bool,
        action: Action,
        locked: Option<String>,
    },
    /// A setting with a few named values, shown as a list that drops down.
    /// The option in effect is the one `selected`.
    Select {
        label: String,
        help: Option<String>,
        options: Vec<Choice>,
        locked: Option<String>,
        note: Option<Note>,
    },
    /// A setting that is typed. Its text is in the input the app keeps for
    /// `id`, not here.
    Field {
        id: FieldId,
        label: String,
        help: Option<String>,
        locked: Option<String>,
        note: Option<Note>,
    },
    /// A setting that is typed once and never shown again, such as a key.
    Secret {
        id: FieldId,
        label: String,
        help: Option<String>,
        state: SecretState,
        locked: Option<String>,
        note: Option<Note>,
    },
    /// Things to press, side by side.
    Buttons(Vec<Press>),
    /// How far long work is: what it is doing, a bar, and the numbers.
    /// Without a `share` the bar says only that work goes on. A bar that is
    /// not `moving` stands still, as for work that waits.
    Progress {
        label: String,
        /// From 0 to 1.
        share: Option<f32>,
        detail: String,
        moving: bool,
    },
    /// The app's logo.
    Logo,
    /// The field that finds a project opened before, on Home. Its input
    /// belongs to the app, which lists what it finds under it; a page drawn
    /// without that input leaves the place empty.
    Search,
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
    pub fn flat(&self) -> Vec<&Block> {
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
                Block::Switch {
                    label, on, locked, ..
                } => out.push(with_state(
                    format!("{label}: {}", if *on { "on" } else { "off" }),
                    locked,
                    &None,
                )),
                Block::Select {
                    label,
                    options,
                    locked,
                    note,
                    ..
                } => {
                    let chosen = options
                        .iter()
                        .find(|option| option.selected)
                        .map_or("", |option| option.label.as_str());
                    let names: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
                    out.push(with_state(
                        format!("{label}: {chosen} ({})", names.join(" / ")),
                        locked,
                        note,
                    ));
                }
                Block::Field {
                    label,
                    locked,
                    note,
                    ..
                } => out.push(with_state(format!("{label}: <typed>"), locked, note)),
                Block::Secret {
                    label,
                    state,
                    locked,
                    note,
                    ..
                } => out.push(with_state(
                    format!(
                        "{label}: {}",
                        state.text().unwrap_or_else(|| "<typed, hidden>".into())
                    ),
                    locked,
                    note,
                )),
                Block::Buttons(buttons) => {
                    let labels: Vec<&str> = buttons.iter().map(|b| b.label.as_str()).collect();
                    out.push(labels.join(" | "));
                }
                Block::Progress {
                    label,
                    share,
                    detail,
                    ..
                } => {
                    let percent = share.map_or(String::new(), |share| {
                        format!(" {}%", (share.clamp(0.0, 1.0) * 100.0).round())
                    });
                    out.push(format!("{label}{percent}: {detail}"));
                }
                Block::Logo | Block::Search => {}
            }
            // The line of help under a control's name.
            if let Block::Switch {
                help: Some(help), ..
            }
            | Block::Select {
                help: Some(help), ..
            }
            | Block::Field {
                help: Some(help), ..
            }
            | Block::Secret {
                help: Some(help), ..
            } = block
            {
                out.push(help.clone());
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
                Block::Rows(rows) => {
                    for row in rows {
                        out.push(row.action.clone());
                        out.extend(row.aside.iter().map(|(_, action)| action.clone()));
                    }
                }
                Block::Alert { button, .. } | Block::Setting { button, .. } => {
                    out.extend(button.iter().map(|(_, action)| action.clone()))
                }
                Block::Choices { choices, .. } => {
                    out.extend(choices.iter().map(|choice| choice.action.clone()))
                }
                // A locked control offers nothing to do.
                Block::Switch {
                    action,
                    locked: None,
                    ..
                } => out.push(action.clone()),
                Block::Select {
                    options,
                    locked: None,
                    ..
                } => out.extend(options.iter().map(|option| option.action.clone())),
                Block::Secret {
                    id,
                    state,
                    locked: None,
                    ..
                } => match state {
                    SecretState::Saved { .. } | SecretState::Unreadable => {
                        out.push(Action::ReplaceSecret(*id));
                        out.push(Action::RemoveSecret(*id));
                    }
                    SecretState::Typing => out.push(Action::KeepSecret(*id)),
                    SecretState::Empty => {}
                },
                Block::Buttons(buttons) => out.extend(
                    buttons
                        .iter()
                        .filter(|button| !button.disabled)
                        .map(|button| button.action.clone()),
                ),
                _ => {}
            }
        }
        out
    }

    /// The fields the page asks the user to type into, in order. A locked
    /// one is listed too: its input shows the value, and takes none.
    pub fn fields(&self) -> Vec<FieldId> {
        self.flat()
            .into_iter()
            .filter_map(|block| match block {
                Block::Field { id, .. } => Some(*id),
                Block::Secret {
                    id,
                    state: SecretState::Empty | SecretState::Typing,
                    ..
                } => Some(*id),
                _ => None,
            })
            .collect()
    }
}

/// A line of [`Doc::words`] for a control, with why it is locked and the
/// news beside it.
#[cfg(test)]
fn with_state(mut line: String, locked: &Option<String>, note: &Option<Note>) -> String {
    if let Some(locked) = locked {
        line.push_str(&format!(" [{locked}]"));
    }
    match note {
        Some(Note::Working) => line.push_str(" (saving)"),
        Some(Note::Saved) => line.push_str(" (saved)"),
        Some(Note::Error(error)) => line.push_str(&format!(" (error: {error})")),
        None => {}
    }
    line
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
    fn a_rows_aside_action_is_listed_after_the_rows_own() {
        let mut doc = Doc::new("t");
        doc.push(Block::Rows(vec![
            Row::new(crate::icons::FOLDER, "one", Action::OpenRepo("a".into()))
                .aside("Keep agents out", Action::KeepAgentsOut("C:/a".into())),
            Row::new(crate::icons::FOLDER, "two", Action::OpenRepo("b".into())),
        ]));
        assert_eq!(
            doc.actions(),
            [
                Action::OpenRepo("a".into()),
                Action::KeepAgentsOut("C:/a".into()),
                Action::OpenRepo("b".into()),
            ]
        );
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
