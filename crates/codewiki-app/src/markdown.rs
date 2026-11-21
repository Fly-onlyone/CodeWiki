//! Text preparation for the note viewer. No UI types, so it is unit-tested.

/// URL scheme the viewer uses for links between notes.
pub const NOTE_SCHEME: &str = "codewiki://note/";
/// URL scheme for diagrams rendered from Mermaid blocks, followed by an index.
pub const DIAGRAM_SCHEME: &str = "codewiki-diagram://";
/// URL scheme of the link around a drawn diagram, followed by the same index.
/// Following it opens the diagram in the viewer, where it can be enlarged.
pub const OPEN_DIAGRAM_SCHEME: &str = "codewiki://diagram/";

pub(crate) fn encode(value: &str) -> String {
    cw_client::encode(value)
}

/// Decode `%XX` escapes produced by [`encode`].
pub fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = value.get(i + 1..i + 3)
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// Turn a vault note into Markdown the viewer can render: frontmatter becomes
/// a tag line and `[[wikilinks]]` become ordinary links with the note scheme.
/// Code spans and fenced blocks are left untouched.
#[cfg(test)]
pub fn prepare_note(text: &str) -> String {
    prepare_note_with(text, &[], |_| Diagram::Source)
}

/// What the viewer shows in place of one Mermaid block.
pub enum Diagram {
    /// The drawn diagram, by its index among the page's diagrams.
    Image(usize),
    /// A line saying the diagram is being drawn.
    Drawing,
    /// The block as code (for example when it does not render).
    Source,
}

/// Line shown where a diagram is still being drawn.
pub const DRAWING: &str = "*Drawing diagram…*";

/// Language of the code block that stands where one of a note's pictures
/// is drawn. The block holds the picture's number among the note's
/// pictures, and the viewer draws that picture in its place.
pub const FIGURE_BLOCK: &str = "codewiki-figure";

/// The number of the picture a [`FIGURE_BLOCK`] with this text stands for.
pub fn figure_index(block: &str) -> Option<usize> {
    block.trim().parse().ok()
}

/// [`prepare_note`] with diagram support. `diagram` receives the source of
/// each Mermaid block and says what to show in its place.
///
/// `figures` are the headings of the sections that have a picture. A
/// [`FIGURE_BLOCK`] is put directly under each such heading: a picture
/// stands inside the section it explains, and the reader meets it before
/// the text it makes easier.
pub fn prepare_note_with(
    text: &str,
    figures: &[&str],
    mut diagram: impl FnMut(&str) -> Diagram,
) -> String {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines: Vec<&str> = text.lines().collect();
    let mut tags = String::new();
    if lines.first().is_some_and(|l| l.trim_end() == "---")
        && let Some(end) = lines.iter().skip(1).position(|l| l.trim_end() == "---")
    {
        for line in &lines[1..=end] {
            if let Some(rest) = line.trim().strip_prefix("tags:") {
                tags = rest.trim().trim_matches(['[', ']']).to_string();
            }
        }
        lines.drain(..end + 2);
    }

    let mut out = String::with_capacity(text.len() + 64);
    let mut in_fence = false;
    let mut tags_placed = tags.is_empty();
    // The pictures that have their block already.
    let mut placed: Vec<usize> = Vec::new();
    // Source of the Mermaid block being collected, with its opening fence.
    let mut mermaid: Option<(String, String)> = None;
    for line in lines {
        if let Some((fence, source)) = &mut mermaid {
            if line.trim_start().starts_with("```") {
                match diagram(source) {
                    // A link, so that a click on the diagram opens the viewer:
                    // a wide diagram is narrowed to the column and is too
                    // small to read there.
                    Diagram::Image(index) => out.push_str(&format!(
                        "[![diagram]({DIAGRAM_SCHEME}{index})]({OPEN_DIAGRAM_SCHEME}{index})\n"
                    )),
                    Diagram::Drawing => out.push_str(&format!("{DRAWING}\n")),
                    Diagram::Source => out.push_str(&format!("{fence}\n{source}{line}\n")),
                }
                mermaid = None;
            } else {
                source.push_str(line);
                source.push('\n');
            }
            continue;
        }
        if !in_fence && line.trim_start().starts_with("```mermaid") {
            mermaid = Some((line.to_string(), String::new()));
            continue;
        }
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            out.push_str(line);
        } else if in_fence {
            out.push_str(line);
        } else {
            out.push_str(&link_line(line));
        }
        out.push('\n');
        // Tags sit directly under the title, as small inline code.
        if !tags_placed && line.starts_with("# ") {
            let rendered: Vec<String> =
                tags.split(',').map(|t| format!("`{}`", t.trim())).collect();
            out.push_str(&format!("\n{}\n", rendered.join(" ")));
            tags_placed = true;
        }
        // The picture of a section, under its heading. A `## ` line inside
        // a code block is code, and a heading met twice gets one picture.
        if !in_fence
            && let Some(heading) = line.strip_prefix("## ")
            && let Some(index) = figures.iter().position(|f| *f == heading.trim())
            && !placed.contains(&index)
        {
            out.push_str(&format!("\n```{FIGURE_BLOCK}\n{index}\n```\n"));
            placed.push(index);
        }
    }
    out
}

