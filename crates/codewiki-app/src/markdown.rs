//! Text preparation for the note viewer. No UI types, so it is unit-tested.

use serde_json::Value;

/// URL scheme the viewer uses for links between notes.
pub const NOTE_SCHEME: &str = "codewiki://note/";
/// URL scheme for links to symbol detail pages.
pub const SYMBOL_SCHEME: &str = "codewiki://symbol/";
/// URL scheme for diagrams rendered from Mermaid blocks, followed by an index.
pub const DIAGRAM_SCHEME: &str = "codewiki-diagram://";

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
    prepare_note_with(text, |_| None)
}

/// [`prepare_note`] with diagram support. `diagram` receives the source of
/// each Mermaid block and returns the image URL to show in its place, or
/// `None` to keep the block as code (for example when it does not render).
pub fn prepare_note_with(text: &str, mut diagram: impl FnMut(&str) -> Option<String>) -> String {
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
    // Source of the Mermaid block being collected, with its opening fence.
    let mut mermaid: Option<(String, String)> = None;
    for line in lines {
        if let Some((fence, source)) = &mut mermaid {
            if line.trim_start().starts_with("```") {
                match diagram(source) {
                    Some(url) => out.push_str(&format!("![diagram]({url})\n")),
                    None => out.push_str(&format!("{fence}\n{source}{line}\n")),
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

pub(crate) fn symbol_link(qualified: &str) -> String {
    let name = qualified.rsplit("::").next().unwrap_or(qualified);
    format!("[`{name}`]({SYMBOL_SCHEME}{})", encode(qualified))
}

fn related_section(title: &str, items: &Value) -> String {
    let rows: Vec<String> = items
        .as_array()
        .into_iter()
        .flatten()
        .take(40)
        .map(|r| {
            format!(
                "- {} — {} `{}` in `{}`",
                symbol_link(r["qualified"].as_str().unwrap_or("")),
                r["relation"].as_str().unwrap_or(""),
                r["qualified"].as_str().unwrap_or(""),
                r["file"].as_str().unwrap_or("")
            )
        })
        .collect();
    if rows.is_empty() {
        String::new()
    } else {
        format!("\n## {title}\n\n{}\n", rows.join("\n"))
    }
}

/// Render the backend's symbol detail as a page. This is the live view of
/// code that the vault deliberately does not hold as notes.
pub fn symbol_page(detail: &Value) -> String {
    let symbol = &detail["symbol"];
    let str_of = |v: &Value| v.as_str().unwrap_or("").to_string();
    let mut out = format!(
        "# {}\n\n`{}` · {} · {}\n\n```rust\n{}\n```\n",
        str_of(&symbol["name"]),
        str_of(&symbol["qualified"]),
        str_of(&symbol["kind"]).replace('_', " "),
        str_of(&symbol["visibility"]),
        str_of(&symbol["signature"]),
    );
    let doc = str_of(&symbol["doc"]);
    if !doc.is_empty() {
        out.push_str(&format!("\n{doc}\n"));
    }
    out.push_str(&format!(
        "\n## Source\n\n- {} — lines {} to {}\n",
        file_link(&str_of(&symbol["file"])),
        symbol["start_line"],
        symbol["end_line"]
    ));
    let members: Vec<String> = detail["members"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|m| {
            format!(
                "- {} — `{}`",
                symbol_link(m["qualified"].as_str().unwrap_or("")),
                m["signature"].as_str().unwrap_or("")
            )
        })
        .collect();
    if !members.is_empty() {
        out.push_str(&format!("\n## Members\n\n{}\n", members.join("\n")));
    }
    out.push_str(&related_section("Depends on", &detail["depends_on"]));
    out.push_str(&related_section("Used by", &detail["used_by"]));
    let notes: Vec<String> = detail["notes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|path| {
            let stem = path
                .rsplit('/')
                .next()
                .unwrap_or(path)
                .trim_end_matches(".md");
            format!(
                "- [{stem}]({NOTE_SCHEME}{})",
                encode(path.trim_end_matches(".md"))
            )
        })
        .collect();
    if !notes.is_empty() {
        out.push_str(&format!("\n## Notes\n\n{}\n", notes.join("\n")));
    }
    out
}

/// Render search hits as a page of links.
pub fn search_page(query: &str, hits: &Value) -> String {
    let mut out = format!("# Search: {query}\n");
    let hits: Vec<&Value> = hits.as_array().into_iter().flatten().collect();
    if hits.is_empty() {
        out.push_str("\nNo results.\n");
        return out;
    }
    for (kind, heading) in [("note", "Notes"), ("symbol", "Symbols"), ("file", "Files")] {
        let rows: Vec<String> = hits
            .iter()
            .filter(|h| h["kind"] == kind)
            .map(|h| {
                let key = h["key"].as_str().unwrap_or("");
                let title = h["title"].as_str().unwrap_or(key);
                let snippet = h["snippet"].as_str().unwrap_or("").replace('\n', " ");
                match kind {
                    "note" => format!(
                        "- [{title}]({NOTE_SCHEME}{}) — {snippet}",
                        encode(key.trim_end_matches(".md"))
                    ),
                    "symbol" => format!("- {} — `{key}`", symbol_link(key)),
                    _ => format!("- {}", file_link(key)),
                }
            })
            .collect();
        if !rows.is_empty() {
            out.push_str(&format!("\n## {heading}\n\n{}\n", rows.join("\n")));
        }
    }
    out
}

/// URL scheme for links to live file pages.
pub const FILE_SCHEME: &str = "codewiki://file/";

pub(crate) fn file_link(path: &str) -> String {
    format!("[`{path}`]({FILE_SCHEME}{})", encode(path))
}

/// Render the hotspot ranking as a table.
pub fn hotspots_page(rows: &Value) -> String {
    let rows: Vec<&Value> = rows.as_array().into_iter().flatten().collect();
    let mut out = String::from(
        "# Hotspots\n\nFiles that are both large and frequently changed. Defects concentrate here, and refactoring here pays off most.\n",
    );
    if rows.is_empty() {
        out.push_str("\nNo commit history is indexed yet. Commit your work, then press Sync.\n");
        return out;
    }
    out.push_str("\n| Score | File | Commits | Lines | Main author | Bus factor |\n|---:|---|---:|---:|---|---:|\n");
    for row in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            row["score"],
            file_link(row["path"].as_str().unwrap_or("")),
            row["commits"],
            row["lines"],
            row["top_owner"].as_str().unwrap_or("—"),
            row["bus_factor"]
        ));
    }
    out.push_str("\nBus factor is the smallest number of authors who together made over half of a file's commits. A value of 1 means one person holds the knowledge.\n");
    out
}

/// Render a live page for one file: risk, symbols, relations and notes.
pub fn file_page(context: &Value, risk: &Value, radius: &Value) -> String {
    let path = context["path"].as_str().unwrap_or("");
    let mut out = format!(
        "# {}\n\n`{path}` · {} · {} lines\n",
        path.rsplit('/').next().unwrap_or(path),
        context["language"].as_str().unwrap_or(""),
        context["lines"]
    );
    out.push_str(&format!(
        "\n## Risk\n\n**{}** — {} of 100\n\n",
        risk["level"].as_str().unwrap_or("unknown"),
        risk["score"]
    ));
    for reason in risk["reasons"].as_array().into_iter().flatten() {
        out.push_str(&format!("- {}\n", reason.as_str().unwrap_or("")));
    }
    let owners: Vec<String> = risk["history"]["owners"]
        .as_array()
        .into_iter()
        .flatten()
        .take(5)
        .map(|o| {
            format!(
                "- {} — {} commits ({:.0}%)",
                o["name"].as_str().unwrap_or(""),
                o["commits"],
                o["share"].as_f64().unwrap_or(0.0) * 100.0
            )
        })
        .collect();
    if !owners.is_empty() {
        out.push_str(&format!("\n## Ownership\n\n{}\n", owners.join("\n")));
    }
    let partners: Vec<String> = risk["history"]["partners"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| {
            format!(
                "- {} — together {} times",
                file_link(p["path"].as_str().unwrap_or("")),
                p["together"]
            )
        })
        .collect();
    if !partners.is_empty() {
        out.push_str(&format!(
            "\n## Usually changes with\n\n{}\n",
            partners.join("\n")
        ));
    }
    let list = |value: &Value| -> Vec<String> {
        value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(|p| format!("- {}", file_link(p)))
            .collect()
    };
    let direct = list(&radius["direct"]);
    let transitive = list(&radius["transitive"]);
    if !direct.is_empty() {
        out.push_str(&format!(
            "\n## Blast radius\n\nDirect dependents:\n\n{}\n",
            direct.join("\n")
        ));
        if !transitive.is_empty() {
            out.push_str(&format!(
                "\nReached through further hops ({}):\n\n{}\n",
                transitive.len(),
                transitive
                    .iter()
                    .take(30)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
    }
    let symbols: Vec<String> = context["symbols"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| s["kind"] != "module")
        .take(60)
        .map(|s| {
            format!(
                "- {} — `{}`",
                symbol_link(s["qualified"].as_str().unwrap_or("")),
                s["signature"].as_str().unwrap_or("")
            )
        })
        .collect();
    if !symbols.is_empty() {
        out.push_str(&format!("\n## Symbols\n\n{}\n", symbols.join("\n")));
    }
    let imports = list(&context["imports"]);
    if !imports.is_empty() {
        out.push_str(&format!("\n## Imports from\n\n{}\n", imports.join("\n")));
    }
    let notes: Vec<String> = context["notes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|path| {
            let stem = path
                .rsplit('/')
                .next()
                .unwrap_or(path)
                .trim_end_matches(".md");
            format!(
                "- [{stem}]({NOTE_SCHEME}{})",
                encode(path.trim_end_matches(".md"))
            )
        })
        .collect();
    if !notes.is_empty() {
        out.push_str(&format!("\n## Notes\n\n{}\n", notes.join("\n")));
    }
    out
}

/// URL scheme for links that apply a theme.
pub const THEME_SCHEME: &str = "codewiki://theme/";

/// URL scheme for links that change a setting: `<setting>/<value>` follows.
pub const SETTING_SCHEME: &str = "codewiki://setting/";

/// The theme picker: every theme as a link, grouped by mode.
pub fn themes_page(themes: &[(String, bool)], active: &str) -> String {
    let mut out =
        String::from("# Themes\n\nSelect a theme to apply it. The choice is remembered.\n");
    for (dark, heading) in [(true, "Dark"), (false, "Light")] {
        let rows: Vec<String> = themes
            .iter()
            .filter(|(_, is_dark)| *is_dark == dark)
            .map(|(name, _)| {
                let mark = if name == active {
                    " — **active**"
                } else {
                    ""
                };
                format!("- [{name}]({THEME_SCHEME}{}){mark}", encode(name))
            })
            .collect();
        if !rows.is_empty() {
            out.push_str(&format!("\n## {heading}\n\n{}\n", rows.join("\n")));
        }
    }
    out.push_str("\nYour own themes load from the `themes` folder in `%APPDATA%\\\\CodeWiki`, in the same JSON format.\n");
    out
}

/// Render one proposal for review.
pub fn proposal_page(detail: &Value) -> String {
    let proposal = &detail["proposal"];
    format!(
        "# Proposed change\n\n`{}` — {}\n\nThis note was edited by hand, so the change waits for your decision. Nothing is written until you approve.\n\n```diff\n{}\n```\n",
        proposal["note_path"].as_str().unwrap_or(""),
        proposal["reason"].as_str().unwrap_or(""),
        detail["patch"].as_str().unwrap_or("").trim_end()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
        let out = prepare_note_with(note, |source| {
            seen.push(source.to_string());
            source
                .starts_with("flowchart")
                .then(|| "codewiki-mermaid://0".to_string())
        });
        assert_eq!(seen, ["flowchart LR\n    A --> B\n", "broken\n"]);
        assert!(out.contains("![diagram](codewiki-mermaid://0)"));
        assert!(!out.contains("flowchart LR"));
        assert!(out.contains("```mermaid\nbroken\n```"));
        assert!(out.contains("```rust\nlet x = 1;\n```"));
    }

    #[test]
    fn renders_hotspots_and_file_pages() {
        let rows = json!([{"score": 100.0, "path": "src/core.rs", "commits": 6, "lines": 40,
            "top_owner": "Ann", "bus_factor": 1}]);
        let page = hotspots_page(&rows);
        assert!(page.contains(
            "| 100.0 | [`src/core.rs`](codewiki://file/src%2Fcore.rs) | 6 | 40 | Ann | 1 |"
        ));
        assert!(hotspots_page(&json!([])).contains("No commit history"));

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
        assert!(page.starts_with("# core.rs\n\n`src/core.rs` · rust · 40 lines\n"));
        assert!(page.contains("**high** — 70 of 100"));
        assert!(page.contains("- Ann — 6 commits (100%)"));
        assert!(page.contains("- [`src/api.rs`](codewiki://file/src%2Fapi.rs) — together 6 times"));
        assert!(page.contains("Reached through further hops (1)"));
        assert!(page.contains("[`compute`](codewiki://symbol/demo%3A%3Acore%3A%3Acompute)"));
        assert!(
            !page.contains("mod core"),
            "the module's own symbol is not listed"
        );
        assert!(page.contains("[Core Module](codewiki://note/01-Core%2Fdemo%2FCore%20Module)"));
        assert!(
            !page.contains("## Imports from"),
            "empty sections are dropped"
        );
    }

    #[test]
    fn renders_the_theme_picker() {
        let themes = vec![("Nord".to_string(), true), ("One Light".to_string(), false)];
        let page = themes_page(&themes, "Nord");
        assert!(page.contains("## Dark\n\n- [Nord](codewiki://theme/Nord) — **active**"));
        assert!(page.contains("## Light\n\n- [One Light](codewiki://theme/One%20Light)\n"));
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
        assert!(page.contains("# double") && page.contains("pub fn double"));
        assert!(page.contains("## Used by") && !page.contains("## Depends on"));
        assert!(page.contains("[`run`](codewiki://symbol/demo%3A%3Arun)"));
        assert!(page.contains("[Math Module](codewiki://note/01-Core%2Fdemo%2FMath%20Module)"));

        let hits = json!([{"kind": "note", "key": "a/B.md", "title": "B", "snippet": "x\ny"}]);
        let search = search_page("q", &hits);
        assert!(search.contains("[B](codewiki://note/a%2FB) — x y"));
        assert!(search_page("q", &json!([])).contains("No results."));
    }
}
