//! Rendering of planned notes into Markdown in the vault's house style.
//!
//! This is the keyless path: every sentence is derived from structural facts
//! and doc comments. An LLM writer may later replace the prose, but the
//! structure produced here is the contract the linter enforces.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use cw_core::{Language, Symbol, SymbolKind, Visibility};
use cw_vault::{NoteKind, Registry};

use crate::analyze::DEPENDENCY_KINDS;
use crate::plan::{
    Plan, Planned, Repo, Subject, ZONE_ARCHITECTURE, ZONE_CORE, ZONE_INTEGRATIONS, ZONE_OPERATIONS,
    ZONE_PATTERNS,
};
use crate::units::{Unit, is_test_file};

const MAX_LISTED: usize = 8;
const MAX_NAMES: usize = 4;
/// Modules listed in the note of a unit that has no root module.
const MAX_UNIT_MODULES: usize = 12;

/// Note-level dependency edges: note id -> other note id -> symbol names.
type NoteDeps = HashMap<String, BTreeMap<String, BTreeSet<String>>>;

pub struct RenderCtx<'a> {
    pub repo: &'a Repo<'a>,
    pub plan: &'a Plan,
    pub registry: &'a Registry,
    pub repo_name: &'a str,
    pub summaries: HashMap<String, String>,
    /// Ids of the notes this run will write or keep.
    planned: std::collections::HashSet<String>,
    depends_on: NoteDeps,
    used_by: NoteDeps,
}

impl<'a> RenderCtx<'a> {
    pub fn new(
        repo: &'a Repo<'a>,
        plan: &'a Plan,
        registry: &'a Registry,
        repo_name: &'a str,
    ) -> Self {
        let mut depends_on: NoteDeps = HashMap::new();
        let mut used_by: NoteDeps = HashMap::new();
        for e in repo
            .facts
            .edges
            .iter()
            .filter(|e| DEPENDENCY_KINDS.contains(&e.kind))
        {
            let (Some(from), Some(to)) = (plan.owner.get(&e.src), plan.owner.get(&e.dst)) else {
                continue;
            };
            if from == to {
                continue;
            }
            let name = |q: &str| q.rsplit("::").next().unwrap_or(q).to_string();
            depends_on
                .entry(from.clone())
                .or_default()
                .entry(to.clone())
                .or_default()
                .insert(name(&e.dst));
            used_by
                .entry(to.clone())
                .or_default()
                .entry(from.clone())
                .or_default()
                .insert(name(&e.src));
        }
        let mut ctx = Self {
            repo,
            plan,
            registry,
            repo_name,
            summaries: HashMap::new(),
            planned: plan.notes.iter().map(|n| n.id.clone()).collect(),
            depends_on,
            used_by,
        };
        ctx.summaries = plan
            .notes
            .iter()
            .map(|n| (n.id.clone(), ctx.summary(n)))
            .collect();
        ctx
    }

    /// Wikilink to the note with `id`, or `None` when it is not in the vault.
    pub fn link(&self, id: &str) -> Option<String> {
        // The registry also holds notes from earlier runs that this run is
        // about to delete. Only link to notes that will exist afterwards:
        // those planned now, and adopted notes.
        if !self.planned.contains(id) && !id.starts_with("adopted:") {
            return None;
        }
        let entry = self.registry.by_id(id)?;
        let stem = entry.path.strip_suffix(".md").unwrap_or(&entry.path);
        Some(match entry.kind {
            NoteKind::Home => "[[_HOME]]".to_string(),
            NoteKind::Index => {
                let dir = stem.strip_suffix("/_index").unwrap_or(stem);
                let label = dir.rsplit('/').next().unwrap_or(dir);
                format!("[[{stem}|{}]]", zone_label(label))
            }
            _ => format!("[[{}]]", stem.rsplit('/').next().unwrap_or(stem)),
        })
    }

    fn name(&self, id: &str) -> String {
        self.registry
            .by_id(id)
            .map(|e| {
                let stem = e.path.strip_suffix(".md").unwrap_or(&e.path);
                stem.rsplit('/').next().unwrap_or(stem).to_string()
            })
            .unwrap_or_else(|| id.to_string())
    }

