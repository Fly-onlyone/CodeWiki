//! Text logic for the note editor: wikilink completion and line endings.

/// The partly typed wikilink target before the cursor, if the cursor is
/// inside an open `[[`. `cursor` is a byte offset into `text`.
///
/// `see [[Par|` gives `Par`; a closed link, a line break or an alias part
/// (`[[Name|`) before the cursor gives `None`.
pub fn wikilink_query(text: &str, cursor: usize) -> Option<&str> {
    let before = text.get(..cursor)?;
    let line = before.rsplit('\n').next().unwrap_or(before);
    let open = line.rfind("[[")?;
    let typed = &line[open + 2..];
    if typed.contains("]]") || typed.contains('|') || typed.contains('[') {
        return None;
    }
    Some(typed)
}

/// Note names to offer for `query`, at most `limit`: names starting with the
/// query first, then names containing it, each group in the given order.
/// Matching ignores case. Map notes (`_index`, `_HOME`) are offered only
/// when the query starts with an underscore.
pub fn completions<'a>(names: &'a [String], query: &str, limit: usize) -> Vec<&'a str> {
    let needle = query.to_lowercase();
    let wants_maps = needle.starts_with('_');
    let mut starts = Vec::new();
    let mut contains = Vec::new();
    for name in names {
        if name.starts_with('_') && !wants_maps {
            continue;
        }
        let lower = name.to_lowercase();
        if lower.starts_with(&needle) {
            starts.push(name.as_str());
        } else if lower.contains(&needle) {
            contains.push(name.as_str());
        }
    }
    starts.extend(contains);
    starts.dedup();
    starts.truncate(limit);
    starts
}

/// Give `edited` the line endings `original` used. The editor works with
/// `\n`; a note saved with `\r\n` must keep them so an edit changes only
/// the lines the author touched.
pub fn restore_line_endings(original: &str, edited: &str) -> String {
    let unix = edited.replace("\r\n", "\n");
    if original.contains("\r\n") {
        unix.replace('\n', "\r\n")
    } else {
        unix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_open_wikilink_before_the_cursor() {
        let text = "see [[Par and more";
        assert_eq!(wikilink_query(text, 9), Some("Par"));
        assert_eq!(wikilink_query(text, 6), Some(""));
        assert_eq!(wikilink_query("a [[Done]] b", 12), None);
        assert_eq!(wikilink_query("a [[Done]] and [[Ne", 19), Some("Ne"));
        assert_eq!(wikilink_query("[[Name|ali", 10), None);
        assert_eq!(wikilink_query("[[open\nnext", 11), None);
        assert_eq!(wikilink_query("plain", 5), None);
        // A cursor inside a multi-byte character is not a valid position.
        assert_eq!(wikilink_query("[[é", 3), None);
        assert_eq!(wikilink_query("[[é", 4), Some("é"));
    }

    #[test]
    fn offers_prefix_matches_before_inner_matches() {
        let names: Vec<String> = ["Graph Module", "Paragraph", "_index", "graph", "Parser"]
            .map(String::from)
            .to_vec();
        assert_eq!(
            completions(&names, "gra", 10),
            ["Graph Module", "graph", "Paragraph"]
        );
        assert_eq!(completions(&names, "par", 1), ["Paragraph"]);
        assert_eq!(completions(&names, "_", 10), ["_index"]);
        assert_eq!(completions(&names, "", 2), ["Graph Module", "Paragraph"]);
        assert!(completions(&names, "zzz", 10).is_empty());
    }

    #[test]
    fn keeps_the_original_line_endings() {
        assert_eq!(restore_line_endings("a\r\nb\r\n", "a\nc\n"), "a\r\nc\r\n");
        assert_eq!(restore_line_endings("a\nb\n", "a\r\nc\n"), "a\nc\n");
        // Saving without changes reproduces the file byte for byte.
        let original = "# T\r\n\r\n> s\r\n";
        assert_eq!(
            restore_line_endings(original, &original.replace("\r\n", "\n")),
            original
        );
    }
}
