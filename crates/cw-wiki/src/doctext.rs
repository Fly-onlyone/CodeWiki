//! From a doc comment to text that may stand in a note.
//!
//! A comment is written for the place it stands in: it may hold a heading
//! (`# Errors`), a doctest, a rustdoc link, a Javadoc tag. A note has rules
//! of its own, which the linter enforces by refusing the whole run: `## `
//! headings only from the note's template, no Markdown links, no tag in the
//! text, no code span that reads as a line number. Every function here
//! returns text that keeps those rules, whatever the comment holds.

use cw_core::Language;
use cw_vault::lint::is_line_ref;

/// Close `text` with a period unless it already ends a sentence.
pub(crate) fn ensure_period(text: &str) -> String {
    let t = text.trim();
    if t.is_empty() || t.ends_with(['.', '!', '?', ':']) {
        t.to_string()
    } else {
        format!("{t}.")
    }
}

/// Make doc-comment text safe to place in a note: rustdoc links become code,
/// Markdown links keep only their text, and wikilink brackets are defused.
pub fn clean_inline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // A code span stays as written: `Agent[/ws/agent]` and `#[derive]`
        // are code, and Obsidian reads no link or tag inside one.
        if chars[i] == '`'
            && let Some(close) = chars[i + 1..]
                .iter()
                .position(|c| *c == '`')
                .map(|p| p + i + 1)
        {
            out.extend(&chars[i..=close]);
            i = close + 1;
            continue;
        }
        if chars[i] == '[' {
            if let Some(close) = chars[i + 1..]
                .iter()
                .position(|c| *c == ']')
                .map(|p| p + i + 1)
            {
                let inner: String = chars[i + 1..close].iter().collect();
                let mut end = close + 1;
                // `[text](target)` and `[text][ref]` keep only the text.
                if let Some(open) = chars.get(end).filter(|c| matches!(c, '(' | '['))
                    && let Some(stop) = chars[end..]
                        .iter()
                        .position(|c| *c == if *open == '(' { ')' } else { ']' })
                {
                    end += stop + 1;
                }
                if !inner.contains('[') {
                    out.push_str(&inner);
                    i = end;
                    continue;
                }
            }
            out.push('(');
            i += 1;
            continue;
        }
        if chars[i] == ']' {
            out.push(')');
        } else if chars[i] == '#'
            && chars
                .get(i + 1)
                .is_some_and(|c| c.is_alphabetic() || *c == '[')
        {
            // Would read as an inline tag.
        } else {
            out.push(chars[i]);
        }
        i += 1;
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// First paragraph of a doc comment, as one cleaned line.
fn first_paragraph(doc: &str) -> String {
    let lines: Vec<&str> = doc
        .lines()
        .map(str::trim)
        .skip_while(|l| l.is_empty())
        .take_while(|l| !l.is_empty() && !l.starts_with("```") && !l.starts_with('#'))
        .collect();
    clean_inline(&lines.join(" "))
}

/// Index of the end of the first sentence, if the paragraph has more than one.
fn sentence_break(paragraph: &str) -> Option<usize> {
    let bytes = paragraph.as_bytes();
    let mut in_code = false;
    for i in 0..bytes.len() {
        match bytes[i] {
            b'`' => in_code = !in_code,
            b'.' | b'!' | b'?' if !in_code && bytes.get(i + 1) == Some(&b' ') => {
                // Skip abbreviations such as "e.g." and "i.e.".
                let before = &paragraph[..i];
                if before.ends_with("e.g") || before.ends_with("i.e") || before.ends_with("etc") {
                    continue;
                }
                return Some(i + 1);
            }
            _ => {}
        }
    }
    None
}

/// First sentence of a doc comment, ending with a period.
pub fn first_sentence(doc: &str) -> String {
    let paragraph = first_paragraph(doc);
    let cut = sentence_break(&paragraph).unwrap_or(paragraph.len());
    ensure_period(&paragraph[..cut])
}

/// One part of a doc comment.
#[derive(Debug, PartialEq, Eq)]
enum Block {
    /// Running text, one source line each.
    Text(Vec<String>),
    /// A heading inside the comment, such as `# Errors`.
    Label(String),
    /// List items, each with its marker (`-` or `1.`).
    List(Vec<(String, String)>),
    /// A fenced code block: its language and its lines.
    Code(String, Vec<String>),
    /// Lines of a table, kept one to a line.
    Table(Vec<String>),
}

