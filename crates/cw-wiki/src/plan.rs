//! Concept selection: decide which notes a repository gets.
//!
//! The vault holds curated concept notes, not a page per file. Selection uses
//! only structural facts, so it works without an LLM and is deterministic.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use std::path::Path;

use cw_core::{Edge, EdgeKind, Language, Symbol, SymbolKind, Visibility};
use cw_index::NodeMetrics;
use cw_parse::RepoFacts;
use cw_vault::NoteKind;

use crate::units::{self, Unit, Units, is_test_file};

pub const ZONE_ARCHITECTURE: &str = "00-Architecture";
pub const ZONE_CORE: &str = "01-Core";
pub const ZONE_INTEGRATIONS: &str = "03-Integrations";
pub const ZONE_PATTERNS: &str = "04-Patterns";
pub const ZONE_OPERATIONS: &str = "05-Operations";

const MAX_TYPES_PER_CRATE: usize = 12;
const MAX_MODULE_NOTES: usize = 300;
const MAX_INTEGRATIONS: usize = 15;
const MAX_PATTERNS: usize = 10;
const MAX_FLOW_MESSAGES: usize = 14;
const MAX_FLOW_DEPTH: usize = 4;
/// Startup flows drawn for one unit with several entry points.
const MAX_FLOWS_PER_UNIT: usize = 3;
/// Outside Rust every file is a module, so only those that hold this many
/// items, or this many symbols counting nested ones, get a note.
const MIN_MODULE_ITEMS: usize = 3;
const MIN_MODULE_SYMBOLS: usize = 5;

/// One step of a startup flow: `from` note calls into `to` note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowMessage {
    pub from: String,
    pub to: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    Home,
    /// Map of Content for a vault folder.
    Index(String),
    Crate(String),
    Module(String),
    Type(String),
    CrateFlow,
    StartupFlow {
        krate: String,
        /// Qualified name of the function or module the flow starts at.
        entry: String,
        messages: Vec<FlowMessage>,
    },
    Integration(String),
    TraitPattern(String),
    Operations,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    /// Stable id, for example `type:cw_core::model::Symbol`.
    pub id: String,
    /// Vault folder the note belongs in.
    pub dir: String,
    pub title: String,
    pub kind: NoteKind,
    pub tags: Vec<String>,
    pub subject: Subject,
    /// Repo-relative source files the note describes, primary first.
    pub sources: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Plan {
    pub notes: Vec<Planned>,
    /// Symbol qualified name -> id of the note that documents it.
    pub owner: HashMap<String, String>,
}

/// Lookup tables over the scanned facts.
pub struct Repo<'a> {
    pub facts: &'a RepoFacts,
    /// Crates and their counterparts in other languages.
    pub units: Units,
    pub symbols: HashMap<&'a str, &'a Symbol>,
    /// Parent qualified name -> children, in name order.
    pub children: HashMap<&'a str, Vec<&'a Symbol>>,
    pub rank: HashMap<&'a str, f64>,
    /// Trait -> implementing types.
    pub implementors: BTreeMap<&'a str, Vec<&'a str>>,
    pub outgoing: HashMap<&'a str, Vec<&'a Edge>>,
    pub incoming: HashMap<&'a str, Vec<&'a Edge>>,
}

impl<'a> Repo<'a> {
    /// Lookup tables from facts alone. Units outside Rust are named from the
    /// layout only; use [`Repo::at`] to also read their manifests.
    pub fn new(facts: &'a RepoFacts, metrics: &'a [(String, NodeMetrics)]) -> Self {
        Self::build(
            facts,
            metrics,
            units::discover(facts, "repository", &|_| None),
        )
    }

