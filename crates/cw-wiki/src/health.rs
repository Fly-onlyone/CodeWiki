//! Code health: dead code, documentation drift and per-file health scores.
//!
//! Every detector here is written from first principles against CodeWiki's own
//! index. Findings state how sure they are, because name resolution runs
//! without a compiler.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use cw_core::{EdgeKind, Result, Symbol, SymbolKind, VAULT_DIR, Visibility};
use cw_index::Index;
use cw_vault::lifecycle::{self, NoteState};
use cw_vault::{Note, VaultMeta};
use serde::{Deserialize, Serialize};

use crate::analyze::DEPENDENCY_KINDS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Certainty {
    /// Could be reached in ways the index cannot see.
    Possible,
    /// Private, unreferenced, and carries no attribute that could wire it up.
    Likely,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeadSymbol {
    pub qualified: String,
    pub kind: String,
    pub file: String,
    pub line: u32,
    pub lines: u32,
    pub certainty: Certainty,
    pub reason: String,
}

/// Attributes that only describe an item and never make it reachable.
pub(crate) const PASSIVE_ATTRS: [&str; 12] = [
    "derive",
    "allow",
    "warn",
    "deny",
    "expect",
    "doc",
    "inline",
    "must_use",
    "cfg",
    "cfg_attr",
    "non_exhaustive",
    "repr",
];

fn is_test_symbol(symbol: &Symbol) -> bool {
    symbol
        .attrs
        .iter()
        .any(|a| a == "test" || a.ends_with("::test"))
        || symbol
            .file
            .split('/')
            .any(|s| matches!(s, "tests" | "benches" | "examples"))
        || symbol.file.ends_with("build.rs")
}

/// Symbols nothing refers to.
///
/// Reported: functions, methods, types, constants and statics with no incoming
/// call, reference or implementation edge. Skipped: entry points, tests,
/// trait items and trait-impl methods (reached by dynamic dispatch), enum and
/// struct members, and anything public in a library crate (it is API).
pub fn dead_code(index: &Index) -> Result<Vec<DeadSymbol>> {
    let symbols = index.symbols()?;
    let by_name: HashMap<&str, &Symbol> =
        symbols.iter().map(|s| (s.qualified.as_str(), s)).collect();
    let library_crates: HashSet<String> = index
        .crates()?
        .into_iter()
        .filter(|c| c.has_lib)
        .map(|c| c.name)
        .collect();
    let mut referenced: HashSet<String> = HashSet::new();
    let mut implements_trait: HashSet<String> = HashSet::new();
    for edge in index.edges()? {
        if DEPENDENCY_KINDS.contains(&edge.kind) {
            referenced.insert(edge.dst.clone());
        }
        if edge.kind == EdgeKind::Implements {
            implements_trait.insert(edge.src);
        }
    }
    // A type is alive when any of its methods is used.
    let mut used_parents: HashSet<&str> = HashSet::new();
    for symbol in &symbols {
        if referenced.contains(&symbol.qualified)
            && let Some(parent) = &symbol.parent
        {
            used_parents.insert(parent.as_str());
        }
    }

    let mut out = Vec::new();
    for symbol in &symbols {
        if matches!(
            symbol.kind,
            SymbolKind::Module | SymbolKind::Impl | SymbolKind::Macro
        ) || referenced.contains(&symbol.qualified)
            || used_parents.contains(symbol.qualified.as_str())
            || is_test_symbol(symbol)
            || (symbol.kind == SymbolKind::Function && symbol.name == "main")
        {
            continue;
        }
        let parent = symbol.parent.as_deref().and_then(|p| by_name.get(p));
        // Trait items and methods of types that implement a trait are reached
        // through the trait, which the index cannot follow.
        if symbol.kind == SymbolKind::Method
            && parent.is_some_and(|p| {
                p.kind == SymbolKind::Trait || implements_trait.contains(&p.qualified)
            })
            && symbol.visibility == Visibility::Public
        {
            continue;
        }
        let krate = symbol.qualified.split("::").next().unwrap_or("");
        let exported = symbol.visibility == Visibility::Public && library_crates.contains(krate);
        if exported {
            continue;
        }
        let wiring: Vec<&String> = symbol
            .attrs
            .iter()
            .filter(|a| !PASSIVE_ATTRS.contains(&a.as_str()))
            .collect();
        let (certainty, reason) = if !wiring.is_empty() {
            (
                Certainty::Possible,
                format!("no reference found, but `#[{}]` may register it", wiring[0]),
            )
        } else if cw_core::Language::from_path(&symbol.file) != cw_core::Language::Rust {
            // Python, TypeScript, JavaScript and Java reach code through
            // reflection, decorators, frameworks and duck typing that a
            // static index cannot follow.
            (
                Certainty::Possible,
                "no static reference found; dynamic dispatch may still reach it".to_string(),
            )
        } else if symbol.visibility == Visibility::Private {
            (
                Certainty::Likely,
                "private and never referenced".to_string(),
            )
        } else {
            (
                Certainty::Possible,
                "never referenced inside the repository".to_string(),
            )
        };
        out.push(DeadSymbol {
            qualified: symbol.qualified.clone(),
            kind: symbol.kind.as_str().to_string(),
            file: symbol.file.clone(),
            line: symbol.start_line,
            lines: symbol.end_line.saturating_sub(symbol.start_line) + 1,
            certainty,
            reason,
        });
    }
    out.sort_by(|a, b| {
        b.certainty
            .cmp(&a.certainty)
            .then(b.lines.cmp(&a.lines))
            .then(a.qualified.cmp(&b.qualified))
    });
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftKind {
    /// A cited source file no longer exists.
    MissingSource,
    /// A cited source changed after the note was written.
    SourceChanged,
    /// The note names a path in backticks that is not in the repository.
    DeadPath,
    /// A wikilink points at no note.
    BrokenLink,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Drift {
    pub note: String,
    pub kind: DriftKind,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocReport {
    pub drift: Vec<Drift>,
    /// Share of source files with symbol support that some note cites.
    pub coverage: f64,
    pub documented_files: usize,
    pub source_files: usize,
    /// Largest undocumented source files, biggest first.
    pub undocumented: Vec<String>,
}

/// Does a backticked span look like a repo path (`src/a.rs`, `crates/x/`)?
fn looks_like_path(span: &str) -> bool {
    let span = span.trim();
    span.contains('/')
        && !span.contains(' ')
        && !span.contains("::")
        && !span.starts_with("http")
        && !span.starts_with('/')
        && !span.contains(['(', '<', '{', '*', '$', '=', '|'])
}

/// Where the documentation no longer matches the code.
pub fn doc_drift(root: &Path, index: &Index) -> Result<DocReport> {
    let vault = root.join(VAULT_DIR);
    let meta = VaultMeta::load(&vault)?;
    let registry = meta.registry()?;
    let files = index.files()?;
    let hashes: HashMap<&str, &str> = files
        .iter()
        .map(|f| (f.path.as_str(), f.hash.as_str()))
        .collect();
    let dirs: HashSet<&str> = files
        .iter()
        .flat_map(|f| {
            let mut out = Vec::new();
            let mut path = f.path.as_str();
            while let Some((dir, _)) = path.rsplit_once('/') {
                out.push(dir);
                path = dir;
            }
            out
        })
        .collect();

    let mut drift = Vec::new();
    let mut cited: HashSet<&str> = HashSet::new();
    for entry in meta.notes.values() {
        let text = std::fs::read_to_string(vault.join(&entry.path)).ok();
        for source in &entry.sources {
            cited.insert(source.path.as_str());
            match hashes.get(source.path.as_str()) {
                None => drift.push(Drift {
                    note: entry.path.clone(),
                    kind: DriftKind::MissingSource,
                    detail: format!("`{}` no longer exists", source.path),
                }),
                // Only human-owned notes can lag: generated ones are rewritten on sync.
                Some(hash) if *hash != source.hash => {
                    let state = lifecycle::state(entry, text.as_deref(), true);
                    if matches!(state, NoteState::HumanEditedStale) {
                        drift.push(Drift {
                            note: entry.path.clone(),
                            kind: DriftKind::SourceChanged,
                            detail: format!(
                                "`{}` changed since this note was last reviewed",
                                source.path
                            ),
                        });
                    }
                }
                Some(_) => {}
            }
        }
        let Some(text) = text else {
            continue;
        };
        let note = Note::parse(&text);
        for link in &note.links {
            if registry.resolve(&link.target, &entry.path).is_none() {
                drift.push(Drift {
                    note: entry.path.clone(),
                    kind: DriftKind::BrokenLink,
                    detail: format!("`[[{}]]` matches no note", link.target),
                });
            }
        }
        let mut seen: HashSet<&str> = HashSet::new();
        let mut in_fence = false;
        for line in text.lines() {
            if line.trim_start().starts_with("```") {
                in_fence = !in_fence;
                continue;
            }
            if in_fence {
                continue;
            }
            for span in line.split('`').skip(1).step_by(2) {
                let path = span.trim().trim_end_matches('/');
                if looks_like_path(span)
                    && !hashes.contains_key(path)
                    && !dirs.contains(path)
                    // Ignored paths (the vault, build output) are not indexed
                    // but are real.
                    && !root.join(path).exists()
                    && !vault.join(path).exists()
                    && seen.insert(path)
                {
                    drift.push(Drift {
                        note: entry.path.clone(),
                        kind: DriftKind::DeadPath,
                        detail: format!("`{path}` is not in the repository"),
                    });
                }
            }
        }
    }
    drift.sort_by(|a, b| a.note.cmp(&b.note).then(a.detail.cmp(&b.detail)));

    let mut sources: Vec<&cw_core::FileRecord> = files
        .iter()
        .filter(|f| f.language.has_symbol_support())
        .filter(|f| {
            !f.path
                .split('/')
                .any(|s| matches!(s, "tests" | "benches" | "examples"))
        })
        .collect();
    let documented = sources
        .iter()
        .filter(|f| cited.contains(f.path.as_str()))
        .count();
    sources.retain(|f| !cited.contains(f.path.as_str()));
    sources.sort_by(|a, b| b.lines.cmp(&a.lines).then(a.path.cmp(&b.path)));
    let total = documented + sources.len();
    Ok(DocReport {
        drift,
        coverage: if total == 0 {
            1.0
        } else {
            documented as f64 / total as f64
        },
        documented_files: documented,
        source_files: total,
        undocumented: sources.iter().take(20).map(|f| f.path.clone()).collect(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// Stable detector id, for example `long-function`.
    pub detector: String,
    pub message: String,
    /// Points this finding takes off the file's score.
    pub penalty: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileHealth {
    pub path: String,
    /// 1 (worst) to 10 (best).
    pub score: u32,
    pub findings: Vec<Finding>,
}

const LONG_FILE_LINES: u32 = 600;
const VERY_LONG_FILE_LINES: u32 = 1200;
const LONG_FUNCTION_LINES: u32 = 80;
const VERY_LONG_FUNCTION_LINES: u32 = 160;
pub(crate) const MANY_METHODS: usize = 25;
const HIGH_FAN_OUT: usize = 15;
const MANY_PARAMETERS: usize = 7;
const WIDE_API: usize = 30;

/// Number of parameters in a function signature, not counting a receiver.
/// Counts top-level commas of the first parenthesised group, so generic
/// arguments and nested tuples do not inflate it.
fn parameter_count(signature: &str) -> usize {
    let Some(open) = signature.find('(') else {
        return 0;
    };
    let mut depth = 0i32;
    let mut parts: Vec<String> = vec![String::new()];
    let mut previous = ' ';
    for c in signature[open..].chars() {
        let arrow = c == '>' && previous == '-';
        previous = c;
        match c {
            _ if arrow => {}
            '(' | '[' | '{' | '<' => {
                depth += 1;
                if depth == 1 {
                    continue;
                }
            }
            ')' | ']' | '}' | '>' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            ',' if depth == 1 => {
                parts.push(String::new());
                continue;
            }
            _ => {}
        }
        if let Some(part) = parts.last_mut() {
            part.push(c);
        }
    }
    parts
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .filter(|p| {
            let name = p.trim_start_matches('&').trim_start_matches("mut ").trim();
            !(name == "self" || name == "cls" || name.starts_with("self:"))
        })
        .count()
}

/// Health score for every source file with symbol support, worst first.
pub fn file_health(index: &Index) -> Result<Vec<FileHealth>> {
    let symbols = index.symbols()?;
    let mut by_file: BTreeMap<&str, Vec<&Symbol>> = BTreeMap::new();
    let mut methods_of: HashMap<&str, usize> = HashMap::new();
    for symbol in &symbols {
        by_file
            .entry(symbol.file.as_str())
            .or_default()
            .push(symbol);
        if symbol.kind == SymbolKind::Method
            && let Some(parent) = &symbol.parent
        {
            *methods_of.entry(parent.as_str()).or_default() += 1;
        }
    }
    let file_of: HashMap<&str, &str> = symbols
        .iter()
        .map(|s| (s.qualified.as_str(), s.file.as_str()))
        .collect();
    let mut fan_out: HashMap<&str, HashSet<&str>> = HashMap::new();
    let edges = index.edges()?;
    for edge in &edges {
        if !DEPENDENCY_KINDS.contains(&edge.kind) {
            continue;
        }
        if let (Some(src), Some(dst)) = (
            file_of.get(edge.src.as_str()),
            file_of.get(edge.dst.as_str()),
        ) && src != dst
        {
            fan_out.entry(src).or_default().insert(dst);
        }
    }
    let dead: HashMap<String, usize> = dead_code(index)?
        .into_iter()
        .filter(|d| d.certainty == Certainty::Likely)
        .fold(HashMap::new(), |mut acc, d| {
            *acc.entry(d.file).or_default() += 1;
            acc
        });

    let mut out = Vec::new();
    for file in index.files()? {
        if !file.language.has_symbol_support() {
            continue;
        }
        let items = by_file
            .get(file.path.as_str())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let mut findings = Vec::new();
        let mut add = |detector: &str, penalty: u32, message: String| {
            findings.push(Finding {
                detector: detector.to_string(),
                message,
                penalty,
            });
        };

        if file.lines >= VERY_LONG_FILE_LINES {
            add(
                "long-file",
                2,
                format!("{} lines; hard to hold in mind at once", file.lines),
            );
        } else if file.lines >= LONG_FILE_LINES {
            add("long-file", 1, format!("{} lines", file.lines));
        }

        let mut longest: Vec<(&str, u32)> = items
            .iter()
            .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
            .map(|s| (s.name.as_str(), s.end_line.saturating_sub(s.start_line) + 1))
            .filter(|(_, lines)| *lines >= LONG_FUNCTION_LINES)
            .collect();
        longest.sort_by_key(|entry| std::cmp::Reverse(entry.1));
        if let Some((name, lines)) = longest.first() {
            let penalty = if *lines >= VERY_LONG_FUNCTION_LINES {
                2
            } else {
                1
            };
            let more = if longest.len() > 1 {
                format!(", and {} more long functions", longest.len() - 1)
            } else {
                String::new()
            };
            add(
                "long-function",
                penalty,
                format!("`{name}` is {lines} lines{more}"),
            );
        }

        if let Some((name, count)) = items
            .iter()
            .filter(|s| s.kind.is_type())
            .map(|s| {
                (
                    s.name.as_str(),
                    methods_of.get(s.qualified.as_str()).copied().unwrap_or(0),
                )
            })
            .filter(|(_, count)| *count >= MANY_METHODS)
            .max_by_key(|(_, count)| *count)
        {
            add(
                "large-type",
                1,
                format!("`{name}` has {count} methods; it may hold several responsibilities"),
            );
        }

        let reach = fan_out
            .get(file.path.as_str())
            .map(HashSet::len)
            .unwrap_or(0);
        if reach >= HIGH_FAN_OUT {
            add("high-fan-out", 1, format!("depends on {reach} other files"));
        }

        // Two files that need each other cannot be understood, tested or
        // moved one at a time.
        let mut mutual: Vec<&str> = fan_out
            .get(file.path.as_str())
            .into_iter()
            .flatten()
            .copied()
            .filter(|other| {
                fan_out
                    .get(other)
                    .is_some_and(|back| back.contains(file.path.as_str()))
            })
            .collect();
        mutual.sort_unstable();
        if let Some(first) = mutual.first() {
            let more = if mutual.len() > 1 {
                format!(" and {} more", mutual.len() - 1)
            } else {
                String::new()
            };
            add(
                "mutual-dependency",
                1,
                format!("depends on `{first}`{more}, which depends back on it"),
            );
        }

        if let Some((name, count)) = items
            .iter()
            .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
            .map(|s| (s.name.as_str(), parameter_count(&s.signature)))
            .filter(|(_, count)| *count >= MANY_PARAMETERS)
            .max_by_key(|(_, count)| *count)
        {
            add(
                "many-parameters",
                1,
                format!("`{name}` takes {count} parameters; consider grouping them"),
            );
        }

        let public: Vec<&&Symbol> = items
            .iter()
            .filter(|s| s.visibility == Visibility::Public && s.kind != SymbolKind::Module)
            .filter(|s| s.kind != SymbolKind::Method || !s.doc.is_empty() || s.name != "new")
            .collect();
        let undocumented = public.iter().filter(|s| s.doc.is_empty()).count();
        if public.len() >= 4 && undocumented * 2 > public.len() {
            add(
                "undocumented-api",
                1,
                format!(
                    "{undocumented} of {} public items have no doc comment",
                    public.len()
                ),
            );
        }

        if public.len() >= WIDE_API {
            add(
                "wide-api",
                1,
                format!(
                    "{} public items; a narrower surface is easier to keep stable",
                    public.len()
                ),
            );
        }

        if let Some(count) = dead.get(&file.path) {
            add(
                "dead-code",
                1,
                format!("{count} private items are never referenced"),
            );
        }

        let penalty: u32 = findings.iter().map(|f| f.penalty).sum();
        out.push(FileHealth {
            path: file.path,
            score: 10u32.saturating_sub(penalty).max(1),
            findings,
        });
    }
    out.sort_by(|a, b| a.score.cmp(&b.score).then(a.path.cmp(&b.path)));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_parameters_without_receivers_or_nested_commas() {
        for (signature, expected) in [
            ("pub fn f()", 0),
            ("fn f(&self)", 0),
            ("fn f(&mut self, a: u8)", 1),
            ("fn f(self: Box<Self>, a: u8, b: u8)", 2),
            ("fn f(a: HashMap<String, u8>, b: (u8, u8), c: [u8; 2])", 3),
            ("fn f<T: Into<String>>(a: T, b: impl Fn(u8, u8) -> u8,)", 2),
            ("def f(self, a, b=(1, 2))", 2),
            ("struct S", 0),
        ] {
            assert_eq!(parameter_count(signature), expected, "{signature}");
        }
    }

    #[test]
    fn recognises_repo_paths_in_backticks() {
        for yes in ["src/lib.rs", "crates/cw-core/", "a/b"] {
            assert!(looks_like_path(yes), "{yes}");
        }
        for no in [
            "cw_core::hash",
            "fn f(a/b)",
            "https://x/y",
            "/v1/health",
            "a / b",
            "Vec<a/b>",
            "x=a/b",
        ] {
            assert!(!looks_like_path(no), "{no}");
        }
    }
}