/// Words rustdoc puts after a code fence that say how to test the block, not
/// what language it is in.
const RUSTDOC_FENCE_WORDS: [&str; 6] = [
    "no_run",
    "ignore",
    "should_panic",
    "compile_fail",
    "edition2021",
    "edition2024",
];

/// The marker and the text of a list item, when `line` starts one.
fn list_item(line: &str) -> Option<(String, &str)> {
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(marker) {
            return Some(("-".to_string(), rest));
        }
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let rest = line[digits..]
        .strip_prefix(". ")
        .or_else(|| line[digits..].strip_prefix(") "))?;
    (1..=3)
        .contains(&digits)
        .then(|| (format!("{}.", &line[..digits]), rest))
}

/// A Javadoc or JSDoc tag line as a list item, or `None` for a tag that says
/// nothing about what the code does (`@author`, `@since`).
fn doc_tag(line: &str) -> Option<String> {
    let (tag, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let rest = rest.trim();
    // JSDoc may give the type first: `@param {string} name the name`.
    let untyped = match rest.strip_prefix('{') {
        Some(after) => after.split_once('}').map_or(rest, |(_, r)| r.trim()),
        None => rest,
    };
    match tag {
        "@param" | "@arg" | "@argument" | "@throws" | "@exception" => {
            let (name, what) = untyped
                .split_once(char::is_whitespace)
                .unwrap_or((untyped, ""));
            let name = name.trim_matches(['[', ']']);
            if name.is_empty() {
                return None;
            }
            let lead = if tag.starts_with("@p") || tag.starts_with("@a") {
                format!("`{name}`")
            } else {
                format!("Throws `{name}`")
            };
            let what = what.trim().trim_start_matches("- ").trim();
            Some(if what.is_empty() {
                lead
            } else if lead.starts_with('T') {
                format!("{lead} {what}")
            } else {
                format!("{lead}: {what}")
            })
        }
        "@return" | "@returns" if !untyped.is_empty() => Some(format!("Returns {untyped}")),
        _ => None,
    }
}

/// `{@link X}` and `{@code X}` become code spans, and `<code>` tags become
/// backticks.
fn inline_tags(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(at) = rest.find("{@") {
        out.push_str(&rest[..at]);
        let Some(end) = rest[at..].find('}') else {
            break;
        };
        let inner = &rest[at + 2..at + end];
        let (tag, text) = inner.split_once(char::is_whitespace).unwrap_or((inner, ""));
        let text = text.trim();
        if matches!(tag, "link" | "linkplain" | "code" | "literal" | "value") && !text.is_empty() {
            // `{@link Type#method label}` names the code first.
            let code = text.split_whitespace().next().unwrap_or(text);
            out.push('`');
            out.push_str(&code.replace('#', "."));
            out.push('`');
        }
        rest = &rest[at + end + 1..];
    }
    out.push_str(rest);
    out.replace("<code>", "`")
        .replace("</code>", "`")
        .replace("<p>", "")
        .replace("</p>", "")
}

fn blocks(doc: &str, language: Language) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    // An open code fence: its indent, its language and its lines.
    let mut fence: Option<(usize, String, Vec<String>)> = None;
    // A Python doctest runs to the next blank line.
    let mut doctest: Option<Vec<String>> = None;
    // A tag that is left out takes its following lines with it.
    let mut skipping = false;
    // A blank line ends a paragraph; a line after it starts a new one.
    let mut open = false;
    for raw in doc.lines() {
        let line = raw.trim();
        if let Some((indent, lang, mut lines)) = fence.take() {
            if line.starts_with("```") {
                out.push(Block::Code(lang, lines));
            } else {
                let cut = raw.len() - raw.trim_start().len();
                lines.push(raw[cut.min(indent)..].trim_end().to_string());
                fence = Some((indent, lang, lines));
            }
            continue;
        }
        if let Some(mut lines) = doctest.take() {
            if line.is_empty() {
                out.push(Block::Code("python".into(), lines));
            } else {
                lines.push(line.to_string());
                doctest = Some(lines);
                continue;
            }
        }
        if line.is_empty() {
            open = false;
            skipping = false;
            continue;
        }
        if let Some(lang) = line.strip_prefix("```") {
            let indent = raw.len() - raw.trim_start().len();
            let word = lang.trim().split([',', ' ']).next().unwrap_or("");
            let rust = language == Language::Rust;
            let lang = match word {
                // The app would draw a diagram, which belongs in a flow note.
                "mermaid" => "text",
                "" if rust => "rust",
                "" if language == Language::Go => "go",
                "" if language == Language::Kotlin => "kotlin",
                w if rust && RUSTDOC_FENCE_WORDS.contains(&w) => "rust",
                w => w,
            };
            fence = Some((indent, lang.to_string(), Vec::new()));
            open = false;
            continue;
        }
        if line.starts_with(">>>") {
            doctest = Some(vec![line.to_string()]);
            open = false;
            continue;
        }
        // A heading: `# Errors`. Its text becomes a label, since a note takes
        // headings only from its template.
        let hashes = line.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) && line[hashes..].starts_with(' ') {
            out.push(Block::Label(line[hashes..].trim().to_string()));
            open = false;
            continue;
        }
        // The underline of a heading written on the line above.
        if line.len() >= 3
            && (line.chars().all(|c| c == '=') || line.chars().all(|c| c == '-'))
            && open
            && let Some(Block::Text(lines)) = out.last_mut()
            && lines.len() == 1
        {
            let label = lines.remove(0);
            out.pop();
            out.push(Block::Label(label));
            open = false;
            continue;
        }
        // A rule between two parts of the comment.
        if line.len() >= 3
            && ["-", "=", "*", "_"]
                .iter()
                .any(|c| line.replace(c, "").is_empty())
        {
            open = false;
            continue;
        }
        if line.starts_with('@') {
            open = false;
            match doc_tag(line) {
                Some(item) => {
                    skipping = false;
                    open = true;
                    match out.last_mut() {
                        Some(Block::List(items)) => items.push(("-".into(), item)),
                        _ => out.push(Block::List(vec![("-".into(), item)])),
                    }
                }
                None => skipping = true,
            }
            continue;
        }
        if skipping {
            continue;
        }
        if line.starts_with('|') {
            match out.last_mut() {
                Some(Block::Table(rows)) if open => rows.push(line.to_string()),
                _ => out.push(Block::Table(vec![line.to_string()])),
            }
            open = true;
            continue;
        }
        if let Some((marker, text)) = list_item(line) {
            // A checklist box is not used in a note.
            let text = ["[ ] ", "[x] ", "[X] "]
                .iter()
                .find_map(|b| text.strip_prefix(b))
                .unwrap_or(text);
            let item = (marker, text.trim().to_string());
            match out.last_mut() {
                Some(Block::List(items)) => items.push(item),
                _ => out.push(Block::List(vec![item])),
            }
            open = true;
            continue;
        }
        // A quote or a callout is plain text in a note.
        let line = line.trim_start_matches(['>', ' ']);
        let line = match line.strip_prefix("[!") {
            Some(rest) => rest.split_once(']').map_or(line, |(_, r)| r.trim()),
            None => line,
        };
        if line.is_empty() {
            continue;
        }
        match out.last_mut() {
            // More of the list item above.
            Some(Block::List(items)) if open => {
                let last = items.len() - 1;
                items[last].1.push(' ');
                items[last].1.push_str(line);
            }
            Some(Block::Text(lines)) if open => lines.push(line.to_string()),
            _ => out.push(Block::Text(vec![line.to_string()])),
        }
        open = true;
    }
    // A fence or a doctest the comment never closes still ends here, or it
    // would take the rest of the note with it.
    if let Some((_, lang, lines)) = fence {
        out.push(Block::Code(lang, lines));
    }
    if let Some(lines) = doctest {
        out.push(Block::Code("python".into(), lines));
    }
    out
}

