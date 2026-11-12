//! Name resolution without a compiler.
//!
//! Turns the per-file facts into graph edges. Resolution is heuristic, so each
//! edge records how it was matched; see [`cw_core::Confidence`].

use std::collections::{BTreeSet, HashMap, HashSet};

use cw_core::{Confidence, Edge, EdgeKind, Language, Symbol, SymbolKind, file_node};

use crate::rust::ParsedFile;

/// A `use` of a crate that is not part of the repository.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExternalUse {
    pub file: String,
    pub krate: String,
}

#[derive(Debug, Default)]
pub struct Resolved {
    pub edges: Vec<Edge>,
    pub external_uses: Vec<ExternalUse>,
}

/// Crates that ship with the toolchain; never reported as integrations.
const TOOLCHAIN_CRATES: [&str; 5] = ["std", "core", "alloc", "proc_macro", "test"];

const MAX_REEXPORT_DEPTH: usize = 6;

/// Method names so common in the standard library that a receiver of unknown
/// type almost never means a repository method of the same name.
const COMMON_METHODS: [&str; 64] = [
    "all",
    "any",
    "as_ref",
    "as_str",
    "build",
    "chain",
    "chars",
    "clear",
    "clone",
    "cloned",
    "cmp",
    "collect",
    "contains",
    "copied",
    "count",
    "default",
    "drain",
    "entry",
    "eq",
    "extend",
    "filter",
    "find",
    "first",
    "flatten",
    "fmt",
    "from",
    "get",
    "hash",
    "insert",
    "into",
    "is_empty",
    "iter",
    "join",
    "keys",
    "last",
    "len",
    "lines",
    "lock",
    "map",
    "max",
    "min",
    "new",
    "next",
    "parse",
    "pop",
    "position",
    "push",
    "read",
    "recv",
    "remove",
    "rev",
    "send",
    "skip",
    "sort",
    "split",
    "sum",
    "take",
    "to_owned",
    "to_string",
    "trim",
    "unwrap",
    "values",
    "write",
    "zip",
];

pub struct Resolver<'a> {
    symbols: HashMap<&'a str, &'a Symbol>,
    by_name: HashMap<&'a str, Vec<&'a Symbol>>,
    /// module -> alias -> absolute path
    imports: HashMap<String, HashMap<String, Vec<String>>>,
    /// module -> absolute paths of glob-imported modules
    globs: HashMap<String, Vec<Vec<String>>>,
    crate_names: HashSet<&'a str>,
}

impl<'a> Resolver<'a> {
    pub fn new(
        symbols: &'a [Symbol],
        files: &[(String, ParsedFile)],
        crate_names: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        let mut map = HashMap::new();
        let mut by_name: HashMap<&str, Vec<&Symbol>> = HashMap::new();
        for s in symbols {
            map.entry(s.qualified.as_str()).or_insert(s);
            by_name.entry(s.name.as_str()).or_default().push(s);
        }
        let mut this = Self {
            symbols: map,
            by_name,
            imports: HashMap::new(),
            globs: HashMap::new(),
            crate_names: crate_names.into_iter().collect(),
        };
        for (_, parsed) in files {
            for u in &parsed.uses {
                let Some(mut abs) = this.absolutize(&u.path, &u.scope, None) else {
                    continue;
                };
                // `use text::shout` inside `my_util` names a local module, not
                // an external crate, when `my_util::text` exists.
                let local = format!("{}::{}", u.scope, abs[0]);
                if !this.crate_names.contains(abs[0].as_str())
                    && this.symbols.contains_key(local.as_str())
                {
                    let mut full: Vec<String> = u.scope.split("::").map(str::to_string).collect();
                    full.append(&mut abs);
                    abs = full;
                }
                if u.glob {
                    this.globs.entry(u.scope.clone()).or_default().push(abs);
                } else {
                    this.imports
                        .entry(u.scope.clone())
                        .or_default()
                        .insert(u.alias.clone(), abs);
                }
            }
        }
        this
    }

    /// Rewrite `crate`, `self`, `super` and `Self` prefixes to absolute form.
    /// Paths with no such prefix are returned unchanged (still relative).
    fn absolutize(
        &self,
        path: &[String],
        module: &str,
        self_type: Option<&str>,
    ) -> Option<Vec<String>> {
        let first = path.first()?;
        let split = |s: &str| s.split("::").map(str::to_string).collect::<Vec<_>>();
        let mut base: Vec<String> = match first.as_str() {
            "crate" => vec![module.split("::").next()?.to_string()],
            "self" => split(module),
            "super" => {
                let mut m = split(module);
                m.pop();
                m
            }
            "Self" => split(self_type?),
            _ => return Some(path.to_vec()),
        };
        let mut rest = &path[1..];
        while rest.first().is_some_and(|s| s == "super") {
            base.pop();
            rest = &rest[1..];
        }
        base.extend(rest.iter().cloned());
        Some(base)
    }