/// Replace wikilinks in one line of prose.
fn link_line(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    let mut in_code = false;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('`') {
            in_code = !in_code;
            out.push('`');
            rest = after;
            continue;
        }
        if !in_code
            && let Some(after) = rest.strip_prefix("[[")
            && let Some(end) = after.find("]]")
        {
            let inner = &after[..end];
            let (target, label) = match inner.split_once('|') {
                Some((target, alias)) => (target, alias),
                None => (inner, inner),
            };
            let target = target.split('#').next().unwrap_or("").trim();
            let label = label.trim().trim_end_matches('/');
            out.push_str(&format!("[{label}]({NOTE_SCHEME}{})", encode(target)));
            rest = &after[end + 2..];
            continue;
        }
        let ch = rest.chars().next().expect("rest is not empty");
        out.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}

/// `##` headings of a note, for the outline pane.
pub fn outline(text: &str) -> Vec<String> {
    let mut in_fence = false;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence && let Some(heading) = line.strip_prefix("## ") {
            out.push(heading.trim().to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_stands_under_the_heading_of_its_section() {
        let note = "---\ntags: [core, module]\n---\n\n# Autostart Module\n\n> Starting the app at sign-in.\n\n## What it does\n\nText.\n\n## Source\n\n- `a.rs`\n\n## How it works\n\nSteps.\n";
        let out = prepare_note_with(note, &["How it works", "What it does"], |_| Diagram::Source);
        assert_eq!(
            out,
            "\n# Autostart Module\n\n`core` `module`\n\n> Starting the app at sign-in.\n\n## What it does\n\n```codewiki-figure\n1\n```\n\nText.\n\n## Source\n\n- `a.rs`\n\n## How it works\n\n```codewiki-figure\n0\n```\n\nSteps.\n"
        );
        // The block names its picture by number.
        assert_eq!(figure_index("1\n"), Some(1));
        assert_eq!(figure_index(" 0 "), Some(0));
        assert_eq!(figure_index(""), None);
        assert_eq!(figure_index("one"), None);

        // No picture, no block; a picture of a section the note lacks has
        // no place.
        assert!(!prepare_note(note).contains(FIGURE_BLOCK));
        let none = prepare_note_with(note, &["Gotchas"], |_| Diagram::Source);
        assert_eq!(none, prepare_note(note));
        // A heading in a code block is code, and a heading met twice gets
        // the picture once.
        let tricky = "# T\n\n```md\n## Gotchas\n```\n\n## Gotchas\n\nA.\n\n## Gotchas\n\nB.\n";
        let out = prepare_note_with(tricky, &["Gotchas"], |_| Diagram::Source);
        assert_eq!(out.matches(FIGURE_BLOCK).count(), 1);
        assert!(out.contains(
            "```md\n## Gotchas\n```\n\n## Gotchas\n\n```codewiki-figure\n0\n```\n\nA.\n"
        ));
    }

    #[test]
    fn converts_wikilinks_outside_code() {
        let note = "---\ntags: [core, demo]\n---\n\n# Title\n\n> Summary with [[Other Note]].\n\nSee [[01-Core/demo/_index|demo/]] and `[[not a link]]`.\n\n```mermaid\nA[[stays]] --> B\n```\n";
        let out = prepare_note(note);
        assert!(!out.contains("tags: ["));
        assert!(out.contains("# Title\n\n`core` `demo`\n"));
        assert!(out.contains("[Other Note](codewiki://note/Other%20Note)"));
        assert!(out.contains("[demo](codewiki://note/01-Core%2Fdemo%2F_index)"));
        assert!(out.contains("`[[not a link]]`"));
        assert!(out.contains("A[[stays]] --> B"));
    }

    #[test]
    fn replaces_renderable_diagrams_and_keeps_the_rest() {
        let note = "# T\n\n```mermaid\nflowchart LR\n    A --> B\n```\n\ntext\n\n```mermaid\nbroken\n```\n\n```rust\nlet x = 1;\n```\n";
        let mut seen = Vec::new();
        let out = prepare_note_with(note, &[], |source| {
            seen.push(source.to_string());
            if source.starts_with("flowchart") {
                Diagram::Image(0)
            } else {
                Diagram::Source
            }
        });
        assert_eq!(seen, ["flowchart LR\n    A --> B\n", "broken\n"]);
        assert!(out.contains("[![diagram](codewiki-diagram://0)](codewiki://diagram/0)\n"));
        assert!(!out.contains("flowchart LR"));
        assert!(out.contains("```mermaid\nbroken\n```"));
        assert!(out.contains("```rust\nlet x = 1;\n```"));
    }

    #[test]
    fn a_diagram_being_drawn_leaves_a_line_in_its_place() {
        let note =
            "# T\n\nbefore\n\n```mermaid\nflowchart LR\n    A --> B\n```\n\nafter [[Other]]\n";
        let out = prepare_note_with(note, &[], |_| Diagram::Drawing);
        assert!(out.contains(&format!("before\n\n{DRAWING}\n\nafter ")));
        assert!(!out.contains("flowchart LR"));
        assert!(!out.contains("```"));
        assert!(out.contains("[Other](codewiki://note/Other)"));
    }

    #[test]
    fn link_targets_round_trip() {
        for name in ["Scan Module", "01-Core/x/_index", "Ünï ✓", "a%b"] {
            assert_eq!(decode(&encode(name)), name);
        }
    }

    #[test]
    fn extracts_outline_skipping_code() {
        let text = "# T\n\n## Source\n\n```\n## not a heading\n```\n\n## See also\n";
        assert_eq!(outline(text), ["Source", "See also"]);
    }
}