    /// Lookup tables for the repository checked out at `root`.
    pub fn at(root: &Path, facts: &'a RepoFacts, metrics: &'a [(String, NodeMetrics)]) -> Self {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "repository".into());
        let read = |path: &str| std::fs::read_to_string(root.join(path)).ok();
        Self::build(facts, metrics, units::discover(facts, &name, &read))
    }

    fn build(facts: &'a RepoFacts, metrics: &'a [(String, NodeMetrics)], units: Units) -> Self {
        let mut symbols = HashMap::new();
        let mut children: HashMap<&str, Vec<&Symbol>> = HashMap::new();
        for s in &facts.symbols {
            symbols.insert(s.qualified.as_str(), s);
            if let Some(parent) = &s.parent {
                children.entry(parent.as_str()).or_default().push(s);
            }
        }
        let mut implementors: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        let mut outgoing: HashMap<&str, Vec<&Edge>> = HashMap::new();
        let mut incoming: HashMap<&str, Vec<&Edge>> = HashMap::new();
        for e in &facts.edges {
            outgoing.entry(e.src.as_str()).or_default().push(e);
            incoming.entry(e.dst.as_str()).or_default().push(e);
            if e.kind == EdgeKind::Implements {
                implementors
                    .entry(e.dst.as_str())
                    .or_default()
                    .push(e.src.as_str());
            }
        }
        Self {
            facts,
            units,
            symbols,
            children,
            rank: metrics
                .iter()
                .map(|(k, m)| (k.as_str(), m.pagerank))
                .collect(),
            implementors,
            outgoing,
            incoming,
        }
    }

    /// Index of the unit that holds the symbol named `qualified`.
    pub fn unit_index(&self, qualified: &str) -> Option<usize> {
        match self.symbols.get(qualified) {
            Some(symbol) => self.units.index_of(symbol),
            None => self.units.rust_index(qualified),
        }
    }

    /// The unit that holds the symbol named `qualified`.
    pub fn unit_of(&self, qualified: &str) -> Option<&Unit> {
        self.unit_index(qualified).map(|i| &self.units.list[i])
    }

    /// Modules directly below the top of `unit`, in name order.
    pub fn top_modules(&self, unit: &Unit) -> Vec<&'a Symbol> {
        if let Some(root) = unit.root.as_deref().filter(|_| unit.is_rust()) {
            return self
                .children_of(root)
                .iter()
                .copied()
                .filter(|s| s.kind == SymbolKind::Module)
                .collect();
        }
        let root = unit.root.as_deref();
        self.facts
            .symbols
            .iter()
            .filter(|s| {
                s.kind == SymbolKind::Module
                    && Some(s.qualified.as_str()) != root
                    && self
                        .unit_of(&s.qualified)
                        .is_some_and(|u| u.name == unit.name)
                    // Nested in another module of the unit: not at the top.
                    && !s.parent.as_deref().is_some_and(|p| {
                        Some(p) != root
                            && self.symbols.get(p).is_some_and(|parent| {
                                parent.kind == SymbolKind::Module
                                    && self.units.index_of(parent) == self.units.index_of(s)
                            })
                    })
            })
            .collect()
    }

    /// Number of source modules in `unit`. A package's `__init__` file is the
    /// package itself, not one of its modules.
    pub fn module_count(&self, unit: &Unit) -> usize {
        self.facts
            .symbols
            .iter()
            .filter(|s| {
                s.kind == SymbolKind::Module
                    && !s.file.contains("__init__.")
                    && self
                        .unit_of(&s.qualified)
                        .is_some_and(|u| u.name == unit.name)
            })
            .count()
    }

    /// Number of symbols nested anywhere below `qualified`.
    pub fn descendants(&self, qualified: &str) -> usize {
        self.children_of(qualified)
            .iter()
            .map(|c| 1 + self.descendants(&c.qualified))
            .sum()
    }

    /// True for test code, which gets no notes of its own.
    pub fn is_test(&self, symbol: &Symbol) -> bool {
        is_test_path(&symbol.qualified)
            || (Language::from_path(&symbol.file) != Language::Rust && is_test_file(&symbol.file))
    }

    pub fn rank_of(&self, qualified: &str) -> f64 {
        self.rank.get(qualified).copied().unwrap_or(0.0)
    }

    pub fn children_of(&self, qualified: &str) -> &[&'a Symbol] {
        self.children
            .get(qualified)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Rank of a symbol plus everything nested in it.
    pub fn subtree_rank(&self, qualified: &str) -> f64 {
        self.rank_of(qualified)
            + self
                .children_of(qualified)
                .iter()
                .map(|c| self.subtree_rank(&c.qualified))
                .sum::<f64>()
    }
}

