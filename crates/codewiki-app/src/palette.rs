//! The command palette's entries and matching. No UI here.

use crate::motion::MotionPref;
use crate::settings::Category;

/// What running a palette entry does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Show the Home page: Open folder and the recent projects.
    Home,
    /// Open the note at this vault path.
    Note(String),
    /// Apply the theme with this name.
    Theme(String),
    /// Switch to the repository with this id.
    Repo(String),
    /// Run a full-text search for this query.
    Search(String),
    Sync,
    /// Open the review page: the proposals that wait for a decision.
    Review,
    Graph,
    Hotspots,
    Ownership,
    Coupling,
    Decisions,
    DeadCode,
    Health,
    Drift,
    Tests,
    Coverage,
    Security,
    Refactor,
    Workspace,
    Contracts,
    /// Show the notes that wait for a coding agent to explain them.
    Waiting,
    Themes,
    /// Open the settings: at this category, or where they were left.
    Settings(Option<Category>),
    /// Choose whether the app animates.
    Motion(MotionPref),
    /// Make the whole app one level larger.
    ZoomIn,
    /// Make the whole app one level smaller.
    ZoomOut,
    /// Bring the app back to 100%.
    ZoomReset,
    NewTab,
    CloseTab,
    /// Hide the sidebar, or show it again.
    ToggleSidebar,
    /// Hide the details pane, or show it again.
    ToggleDetails,
    /// Try to reach the backend again; listed only while it is unreachable.
    Reconnect,
    /// Ask for a folder and open it as a repository.
    OpenFolder,
    /// Look for a newer release and show the result in the settings.
    CheckUpdates,
    /// Install the newer release and restart; listed only when there is one.
    InstallUpdate,
    /// Show the folder of the log files in Explorer.
    OpenLogs,
    /// End the app and its backend. Closing the window only hides it.
    Quit,
    /// Start the app when the user signs in to Windows, or stop doing so;
    /// listed only for an installed copy.
    Autostart(bool),
    /// Keep coding agents out of the open project's folder; listed while
    /// nothing covers it.
    KeepAgentsOut,
    /// Let coding agents into the open project again; listed while an entry
    /// for exactly its folder covers it.
    LetAgentsIn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub label: String,
    /// Short category shown beside the label.
    pub hint: &'static str,
    pub command: Command,
}

impl Entry {
    pub fn new(label: impl Into<String>, hint: &'static str, command: Command) -> Self {
        Self {
            label: label.into(),
            hint,
            command,
        }
    }
}

/// The command that changes the list for the open project, judged by its
/// registered folder. None when a parent entry covers it, or when it holds
/// an entry: those are changed in Settings.
pub fn agents_entry(kept: &crate::live::Kept) -> Option<Entry> {
    use crate::live::Kept;
    match kept {
        Kept::Open => Some(Entry::new(
            "Keep agents out of this project",
            "Command",
            Command::KeepAgentsOut,
        )),
        Kept::Own => Some(Entry::new(
            "Let agents into this project",
            "Command",
            Command::LetAgentsIn,
        )),
        Kept::Under(_) | Kept::Contains(_) => None,
    }
}

/// The fixed commands, in the order shown for an empty query.
pub fn actions() -> Vec<Entry> {
    [
        ("Home", Command::Home),
        ("Sync wiki", Command::Sync),
        ("Review", Command::Review),
        ("Open graph", Command::Graph),
        ("Show hotspots", Command::Hotspots),
        ("Show ownership", Command::Ownership),
        ("Show change coupling", Command::Coupling),
        ("Show decisions", Command::Decisions),
        ("Show code health", Command::Health),
        ("Show dead code", Command::DeadCode),
        ("Show documentation drift", Command::Drift),
        ("Show tests", Command::Tests),
        ("Show coverage", Command::Coverage),
        ("Show security findings", Command::Security),
        ("Show refactoring plan", Command::Refactor),
        ("Show workspace", Command::Workspace),
        ("Show API contracts", Command::Contracts),
        ("Show waiting notes", Command::Waiting),
        ("Choose theme", Command::Themes),
        ("Open settings", Command::Settings(None)),
        ("New tab", Command::NewTab),
        ("Close tab", Command::CloseTab),
        ("Toggle sidebar", Command::ToggleSidebar),
        ("Toggle details pane", Command::ToggleDetails),
        ("Zoom in", Command::ZoomIn),
        ("Zoom out", Command::ZoomOut),
        ("Reset zoom", Command::ZoomReset),
        ("Open folder…", Command::OpenFolder),
        ("Check for updates", Command::CheckUpdates),
        ("Open logs folder", Command::OpenLogs),
        ("Quit CodeWiki", Command::Quit),
    ]
    .into_iter()
    .map(|(label, command)| Entry::new(label, "Command", command))
    .chain(MotionPref::ALL.into_iter().map(|pref| {
        Entry::new(
            format!("Animations: {}", pref.label()),
            "Setting",
            Command::Motion(pref),
        )
    }))
    // Each category of the settings, so that typing its name leads there.
    .chain(Category::ALL.into_iter().map(|category| {
        Entry::new(
            format!("Settings: {}", category.label()),
            "Settings",
            Command::Settings(Some(category)),
        )
    }))
    .collect()
}

