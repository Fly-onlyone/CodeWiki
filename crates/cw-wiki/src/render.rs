//! Rendering of planned notes into Markdown in the vault's house style.
//!
//! This is the keyless path: every sentence is derived from structural facts
//! and doc comments. An LLM writer may later replace the prose, but the
//! structure produced here is the contract the linter enforces.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use cw_core::{Language, Symbol, SymbolKind, Visibility};
use cw_parse::PmInfo;
use cw_vault::{NoteKind, Registry};

use crate::analyze::DEPENDENCY_KINDS;
use crate::doctext::{clean_inline, doc_body, ensure_period, first_sentence};
use crate::explain;
use crate::filemap;
use crate::manifests;
use crate::plan::{
    Plan, Planned, Repo, Subject, ZONE_ARCHITECTURE, ZONE_CORE, ZONE_INTEGRATIONS, ZONE_OPERATIONS,
    ZONE_PATTERNS,
};
use crate::units::{Unit, is_test_file};

const MAX_LISTED: usize = 8;
const MAX_NAMES: usize = 4;
/// Build files named on one line of an integration note.
const MAX_MANIFESTS: usize = 3;
/// Modules listed in the note of a unit that has no root module.
const MAX_UNIT_MODULES: usize = 12;
/// Notes the home note names of a folder of hand-written notes that has
/// no map note of its own.
const MAX_HAND_NOTES: usize = 8;

/// Note-level dependency edges: note id -> other note id -> symbol names.
type NoteDeps = HashMap<String, BTreeMap<String, BTreeSet<String>>>;

/// A top folder of the vault that holds notes their owner wrote by hand.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HandZone {
    /// Whether the folder has a map note of its owner's, `_index.md`.
    pub index: bool,
    /// Names of its notes, without the map note.
    pub notes: Vec<String>,
}