/// Title of the note about `module`. A TypeScript `index` file is named
/// after its folder.
fn module_title(module: &Symbol) -> String {
    let mut segs = module.qualified.rsplit("::");
    let name = match (segs.next(), segs.next()) {
        (Some("index"), Some(folder)) => folder,
        _ => module.name.as_str(),
    };
    format!("{} Module", title_case(name))
}

/// `hash` -> `Hash`, `note_lifecycle` -> `Note Lifecycle`.
pub fn title_case(snake: &str) -> String {
    snake
        .split(['_', '-'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_test_path(qualified: &str) -> bool {
    qualified
        .split("::")
        .any(|s| matches!(s, "tests" | "test" | "benches" | "examples"))
}

/// Vault folder for notes about `module` and the types it defines.
fn module_dir(unit: &Unit, module: &str) -> String {
    let mut segs: Vec<&str> = module.split("::").skip(unit.strip).collect();
    segs.pop();
    let mut dir = format!("{ZONE_CORE}/{}", unit.slug);
    for s in segs {
        dir.push('/');
        dir.push_str(s);
    }
    dir
}

/// Nearest enclosing module of a symbol (itself when it is a module).
fn enclosing_module<'a>(repo: &Repo<'a>, symbol: &'a Symbol) -> &'a str {
    let mut current = symbol;
    loop {
        if current.kind == SymbolKind::Module {
            return &current.qualified;
        }
        match current.parent.as_deref().and_then(|p| repo.symbols.get(p)) {
            Some(parent) => current = parent,
            None => return current.qualified.split("::").next().unwrap_or(""),
        }
    }
}

/// Select notes. `covered` maps source files already documented by adopted
/// notes to the id of the adopted note; those files get no new module or type
/// notes.
pub fn plan(repo: &Repo, covered: &HashMap<String, String>) -> Plan {
    let facts = repo.facts;
    let mut notes: Vec<Planned> = Vec::new();

    for unit in &repo.units.list {
        let mut sources: Vec<String> = unit.manifests.iter().map(|m| m.path.clone()).collect();
        match unit.root.as_deref().and_then(|r| repo.symbols.get(r)) {
            Some(root) => sources.push(root.file.clone()),
            // Without a root module the largest modules stand for the unit.
            None => {
                let mut top = repo.top_modules(unit);
                top.sort_by(|a, b| {
                    repo.subtree_rank(&b.qualified)
                        .total_cmp(&repo.subtree_rank(&a.qualified))
                        .then_with(|| a.qualified.cmp(&b.qualified))
                });
                sources.extend(top.iter().take(5).map(|m| m.file.clone()));
            }
        }
        notes.push(Planned {
            id: format!("crate:{}", unit.name),
            dir: format!("{ZONE_CORE}/{}", unit.slug),
            title: unit.package.clone(),
            kind: NoteKind::Concept,
            tags: vec!["core".into(), unit.slug.clone()],
            subject: Subject::Crate(unit.name.clone()),
            sources,
        });
    }

    // Modules: everything below a crate root that defines something.
    let mut modules: Vec<&Symbol> = facts
        .symbols
        .iter()
        .filter(|s| {
            let items = repo.children_of(&s.qualified).len();
            s.kind == SymbolKind::Module
                && !repo.is_test(s)
                && !covered.contains_key(&s.file)
                && items > 0
                && repo.unit_of(&s.qualified).is_some_and(|unit| {
                    if unit.root.as_deref() == Some(s.qualified.as_str()) {
                        false
                    } else if unit.is_rust() {
                        s.qualified.contains("::")
                            // Small private inline modules are implementation detail.
                            && (s.visibility != Visibility::Private || items >= 3)
                    } else {
                        items >= MIN_MODULE_ITEMS
                            || repo.descendants(&s.qualified) >= MIN_MODULE_SYMBOLS
                    }
                })
        })
        .collect();
    modules.sort_by(|a, b| {
        repo.subtree_rank(&b.qualified)
            .total_cmp(&repo.subtree_rank(&a.qualified))
            .then_with(|| a.qualified.cmp(&b.qualified))
    });
    modules.truncate(MAX_MODULE_NOTES);
    modules.sort_by(|a, b| a.qualified.cmp(&b.qualified));
    for m in &modules {
        let unit = repo.unit_of(&m.qualified).expect("filtered above");
        notes.push(Planned {
            id: format!("mod:{}", m.qualified),
            dir: module_dir(unit, &m.qualified),
            title: module_title(m),
            kind: NoteKind::Concept,
            tags: vec!["core".into(), unit.slug.clone(), "module".into()],
            subject: Subject::Module(m.qualified.clone()),
            sources: vec![m.file.clone()],
        });
    }

    // Key types per crate.
    for (index, unit) in repo.units.list.iter().enumerate() {
        let mut types: Vec<&Symbol> = facts
            .symbols
            .iter()
            .filter(|s| {
                s.kind.is_type()
                    && s.visibility != Visibility::Private
                    && repo.units.index_of(s) == Some(index)
                    && !repo.is_test(s)
                    && !covered.contains_key(&s.file)
            })
            .collect();
        types.sort_by(|a, b| {
            repo.subtree_rank(&b.qualified)
                .total_cmp(&repo.subtree_rank(&a.qualified))
                .then_with(|| a.qualified.cmp(&b.qualified))
        });
        let top = types.len().div_ceil(4);
        let mut chosen: Vec<&Symbol> = types
            .iter()
            .enumerate()
            .filter(|(i, s)| {
                *i < top
                    || repo.children_of(&s.qualified).len() >= 3
                    || repo
                        .implementors
                        .get(s.qualified.as_str())
                        .is_some_and(|v| v.len() >= 2)
            })
            .map(|(_, s)| *s)
            .take(MAX_TYPES_PER_CRATE)
            .collect();
        chosen.sort_by(|a, b| a.qualified.cmp(&b.qualified));
        for t in chosen {
            let module = enclosing_module(repo, t);
            notes.push(Planned {
                id: format!("type:{}", t.qualified),
                // Types sit beside the note of the module that defines them.
                dir: module_dir(unit, module),
                title: t.name.clone(),
                kind: NoteKind::Concept,
                tags: vec!["core".into(), unit.slug.clone()],
                subject: Subject::Type(t.qualified.clone()),
                sources: vec![t.file.clone()],
            });
        }
    }

    // Ownership: which note documents each symbol.
    let planned_ids: HashSet<String> = notes.iter().map(|n| n.id.clone()).collect();
    let roots: HashMap<&str, &Unit> = repo
        .units
        .list
        .iter()
        .filter_map(|u| Some((u.root.as_deref()?, u)))
        .collect();
    let mut owner: HashMap<String, String> = HashMap::new();
    for s in &facts.symbols {
        // Code in a file an adopted note describes belongs to that note.
        if let Some(adopted) = covered.get(&s.file) {
            owner.insert(s.qualified.clone(), adopted.clone());
            continue;
        }
        let mut current = Some(s);
        let mut found = None;
        while let Some(sym) = current {
            let candidate = match (sym.kind, roots.get(sym.qualified.as_str())) {
                (SymbolKind::Module, Some(unit)) => format!("crate:{}", unit.name),
                (SymbolKind::Module, None) => format!("mod:{}", sym.qualified),
                _ => format!("type:{}", sym.qualified),
            };
            if planned_ids.contains(&candidate) {
                found = Some(candidate);
                break;
            }
            current = sym
                .parent
                .as_deref()
                .and_then(|p| repo.symbols.get(p))
                .copied();
        }
        let id = found.or_else(|| {
            let index = repo.units.index_of(s)?;
            Some(format!("crate:{}", repo.units.list[index].name))
        });
        if let Some(id) = id {
            owner.insert(s.qualified.clone(), id);
        }
    }

    // Architecture flows.
    if repo.units.list.len() >= 2 {
        notes.push(Planned {
            id: "flow:crates".into(),
            dir: ZONE_ARCHITECTURE.into(),
            title: if repo.units.all_rust() {
                "Crate Dependency Flow".into()
            } else {
                "Package Dependency Flow".into()
            },
            kind: NoteKind::Flow,
            tags: vec!["architecture".into(), "flow".into()],
            subject: Subject::CrateFlow,
            // Several units can share one manifest; it is cited once.
            sources: repo
                .units
                .list
                .iter()
                .filter_map(|u| Some(u.manifests.first()?.path.clone()))
                .collect::<BTreeSet<String>>()
                .into_iter()
                .collect(),
        });
    }
    for unit in &repo.units.list {
        let mut flows = 0;
        for entry in &unit.entries {
            let Some(symbol) = repo.symbols.get(entry.symbol.as_str()) else {
                continue;
            };
            let messages = startup_messages(repo, &owner, &entry.symbol);
            if messages.len() < 2 {
                continue;
            }
            // The unit's one obvious entry carries the unit's name; a further
            // script is named after itself.
            let by_unit = format!("flow:startup:{}", unit.name);
            let own = entry.declared && !notes.iter().any(|n| n.id == by_unit);
            let label = match symbol.kind {
                SymbolKind::Module => symbol.name.as_str(),
                _ => {
                    let parent = symbol.parent.as_deref().unwrap_or(&symbol.name);
                    parent.rsplit("::").next().unwrap_or(parent)
                }
            };
            notes.push(Planned {
                id: if own {
                    by_unit
                } else {
                    format!("flow:startup:{}", entry.symbol)
                },
                dir: ZONE_ARCHITECTURE.into(),
                title: if own {
                    format!("{} Startup Flow", unit.package)
                } else {
                    format!("{} Startup Flow", title_case(label))
                },
                kind: NoteKind::Flow,
                tags: vec!["architecture".into(), "flow".into()],
                subject: Subject::StartupFlow {
                    krate: unit.name.clone(),
                    entry: entry.symbol.clone(),
                    messages,
                },
                sources: vec![symbol.file.clone()],
            });
            flows += 1;
            if flows == MAX_FLOWS_PER_UNIT {
                break;
            }
        }
    }

    // Integrations: external crates used widely enough to matter.
    let mut usage: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for u in &facts.external_uses {
        usage
            .entry(u.krate.as_str())
            .or_default()
            .insert(u.file.as_str());
    }
    let code_files = facts
        .files
        .iter()
        .filter(|f| f.language.has_symbol_support())
        .count();
    let threshold = if code_files < 30 { 2 } else { 3 };
    let mut integrations: Vec<(&str, &BTreeSet<&str>)> = usage
        .iter()
        .filter(|(_, files)| files.len() >= threshold)
        .map(|(k, f)| (*k, f))
        .collect();
    integrations.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    integrations.truncate(MAX_INTEGRATIONS);
    integrations.sort_by(|a, b| a.0.cmp(b.0));
    for (krate, files) in integrations {
        notes.push(Planned {
            id: format!("integration:{krate}"),
            dir: ZONE_INTEGRATIONS.into(),
            title: krate.to_string(),
            kind: NoteKind::Integration,
            tags: vec!["integration".into()],
            subject: Subject::Integration(krate.to_string()),
            sources: files.iter().take(10).map(|f| f.to_string()).collect(),
        });
    }

    // Patterns: traits implemented by several types.
    let mut patterns: Vec<(&str, &Vec<&str>)> = repo
        .implementors
        .iter()
        .filter(|(t, impls)| {
            impls.len() >= 2 && repo.symbols.get(*t).is_some_and(|s| !repo.is_test(s))
        })
        .map(|(t, i)| (*t, i))
        .collect();
    patterns.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    patterns.truncate(MAX_PATTERNS);
    patterns.sort_by(|a, b| a.0.cmp(b.0));
    for (tr, _) in patterns {
        let symbol = repo.symbols[tr];
        notes.push(Planned {
            id: format!("pattern:{tr}"),
            dir: ZONE_PATTERNS.into(),
            title: if symbol.kind == SymbolKind::Class {
                format!("Extending {} Pattern", symbol.name)
            } else {
                format!("Implementing {} Pattern", symbol.name)
            },
            kind: NoteKind::Pattern,
            tags: vec!["pattern".into()],
            subject: Subject::TraitPattern(tr.to_string()),
            sources: vec![symbol.file.clone()],
        });
    }

    if !repo.units.list.is_empty() {
        let manifests: HashSet<&str> = repo
            .units
            .list
            .iter()
            .filter(|u| !u.is_rust())
            .flat_map(|u| u.manifests.iter().map(|m| m.path.as_str()))
            .collect();
        let mut sources: Vec<String> = facts
            .files
            .iter()
            .map(|f| f.path.as_str())
            .filter(|p| {
                *p == "Cargo.toml"
                    || p.starts_with(".github/workflows/")
                    || p.rsplit('/')
                        .next()
                        .is_some_and(|n| n.starts_with("Dockerfile"))
                    || *p == "rust-toolchain.toml"
                    || *p == "deny.toml"
                    || manifests.contains(p)
            })
            .map(str::to_string)
            .collect();
        sources.sort();
        notes.push(Planned {
            id: "ops:build".into(),
            dir: ZONE_OPERATIONS.into(),
            title: "Build and Test".into(),
            kind: NoteKind::Operations,
            tags: vec!["operations".into()],
            subject: Subject::Operations,
            sources,
        });
    }

    // Map notes: one per folder that holds notes, plus every ancestor folder.
    let mut dirs: BTreeSet<String> = BTreeSet::new();
    for n in &notes {
        let mut dir = n.dir.as_str();
        loop {
            dirs.insert(dir.to_string());
            match dir.rsplit_once('/') {
                Some((parent, _)) => dir = parent,
                None => break,
            }
        }
    }
    for dir in dirs {
        let zone_tag = match dir.split('/').next().unwrap_or("") {
            ZONE_ARCHITECTURE => "architecture",
            ZONE_CORE => "core",
            ZONE_INTEGRATIONS => "integration",
            ZONE_PATTERNS => "pattern",
            ZONE_OPERATIONS => "operations",
            _ => "notes",
        };
        let mut tags = vec!["moc".to_string(), zone_tag.to_string()];
        if let Some(last) = dir.rsplit('/').next().filter(|_| dir.contains('/')) {
            tags.push(last.to_string());
        }
        notes.push(Planned {
            id: format!("index:{dir}"),
            dir: dir.clone(),
            title: "_index".into(),
            kind: NoteKind::Index,
            tags,
            subject: Subject::Index(dir),
            sources: Vec::new(),
        });
    }
    notes.push(Planned {
        id: "home".into(),
        dir: String::new(),
        title: "_HOME".into(),
        kind: NoteKind::Home,
        tags: vec!["moc".into(), "home".into()],
        subject: Subject::Home,
        sources: Vec::new(),
    });

    Plan { notes, owner }
}

/// Follow calls outward from an entry point and keep those that cross from
/// one note into another.
fn startup_messages(repo: &Repo, owner: &HashMap<String, String>, entry: &str) -> Vec<FlowMessage> {
    let mut messages: Vec<FlowMessage> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::from([entry]);
    let mut queue: VecDeque<(&str, usize)> = VecDeque::from([(entry, 0)]);
    // A script also runs the initialisers of its module-level values.
    if repo
        .symbols
        .get(entry)
        .is_some_and(|s| s.kind == SymbolKind::Module)
    {
        for child in repo.children_of(entry) {
            if matches!(child.kind, SymbolKind::Const | SymbolKind::Static)
                && seen.insert(child.qualified.as_str())
            {
                queue.push_back((child.qualified.as_str(), 0));
            }
        }
    }
    while let Some((current, depth)) = queue.pop_front() {
        let Some(from) = owner.get(current) else {
            continue;
        };
        let mut calls: Vec<&&Edge> = repo
            .outgoing
            .get(current)
            .map(|v| v.iter().filter(|e| e.kind == EdgeKind::Calls).collect())
            .unwrap_or_default();
        calls.sort_by(|a, b| a.dst.cmp(&b.dst));
        for edge in calls {
            let callee = edge.dst.as_str();
            if !seen.insert(callee) {
                continue;
            }
            if let Some(to) = owner.get(callee)
                && to != from
            {
                if messages.len() >= MAX_FLOW_MESSAGES {
                    return messages;
                }
                let name = callee.rsplit("::").next().unwrap_or(callee);
                messages.push(FlowMessage {
                    from: from.clone(),
                    to: to.clone(),
                    label: format!("{name}()"),
                });
            }
            if depth + 1 < MAX_FLOW_DEPTH {
                queue.push_back((callee, depth + 1));
            }
        }
    }
    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_cases_identifiers() {
        assert_eq!(title_case("hash"), "Hash");
        assert_eq!(title_case("note_lifecycle"), "Note Lifecycle");
        assert_eq!(title_case("cw-core"), "Cw Core");
    }

    #[test]
    fn detects_test_paths() {
        assert!(is_test_path("app::tests::helper"));
        assert!(!is_test_path("app::testing_tools"));
    }
}