    /// Resolve an absolute path, following re-exports through `use` aliases
    /// and glob imports of the modules along the way.
    fn resolve_abs(&self, segs: &[String], depth: usize) -> Option<&'a Symbol> {
        if let Some(s) = self.symbols.get(segs.join("::").as_str()) {
            return Some(s);
        }
        if depth >= MAX_REEXPORT_DEPTH {
            return None;
        }
        for i in (1..segs.len()).rev() {
            let module = segs[..i].join("::");
            if let Some(target) = self.imports.get(&module).and_then(|m| m.get(&segs[i])) {
                let mut next = target.clone();
                next.extend(segs[i + 1..].iter().cloned());
                if next != segs
                    && let Some(s) = self.resolve_abs(&next, depth + 1)
                {
                    return Some(s);
                }
            }
            if let Some(globs) = self.globs.get(&module) {
                for g in globs {
                    let mut next = g.clone();
                    next.extend(segs[i..].iter().cloned());
                    if next != segs
                        && let Some(s) = self.resolve_abs(&next, depth + 1)
                    {
                        return Some(s);
                    }
                }
            }
        }
        None
    }

    /// Resolve a path as written at a use site inside `module`.
    pub fn lookup(
        &self,
        path: &[String],
        module: &str,
        self_type: Option<&str>,
    ) -> Option<&'a Symbol> {
        let abs = self.absolutize(path, module, self_type)?;
        let explicit = matches!(path[0].as_str(), "crate" | "self" | "super" | "Self");
        if explicit {
            return self.resolve_abs(&abs, 0);
        }
        let mut candidates: Vec<Vec<String>> = Vec::new();
        if let Some(target) = self.imports.get(module).and_then(|m| m.get(&path[0])) {
            let mut c = target.clone();
            c.extend(path[1..].iter().cloned());
            candidates.push(c);
        }
        let mut local: Vec<String> = module.split("::").map(str::to_string).collect();
        local.extend(path.iter().cloned());
        candidates.push(local);
        if self.crate_names.contains(path[0].as_str()) {
            candidates.push(path.to_vec());
        }
        if let Some(globs) = self.globs.get(module) {
            for g in globs {
                let mut c = g.clone();
                c.extend(path.iter().cloned());
                candidates.push(c);
            }
        }
        candidates.iter().find_map(|c| self.resolve_abs(c, 0))
    }

    /// [`Self::lookup`], then the scopes enclosing `from`: an item declared
    /// inside a function is visible to the code around it.
    pub fn lookup_from(
        &self,
        path: &[String],
        module: &str,
        self_type: Option<&str>,
        from: &str,
    ) -> Option<&'a Symbol> {
        if let Some(found) = self.lookup(path, module, self_type) {
            return Some(found);
        }
        if matches!(path[0].as_str(), "crate" | "self" | "super" | "Self") {
            return None;
        }
        let mut scope: Vec<&str> = from.split("::").collect();
        let floor = module.split("::").count();
        while scope.len() > floor {
            // A class body is not a scope for the code inside its methods:
            // a bare name there never means a sibling member.
            let is_class = self
                .symbols
                .get(scope.join("::").as_str())
                .is_some_and(|s| matches!(s.kind, SymbolKind::Class | SymbolKind::Interface));
            if is_class {
                scope.pop();
                continue;
            }
            let mut candidate: Vec<String> = scope.iter().map(|s| s.to_string()).collect();
            candidate.extend(path.iter().cloned());
            if let Some(found) = self.resolve_abs(&candidate, 0) {
                return Some(found);
            }
            scope.pop();
        }
        None
    }

    /// Fall back to matching by bare name when path resolution fails.
    fn by_bare_name(
        &self,
        name: &str,
        module: &str,
        language: Language,
        accept: impl Fn(SymbolKind) -> bool,
    ) -> Option<(&'a Symbol, Confidence)> {
        // A name never matches a symbol written in another language.
        let all: Vec<&Symbol> = self
            .by_name
            .get(name)?
            .iter()
            .copied()
            .filter(|s| accept(s.kind) && Language::from_path(&s.file) == language)
            .collect();
        if all.len() == 1 {
            return Some((all[0], Confidence::Medium));
        }
        let krate = module.split("::").next().unwrap_or("");
        let prefix = format!("{krate}::");
        let same: Vec<&Symbol> = all
            .into_iter()
            .filter(|s| s.qualified.starts_with(&prefix))
            .collect();
        (same.len() == 1).then(|| (same[0], Confidence::Low))
    }

    /// Build every edge for the repository.
    pub fn resolve(&self, files: &[(String, ParsedFile)]) -> Resolved {
        let mut edges: BTreeSet<(String, String, EdgeKind, Confidence)> = BTreeSet::new();
        let mut external: BTreeSet<ExternalUse> = BTreeSet::new();
        let mut add = |src: String, dst: String, kind: EdgeKind, conf: Confidence| {
            if src != dst {
                edges.insert((src, dst, kind, conf));
            }
        };
        let callable = |k: SymbolKind| matches!(k, SymbolKind::Function | SymbolKind::Method);

        for (file, parsed) in files {
            let language = Language::from_path(file);
            for u in &parsed.uses {
                match self.lookup(&u.path, &u.scope, None) {
                    Some(target) => add(
                        file_node(file),
                        file_node(&target.file),
                        EdgeKind::Imports,
                        Confidence::High,
                    ),
                    None => {
                        let first = u.path[0].as_str();
                        let relative = !matches!(first, "crate" | "self" | "super");
                        let local_module = self
                            .symbols
                            .contains_key(format!("{}::{first}", u.scope).as_str());
                        if relative
                            && !local_module
                            && !self.crate_names.contains(first)
                            && !TOOLCHAIN_CRATES.contains(&first)
                        {
                            external.insert(ExternalUse {
                                file: file.clone(),
                                krate: first.to_string(),
                            });
                        }
                    }
                }
            }

            for m in &parsed.mods {
                let child = format!("{}::{}", m.parent, m.name);
                if self.symbols.contains_key(child.as_str()) {
                    add(
                        m.parent.clone(),
                        child,
                        EdgeKind::Declares,
                        Confidence::High,
                    );
                }
            }

            for i in &parsed.impls {
                let Some(trait_path) = &i.trait_path else {
                    continue;
                };
                let ty = self.lookup(&i.type_path, &i.module, None);
                let tr = self.lookup(trait_path, &i.module, None);
                if let (Some(ty), Some(tr)) = (ty, tr) {
                    add(
                        ty.qualified.clone(),
                        tr.qualified.clone(),
                        EdgeKind::Implements,
                        Confidence::High,
                    );
                }
            }

            for c in &parsed.calls {
                let self_type = c.self_type.as_deref();
                let name = c.path.last().map(String::as_str).unwrap_or("");
                let hit = if c.method {
                    self_type
                        .and_then(|t| self.symbols.get(format!("{t}::{name}").as_str()))
                        .map(|s| (*s, Confidence::Medium))
                        .or_else(|| {
                            if COMMON_METHODS.contains(&name) {
                                return None;
                            }
                            self.by_bare_name(name, &c.module, language, |k| {
                                k == SymbolKind::Method
                            })
                            .map(|(s, _)| (s, Confidence::Low))
                        })
                } else {
                    // A path that does not resolve names something outside the
                    // repository (std, a dependency, a closure); never guess.
                    self.lookup_from(&c.path, &c.module, self_type, &c.caller)
                        .filter(|s| callable(s.kind))
                        .map(|s| (s, Confidence::High))
                };
                if let Some((target, conf)) = hit {
                    add(
                        c.caller.clone(),
                        target.qualified.clone(),
                        EdgeKind::Calls,
                        conf,
                    );
                }
            }

            for v in &parsed.value_refs {
                let Some(target) =
                    self.lookup_from(&v.path, &v.module, v.self_type.as_deref(), &v.from)
                else {
                    continue;
                };
                let kind = match target.kind {
                    k if callable(k) && v.call => EdgeKind::Calls,
                    // A function named without being called is passed as a
                    // value; a constant or static is read.
                    k if callable(k) => EdgeKind::References,
                    SymbolKind::Const | SymbolKind::Static => EdgeKind::References,
                    k if k.is_type() => EdgeKind::References,
                    _ => continue,
                };
                add(
                    v.from.clone(),
                    target.qualified.clone(),
                    kind,
                    Confidence::High,
                );
            }

            for r in &parsed.type_refs {
                let hit = self
                    .lookup_from(&r.path, &r.module, None, &r.from)
                    .filter(|s| s.kind.is_type() || s.kind == SymbolKind::TypeAlias)
                    .map(|s| (s, Confidence::High));
                if let Some((target, conf)) = hit {
                    add(
                        r.from.clone(),
                        target.qualified.clone(),
                        EdgeKind::References,
                        conf,
                    );
                }
            }
        }

        Resolved {
            edges: edges
                .into_iter()
                .map(|(src, dst, kind, confidence)| Edge {
                    src,
                    dst,
                    kind,
                    confidence,
                })
                .collect(),
            external_uses: external.into_iter().collect(),
        }
    }
}
