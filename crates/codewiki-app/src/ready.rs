//! The lines the app writes to its log when a page has been drawn with its
//! data.
//!
//! A program that drives the app from outside cannot see what the window
//! shows, and UI Automation cannot read it. Such a program waits for these
//! lines instead of a fixed time: `Wait-AppPage` in
//! `scripts/vm/test-release.ps1`.
//!
//! A line names the kind of the page and, for a note, the note's name. It
//! never holds a search, a palette query, a file's path or the title of a
//! generated page, which can be any of them.

use crate::Page;

/// The log target of the lines. `cw_telemetry` leaves it out of the error
/// reports, so a note's name stays on the machine.
pub const TARGET: &str = "page";

/// The kind of a page, as its line names it.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Home,
    /// A note, by the name of its file without `.md`.
    Note(String),
    /// A note as a coding agent reads it.
    Agent(String),
    Graph,
    Review,
    Waiting,
    /// The settings, at the category with this slug.
    Settings(&'static str),
    /// Symbol detail, a search, a report: any page built from backend data.
    Generated,
    /// A message with something to press, such as "Try again".
    Notice,
}

/// What arrives after a page was drawn and changes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fill {
    /// The diagrams of a note, which are drawn off the UI thread.
    Diagrams,
    /// The models of the embedding server, on the settings page.
    Models,
}

/// What the app holds of the data its pages are built from.
#[derive(Debug, Clone, Copy, Default)]
pub struct Facts {
    /// A backend answered.
    pub connected: bool,
    /// A repository is open.
    pub repo_open: bool,
    /// The open repository's list of notes has arrived.
    pub notes_loaded: bool,
    /// The list of the notes that wait has arrived.
    pub waiting_loaded: bool,
    /// The backend's settings have arrived, or the reason they did not.
    pub settings_loaded: bool,
}

/// The kind of `page`, once the data it shows has arrived. A plain message is
/// never announced: it stands in for a page that is still to come.
pub fn kind(page: &Page, facts: Facts) -> Option<Kind> {
    match page {
        Page::Message(_) => None,
        Page::Notice(_) => Some(Kind::Notice),
        Page::Note { path, agent, .. } => {
            let name = stem(path).to_string();
            Some(if *agent {
                Kind::Agent(name)
            } else {
                Kind::Note(name)
            })
        }
        Page::Generated { .. } => Some(Kind::Generated),
        Page::Review { .. } => Some(Kind::Review),
        Page::Graph => Some(Kind::Graph),
        Page::Home => {
            (facts.connected && (!facts.repo_open || facts.notes_loaded)).then_some(Kind::Home)
        }
        Page::Waiting => facts.waiting_loaded.then_some(Kind::Waiting),
        Page::Settings(category) => facts
            .settings_loaded
            .then_some(Kind::Settings(category.slug())),
    }
}

/// The line for a page that was drawn. `asked` is the label of the view a
/// launch named, when this page is the answer to it.
pub fn drawn(kind: &Kind, asked: Option<&str>) -> String {
    let page = match kind {
        Kind::Home => "home".to_string(),
        Kind::Note(name) => format!("note {name}"),
        Kind::Agent(name) => format!("agent {name}"),
        Kind::Graph => "graph".to_string(),
        Kind::Review => "review".to_string(),
        Kind::Waiting => "waiting".to_string(),
        Kind::Settings(slug) => format!("settings:{slug}"),
        Kind::Generated => "generated".to_string(),
        Kind::Notice => "notice".to_string(),
    };
    match asked {
        Some(label) => format!("page drawn: {page} (asked {label})"),
        None => format!("page drawn: {page}"),
    }
}

/// The line for something that arrived after its page was drawn.
pub fn filled(fill: Fill) -> String {
    let what = match fill {
        Fill::Diagrams => "diagrams",
        Fill::Models => "models",
    };
    format!("page filled: {what}")
}

/// The label of the view a launch named, as `drawn` repeats it: the argument
/// as it was given, without a palette query or a file's path, and `default`
/// for a launch that named none.
pub fn label(argument: &str) -> String {
    if argument.is_empty() {
        return "default".to_string();
    }
    for form in ["palette:", "file:"] {
        if argument.starts_with(form) {
            return form.to_string();
        }
    }
    argument.to_string()
}