/// A line of comment text, safe in a note: links and tags cleaned, and no
/// code span that the linter would read as a file with a line number.
fn clean_line(text: &str) -> String {
    let cleaned = clean_inline(&inline_tags(text));
    let parts: Vec<&str> = cleaned.split('`').collect();
    // An unpaired backtick would open a span that swallows the next one.
    if parts.len().is_multiple_of(2) {
        return cleaned.replace('`', "'");
    }
    let mut out = String::with_capacity(cleaned.len());
    for (i, part) in parts.iter().enumerate() {
        // Every second part stands between two backticks. One that reads as
        // a line reference, such as `127.0.0.1:8080`, loses them.
        if i % 2 == 1 && !is_line_ref(part) {
            out.push('`');
            out.push_str(part);
            out.push('`');
        } else {
            out.push_str(part);
        }
    }
    out
}

/// Whole text of a doc comment for a note: its paragraphs, lists and code
/// blocks, in order. With `after_first_sentence`, the first sentence is left
/// out, for a note whose summary already is that sentence.
pub fn doc_body(doc: &str, language: Language, after_first_sentence: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (i, block) in blocks(doc, language).into_iter().enumerate() {
        match block {
            Block::Text(lines) => {
                let mut text = clean_line(&lines.join(" "));
                if i == 0 && after_first_sentence {
                    let cut = sentence_break(&text).unwrap_or(text.len());
                    text = text[cut..].trim().to_string();
                }
                if !text.is_empty() {
                    parts.push(text);
                }
            }
            Block::Label(label) => {
                let label = clean_line(&label);
                if !label.is_empty() {
                    parts.push(format!("{}:", label.trim_end_matches([':', '.'])));
                }
            }
            Block::List(items) => parts.push(
                items
                    .iter()
                    .map(|(marker, text)| format!("{marker} {}", clean_line(text)))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            Block::Code(lang, lines) => {
                let rust = lang == "rust";
                let lines: Vec<&str> = lines
                    .iter()
                    .map(String::as_str)
                    // A line rustdoc hides from the reader of an example.
                    .filter(|l| !(rust && (l.trim() == "#" || l.trim_start().starts_with("# "))))
                    .collect();
                if lines.iter().any(|l| !l.trim().is_empty()) {
                    parts.push(format!("```{lang}\n{}\n```", lines.join("\n")));
                }
            }
            Block::Table(rows) => parts.push(
                rows.iter()
                    .map(|row| clean_line(row))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        }
    }
    parts.join("\n\n")
}

/// A doc comment as the text of one list item: its paragraphs run on as one,
/// its own lists follow as nested items, and its code blocks are left out.
/// Lines after the first are indented by two spaces, under the item.
pub fn item_text(doc: &str, language: Language) -> String {
    let all = blocks(doc, language);
    // The item's own line first, then the lines under it.
    let mut lines: Vec<String> = vec![String::new()];
    let mut after_list = false;
    for (i, block) in all.iter().enumerate() {
        let prose = match block {
            Block::Text(text) => Some(ensure_period(&clean_line(&text.join(" ")))),
            // A label over nothing says nothing: `Examples:` before a code
            // block that is left out.
            Block::Label(label) => matches!(all.get(i + 1), Some(Block::Text(_) | Block::List(_)))
                .then(|| {
                    let label = clean_line(label);
                    format!("{}:", label.trim_end_matches([':', '.']))
                }),
            Block::List(items) => {
                for (marker, text) in items {
                    lines.push(format!("  {marker} {}", clean_line(text)));
                }
                after_list = true;
                None
            }
            Block::Code(..) | Block::Table(_) => None,
        };
        let Some(prose) = prose.filter(|p| p.len() > 1) else {
            continue;
        };
        if after_list {
            lines.push(format!("  {prose}"));
            after_list = false;
        } else if let Some(last) = lines.last_mut() {
            if !last.is_empty() {
                last.push(' ');
            }
            last.push_str(&prose);
        }
    }
    lines.retain(|line| !line.trim().is_empty());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUST: Language = Language::Rust;

    #[test]
    fn splits_doc_comments_into_sentences() {
        let doc = "Parses a note. It keeps `a.b` intact, e.g. paths.\nSecond line here.\n\nNext paragraph.";
        assert_eq!(first_sentence(doc), "Parses a note.");
        assert_eq!(first_sentence("No period"), "No period.");
        assert_eq!(first_sentence(""), "");
    }

    #[test]
    fn cleans_doc_text() {
        assert_eq!(
            clean_inline("See [`Confidence`] and [docs](https://x.y)."),
            "See `Confidence` and docs."
        );
        assert_eq!(
            clean_inline("Uses [[wiki]] and #tag here"),
            "Uses (wiki) and tag here"
        );
        assert_eq!(
            clean_inline("Issue #12 and #[derive]"),
            "Issue #12 and derive"
        );
        // Code spans keep their brackets and `#`.
        assert_eq!(
            clean_inline("A label that is a path, `Agent[/ws/agent]`, or `#[derive]`."),
            "A label that is a path, `Agent[/ws/agent]`, or `#[derive]`."
        );
    }

    #[test]
    fn takes_the_whole_comment() {
        let doc = "Starting the app at sign-in.\n\nWindows runs what the values under the `Run` key\nsay. That value is the whole setting.\n\nOnly an installed copy offers this.";
        assert_eq!(
            doc_body(doc, RUST, true),
            "Windows runs what the values under the `Run` key say. That value is the whole setting.\n\nOnly an installed copy offers this."
        );
        // The first sentence too, where the summary is not taken from it.
        assert!(doc_body(doc, RUST, false).starts_with("Starting the app at sign-in.\n\nWindows"));
        // More sentences in the first paragraph stay.
        assert_eq!(
            doc_body("One. Two, e.g. this. Three.", RUST, true),
            "Two, e.g. this. Three."
        );
        assert_eq!(doc_body("Only one sentence.", RUST, true), "");
        assert_eq!(doc_body("", RUST, true), "");
    }

    #[test]
    fn a_heading_in_a_comment_is_no_heading_in_the_note() {
        let doc = "Reads the file.\n\n# Errors\n\nFails when the file is gone.\n\n## Panics\nNever.\n\nSafety\n------\nNone needed.";
        let body = doc_body(doc, RUST, false);
        assert_eq!(
            body,
            "Reads the file.\n\nErrors:\n\nFails when the file is gone.\n\nPanics:\n\nNever.\n\nSafety:\n\nNone needed."
        );
        assert!(!body.lines().any(|l| l.starts_with('#')));
    }

    #[test]
    fn keeps_lists_and_code_and_closes_a_fence() {
        let doc = "Steps:\n\n- first\n  goes on\n- [ ] second\n1. numbered\n\n```\n# use x::y;\nlet a = run();\n```\n\n```no_run\nrun();\n```\n\n```mermaid\nflowchart LR\n```\n\n```text\nnever closed";
        assert_eq!(
            doc_body(doc, RUST, false),
            "Steps:\n\n- first goes on\n- second\n1. numbered\n\n```rust\nlet a = run();\n```\n\n```rust\nrun();\n```\n\n```text\nflowchart LR\n```\n\n```text\nnever closed\n```"
        );
        // A `#` line is a comment in Python, not a hidden doctest line.
        assert_eq!(
            doc_body("```python\n# set up\nrun()\n```", Language::Python, false),
            "```python\n# set up\nrun()\n```"
        );
        assert_eq!(
            doc_body(
                "Adds.\n\n>>> add(1, 2)\n3\n\nDone.",
                Language::Python,
                false
            ),
            "Adds.\n\n```python\n>>> add(1, 2)\n3\n```\n\nDone."
        );
    }

    #[test]
    fn no_code_span_reads_as_a_line_number() {
        let text = doc_body(
            "Listens on `127.0.0.1:8080`, see `src/lib.rs:42` and `run`.",
            RUST,
            false,
        );
        assert_eq!(
            text,
            "Listens on 127.0.0.1:8080, see src/lib.rs:42 and `run`."
        );
        assert_eq!(
            doc_body("An odd ` tick and `code`.", RUST, false),
            "An odd ' tick and 'code'."
        );
        // Quotes, callouts, links and tags are plain text.
        assert_eq!(
            doc_body("> [!note] Mind #this and [that](https://x.y).", RUST, false),
            "Mind this and that."
        );
    }

    #[test]
    fn an_untagged_block_in_a_go_comment_is_go() {
        assert_eq!(
            doc_body(
                "Runs it.

```
run()
```",
                Language::Go,
                false
            ),
            "Runs it.

```go
run()
```"
        );
    }

    #[test]
    fn reads_javadoc_and_jsdoc_tags() {
        let doc = "Finds a user by {@link User#id}.\n<p>\nUses <code>cache</code>.\n\n@param id the id\n  of the user\n@param {string} [name] - the name\n@return the user\n@throws NotFound when there is none\n@since 1.2\n  and more\n@author someone";
        assert_eq!(
            doc_body(doc, Language::Java, false),
            "Finds a user by `User.id`. Uses `cache`.\n\n- `id`: the id of the user\n- `name`: the name\n- Returns the user\n- Throws `NotFound` when there is none"
        );
    }

    #[test]
    fn a_comment_as_one_list_item() {
        let doc = "Whether `value` starts the installation.\nWindows reads paths without regard to case.\n\nA value that starts another copy is not this one's.\n\n# Errors\n\n- when the key is gone\n- when it is locked\n\n# Examples\n\n```\nmatches(None, dir);\n```";
        assert_eq!(
            item_text(doc, RUST),
            "Whether `value` starts the installation. Windows reads paths without regard to case. A value that starts another copy is not this one's. Errors:\n  - when the key is gone\n  - when it is locked"
        );
        // A label over nothing but an example goes with the example.
        assert_eq!(
            item_text("Runs.\n\n# Examples\n\n```\nrun();\n```", RUST),
            "Runs."
        );
        assert_eq!(item_text("", RUST), "");
    }
}