    fn summary(&self, note: &Planned) -> String {
        let repo = self.repo;
        match &note.subject {
            Subject::Crate(name) => {
                let unit = repo.units.by_name(name);
                let doc = unit
                    .and_then(|u| u.root.as_deref())
                    .and_then(|r| repo.symbols.get(r))
                    .map(|s| first_sentence(&s.doc));
                unit.map(|u| u.description.trim().to_string())
                    .filter(|d| !d.is_empty())
                    .map(|d| ensure_period(&clean_inline(&d)))
                    .or(doc.filter(|d| !d.is_empty()))
                    .unwrap_or_else(|| match unit.filter(|u| !u.is_rust()) {
                        Some(unit) => format!(
                            "{} {} `{}` with {}.",
                            language_name(unit.language),
                            unit_noun(unit.language),
                            note.title,
                            count(repo.module_count(unit), "module")
                        ),
                        None => {
                            let modules = repo
                                .children_of(name)
                                .iter()
                                .filter(|s| s.kind == SymbolKind::Module)
                                .count();
                            format!(
                                "Rust crate `{}` with {modules} top-level modules.",
                                note.title
                            )
                        }
                    })
            }
            Subject::Module(q) | Subject::Type(q) => {
                let symbol = repo.symbols[q.as_str()];
                let doc = first_sentence(&symbol.doc);
                if !doc.is_empty() {
                    return doc;
                }
                let children = repo.children_of(q);
                match symbol.kind {
                    SymbolKind::Module => {
                        let names: Vec<String> = top_children(repo, q, 3)
                            .iter()
                            .map(|s| format!("`{}`", s.name))
                            .collect();
                        format!(
                            "Module `{q}` defining {} items, including {}.",
                            children.len(),
                            join_and(&names)
                        )
                    }
                    kind => {
                        let module = symbol.parent.as_deref().unwrap_or("");
                        if children.is_empty() {
                            format!("`{}` {} defined in `{module}`.", symbol.name, kind.as_str())
                        } else {
                            format!(
                                "`{}` {} defined in `{module}` with {} methods.",
                                symbol.name,
                                kind.as_str(),
                                children.len()
                            )
                        }
                    }
                }
            }
            Subject::CrateFlow if repo.units.all_rust() => format!(
                "How the {} workspace crates depend on each other, from entry points down to shared foundations.",
                repo.units.list.len()
            ),
            Subject::CrateFlow => format!(
                "How the {} packages of the repository depend on each other, from entry points down to shared foundations.",
                repo.units.list.len()
            ),
            Subject::StartupFlow {
                krate,
                entry,
                messages,
            } => {
                let package = repo
                    .units
                    .by_name(krate)
                    .map(|c| c.package.as_str())
                    .unwrap_or(krate);
                let parts: BTreeSet<&str> = messages
                    .iter()
                    .flat_map(|m| [m.from.as_str(), m.to.as_str()])
                    .collect();
                format!(
                    "Call sequence from the `{}` entry point of `{package}` across {} parts of the codebase.",
                    self.entry_label(entry),
                    parts.len()
                )
            }
            Subject::Integration(krate) => {
                let files: BTreeSet<&str> = repo
                    .facts
                    .external_uses
                    .iter()
                    .filter(|u| &u.krate == krate)
                    .map(|u| u.file.as_str())
                    .collect();
                format!(
                    "External {} `{krate}`, imported by {} source files.",
                    external_noun(&files),
                    files.len()
                )
            }
            Subject::TraitPattern(q) => {
                let symbol = repo.symbols[q.as_str()];
                let count = repo.implementors.get(q.as_str()).map(Vec::len).unwrap_or(0);
                let how = match symbol.kind {
                    SymbolKind::Class => "extending the class",
                    SymbolKind::Interface => "implementing the interface",
                    _ => "implementing the trait",
                };
                format!(
                    "Add a new `{}` by {how}; {count} types already follow this shape.",
                    symbol.name
                )
            }
            Subject::Operations => {
                "Commands and automation that build, check and test the repository.".into()
            }
            Subject::Index(dir) => {
                let count = self
                    .plan
                    .notes
                    .iter()
                    .filter(|n| &n.dir == dir && n.kind != NoteKind::Index)
                    .count();
                match dir.as_str() {
                    ZONE_ARCHITECTURE => {
                        "End-to-end flows that show how the parts of the system work together."
                            .into()
                    }
                    ZONE_CORE if repo.units.all_rust() => {
                        "Crates, modules and key types that make up the codebase.".into()
                    }
                    ZONE_CORE => {
                        "Packages, modules and key types that make up the codebase.".into()
                    }
                    ZONE_INTEGRATIONS if repo.units.all_rust() => {
                        "External crates and services the codebase depends on.".into()
                    }
                    ZONE_INTEGRATIONS => {
                        "External packages and services the codebase depends on.".into()
                    }
                    ZONE_PATTERNS => "Conventions to follow when adding code.".into(),
                    ZONE_OPERATIONS => "How to build, test and run the project.".into(),
                    _ => format!(
                        "Notes for `{}`: {count} concepts.",
                        dir.rsplit('/').next().unwrap_or(dir)
                    ),
                }
            }
            Subject::Home => {
                let files = repo.facts.files.len();
                let tail = "This vault maps its architecture, modules and conventions.";
                if repo.units.list.is_empty() {
                    return format!(
                        "**{}** is a repository of {files} files with no code in a language CodeWiki maps. {tail}",
                        self.repo_name
                    );
                }
                let (languages, parts) = self.composition();
                format!(
                    "**{}** is a {} codebase of {} and {files} source files. {tail}",
                    self.repo_name,
                    join_and(&languages),
                    parts.join(", ")
                )
            }
        }
    }

    /// The languages of the repository and how many units each has, the
    /// language with the most source files first.
    fn composition(&self) -> (Vec<String>, Vec<String>) {
        let mut counts: Vec<(Language, usize)> = Vec::new();
        for unit in &self.repo.units.list {
            match counts.iter_mut().find(|(l, _)| *l == unit.language) {
                Some((_, n)) => *n += 1,
                None => counts.push((unit.language, 1)),
            }
        }
        let files = |language: Language| {
            self.repo
                .facts
                .files
                .iter()
                .filter(|f| f.language == language)
                .count()
        };
        counts.sort_by(|a, b| {
            files(b.0)
                .cmp(&files(a.0))
                .then((b.0 == Language::Rust).cmp(&(a.0 == Language::Rust)))
                .then(a.0.as_str().cmp(b.0.as_str()))
        });
        let mixed = counts.len() > 1;
        let parts = counts
            .iter()
            .map(|(language, n)| {
                let what = count(*n, unit_noun(*language));
                if mixed && *language != Language::Rust {
                    let (number, noun) = what.split_once(' ').unwrap_or(("", &what));
                    format!("{number} {} {noun}", language_name(*language))
                } else {
                    what
                }
            })
            .collect();
        (
            counts
                .iter()
                .map(|(l, _)| language_name(*l).to_string())
                .collect(),
            parts,
        )
    }

    /// How a reader names the entry point `entry`: `main` in Rust, the file
    /// for a script, `Class.main` in Java.
    fn entry_label(&self, entry: &str) -> String {
        let Some(symbol) = self.repo.symbols.get(entry) else {
            return "main".into();
        };
        if Language::from_path(&symbol.file) == Language::Rust {
            return "main".into();
        }
        match symbol.kind {
            SymbolKind::Module => symbol
                .file
                .rsplit('/')
                .next()
                .unwrap_or(&symbol.file)
                .to_string(),
            _ => {
                let parent = symbol.parent.as_deref().unwrap_or("");
                format!(
                    "{}.{}",
                    parent.rsplit("::").next().unwrap_or(parent),
                    symbol.name
                )
            }
        }
    }

    /// Structural illustration for a note, as OpenUI Lang source. Only notes
    /// whose facts make a useful picture get one.
    pub fn illustration(&self, note: &Planned) -> Option<String> {
        let repo = self.repo;
        match &note.subject {
            Subject::Crate(name) => {
                let unit = repo.units.by_name(name)?;
                let index = repo.units.list.iter().position(|u| u.name == *name);
                let prefix = format!("{name}::");
                let inside = |s: &&Symbol| {
                    if unit.is_rust() {
                        s.qualified.starts_with(&prefix)
                    } else {
                        repo.units.index_of(s) == index
                            && unit.root.as_deref() != Some(s.qualified.as_str())
                    }
                };
                let symbols = &repo.facts.symbols;
                let modules = if unit.is_rust() {
                    symbols
                        .iter()
                        .filter(inside)
                        .filter(|s| s.kind == SymbolKind::Module)
                        .count()
                } else {
                    repo.module_count(unit)
                };
                let types = symbols
                    .iter()
                    .filter(inside)
                    .filter(|s| s.kind.is_type() && s.visibility == Visibility::Public)
                    .count();
                let functions = symbols
                    .iter()
                    .filter(inside)
                    .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
                    .count();
                let dependents = repo
                    .units
                    .list
                    .iter()
                    .filter(|c| c.deps.iter().any(|d| d == name))
                    .count();
                let figure = |label: &str, value: usize| cw_illustrate::Figure {
                    label: label.to_string(),
                    value: value.to_string(),
                    hint: None,
                };
                let mut bars: Vec<(String, f64)> = repo
                    .top_modules(unit)
                    .iter()
                    .map(|m| {
                        let below = format!("{}::", m.qualified);
                        let count = symbols
                            .iter()
                            .filter(|s| s.qualified.starts_with(&below))
                            .count();
                        (m.name.clone(), count as f64)
                    })
                    .filter(|(_, count)| *count > 0.0)
                    .collect();
                bars.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
                bars.truncate(MAX_LISTED);
                Some(cw_illustrate::overview(
                    &note.title,
                    &self.summaries[&note.id],
                    &[
                        figure("Modules", modules),
                        figure("Public types", types),
                        figure("Functions", functions),
                        figure(
                            if unit.is_rust() {
                                "Used by crates"
                            } else {
                                "Used by packages"
                            },
                            dependents,
                        ),
                    ],
                    &bars,
                    "Symbols per module",
                ))
            }
            Subject::StartupFlow { messages, .. } => {
                let items: Vec<(String, String)> = messages
                    .iter()
                    .map(|m| {
                        (
                            m.label.clone(),
                            format!("{} calls into {}", self.name(&m.from), self.name(&m.to)),
                        )
                    })
                    .collect();
                Some(cw_illustrate::steps(
                    &note.title,
                    &self.summaries[&note.id],
                    &items,
                ))
            }
            _ => None,
        }
    }