pub struct RenderCtx<'a> {
    pub repo: &'a Repo<'a>,
    pub plan: &'a Plan,
    pub registry: &'a Registry,
    pub repo_name: &'a str,
    pub summaries: HashMap<String, String>,
    /// The folders of the notes written by hand, by folder name. The home
    /// note links each one that none of its other sections reaches, so
    /// that a note of its owner is never cut off from the map.
    pub hand_zones: BTreeMap<String, HandZone>,
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
            // A Go test is no user of the code it tests: its package owns it,
            // and a note would list its own tests among the code that uses it.
            if repo.symbols.get(e.src.as_str()).is_some_and(|s| {
                Language::from_path(&s.file) == Language::Go && is_test_file(&s.file)
            }) {
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
            hand_zones: BTreeMap::new(),
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
                            "{} {}{} `{}` with {}.",
                            self.unit_language(unit),
                            if unit.tests { "test " } else { "" },
                            unit_noun(unit.language),
                            note.title,
                            count(
                                repo.module_count(unit),
                                // A Go package is a folder, not a file's module.
                                if unit.language == Language::Go {
                                    "package"
                                } else {
                                    "module"
                                }
                            )
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
                    SymbolKind::Struct if Language::from_path(&symbol.file) == Language::Go => {
                        "embedding the struct"
                    }
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
            Subject::FileUnit(name) => self
                .plan
                .files
                .unit(name)
                .map(filemap::unit_summary)
                .unwrap_or_default(),
            Subject::Folder(dir) => self
                .plan
                .files
                .folder(dir)
                .map(|f| filemap::folder_summary(&self.plan.files, f))
                .unwrap_or_default(),
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
                    ZONE_CORE if repo.units.list.is_empty() && !self.plan.files.is_empty() => {
                        "Projects and folders of source files that make up the codebase, mapped at file level."
                            .into()
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
                    if !self.plan.files.is_empty() {
                        return self.plan.files.home_summary(self.repo_name, tail);
                    }
                    return format!(
                        "**{}** is a repository of {files} files with no code in a language CodeWiki maps. {tail}",
                        self.repo_name
                    );
                }
                let (languages, parts) = self.composition();
                // Source no parser maps is told apart from the rest.
                let unmapped = if self.plan.files.is_empty() {
                    String::new()
                } else {
                    format!(" {}", self.plan.files.mixed_sentence())
                };
                format!(
                    "**{}** is a {} codebase of {} and {files} source files.{unmapped} {tail}",
                    self.repo_name,
                    join_and(&languages),
                    parts.join(", ")
                )
            }
        }
    }

    /// The language a unit is called after: the one with more files. A
    /// Gradle or Maven module that holds both Java and Kotlin is named for
    /// both, the larger first (`Kotlin and Java`, and Kotlin on a tie).
    fn unit_language(&self, unit: &Unit) -> String {
        if matches!(unit.language, Language::Java | Language::Kotlin) {
            let files = |language: Language| {
                self.repo
                    .facts
                    .files
                    .iter()
                    .filter(|f| f.language == language)
                    .filter(|f| {
                        self.repo
                            .units
                            .of_file(&f.path)
                            .is_some_and(|u| u.name == unit.name)
                    })
                    .count()
            };
            let (other, name) = if unit.language == Language::Kotlin {
                (Language::Java, "Kotlin and Java")
            } else {
                (Language::Kotlin, "Java and Kotlin")
            };
            if files(other) > 0 {
                return name.to_string();
            }
        }
        language_name(unit.language).to_string()
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
    /// for a script or for the top-level statements of a C# file,
    /// `Class.main` in Java.
    fn entry_label(&self, entry: &str) -> String {
        let Some(symbol) = self.repo.symbols.get(entry) else {
            return "main".into();
        };
        if Language::from_path(&symbol.file) == Language::Rust {
            return "main".into();
        }
        if Language::from_path(&symbol.file) == Language::Go {
            // The folder of the program, relative to its module: what
            // `go run` is given.
            let unit_dir = self
                .repo
                .units
                .of_file(&symbol.file)
                .map_or("", |u| u.dir.as_str());
            let folder = symbol.file.rsplit_once('/').map_or("", |(dir, _)| dir);
            let relative = folder
                .strip_prefix(unit_dir)
                .unwrap_or(folder)
                .trim_start_matches('/');
            return if relative.is_empty() {
                symbol
                    .file
                    .rsplit('/')
                    .next()
                    .unwrap_or(&symbol.file)
                    .to_string()
            } else {
                relative.to_string()
            };
        }
        // A function at the top of a C# or Kotlin file stands in no type: it
        // is known by its file.
        let top_level = symbol.kind == SymbolKind::Function
            && matches!(
                Language::from_path(&symbol.file),
                Language::CSharp | Language::Kotlin | Language::C | Language::Cpp
            );
        match symbol.kind {
            SymbolKind::Module => symbol
                .file
                .rsplit('/')
                .next()
                .unwrap_or(&symbol.file)
                .to_string(),
            _ if top_level => symbol
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
            Subject::FileUnit(name) => {
                if let Some(unit) = self.plan.files.unit(name) {
                    let link = |id: &str| self.link(id);
                    for (heading, body) in filemap::unit_sections(&self.plan.files, unit, &link) {
                        doc.section(heading, body);
                    }
                }
            }
            Subject::Folder(dir) => {
                if let Some(folder) = self.plan.files.folder(dir) {
                    let link = |id: &str| self.link(id);
                    for (heading, body) in filemap::folder_sections(&self.plan.files, folder, &link)
                    {
                        doc.section(heading, body);
                    }
                }
            }
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
        if let Some(root) = root {
            // The summary is the manifest's description when there is one,
            // and the comment's first sentence otherwise.
            // A comment often opens with the description in other words;
            // the reader has just read that.
            let opening = first_sentence(&root.doc).to_lowercase();
            let summary = self.summaries[&note.id].to_lowercase();
            let repeated = summary.starts_with(opening.trim_end_matches('.'));
            let described = krate.is_some_and(|k| !k.description.trim().is_empty());
            let language = Language::from_path(&root.file);
            doc.section(
                "What it does",
                doc_body(&root.doc, language, !described || repeated),
            );
        }
        doc.section("Source", sources.join("\n"));

        let mut body = Vec::new();
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
                // A Go package is a folder of files, not a module of one.
                let noun = if k.language == Language::Go {
                    "package"
                } else {
                    "module"
                };
                let mut shape = format!("Holds {} {place}.", count(repo.module_count(k), noun));
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
        let place = if rust { "crate" } else { "package" };
        let walk = explain::walk(self, &note.id);
        let subject = root.map_or("", |r| r.qualified.as_str());
        let steps = walk.text(self, &note.id, subject, place);
        if !steps.is_empty() {
            body.push(steps);
        }
        doc.section("How it works", body.join("\n\n"));
        let mut listed = Vec::new();
        if !modules.is_empty() {
            listed.push(format!("Modules:\n\n{}", modules.join("\n")));
        }
        let items = root
            .map(|r| explain::items(self, &r.qualified, &walk))
            .unwrap_or_default();
        if !items.is_empty() {
            listed.push(format!("Defined at the {place} root:\n\n{items}"));
        }
        doc.section("Items", listed.join("\n\n"));
        doc.section(
            "Depends on",
            self.dep_bullets(&self.depends_on, &note.id, "uses"),
        );
        doc.section("Used by", self.dep_bullets(&self.used_by, &note.id, "from"));
        doc.section("See also", self.see_also(&[self.link("flow:crates")]));
    }

    fn render_module(&self, doc: &mut Doc, note: &Planned, q: &str) {
        let symbol = self.repo.symbols[q];
        let language = Language::from_path(&symbol.file);
        // The summary is the comment's first sentence; the rest is here.
        doc.section("What it does", doc_body(&symbol.doc, language, true));
        let mut source = format!("- `{}` — module `{q}`", symbol.file);
        for file in implementation_files(symbol) {
            source.push_str(&format!("\n- `{file}` — implementation of module `{q}`"));
        }
        doc.section("Source", source);
        let walk = explain::walk(self, &note.id);
        doc.section("How it works", walk.text(self, &note.id, q, "module"));
        let mut body = Vec::new();
        let items = explain::items(self, q, &walk);
        if !items.is_empty() {
            body.push(items);
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
        doc.section("Items", body.join("\n\n"));
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
        let language = Language::from_path(&symbol.file);
        doc.section("What it does", doc_body(&symbol.doc, language, true));
        let mut source = format!("- `{}` — definition of `{}`", symbol.file, symbol.name);
        for file in implementation_files(symbol) {
            source.push_str(&format!(
                "\n- `{file}` — implementation of `{}`",
                symbol.name
            ));
        }
        doc.section("Source", source);
        let walk = explain::walk(self, &note.id);
        doc.section("How it works", walk.text(self, &note.id, q, "type"));
        let mut body = vec![format!("Declared as `{}`.", symbol.signature)];
        let members = explain::items(self, q, &walk);
        if !members.is_empty() {
            let label = if matches!(symbol.kind, SymbolKind::Trait | SymbolKind::Interface) {
                "Required and provided methods"
            } else {
                "Members"
            };
            body.push(format!("{label}:\n\n{members}"));
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
        doc.section("Items", body.join("\n\n"));
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
        let csharp = crates.iter().any(|c| c.language == Language::CSharp);
        let go = crates.iter().any(|c| c.language == Language::Go);
        let mut prose = vec![if rust {
            "An arrow points from a crate to a crate it depends on.".to_string()
        } else if go {
            let mut declaring: Vec<String> = Vec::new();
            if csharp || crates.iter().any(|c| c.is_rust()) {
                declaring.push("Rust crates".to_string());
            }
            if csharp {
                declaring.push("C# projects".to_string());
            }
            declaring.push("Go modules".to_string());
            format!(
                "An arrow points from a package to a package it depends on. {} declare their dependencies; for other languages they are read from imports.",
                join_and(&declaring)
            )
        } else if csharp {
            "An arrow points from a package to a package it depends on. Rust crates and C# projects declare their dependencies; for other languages they are read from imports.".to_string()
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
        if let Some(what) = repo.packages.descriptions.get(krate) {
            doc.section("What it does", ensure_period(&clean_inline(what)));
        }
        let uses: Vec<&cw_parse::ExternalUse> = repo
            .facts
            .external_uses
            .iter()
            .filter(|u| u.krate == krate)
            .collect();
        let files: BTreeSet<&str> = uses.iter().map(|u| u.file.as_str()).collect();
        // Each note whose code imports the package: in how many files, and
        // which names.
        let mut users: BTreeMap<String, (usize, BTreeSet<&str>)> = BTreeMap::new();
        for used in &uses {
            let owner = repo
                .facts
                .symbols
                .iter()
                .find(|s| s.kind == SymbolKind::Module && s.file == used.file)
                .and_then(|s| self.plan.owner.get(&s.qualified));
            if let Some(link) = owner.and_then(|id| self.link(id)) {
                let user = users.entry(link).or_default();
                user.0 += 1;
                user.1.extend(used.names.iter().map(String::as_str));
            }
        }
        let mut rows: Vec<(&String, &(usize, BTreeSet<&str>))> = users.iter().collect();
        rows.sort_by(|a, b| b.1.0.cmp(&a.1.0).then(a.0.cmp(b.0)));
        let used_for = rows
            .iter()
            .take(MAX_LISTED)
            .map(|(link, (_, names))| {
                // A name in backticks must not read as a link or a line.
                let fit: Vec<&&str> = names.iter().filter(|n| !n.contains('`')).collect();
                if fit.is_empty() {
                    return format!("- {link}");
                }
                let listed: Vec<String> = fit
                    .iter()
                    .take(MAX_NAMES)
                    .map(|n| format!("`{n}`"))
                    .collect();
                let more = fit.len().saturating_sub(MAX_NAMES);
                if more > 0 {
                    format!("- {link} — imports {} and {more} more", listed.join(", "))
                } else {
                    format!("- {link} — imports {}", listed.join(", "))
                }
            })
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
        // What each build file asks for, as far as it was read. A workspace
        // names one dependency in many manifests, mostly in the same words:
        // those share a line.
        let mut asking: Vec<(String, Vec<String>)> = Vec::new();
        for d in manifests::declared(repo, krate) {
            let code =
                |text: &str| (!text.is_empty() && !text.contains('`')).then(|| format!("`{text}`"));
            let mut asked: Vec<String> = Vec::new();
            if let Some(version) = code(&d.requirement) {
                asked.push(format!("version {version}"));
            }
            if let Some(locked) = code(&d.locked) {
                asked.push(format!("locked {locked}"));
            }
            let features: Vec<String> = d.features.iter().filter_map(|f| code(f)).collect();
            if !features.is_empty() {
                asked.push(format!("features {}", features.join(", ")));
            }
            if !d.default_features {
                asked.push("without its default features".to_string());
            }
            let asked = asked.join(", ");
            match asking.iter_mut().find(|(known, _)| *known == asked) {
                Some((_, manifests)) => manifests.push(d.manifest),
                None => asking.push((asked, vec![d.manifest])),
            }
        }
        let declaring: Vec<String> = asking
            .iter()
            .map(|(asked, manifests)| {
                let mut files: Vec<String> = manifests
                    .iter()
                    .take(MAX_MANIFESTS)
                    .map(|m| format!("`{m}`"))
                    .collect();
                if manifests.len() > MAX_MANIFESTS {
                    files.push(format!("and {} more", manifests.len() - MAX_MANIFESTS));
                }
                let verb = if manifests.len() == 1 {
                    "declares"
                } else {
                    "declare"
                };
                let mut line = format!("- {} — {verb} the dependency", files.join(", "));
                if !asked.is_empty() {
                    line.push_str(&format!(": {asked}"));
                }
                line
            })
            .collect();
        doc.section("Configuration", declaring.join("\n"));
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
            Language::Kotlin => {
                // An interface is implemented; a class is extended through
                // what it lets be overridden.
                let parameters = kotlin_type_parameters(&symbol.signature, &symbol.name);
                let generic = if parameters.is_empty() {
                    String::new()
                } else {
                    format!("<{}>", parameters.join(", "))
                };
                let supertype = if symbol.kind == SymbolKind::Class {
                    format!("{}{generic}()", symbol.name)
                } else {
                    format!("{}{generic}", symbol.name)
                };
                let mut code = format!("class NewType{generic} : {supertype} {{\n");
                for m in methods
                    .iter()
                    .filter(|m| m.visibility != Visibility::Private && m.name != symbol.name)
                    .filter(|m| {
                        symbol.kind != SymbolKind::Class
                            || m.signature
                                .split_whitespace()
                                .any(|w| matches!(w, "abstract" | "open" | "override"))
                    })
                    .take(MAX_LISTED)
                {
                    code.push_str(&format!(
                        "    {} {{\n        TODO()\n    }}\n",
                        kotlin_override(&m.signature)
                    ));
                }
                code.push('}');
                ("kotlin", code)
            }
            Language::Go if symbol.kind == SymbolKind::Interface => {
                let mut code = "type NewType struct{}".to_string();
                for m in methods.iter().take(MAX_LISTED) {
                    code.push_str(&format!(
                        "\n\nfunc (t *NewType) {} {{\n\tpanic(\"not implemented\")\n}}",
                        m.signature
                    ));
                }
                ("go", code)
            }
            // A struct is taken on by embedding it.
            Language::Go => (
                "go",
                format!("type NewType struct {{\n\t{}\n}}", symbol.name),
            ),
            Language::CSharp => {
                let mut code = format!("class NewType : {} {{\n", symbol.name);
                for m in methods
                    .iter()
                    .filter(|m| m.visibility != Visibility::Private && m.name != symbol.name)
                    // A class is extended through what it lets be overridden.
                    .filter(|m| {
                        symbol.kind != SymbolKind::Class
                            || m.signature
                                .split_whitespace()
                                .any(|w| matches!(w, "abstract" | "virtual"))
                    })
                    .take(MAX_LISTED)
                {
                    code.push_str(&format!(
                        "    {} {{\n        throw new NotImplementedException();\n    }}\n",
                        csharp_override(&m.signature)
                    ));
                }
                code.push('}');
                ("csharp", code)
            }
            Language::C | Language::Cpp => {
                let mut code = format!("class NewType : public {} {{\npublic:\n", symbol.name);
                for m in methods
                    .iter()
                    .filter(|m| m.visibility == Visibility::Public)
                    .filter(|m| {
                        !m.attrs
                            .iter()
                            .any(|a| matches!(a.as_str(), "destructor" | "special"))
                            && m.name != symbol.name
                    })
                    // A class is extended through what it lets be overridden.
                    .filter(|m| {
                        symbol.kind != SymbolKind::Class || m.attrs.iter().any(|a| a == "virtual")
                    })
                    .take(MAX_LISTED)
                {
                    code.push_str(&format!(
                        "    {} {{\n        throw std::logic_error(\"not implemented\");\n    }}\n",
                        cpp_override(&m.signature)
                    ));
                }
                code.push_str("};");
                ("cpp", code)
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
        // The whole comment: a reason is seldom in its first sentence.
        doc.section("Why", doc_body(&symbol.doc, language, false));
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
                let pm = unit.pm.clone().unwrap_or_else(|| PmInfo::plain(&unit.dir));
                out.push(crate::pm::install(&pm));
                for script in ["build", "test", "lint", "start"] {
                    if unit.scripts.iter().any(|s| s == script) {
                        out.push(crate::pm::run_script(&pm, &unit.dir, script));
                    }
                }
            }
            Language::CSharp => {
                // One solution builds and tests every project; without one
                // each project does.
                let solution = repo
                    .facts
                    .files
                    .iter()
                    .map(|f| f.path.as_str())
                    .filter(|p| p.ends_with(".sln") || p.ends_with(".slnx"))
                    .filter(|p| {
                        let dir = dir_of(p);
                        dir.is_empty()
                            || unit.dir == dir
                            || unit.dir.starts_with(&format!("{dir}/"))
                    })
                    .min_by_key(|p| (p.matches('/').count(), p.len()));
                let project = unit
                    .manifests
                    .first()
                    .filter(|m| m.path.ends_with(".csproj"));
                match (solution, project) {
                    (Some(solution), _) => {
                        out.push(format!("dotnet build {}", quote(solution)));
                        if repo.units.list.iter().any(|u| u.tests) {
                            out.push(format!("dotnet test {}", quote(solution)));
                        }
                    }
                    (None, Some(project)) => {
                        out.push(format!("dotnet build {}", quote(&project.path)));
                        if unit.tests {
                            out.push(format!("dotnet test {}", quote(&project.path)));
                        }
                    }
                    (None, None) => {}
                }
                if let Some(project) = project
                    && !unit.entries.is_empty()
                {
                    out.push(format!("dotnet run --project {}", quote(&project.path)));
                }
            }
            Language::Go => {
                if !unit
                    .manifests
                    .first()
                    .is_some_and(|m| m.path.ends_with("go.mod"))
                {
                    return out;
                }
                // `-C` runs the command in the module's folder, so that every
                // command runs from the repository root.
                let from = if unit.dir.is_empty() {
                    String::new()
                } else {
                    format!(" -C {}", quote(&unit.dir))
                };
                out.push(format!("go build{from} ./..."));
                let tested = repo.facts.files.iter().any(|f| {
                    f.path.ends_with("_test.go")
                        && repo
                            .units
                            .of_file(&f.path)
                            .is_some_and(|u| u.name == unit.name)
                });
                if tested {
                    out.push(format!("go test{from} ./..."));
                }
                for entry in &unit.entries {
                    let Some(symbol) = repo.symbols.get(entry.symbol.as_str()) else {
                        continue;
                    };
                    let folder = dir_of(&symbol.file);
                    let relative = folder
                        .strip_prefix(unit.dir.as_str())
                        .unwrap_or(&folder)
                        .trim_start_matches('/');
                    out.push(if relative.is_empty() {
                        format!("go run{from} .")
                    } else {
                        format!("go run{from} ./{}", quote(relative))
                    });
                }
            }
            Language::C | Language::Cpp => {
                // CMake builds in a folder next to the sources. Presets name
                // a folder of their own (`binaryDir`), so no command here
                // follows them: the reader sees `CMakePresets.json` among the
                // sources, and the commands that follow assume `build`.
                if !unit
                    .manifests
                    .iter()
                    .any(|m| file_name(&m.path) == "CMakeLists.txt")
                {
                    return out;
                }
                out.push("cmake -S . -B build".into());
                out.push("cmake --build build".into());
                if repo.units.c_tests() {
                    out.push("ctest --test-dir build".into());
                }
            }
            Language::Java | Language::Kotlin => {
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
        // Without a unit there are no flows, crates or modules to name.
        let tour = if self.repo.units.list.is_empty() && !self.plan.files.is_empty() {
            "A graph-linked tour of the codebase, mapped at file level: its projects, its folders and the files in them."
                .to_string()
        } else if self.repo.units.list.is_empty() {
            "A graph-linked tour of the repository.".to_string()
        } else {
            format!(
                "A graph-linked tour of the codebase. **Start with the architecture flows**, then drill into {}, modules and patterns.",
                if self.repo.units.all_rust() {
                    "crates"
                } else {
                    "packages"
                }
            )
        };
        out.push_str(&format!(
            "# {} — Knowledge Vault\n\n{tour}\n\n> {}\n",
            self.repo_name, self.summaries[&note.id]
        ));
        // Top folders a section of this note links.
        let mut reached: Vec<&str> = Vec::new();
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
                reached.push(ZONE_ARCHITECTURE);
            }
        }

        let crates = &self.repo.units.list;
        if !crates.is_empty() || !self.plan.files.is_empty() {
            out.push_str(
                if self.repo.units.all_rust() && self.plan.files.is_empty() {
                    "\n## 🦀 Core\n\n"
                } else {
                    "\n## 📦 Core\n\n"
                },
            );
            let parts = crates
                .iter()
                .map(|c| (format!("crate:{}", c.name), &c.slug, &c.package))
                .chain(
                    self.plan
                        .files
                        .units
                        .iter()
                        .map(|u| (format!("unit:{}", u.name), &u.slug, &u.package)),
                );
            for (id, slug, package) in parts {
                let dir = format!("{ZONE_CORE}/{slug}");
                if self.registry.by_id(&format!("index:{dir}")).is_some() {
                    let summary = &self.summaries[&id];
                    out.push_str(&format!(
                        "- [[{dir}/_index|{package}/]] — {}\n",
                        lower_first(summary)
                    ));
                    if !reached.contains(&ZONE_CORE) {
                        reached.push(ZONE_CORE);
                    }
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
            reached.push(zone);
        }

        // The folders of the notes their owner wrote, which no rule knows.
        let by_hand: Vec<(&String, &HandZone)> = self
            .hand_zones
            .iter()
            .filter(|(folder, zone)| {
                !reached.contains(&folder.as_str()) && (zone.index || !zone.notes.is_empty())
            })
            .collect();
        if !by_hand.is_empty() {
            out.push_str("\n## 📝 Notes written by hand\n\n");
            for (folder, zone) in by_hand {
                let count = match zone.notes.len() {
                    1 => "1 note".to_string(),
                    n => format!("{n} notes"),
                };
                if zone.index {
                    out.push_str(&format!("- [[{folder}/_index|{folder}/]] — {count}\n"));
                } else {
                    let mut names: Vec<&str> = zone.notes.iter().map(String::as_str).collect();
                    names.sort_unstable();
                    let more = names.len().saturating_sub(MAX_HAND_NOTES);
                    let mut links: Vec<String> = names
                        .iter()
                        .take(MAX_HAND_NOTES)
                        .map(|name| format!("[[{name}]]"))
                        .collect();
                    if more > 0 {
                        links.push(format!("and {more} more"));
                    }
                    out.push_str(&format!("- {folder}/ — {count}: {}\n", links.join(", ")));
                }
            }
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
    head: String,
    sections: Vec<cw_vault::Section>,
}

impl Doc {
    fn new(note: &Planned, summary: &str) -> Self {
        let mut head = front(&note.tags);
        head.push_str(&format!("# {}\n\n> {summary}\n", note.title));
        Self {
            head,
            sections: Vec::new(),
        }
    }

    fn section(&mut self, heading: &str, body: String) {
        self.sections.push(cw_vault::Section {
            heading: heading.to_string(),
            body,
        });
    }

    /// The note. Its layout is the vault's, which also puts an agent's
    /// sections into a note: the two cannot drift apart.
    fn finish(self) -> String {
        cw_vault::note::join(&self.head, &self.sections)
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

pub(crate) fn join_and(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a} and {b}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

/// `1 module`, `3 modules`.
pub(crate) fn count(n: usize, noun: &str) -> String {
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
        Language::Kotlin => "Kotlin",
        Language::CSharp => "C#",
        Language::Go => "Go",
        Language::C => "C",
        Language::Cpp => "C++",
        _ => "source",
    }
}

/// What one unit of a language is called.
fn unit_noun(language: Language) -> &'static str {
    match language {
        Language::Rust => "crate",
        Language::Java | Language::Kotlin => "module",
        Language::CSharp => "project",
        Language::Go => "module",
        Language::C | Language::Cpp => "project",
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

/// The files, besides the header a symbol stands in, that define what the
/// header declares (C and C++).
fn implementation_files(symbol: &Symbol) -> Vec<&str> {
    symbol
        .attrs
        .iter()
        .filter_map(|a| a.strip_prefix(cw_parse::IMPL_ATTR))
        .collect()
}

/// A member of a C++ interface as the override a class that takes it on
/// writes: `virtual` goes, `override` comes, and a pure virtual's `= 0` goes.
fn cpp_override(signature: &str) -> String {
    let mut text = signature.trim().trim_end_matches(';').trim().to_string();
    if let Some(rest) = text.strip_prefix("virtual ") {
        text = rest.to_string();
    }
    for tail in ["= 0", "= default", "= delete"] {
        if let Some(rest) = text.strip_suffix(tail) {
            text = rest.trim_end().to_string();
        }
    }
    if text.split_whitespace().any(|w| w == "override") {
        text
    } else {
        format!("{text} override")
    }
}

/// A C# member as a stub in a type that extends or implements it: an
/// interface member has no modifier and becomes public, an abstract or
/// virtual one is overridden.
fn csharp_override(signature: &str) -> String {
    let words: Vec<&str> = signature.split_whitespace().collect();
    let has = |w: &str| words.contains(&w);
    if has("abstract") || has("virtual") {
        return words
            .iter()
            .map(|w| {
                if matches!(*w, "abstract" | "virtual") {
                    "override"
                } else {
                    w
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
    }
    if has("public") || has("protected") || has("internal") {
        signature.to_string()
    } else {
        format!("public {signature}")
    }
}

/// The names of the type parameters a Kotlin declaration lists after its
/// name: `T` and `R` for `interface Mapper<in T, out R : Any> : Base<T>`.
fn kotlin_type_parameters(signature: &str, name: &str) -> Vec<String> {
    let Some(at) = signature.find(&format!(" {name}<")) else {
        return Vec::new();
    };
    let rest = &signature[at + name.len() + 2..];
    let mut depth = 1usize;
    let mut parts: Vec<String> = vec![String::new()];
    for c in rest.chars() {
        match c {
            '<' | '(' => depth += 1,
            '>' | ')' => {
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
        if let Some(last) = parts.last_mut() {
            last.push(c);
        }
    }
    parts
        .iter()
        .filter_map(|part| {
            part.split_whitespace()
                .find(|w| !matches!(*w, "in" | "out" | "reified"))
                .map(str::to_string)
        })
        .collect()
}

/// A Kotlin member as a stub in a type that extends or implements it: what
/// let it be overridden is replaced by `override`.
fn kotlin_override(signature: &str) -> String {
    let words: Vec<&str> = signature
        .split_whitespace()
        .filter(|w| {
            !matches!(
                *w,
                "abstract" | "open" | "override" | "final" | "public" | "protected"
            )
        })
        .collect();
    format!("override {}", words.join(" "))
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
        "CMakeLists.txt" => "CMake build",
        "CMakePresets.json" => "CMake presets",
        "vcpkg.json" => "vcpkg dependencies",
        "conanfile.txt" | "conanfile.py" => "Conan dependencies",
        "conan.lock" => "Conan lock file",
        "go.mod" => "Go module",
        "go.work" => "Go workspace",
        "go.sum" => "Go module checksums",
        "Directory.Packages.props" => "central NuGet package versions",
        "Directory.Build.props" | "Directory.Build.targets" => "shared MSBuild properties",
        "global.json" => ".NET SDK version",
        "nuget.config" | "NuGet.Config" => "NuGet sources",
        _ if name.ends_with(".csproj") => "C# project",
        _ if name.ends_with(".sln") || name.ends_with(".slnx") => "solution",
        _ if name.starts_with("requirements") => "Python dependencies",
        _ => "build file",
    }
}

/// Lowercase the first letter unless the text starts with code or an acronym.
pub(crate) fn lower_first(text: &str) -> String {
    let word = text.split([' ', ',', ':', '.']).next().unwrap_or("");
    let mut chars = word.chars();
    // Only a plain capitalized word is lowered: code, acronyms, CamelCase
    // identifiers, one-letter words and proper nouns keep their capital.
    let plain = chars.next().is_some_and(char::is_uppercase)
        && chars.clone().next().is_some()
        && chars.all(char::is_lowercase)
        && !PROPER_NOUNS.contains(&word)
        && !starts_with_language_unit(text);
    if plain {
        let first = text.chars().next().unwrap_or_default();
        first.to_lowercase().collect::<String>() + &text[first.len_utf8()..]
    } else {
        text.to_string()
    }
}

/// True for the sentence of a unit that starts with the name of its
/// language and that name is an ordinary word: `Go module `app` with 3
/// packages.` keeps its capital, a sentence that starts with "Go to" does
/// not. (`C`, `C++` and `C#` are never lowered, whatever follows.)
fn starts_with_language_unit(text: &str) -> bool {
    text.strip_prefix("Go ").is_some_and(|rest| {
        let rest = rest.strip_prefix("test ").unwrap_or(rest);
        rest.starts_with("module `")
    })
}

/// Product and technology names that keep their capital at the start of a
/// list item.
const PROPER_NOUNS: [&str; 18] = [
    "Obsidian", "Rust", "Markdown", "Mermaid", "Windows", "Docker", "Git", "Cargo", "Claude",
    "Codex", "Unicode", "Louvain", "Tarjan", "Brandes", "Python", "Linux", "Java", "Kotlin",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stubs_a_csharp_member_for_the_type_that_takes_it_on() {
        assert_eq!(csharp_override("string Name()"), "public string Name()");
        assert_eq!(
            csharp_override("public string Name()"),
            "public string Name()"
        );
        assert_eq!(
            csharp_override("protected abstract void Run(int a)"),
            "protected override void Run(int a)"
        );
        assert_eq!(
            csharp_override("public virtual Task<int> Load()"),
            "public override Task<int> Load()"
        );
        assert_eq!(build_role("src/App/App.Core.csproj"), "C# project");
        assert_eq!(
            build_role("Directory.Packages.props"),
            "central NuGet package versions"
        );
        assert_eq!(build_role("a/b/App.slnx"), "solution");
        assert_eq!(language_name(Language::CSharp), "C#");
        assert_eq!(unit_noun(Language::CSharp), "project");
        assert_eq!(language_name(Language::Go), "Go");
        assert_eq!(unit_noun(Language::Go), "module");
        assert_eq!(build_role("services/api/go.mod"), "Go module");
        assert_eq!(build_role("go.work"), "Go workspace");
        assert_eq!(language_name(Language::Kotlin), "Kotlin");
        assert_eq!(unit_noun(Language::Kotlin), "module");
        assert_eq!(build_role("app/build.gradle.kts"), "Gradle build");
        // A Kotlin name keeps its capital at the start of a list item.
        assert_eq!(lower_first("Kotlin module `app`."), "Kotlin module `app`.");
    }

    #[test]
    fn a_kotlin_stub_repeats_the_type_parameters_of_its_supertype() {
        let parameters = |signature: &str, name: &str| kotlin_type_parameters(signature, name);
        assert_eq!(
            parameters("interface Codec<T> : Encoding<T>", "Codec"),
            ["T"]
        );
        assert_eq!(
            parameters(
                "interface Mapper<in T, out R : Comparable<R>> : Base<T>",
                "Mapper"
            ),
            ["T", "R"]
        );
        assert!(parameters("interface Named", "Named").is_empty());
        // The name's own generics, not a supertype's.
        assert!(parameters("interface Named : Base<T>", "Named").is_empty());
        assert_eq!(
            kotlin_override("abstract fun total(): Int"),
            "override fun total(): Int"
        );
        assert_eq!(
            kotlin_override("open suspend fun load(id: Int): Item?"),
            "override suspend fun load(id: Int): Item?"
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

    #[test]
    fn a_unit_sentence_keeps_the_name_of_its_language() {
        // The names that are no plain word keep their spelling.
        for text in [
            "C project `core` with 3 modules.",
            "C++ project `net` with 1 module.",
            "C# project `App` with 2 modules.",
            "Go module `ledger` with 5 packages.",
            "Go test module `ledger` with 1 package.",
        ] {
            assert_eq!(lower_first(text), text);
        }
        // An ordinary sentence that starts with the word still lowers it.
        assert_eq!(lower_first("Go to the store."), "go to the store.");
        assert_eq!(
            lower_first("Go modules are folders."),
            "go modules are folders."
        );
        assert_eq!(language_name(Language::C), "C");
        assert_eq!(language_name(Language::Cpp), "C++");
        assert_eq!(unit_noun(Language::Cpp), "project");
    }

    #[test]
    fn stubs_a_cpp_member_for_the_class_that_takes_it_on() {
        assert_eq!(
            cpp_override("virtual double area() const = 0"),
            "double area() const override"
        );
        assert_eq!(
            cpp_override("virtual void run() override;"),
            "void run() override"
        );
        assert_eq!(
            cpp_override("int size() const"),
            "int size() const override"
        );
    }
}