/// The name of the note at the vault path `path`: its file name without `.md`.
fn stem(path: &str) -> &str {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.strip_suffix(".md").unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::{Facts, Fill, Kind, drawn, filled, kind, label, stem};
    use crate::Page;
    use crate::settings::Category;

    fn note(path: &str, agent: bool) -> Page {
        Page::Note {
            path: path.to_string(),
            text: String::new(),
            sources: Vec::new(),
            state: String::new(),
            figures: Vec::new(),
            explanation: String::new(),
            explained_by: String::new(),
            behind: Vec::new(),
            agent,
            left_out: Vec::new(),
        }
    }

    #[test]
    fn a_page_is_announced_only_with_its_data() {
        let nothing = Facts::default();
        assert_eq!(kind(&Page::Message("Loading".into()), nothing), None);
        assert_eq!(kind(&Page::Home, nothing), None);
        assert_eq!(kind(&Page::Waiting, nothing), None);
        assert_eq!(kind(&Page::Settings(Category::Search), nothing), None);

        let connected = Facts {
            connected: true,
            ..nothing
        };
        assert_eq!(kind(&Page::Home, connected), Some(Kind::Home));
        // With a repository open, Home lists it with its notes.
        let opening = Facts {
            repo_open: true,
            ..connected
        };
        assert_eq!(kind(&Page::Home, opening), None);
        let open = Facts {
            notes_loaded: true,
            ..opening
        };
        assert_eq!(kind(&Page::Home, open), Some(Kind::Home));

        let waiting = Facts {
            waiting_loaded: true,
            ..connected
        };
        assert_eq!(kind(&Page::Waiting, waiting), Some(Kind::Waiting));
        let settings = Facts {
            settings_loaded: true,
            ..connected
        };
        assert_eq!(
            kind(&Page::Settings(Category::Search), settings),
            Some(Kind::Settings("search"))
        );
        // These are shown only once their data is there.
        assert_eq!(kind(&Page::Graph, nothing), Some(Kind::Graph));
        assert_eq!(
            kind(&note("01-Core/cw-core/_index.md", false), nothing),
            Some(Kind::Note("_index".into()))
        );
        assert_eq!(
            kind(&note("_HOME.md", true), nothing),
            Some(Kind::Agent("_HOME".into()))
        );
    }

    #[test]
    fn the_lines_read_as_the_rig_expects() {
        assert_eq!(drawn(&Kind::Home, None), "page drawn: home");
        assert_eq!(
            drawn(&Kind::Home, Some("default")),
            "page drawn: home (asked default)"
        );
        assert_eq!(
            drawn(&Kind::Note("_HOME".into()), Some("_HOME")),
            "page drawn: note _HOME (asked _HOME)"
        );
        assert_eq!(
            drawn(&Kind::Agent("Logs Module".into()), None),
            "page drawn: agent Logs Module"
        );
        assert_eq!(
            drawn(&Kind::Settings("search"), Some("settings:search")),
            "page drawn: settings:search (asked settings:search)"
        );
        assert_eq!(
            drawn(&Kind::Generated, Some("health:")),
            "page drawn: generated (asked health:)"
        );
        assert_eq!(
            drawn(&Kind::Graph, Some("graph:")),
            "page drawn: graph (asked graph:)"
        );
        assert_eq!(drawn(&Kind::Review, None), "page drawn: review");
        assert_eq!(drawn(&Kind::Waiting, None), "page drawn: waiting");
        assert_eq!(drawn(&Kind::Notice, None), "page drawn: notice");
        assert_eq!(filled(Fill::Diagrams), "page filled: diagrams");
        assert_eq!(filled(Fill::Models), "page filled: models");
    }

    #[test]
    fn the_line_leaves_out_a_palette_query_and_a_file_path() {
        assert_eq!(label(""), "default");
        assert_eq!(label("home:"), "home:");
        assert_eq!(label("settings:search"), "settings:search");
        assert_eq!(label("edit:_HOME"), "edit:_HOME");
        assert_eq!(label("palette:Quit CodeWiki"), "palette:");
        assert_eq!(label(r"file:C:\Users\me\secret plan.rs"), "file:");
        assert_eq!(label("Logs Module"), "Logs Module");
    }

    #[test]
    fn a_note_is_named_by_its_file() {
        assert_eq!(stem("01-Core/cw-core/_index.md"), "_index");
        assert_eq!(stem(r"01-Core\cw-core\Logs Module.md"), "Logs Module");
        assert_eq!(stem("_HOME"), "_HOME");
    }
}