    /// Bullets for a note-level dependency map.
    fn dep_bullets(&self, map: &NoteDeps, id: &str, verb: &str) -> String {
        let Some(targets) = map.get(id) else {
            return String::new();
        };
        let mut rows: Vec<(&String, &BTreeSet<String>)> = targets.iter().collect();
        rows.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
        rows.iter()
            .filter_map(|(target, names)| {
                let link = self.link(target)?;
                let listed: Vec<String> = names
                    .iter()
                    .take(MAX_NAMES)
                    .map(|n| format!("`{n}`"))
                    .collect();
                Some(format!("- {link} — {verb} {}", listed.join(", ")))
            })
            .take(MAX_LISTED)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn render(&self, note: &Planned) -> String {
        let mut doc = Doc::new(note, &self.summaries[&note.id]);
        match &note.subject {
            Subject::Crate(name) => self.render_crate(&mut doc, note, name),
            Subject::Module(q) => self.render_module(&mut doc, note, q),
            Subject::Type(q) => self.render_type(&mut doc, note, q),
            Subject::CrateFlow => self.render_crate_flow(&mut doc, note),
            Subject::StartupFlow {
                entry, messages, ..
            } => self.render_startup_flow(&mut doc, note, entry, messages),
            Subject::Integration(krate) => self.render_integration(&mut doc, note, krate),
            Subject::TraitPattern(q) => self.render_pattern(&mut doc, note, q),
            Subject::Operations => self.render_operations(&mut doc, note),
            Subject::Index(dir) => return self.render_index(note, dir),
            Subject::Home => return self.render_home(note),
        }
        doc.finish()
    }

    fn see_also(&self, extra: &[Option<String>]) -> String {
        let mut lines = vec!["- [[_index]]".to_string()];
        for link in extra.iter().flatten() {
            let line = format!("- {link}");
            if !lines.contains(&line) {
                lines.push(line);
            }
        }
        lines.join("\n")
    }

    fn render_crate(&self, doc: &mut Doc, note: &Planned, name: &str) {
        let repo = self.repo;
        let krate = repo.units.by_name(name);
        let rust = krate.is_none_or(Unit::is_rust);
        let root = krate
            .and_then(|k| k.root.as_deref())
            .and_then(|r| repo.symbols.get(r));
        let mut sources = Vec::new();
        for (i, path) in note.sources.iter().enumerate() {
            let role = if rust {
                if i == 0 {
                    "package manifest"
                } else {
                    "crate root"
                }
            } else if krate.is_some_and(|k| k.manifests.iter().any(|m| &m.path == path)) {
                build_role(path)
            } else if root.is_some_and(|r| &r.file == path) {
                "package root"
            } else {
                "source module"
            };
            sources.push(format!("- `{path}` — {role}"));
        }
        doc.section("Source", sources.join("\n"));

        let mut body = Vec::new();
        if let Some(root) = root {
            let rest = doc_remainder(&root.doc);
            if !rest.is_empty() {
                body.push(rest);
            }
        }
        let mut modules: Vec<&Symbol> = krate.map(|k| repo.top_modules(k)).unwrap_or_default();
        match krate {
            Some(k) if rust => {
                let shape = match (k.has_lib, k.has_bin) {
                    (true, true) => "Builds a library and a binary.",
                    (false, true) => "Builds a binary.",
                    _ => "Builds a library.",
                };
                body.push(shape.to_string());
            }
            Some(k) => {
                let place = if k.dir.is_empty() {
                    "at the repository root".to_string()
                } else {
                    format!("under `{}`", k.dir)
                };
                let mut shape = format!("Holds {} {place}.", count(repo.module_count(k), "module"));
                let entries: Vec<String> = k
                    .entries
                    .iter()
                    .take(MAX_NAMES)
                    .map(|e| format!("`{}`", self.entry_label(&e.symbol)))
                    .collect();
                if !entries.is_empty() {
                    shape.push_str(&format!(" Runs from {}.", join_and(&entries)));
                }
                body.push(shape);
                // A unit can hold hundreds of files: the weightiest lead.
                modules.sort_by(|a, b| {
                    repo.subtree_rank(&b.qualified)
                        .total_cmp(&repo.subtree_rank(&a.qualified))
                        .then_with(|| a.qualified.cmp(&b.qualified))
                });
            }
            None => {}
        }
        let modules: Vec<String> = modules
            .iter()
            .filter_map(|m| {
                let link = self.link(&format!("mod:{}", m.qualified))?;
                let summary = first_sentence(&m.doc);
                Some(if summary.is_empty() {
                    format!("- {link}")
                } else {
                    format!("- {link} — {}", lower_first(&summary))
                })
            })
            .take(if rust { usize::MAX } else { MAX_UNIT_MODULES })
            .collect();
        if !modules.is_empty() {
            body.push(format!("Modules:\n\n{}", modules.join("\n")));
        }
        let items = root
            .map(|r| self.item_bullets(&r.qualified))
            .unwrap_or_default();
        if !items.is_empty() {
            let place = if rust { "crate" } else { "package" };
            body.push(format!("Defined at the {place} root:\n\n{items}"));
        }
        doc.section("How it works", body.join("\n\n"));
        doc.section(
            "Depends on",
            self.dep_bullets(&self.depends_on, &note.id, "uses"),
        );
        doc.section("Used by", self.dep_bullets(&self.used_by, &note.id, "from"));
        doc.section("See also", self.see_also(&[self.link("flow:crates")]));
    }

    /// Bullets for the notable items directly inside `parent`.
    fn item_bullets(&self, parent: &str) -> String {
        top_children(self.repo, parent, MAX_LISTED)
            .iter()
            .filter(|s| s.kind != SymbolKind::Module)
            .map(|s| {
                let label = self
                    .link(&format!("type:{}", s.qualified))
                    .unwrap_or_else(|| format!("`{}`", s.name));
                let summary = first_sentence(&s.doc);
                if summary.is_empty() {
                    format!("- {label} — {}", s.kind.as_str().replace('_', " "))
                } else {
                    format!("- {label} — {}", lower_first(&summary))
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_module(&self, doc: &mut Doc, note: &Planned, q: &str) {
        let symbol = self.repo.symbols[q];
        doc.section("Source", format!("- `{}` — module `{q}`", symbol.file));
        let mut body = Vec::new();
        let rest = doc_remainder(&symbol.doc);
        if !rest.is_empty() {
            body.push(rest);
        }
        let items = self.item_bullets(q);
        if !items.is_empty() {
            body.push(format!("Main items:\n\n{items}"));
        }
        let submodules: Vec<String> = self
            .repo
            .children_of(q)
            .iter()
            .filter(|s| s.kind == SymbolKind::Module)
            .filter_map(|m| self.link(&format!("mod:{}", m.qualified)))
            .collect();
        if !submodules.is_empty() {
            body.push(format!("Submodules: {}.", submodules.join(", ")));
        }
        doc.section("How it works", body.join("\n\n"));
        doc.section(
            "Depends on",
            self.dep_bullets(&self.depends_on, &note.id, "uses"),
        );
        doc.section("Used by", self.dep_bullets(&self.used_by, &note.id, "from"));
        let krate = self
            .repo
            .unit_of(q)
            .map_or(q.split("::").next().unwrap_or(""), |u| u.name.as_str());
        doc.section(
            "See also",
            self.see_also(&[self.link(&format!("crate:{krate}"))]),
        );
    }

    fn render_type(&self, doc: &mut Doc, note: &Planned, q: &str) {
        let repo = self.repo;
        let symbol = repo.symbols[q];
        doc.section(
            "Source",
            format!("- `{}` — definition of `{}`", symbol.file, symbol.name),
        );
        let mut body = vec![format!("Declared as `{}`.", symbol.signature)];
        let rest = doc_remainder(&symbol.doc);
        if !rest.is_empty() {
            body.push(rest);
        }
        let mut methods: Vec<&&Symbol> = repo
            .children_of(q)
            .iter()
            .filter(|s| s.visibility != Visibility::Private)
            .collect();
        // Documented methods say the most, so they are listed first.
        methods.sort_by(|a, b| {
            a.doc
                .is_empty()
                .cmp(&b.doc.is_empty())
                .then(
                    repo.rank_of(&b.qualified)
                        .total_cmp(&repo.rank_of(&a.qualified)),
                )
                .then(a.start_line.cmp(&b.start_line))
        });
        let bullets: Vec<String> = methods
            .iter()
            .take(6)
            .map(|m| {
                let summary = first_sentence(&m.doc);
                if summary.is_empty() {
                    format!("- `{}`", m.signature)
                } else {
                    format!("- `{}` — {}", m.signature, lower_first(&summary))
                }
            })
            .collect();
        if !bullets.is_empty() {
            let label = if matches!(symbol.kind, SymbolKind::Trait | SymbolKind::Interface) {
                "Required and provided methods"
            } else {
                "Public API"
            };
            body.push(format!("{label}:\n\n{}", bullets.join("\n")));
        }
        if let Some(impls) = repo.implementors.get(q) {
            let names: Vec<String> = impls
                .iter()
                .take(MAX_LISTED)
                .map(|t| {
                    self.link(&format!("type:{t}"))
                        .unwrap_or_else(|| format!("`{}`", t.rsplit("::").next().unwrap_or(t)))
                })
                .collect();
            let verb = if symbol.kind == SymbolKind::Class {
                "Extended"
            } else {
                "Implemented"
            };
            body.push(format!("{verb} by {}.", join_and(&names)));
        }
        doc.section("How it works", body.join("\n\n"));
        doc.section(
            "Depends on",
            self.dep_bullets(&self.depends_on, &note.id, "uses"),
        );
        doc.section("Used by", self.dep_bullets(&self.used_by, &note.id, "from"));
        let unit = repo.unit_of(q);
        let module_note = symbol.parent.as_deref().map(|p| match unit {
            Some(u) if u.root.as_deref() == Some(p) => format!("crate:{}", u.name),
            Some(u) if !u.is_rust() => format!("mod:{p}"),
            _ if p.contains("::") => format!("mod:{p}"),
            _ => format!("crate:{p}"),
        });
        // A small module has no note of its own; its unit stands in.
        let fallback = unit
            .filter(|u| !u.is_rust())
            .and_then(|u| self.link(&format!("crate:{}", u.name)));
        doc.section(
            "See also",
            self.see_also(&[
                module_note.and_then(|id| self.link(&id)).or(fallback),
                self.link(&format!("pattern:{q}")),
            ]),
        );
    }

    fn render_crate_flow(&self, doc: &mut Doc, note: &Planned) {
        let crates = &self.repo.units.list;
        let rust = self.repo.units.all_rust();
        doc.section(
            "Source",
            note.sources
                .iter()
                .map(|s| format!("- `{s}` — declares dependencies"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let index: HashMap<&str, usize> = crates
            .iter()
            .enumerate()
            .map(|(i, c)| (c.name.as_str(), i))
            .collect();
        let mut diagram = String::from("```mermaid\nflowchart LR\n");
        for (i, c) in crates.iter().enumerate() {
            let plain = c
                .package
                .chars()
                .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_'));
            if plain {
                diagram.push_str(&format!("    n{i}[{}]\n", c.package));
            } else {
                diagram.push_str(&format!(
                    "    n{i}[\"{}\"]\n",
                    c.package.replace(['"', '[', ']'], " ")
                ));
            }
        }
        let mut dependents: HashMap<&str, usize> = HashMap::new();
        let mut internal_deps: HashMap<&str, usize> = HashMap::new();
        for (i, c) in crates.iter().enumerate() {
            for dep in &c.deps {
                if let Some(&j) = index.get(dep.as_str()) {
                    diagram.push_str(&format!("    n{i} --> n{j}\n"));
                    *dependents.entry(dep.as_str()).or_default() += 1;
                    *internal_deps.entry(c.name.as_str()).or_default() += 1;
                }
            }
        }
        diagram.push_str("```");
        let names = |pick: &dyn Fn(&str) -> bool| -> Vec<String> {
            crates
                .iter()
                .filter(|c| pick(&c.name))
                .map(|c| format!("`{}`", c.package))
                .collect()
        };
        let foundations = names(&|n| !internal_deps.contains_key(n) && dependents.contains_key(n));
        let tops = names(&|n| !dependents.contains_key(n) && internal_deps.contains_key(n));
        let mut prose = vec![if rust {
            "An arrow points from a crate to a crate it depends on.".to_string()
        } else {
            "An arrow points from a package to a package it depends on. Rust crates declare their dependencies; for other languages they are read from imports.".to_string()
        }];
        if !foundations.is_empty() {
            prose.push(if rust {
                format!(
                    "Foundations with no workspace dependencies: {}.",
                    join_and(&foundations)
                )
            } else {
                format!(
                    "Foundations that depend on no other package here: {}.",
                    join_and(&foundations)
                )
            });
        }
        if !tops.is_empty() {
            prose.push(format!(
                "Nothing in the {} depends on {}.",
                if rust { "workspace" } else { "repository" },
                join_and(&tops)
            ));
        }
        doc.section("How it works", format!("{diagram}\n\n{}", prose.join(" ")));
        let links: Vec<String> = crates
            .iter()
            .filter_map(|c| {
                let link = self.link(&format!("crate:{}", c.name))?;
                let count = dependents.get(c.name.as_str()).copied().unwrap_or(0);
                Some(if rust {
                    format!("- {link} — used by {count} workspace crates")
                } else {
                    format!("- {link} — used by {count} other packages")
                })
            })
            .collect();
        doc.section("Depends on", links.join("\n"));
        doc.section("See also", self.see_also(&[]));
    }

    fn render_startup_flow(
        &self,
        doc: &mut Doc,
        note: &Planned,
        entry: &str,
        messages: &[crate::plan::FlowMessage],
    ) {
        doc.section(
            "Source",
            note.sources
                .iter()
                .map(|s| format!("- `{s}` — entry point"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let mut participants: Vec<&str> = Vec::new();
        for m in messages {
            for id in [m.from.as_str(), m.to.as_str()] {
                if !participants.contains(&id) {
                    participants.push(id);
                }
            }
        }
        let alias = |id: &str| {
            format!(
                "p{}",
                participants.iter().position(|p| *p == id).unwrap_or(0)
            )
        };
        let mut diagram = String::from("```mermaid\nsequenceDiagram\n    autonumber\n");
        for id in &participants {
            diagram.push_str(&format!(
                "    participant {} as {}\n",
                alias(id),
                self.name(id)
            ));
        }
        for m in messages {
            diagram.push_str(&format!(
                "    {}->>{}: {}\n",
                alias(&m.from),
                alias(&m.to),
                m.label
            ));
        }
        diagram.push_str("```");
        let label = self.entry_label(entry);
        let limit = if self
            .repo
            .symbols
            .get(entry)
            .is_none_or(|s| Language::from_path(&s.file) == Language::Rust)
        {
            "Calls reached through trait objects or macros do not appear."
        } else {
            "Calls on values whose type is not known before the program runs may be missing."
        };
        doc.section(
            "How it works",
            format!(
                "{diagram}\n\nThe sequence follows resolved calls outward from `{label}`, keeping only calls that cross from one documented part into another. {limit}"
            ),
        );
        let links: Vec<String> = participants
            .iter()
            .filter_map(|id| self.link(id))
            .map(|l| format!("- {l}"))
            .collect();
        doc.section("Depends on", links.join("\n"));
        doc.section("See also", self.see_also(&[self.link("flow:crates")]));
    }

    fn render_integration(&self, doc: &mut Doc, _note: &Planned, krate: &str) {
        let repo = self.repo;
        let files: BTreeSet<&str> = repo
            .facts
            .external_uses
            .iter()
            .filter(|u| u.krate == krate)
            .map(|u| u.file.as_str())
            .collect();
        let mut users: BTreeMap<String, usize> = BTreeMap::new();
        for file in &files {
            let owner = repo
                .facts
                .symbols
                .iter()
                .find(|s| s.kind == SymbolKind::Module && s.file == *file)
                .and_then(|s| self.plan.owner.get(&s.qualified));
            if let Some(link) = owner.and_then(|id| self.link(id)) {
                *users.entry(link).or_default() += 1;
            }
        }
        let mut rows: Vec<(&String, &usize)> = users.iter().collect();
        rows.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        let used_for = rows
            .iter()
            .take(MAX_LISTED)
            .map(|(link, _)| format!("- {link}"))
            .collect::<Vec<_>>()
            .join("\n");
        doc.section(
            "Used for",
            if used_for.is_empty() {
                format!("Imported by {} source files.", files.len())
            } else {
                format!(
                    "Imported by {} source files. Main users:\n\n{used_for}",
                    files.len()
                )
            },
        );
        let mut declaring: Vec<String> = Vec::new();
        for unit in &repo.units.list {
            let manifests: Vec<&str> = if unit.is_rust() {
                unit.deps
                    .iter()
                    .any(|d| d == krate)
                    .then(|| unit.manifests.first().map(|m| m.path.as_str()))
                    .flatten()
                    .into_iter()
                    .collect()
            } else {
                unit.declares(krate)
            };
            for manifest in manifests {
                let line = format!("- `{manifest}` — declares the dependency");
                if !declaring.contains(&line) {
                    declaring.push(line);
                }
            }
        }
        doc.section("Configuration", declaring.join("\n"));
        doc.section(
            "Wire-up",
            files
                .iter()
                .take(MAX_LISTED)
                .map(|f| format!("- `{f}` — imports `{krate}`"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        doc.section("See also", self.see_also(&[]));
    }

    fn render_pattern(&self, doc: &mut Doc, _note: &Planned, q: &str) {
        let repo = self.repo;
        let symbol = repo.symbols[q];
        let impls: Vec<String> = repo
            .implementors
            .get(q)
            .map(|v| {
                v.iter()
                    .take(MAX_LISTED)
                    .map(|t| {
                        self.link(&format!("type:{t}"))
                            .unwrap_or_else(|| format!("`{}`", t.rsplit("::").next().unwrap_or(t)))
                    })
                    .collect()
            })
            .unwrap_or_default();
        doc.section(
            "When to apply",
            format!(
                "When adding a type that must act as a `{}`. Existing implementations: {}.",
                symbol.name,
                join_and(&impls)
            ),
        );
        let methods: Vec<&&Symbol> = repo
            .children_of(q)
            .iter()
            .filter(|m| m.kind == SymbolKind::Method)
            .collect();
        let language = Language::from_path(&symbol.file);
        let keyword = if symbol.kind == SymbolKind::Class {
            "extends"
        } else {
            "implements"
        };
        let (fence, code) = match language {
            Language::Python => {
                let mut code = format!("class NewType({}):\n", symbol.name);
                let public: Vec<&&&Symbol> = methods
                    .iter()
                    .filter(|m| m.visibility != Visibility::Private && m.name != "__init__")
                    .take(MAX_LISTED)
                    .collect();
                for m in &public {
                    code.push_str(&format!("    {}:\n        ...\n", m.signature));
                }
                if public.is_empty() {
                    code.push_str("    ...\n");
                }
                ("python", code.trim_end().to_string())
            }
            Language::Java => {
                let mut code = format!("class NewType {keyword} {} {{\n", symbol.name);
                for m in methods
                    .iter()
                    .filter(|m| m.visibility != Visibility::Private && m.name != symbol.name)
                    .take(MAX_LISTED)
                {
                    code.push_str(&format!(
                        "    {} {{\n        throw new UnsupportedOperationException();\n    }}\n",
                        m.signature
                    ));
                }
                code.push('}');
                ("java", code)
            }
            Language::TypeScript | Language::JavaScript => {
                let mut code = format!("class NewType {keyword} {} {{\n", symbol.name);
                for m in methods
                    .iter()
                    .filter(|m| m.visibility != Visibility::Private && m.name != "constructor")
                    .take(MAX_LISTED)
                {
                    code.push_str(&format!(
                        "  {} {{\n    throw new Error(\"not implemented\");\n  }}\n",
                        m.signature
                    ));
                }
                code.push('}');
                (
                    if language == Language::TypeScript {
                        "ts"
                    } else {
                        "js"
                    },
                    code,
                )
            }
            _ => {
                let mut code = format!("impl {} for NewType {{\n", symbol.name);
                for m in &methods {
                    code.push_str(&format!(
                        "    {} {{\n        todo!()\n    }}\n",
                        m.signature
                    ));
                }
                code.push('}');
                ("rust", code)
            }
        };
        doc.section("The pattern", format!("```{fence}\n{code}\n```"));
        let why = doc_remainder(&symbol.doc);
        let lead = first_sentence(&symbol.doc);
        doc.section(
            "Why",
            [lead, why]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
        );
        doc.section(
            "See also",
            self.see_also(&[
                self.link(&format!("type:{q}")),
                self.plan.owner.get(q).and_then(|id| self.link(id)),
            ]),
        );
    }

    fn render_operations(&self, doc: &mut Doc, note: &Planned) {
        let units = &self.repo.units.list;
        let role = |path: &str| -> &str {
            if path.starts_with(".github/workflows/") {
                "CI workflow"
            } else if path.contains("Dockerfile") {
                "container image"
            } else if path == "deny.toml" {
                "license and advisory policy"
            } else if path == "rust-toolchain.toml" {
                "pinned toolchain"
            } else if path.ends_with("Cargo.toml") {
                "workspace manifest"
            } else {
                build_role(path)
            }
        };
        doc.section(
            "Source",
            note.sources
                .iter()
                .map(|s| format!("- `{s}` — {}", role(s)))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let crates = units.iter().filter(|u| u.is_rust()).count();
        let mut commands: Vec<String> = Vec::new();
        if crates > 0 {
            commands.extend(
                [
                    "cargo build --workspace",
                    "cargo test --workspace",
                    "cargo fmt --all",
                    "cargo clippy --workspace --all-targets -- -D warnings",
                ]
                .map(str::to_string),
            );
            if note.sources.iter().any(|s| s == "deny.toml") {
                commands.push("cargo deny check".into());
            }
        }
        for unit in units.iter().filter(|u| !u.is_rust()) {
            for command in self.unit_commands(unit) {
                if !commands.contains(&command) {
                    commands.push(command);
                }
            }
        }
        let mut body = if self.repo.units.all_rust() {
            format!(
                "The repository is a Cargo workspace of {crates} packages. Run every command from the repository root."
            )
        } else {
            format!(
                "The repository holds {}. Run every command from the repository root.",
                join_and(&self.composition().1)
            )
        };
        if commands.is_empty() {
            body.push_str(" No build file was found, so the code runs from source as it is.");
        } else {
            body.push_str(&format!("\n\n```sh\n{}\n```", commands.join("\n")));
        }
        let ci: Vec<String> = note
            .sources
            .iter()
            .filter(|s| s.starts_with(".github/workflows/"))
            .map(|s| format!("`{s}`"))
            .collect();
        if !ci.is_empty() {
            body.push_str(&format!(
                "\n\nContinuous integration runs the same checks from {}.",
                join_and(&ci)
            ));
        }
        doc.section("How it works", body);
        let crates: Vec<String> = units
            .iter()
            .filter(|c| c.has_bin)
            .filter_map(|c| {
                let link = self.link(&format!("crate:{}", c.name))?;
                Some(match c.entries.first().filter(|_| !c.is_rust()) {
                    Some(entry) => {
                        format!("- {link} — runs from `{}`", self.entry_label(&entry.symbol))
                    }
                    None => format!("- {link} — builds a binary"),
                })
            })
            .collect();
        doc.section("Depends on", crates.join("\n"));
        doc.section("See also", self.see_also(&[self.link("flow:crates")]));
    }

    /// Commands that install, run and test a unit written in another
    /// language than Rust, read off its build files and entry points.
    fn unit_commands(&self, unit: &Unit) -> Vec<String> {
        let repo = self.repo;
        let quote = |path: &str| {
            if path.contains(' ') {
                format!("\"{path}\"")
            } else {
                path.to_string()
            }
        };
        let file_name = |path: &str| path.rsplit('/').next().unwrap_or(path).to_string();
        let dir_of = |path: &str| path.rsplit_once('/').map_or("", |(d, _)| d).to_string();
        let mut out: Vec<String> = Vec::new();
        match unit.language {
            Language::Python => {
                let mut project = None;
                for m in &unit.manifests {
                    let name = file_name(&m.path);
                    if name.starts_with("requirements") {
                        out.push(format!("python -m pip install -r {}", quote(&m.path)));
                    } else if matches!(name.as_str(), "pyproject.toml" | "setup.py") {
                        project = Some(dir_of(&m.path));
                    }
                }
                if let Some(dir) = project {
                    out.push(format!(
                        "python -m pip install -e {}",
                        if dir.is_empty() {
                            ".".to_string()
                        } else {
                            quote(&dir)
                        }
                    ));
                }
                for entry in unit.entries.iter().take(1) {
                    let Some(symbol) = repo.symbols.get(entry.symbol.as_str()) else {
                        continue;
                    };
                    match symbol.qualified.strip_suffix("::__main__") {
                        Some(package) => {
                            out.push(format!("python -m {}", package.replace("::", ".")))
                        }
                        None => out.push(format!("python {}", quote(&symbol.file))),
                    }
                }
                let tested = repo
                    .facts
                    .files
                    .iter()
                    .any(|f| f.language == Language::Python && is_test_file(&f.path));
                if tested {
                    out.push("python -m pytest".into());
                }
            }
            Language::TypeScript | Language::JavaScript => {
                if unit.manifests.is_empty() {
                    return out;
                }
                let npm = if unit.dir.is_empty() {
                    "npm".to_string()
                } else {
                    format!("npm --prefix {}", quote(&unit.dir))
                };
                out.push(format!("{npm} install"));
                for script in ["build", "test", "lint", "start"] {
                    if unit.scripts.iter().any(|s| s == script) {
                        out.push(format!("{npm} run {script}"));
                    }
                }
            }
            Language::Java => {
                for m in &unit.manifests {
                    let dir = dir_of(&m.path);
                    if file_name(&m.path) == "pom.xml" {
                        out.push(if dir.is_empty() {
                            "mvn package".to_string()
                        } else {
                            format!("mvn -f {} package", quote(&m.path))
                        });
                    } else {
                        let wrapper = repo.facts.files.iter().any(|f| f.path == "gradlew");
                        let gradle = if wrapper { "./gradlew" } else { "gradle" };
                        out.push(if dir.is_empty() {
                            format!("{gradle} build")
                        } else {
                            format!("{gradle} -p {} build", quote(&dir))
                        });
                    }
                }
            }
            _ => {}
        }
        out
    }

    fn render_index(&self, note: &Planned, dir: &str) -> String {
        let label = dir
            .split('/')
            .map(zone_label)
            .collect::<Vec<_>>()
            .join(" / ");
        let mut out = front(&note.tags);
        out.push_str(&format!(
            "# {label} — Map of Content\n\n> {}\n",
            self.summaries[&note.id]
        ));
        let mut notes: Vec<&Planned> = self
            .plan
            .notes
            .iter()
            .filter(|n| n.dir == dir && !matches!(n.kind, NoteKind::Index | NoteKind::Home))
            .collect();
        notes.sort_by_key(|a| a.title.to_lowercase());
        let bullets: Vec<String> = notes
            .iter()
            .filter_map(|n| {
                Some(format!(
                    "- {} — {}",
                    self.link(&n.id)?,
                    lower_first(&self.summaries[&n.id])
                ))
            })
            .collect();
        if !bullets.is_empty() {
            out.push_str(&format!("\n## Notes\n\n{}\n", bullets.join("\n")));
        }
        let prefix = format!("{dir}/");
        let folders: Vec<String> = self
            .plan
            .notes
            .iter()
            .filter(|n| {
                n.kind == NoteKind::Index
                    && n.dir.starts_with(&prefix)
                    && !n.dir[prefix.len()..].contains('/')
            })
            .filter_map(|n| {
                let count = self
                    .plan
                    .notes
                    .iter()
                    .filter(|m| {
                        (m.dir == n.dir || m.dir.starts_with(&format!("{}/", n.dir)))
                            && !matches!(m.kind, NoteKind::Index)
                    })
                    .count();
                Some(format!("- {} — {count} notes", self.link(&n.id)?))
            })
            .collect();
        if !folders.is_empty() {
            out.push_str(&format!("\n## Folders\n\n{}\n", folders.join("\n")));
        }
        let mut see = vec!["- [[_HOME]]".to_string()];
        if let Some((parent, _)) = dir.rsplit_once('/')
            && let Some(link) = self.link(&format!("index:{parent}"))
        {
            see.push(format!("- {link}"));
        }
        out.push_str(&format!("\n## See also\n\n{}\n", see.join("\n")));
        out
    }

    fn render_home(&self, note: &Planned) -> String {
        let mut out = front(&note.tags);
        out.push_str(&format!(
            "# {} — Knowledge Vault\n\nA graph-linked tour of the codebase. **Start with the architecture flows**, then drill into {}, modules and patterns.\n\n> {}\n",
            self.repo_name,
            if self.repo.units.all_rust() {
                "crates"
            } else {
                "packages"
            },
            self.summaries[&note.id]
        ));
        let in_zone = |zone: &str| -> Vec<&Planned> {
            let mut notes: Vec<&Planned> = self
                .plan
                .notes
                .iter()
                .filter(|n| n.dir == zone && !matches!(n.kind, NoteKind::Index | NoteKind::Home))
                .collect();
            notes.sort_by(|a, b| a.title.cmp(&b.title));
            notes
        };
        let zone_link = |zone: &str, label: &str| -> Option<String> {
            let entry = self.registry.by_id(&format!("index:{zone}"))?;
            let stem = entry.path.strip_suffix(".md").unwrap_or(&entry.path);
            Some(format!("[[{stem}|{label}]]"))
        };

        let flows = in_zone(ZONE_ARCHITECTURE);
        if !flows.is_empty() {
            out.push_str("\n## 🧭 Start here — Architecture flows\n\n");
            for n in &flows {
                if let Some(link) = self.link(&n.id) {
                    out.push_str(&format!(
                        "- {link} — {}\n",
                        lower_first(&self.summaries[&n.id])
                    ));
                }
            }
            if let Some(link) = zone_link(ZONE_ARCHITECTURE, "All architecture concepts") {
                out.push_str(&format!("\n→ {link}\n"));
            }
        }

        let crates = &self.repo.units.list;
        if !crates.is_empty() {
            out.push_str(if self.repo.units.all_rust() {
                "\n## 🦀 Core\n\n"
            } else {
                "\n## 📦 Core\n\n"
            });
            for c in crates {
                let dir = format!("{ZONE_CORE}/{}", c.slug);
                if self.registry.by_id(&format!("index:{dir}")).is_some() {
                    let summary = &self.summaries[&format!("crate:{}", c.name)];
                    out.push_str(&format!(
                        "- [[{dir}/_index|{}/]] — {}\n",
                        c.package,
                        lower_first(summary)
                    ));
                }
            }
        }

        for (zone, heading, label) in [
            (
                ZONE_INTEGRATIONS,
                "🔌 External Integrations",
                "All integrations",
            ),
            (ZONE_PATTERNS, "🧩 Patterns", "All patterns"),
            (ZONE_OPERATIONS, "🛠️ Operations", "All operations"),
        ] {
            let notes = in_zone(zone);
            let Some(link) = zone_link(zone, label) else {
                continue;
            };
            let names: Vec<&str> = notes.iter().take(10).map(|n| n.title.as_str()).collect();
            out.push_str(&format!(
                "\n## {heading}\n\n→ {link} — {}.\n",
                names.join(", ")
            ));
        }

        // Reading path: a flow, then the most depended-upon crate, then its top type.
        let mut path: Vec<String> = Vec::new();
        if let Some(flow) = flows.first().and_then(|n| self.link(&n.id)) {
            path.push(flow);
        }
        let mut ranked: Vec<&Planned> = self
            .plan
            .notes
            .iter()
            .filter(|n| matches!(n.subject, Subject::Crate(_)))
            .collect();
        ranked.sort_by(|a, b| {
            let used = |n: &Planned| self.used_by.get(&n.id).map(|m| m.len()).unwrap_or(0);
            used(b).cmp(&used(a)).then(a.title.cmp(&b.title))
        });
        if let Some(top) = ranked.first() {
            path.extend(self.link(&top.id));
            let mut types: Vec<&Planned> = self
                .plan
                .notes
                .iter()
                .filter(|n| matches!(&n.subject, Subject::Type(_)) && n.dir.starts_with(&top.dir))
                .collect();
            types.sort_by(|a, b| {
                let rank = |n: &Planned| match &n.subject {
                    Subject::Type(q) => self.repo.subtree_rank(q),
                    _ => 0.0,
                };
                rank(b).total_cmp(&rank(a)).then(a.title.cmp(&b.title))
            });
            path.extend(types.first().and_then(|n| self.link(&n.id)));
        }
        path.extend(self.link("ops:build"));
        if path.len() >= 2 {
            out.push_str(&format!(
                "\n## 🎯 Reading paths\n\n**New contributor:** {}\n",
                path.join(" → ")
            ));
        }
        out
    }
}

/// Accumulates a note's sections; empty ones are dropped.
struct Doc {
    out: String,
}

impl Doc {
    fn new(note: &Planned, summary: &str) -> Self {
        let mut out = front(&note.tags);
        out.push_str(&format!("# {}\n\n> {summary}\n", note.title));
        Self { out }
    }

    fn section(&mut self, heading: &str, body: String) {
        let body = body.trim();
        if !body.is_empty() {
            self.out.push_str(&format!("\n## {heading}\n\n{body}\n"));
        }
    }

    fn finish(self) -> String {
        self.out
    }
}

fn front(tags: &[String]) -> String {
    format!("---\ntags: [{}]\n---\n\n", tags.join(", "))
}

/// `01-Core` -> `Core`; other folder names are kept.
fn zone_label(segment: &str) -> &str {
    match segment.split_once('-') {
        Some((number, rest)) if number.len() == 2 && number.chars().all(|c| c.is_ascii_digit()) => {
            rest
        }
        _ => segment,
    }
}

fn top_children<'a>(repo: &Repo<'a>, parent: &str, limit: usize) -> Vec<&'a Symbol> {
    let mut items: Vec<&Symbol> = repo
        .children_of(parent)
        .iter()
        .copied()
        .filter(|s| s.kind != SymbolKind::Module)
        .collect();
    items.sort_by(|a, b| {
        (b.visibility == Visibility::Public)
            .cmp(&(a.visibility == Visibility::Public))
            .then(
                repo.subtree_rank(&b.qualified)
                    .total_cmp(&repo.subtree_rank(&a.qualified)),
            )
            .then(a.name.cmp(&b.name))
    });
    items.truncate(limit);
    items
}

fn join_and(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a} and {b}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

/// `1 module`, `3 modules`.
fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

fn language_name(language: Language) -> &'static str {
    match language {
        Language::Rust => "Rust",
        Language::Python => "Python",
        Language::TypeScript => "TypeScript",
        Language::JavaScript => "JavaScript",
        Language::Java => "Java",
        _ => "source",
    }
}

/// What one unit of a language is called.
fn unit_noun(language: Language) -> &'static str {
    match language {
        Language::Rust => "crate",
        Language::Java => "module",
        _ => "package",
    }
}

/// `crate` when only Rust files import the dependency, else `package`.
fn external_noun(files: &BTreeSet<&str>) -> &'static str {
    if files
        .iter()
        .all(|f| Language::from_path(f) == Language::Rust)
    {
        "crate"
    } else {
        "package"
    }
}

/// What a build file outside Cargo is for.
fn build_role(path: &str) -> &'static str {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name {
        "pyproject.toml" => "Python project manifest",
        "setup.py" | "setup.cfg" => "Python packaging configuration",
        "Pipfile" => "Python dependencies",
        "package.json" => "package manifest",
        "pom.xml" => "Maven build",
        "build.gradle" | "build.gradle.kts" => "Gradle build",
        _ if name.starts_with("requirements") => "Python dependencies",
        _ => "build file",
    }
}

fn ensure_period(text: &str) -> String {
    let t = text.trim();
    if t.is_empty() || t.ends_with(['.', '!', '?']) {
        t.to_string()
    } else {
        format!("{t}.")
    }
}

/// Lowercase the first letter unless the text starts with code or an acronym.
fn lower_first(text: &str) -> String {
    let word = text.split([' ', ',', ':', '.']).next().unwrap_or("");
    let mut chars = word.chars();
    // Only a plain capitalized word is lowered: code, acronyms, CamelCase
    // identifiers, one-letter words and proper nouns keep their capital.
    let plain = chars.next().is_some_and(char::is_uppercase)
        && chars.clone().next().is_some()
        && chars.all(char::is_lowercase)
        && !PROPER_NOUNS.contains(&word);
    if plain {
        let first = text.chars().next().unwrap_or_default();
        first.to_lowercase().collect::<String>() + &text[first.len_utf8()..]
    } else {
        text.to_string()
    }
}

/// Product and technology names that keep their capital at the start of a
/// list item.
const PROPER_NOUNS: [&str; 17] = [
    "Obsidian", "Rust", "Markdown", "Mermaid", "Windows", "Docker", "Git", "Cargo", "Claude",
    "Codex", "Unicode", "Louvain", "Tarjan", "Brandes", "Python", "Linux", "Java",
];

/// Make doc-comment text safe to place in a note: rustdoc links become code,
/// Markdown links keep only their text, and wikilink brackets are defused.
pub fn clean_inline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
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

/// The first paragraph after its first sentence, capped at about 60 words.
fn doc_remainder(doc: &str) -> String {
    let paragraph = first_paragraph(doc);
    let Some(cut) = sentence_break(&paragraph) else {
        return String::new();
    };
    let words: Vec<&str> = paragraph[cut..].split_whitespace().collect();
    if words.len() > 60 {
        ensure_period(&(words[..60].join(" ") + " …"))
    } else {
        ensure_period(&words.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_doc_comments_into_sentences() {
        let doc = "Parses a note. It keeps `a.b` intact, e.g. paths.\nSecond line here.\n\nNext paragraph.";
        assert_eq!(first_sentence(doc), "Parses a note.");
        assert_eq!(
            doc_remainder(doc),
            "It keeps `a.b` intact, e.g. paths. Second line here."
        );
        assert_eq!(first_sentence("No period"), "No period.");
        assert_eq!(first_sentence(""), "");
        assert_eq!(doc_remainder("Only one sentence."), "");
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
    }

    #[test]
    fn formats_lists_and_labels() {
        assert_eq!(
            join_and(&["a".into(), "b".into(), "c".into()]),
            "a, b, and c"
        );
        assert_eq!(zone_label("01-Core"), "Core");
        assert_eq!(zone_label("cw-core"), "cw-core");
        assert_eq!(lower_first("Holds values."), "holds values.");
        assert_eq!(lower_first("HTTP server."), "HTTP server.");
        assert_eq!(lower_first("`code` first."), "`code` first.");
        assert_eq!(
            lower_first("CodeWiki command line."),
            "CodeWiki command line."
        );
        assert_eq!(
            lower_first("Obsidian vault model."),
            "Obsidian vault model."
        );
        assert_eq!(lower_first("A graph."), "A graph.");
    }
}
