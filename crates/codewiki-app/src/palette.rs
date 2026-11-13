//! The command palette's entries and matching. No UI here.

use crate::motion::MotionPref;

/// What running a palette entry does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Open the note at this vault path.
    Note(String),
    /// Apply the theme with this name.
    Theme(String),
    /// Switch to the repository with this id.
    Repo(String),
    /// Run a full-text search for this query.
    Search(String),
    Sync,
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
    Themes,
    Settings,
    /// Choose whether the app animates.
    Motion(MotionPref),
    NewTab,
    CloseTab,
    OpenInObsidian,
    /// Try to reach the backend again; listed only while it is unreachable.
    Reconnect,
    /// Ask for a folder and open it as a repository.
    OpenFolder,
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

/// The fixed commands, in the order shown for an empty query.
pub fn actions() -> Vec<Entry> {
    [
        ("Sync wiki", Command::Sync),
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
        ("Choose theme", Command::Themes),
        ("Open settings", Command::Settings),
        ("New tab", Command::NewTab),
        ("Close tab", Command::CloseTab),
        ("Open note in Obsidian", Command::OpenInObsidian),
        ("Open folder…", Command::OpenFolder),
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
    fn empty_query_lists_entries_in_order_without_search() {
        let all = filter(&entries(), "  ", 3);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].command, Command::Sync);
        assert!(all.iter().all(|e| e.hint != "Search"));
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