/// How well `label` matches the query words; lower is better, `None` is no
/// match. Every word must occur. A match at the start of the label beats a
/// match at the start of a word, which beats a match inside a word.
fn rank(label: &str, words: &[String]) -> Option<usize> {
    let label = label.to_lowercase();
    let mut total = 0;
    for word in words {
        let position = label.find(word.as_str())?;
        let at_word_start = position == 0
            || label[..position]
                .chars()
                .next_back()
                .is_some_and(|c| !c.is_alphanumeric());
        total += match (position, at_word_start) {
            (0, _) => 0,
            (_, true) => 100,
            _ => 200,
        } + position.min(99);
    }
    Some(total)
}

/// Entries matching `query`, best first, at most `limit`. Ties keep the
/// order of `entries`. A non-empty query always ends with a full-text search
/// entry, so Enter does something even when nothing matches.
pub fn filter(entries: &[Entry], query: &str, limit: usize) -> Vec<Entry> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return entries.iter().take(limit).cloned().collect();
    }
    let mut ranked: Vec<(usize, &Entry)> = entries
        .iter()
        .filter_map(|entry| rank(&entry.label, &words).map(|score| (score, entry)))
        .collect();
    ranked.sort_by_key(|(score, _)| *score);
    let mut out: Vec<Entry> = ranked
        .into_iter()
        .take(limit.saturating_sub(1))
        .map(|(_, entry)| entry.clone())
        .collect();
    let query = query.trim();
    out.push(Entry::new(
        format!("Search for \"{query}\""),
        "Search",
        Command::Search(query.to_string()),
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<Entry> {
        vec![
            Entry::new("Sync wiki", "Command", Command::Sync),
            Entry::new(
                "Graph Module",
                "Note",
                Command::Note("a/Graph Module.md".into()),
            ),
            Entry::new("Open graph", "Command", Command::Graph),
            Entry::new("Paragraph", "Note", Command::Note("a/Paragraph.md".into())),
        ]
    }

    #[test]
    fn the_agents_command_follows_the_projects_state() {
        use crate::live::Kept;
        use std::path::PathBuf;
        let command = |kept: Kept| agents_entry(&kept).map(|entry| entry.command);
        assert_eq!(command(Kept::Open), Some(Command::KeepAgentsOut));
        assert_eq!(command(Kept::Own), Some(Command::LetAgentsIn));
        assert_eq!(command(Kept::Under(PathBuf::from(r"C:\code"))), None);
        assert_eq!(
            command(Kept::Contains(PathBuf::from(r"C:\code\alpha\acme"))),
            None
        );
    }

    #[test]
    fn empty_query_lists_entries_in_order_without_search() {
        let all = filter(&entries(), "  ", 3);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].command, Command::Sync);
        assert!(all.iter().all(|e| e.hint != "Search"));
    }

    #[test]
    fn each_category_of_the_settings_has_an_entry() {
        let all = actions();
        for category in Category::ALL {
            assert!(
                all.iter()
                    .any(|entry| entry.command == Command::Settings(Some(category))),
                "{category:?}"
            );
        }
        let found = filter(&all, "settings search", 5);
        assert_eq!(found[0].label, "Settings: Semantic search");
        assert_eq!(found[0].command, Command::Settings(Some(Category::Search)));
        // The plain entry still opens the settings where they were left.
        let found = filter(&all, "open settings", 5);
        assert_eq!(found[0].command, Command::Settings(None));
    }

    #[test]
    fn the_logs_folder_opens_from_the_palette() {
        let found = filter(&actions(), "logs", 5);
        assert_eq!(found[0].label, "Open logs folder");
        assert_eq!(found[0].command, Command::OpenLogs);
    }

    #[test]
    fn the_waiting_notes_open_from_the_palette() {
        let found = filter(&actions(), "waiting", 5);
        assert_eq!(found[0].label, "Show waiting notes");
        assert_eq!(found[0].command, Command::Waiting);
    }

    #[test]
    fn zoom_is_in_the_palette() {
        let all = actions();
        assert_eq!(filter(&all, "zoom in", 5)[0].command, Command::ZoomIn);
        assert_eq!(filter(&all, "zoom out", 5)[0].command, Command::ZoomOut);
        assert_eq!(filter(&all, "reset zoom", 5)[0].command, Command::ZoomReset);
        let labels: std::collections::HashSet<&str> =
            all.iter().map(|entry| entry.label.as_str()).collect();
        assert_eq!(labels.len(), all.len(), "labels must be unique");
    }

    #[test]
    fn ranks_label_start_then_word_start_then_inside() {
        let hits = filter(&entries(), "GRAPH", 10);
        let labels: Vec<&str> = hits.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Graph Module",
                "Open graph",
                "Paragraph",
                "Search for \"GRAPH\""
            ]
        );
    }

    #[test]
    fn every_word_must_match_and_search_is_always_last() {
        let hits = filter(&entries(), "graph mod", 10);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].label, "Graph Module");
        assert_eq!(hits[1].command, Command::Search("graph mod".into()));
        let none = filter(&entries(), "zzz", 10);
        assert_eq!(none.len(), 1);
        assert_eq!(none[0].command, Command::Search("zzz".into()));
    }

    #[test]
    fn limit_counts_the_search_entry() {
        assert_eq!(filter(&entries(), "a", 2).len(), 2);
    }

    #[test]
    fn actions_have_unique_labels() {
        let mut labels: Vec<String> = actions().into_iter().map(|e| e.label).collect();
        let count = labels.len();
        labels.sort();
        labels.dedup();
        assert_eq!(labels.len(), count);
    }
}
