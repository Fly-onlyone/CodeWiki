//! Name resolution without a compiler.
//!
//! Turns the per-file facts into graph edges. Resolution is heuristic, so each
//! edge records how it was matched; see [`cw_core::Confidence`].

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::rc::Rc;

use cw_core::{Confidence, Edge, EdgeKind, Language, Symbol, SymbolKind, Visibility, file_node};

use crate::kotlin::EXTENSION_ATTR;
use crate::rust::ParsedFile;
use crate::workspace::CrateInfo;

/// A `use` of a crate that is not part of the repository.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExternalUse {
    pub file: String,
    pub krate: String,
    /// What the file imports from the crate, as its language writes it
    /// (`extract::Path`, `adapters.HTTPAdapter`, `*`), in name order. Empty
    /// when the file names only the crate itself. A module is left out
    /// when a name inside it is listed.
    pub names: Vec<String>,
}

/// `names` without each one that only leads to another: `de` beside
/// `de::Visitor`.
fn leaves(names: BTreeSet<String>) -> Vec<String> {
    names
        .iter()
        .filter(|name| {
            !names.iter().any(|other| {
                other
                    .strip_prefix(name.as_str())
                    .is_some_and(|rest| rest.starts_with("::") || rest.starts_with('.'))
            })
        })
        .cloned()
        .collect()
}

#[derive(Debug, Default)]
pub struct Resolved {
    pub edges: Vec<Edge>,
    pub external_uses: Vec<ExternalUse>,
}

/// Crates that ship with the toolchain; never reported as integrations.
const TOOLCHAIN_CRATES: [&str; 5] = ["std", "core", "alloc", "proc_macro", "test"];

const MAX_REEXPORT_DEPTH: usize = 6;

/// The module a Rust symbol is declared in: its qualified name without its
/// own name, and without the type for a method (`module::Type::name`).
fn home_module(symbol: &Symbol) -> &str {
    let levels = if symbol.kind == SymbolKind::Method {
        2
    } else {
        1
    };
    let mut end = symbol.qualified.len();
    for _ in 0..levels {
        end = symbol.qualified[..end].rfind("::").unwrap_or(0);
    }
    &symbol.qualified[..end]
}

/// `module` is `outer` or nested inside it.
fn within(module: &str, outer: &str) -> bool {
    module
        .strip_prefix(outer)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with("::"))
}

/// A file of an integration test, a bench or an example: a crate of its
/// own, whose items the library cannot name.
fn is_separate_target(file: &str) -> bool {
    let dirs: Vec<&str> = file.split('/').collect();
    let dirs = &dirs[..dirs.len().saturating_sub(1)];
    !dirs.contains(&"src")
        && dirs
            .iter()
            .any(|d| matches!(*d, "tests" | "benches" | "examples"))
}

/// Whether Rust code in `module` can name `symbol`: a private item in its
/// own module and the ones inside it, a `pub(crate)` item in its crate, and
/// an item of a test, bench or example only from that file.
fn visible_from(symbol: &Symbol, module: &str) -> bool {
    let home = home_module(symbol);
    if is_separate_target(&symbol.file) {
        return within(module, home);
    }
    match symbol.visibility {
        Visibility::Public => true,
        Visibility::Crate => module.split("::").next() == home.split("::").next(),
        Visibility::Private => within(module, home),
    }
}

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

/// The same for C#, whose method names are capitalized. The lookup is by
/// exact name, so `Add` and `add` stay apart.
const COMMON_CSHARP_METHODS: [&str; 52] = [
    "Add",
    "AddRange",
    "All",
    "Any",
    "Append",
    "AsEnumerable",
    "Clear",
    "Clone",
    "Close",
    "CompareTo",
    "Contains",
    "ContainsKey",
    "Copy",
    "Count",
    "Dispose",
    "DisposeAsync",
    "Equals",
    "Exists",
    "First",
    "FirstOrDefault",
    "ForEach",
    "Get",
    "GetHashCode",
    "GetType",
    "Insert",
    "Invoke",
    "Join",
    "Last",
    "Load",
    "Max",
    "Min",
    "Parse",
    "Read",
    "ReadAsync",
    "Remove",
    "Reverse",
    "Select",
    "Send",
    "Set",
    "Single",
    "Skip",
    "Sort",
    "Split",
    "Start",
    "Stop",
    "Sum",
    "Take",
    "ToArray",
    "ToList",
    "ToString",
    "TryGetValue",
    "Where",
];

/// The same for Go, whose exported names are capitalized: methods of the
/// standard library that nearly every receiver of unknown type has.
const COMMON_GO_METHODS: [&str; 52] = [
    "Add",
    "Append",
    "Clear",
    "Clone",
    "Close",
    "Contains",
    "Copy",
    "Delete",
    "Do",
    "Done",
    "Err",
    "Error",
    "Exec",
    "Flush",
    "Format",
    "Get",
    "Handle",
    "Info",
    "Init",
    "Join",
    "Len",
    "Less",
    "Load",
    "Lock",
    "Log",
    "Marshal",
    "Next",
    "Open",
    "Parse",
    "Print",
    "Printf",
    "Put",
    "Query",
    "Read",
    "Remove",
    "Reset",
    "Run",
    "Scan",
    "Send",
    "Set",
    "Start",
    "Stop",
    "Store",
    "String",
    "Swap",
    "Unlock",
    "Unmarshal",
    "Update",
    "Wait",
    "Write",
    "WriteString",
    "Value",
];

/// The same for Kotlin: the scope functions, the collection and string
/// extensions and the infix functions of its standard library, which any
/// receiver of unknown type may be given. They are extension functions of
/// the platform, so a repository function of the same name is no answer.
const COMMON_KOTLIN_METHODS: [&str; 81] = [
    "also",
    "any",
    "apply",
    "associate",
    "associateBy",
    "contains",
    "count",
    "downTo",
    "drop",
    "emptyList",
    "filter",
    "filterIsInstance",
    "filterNot",
    "filterNotNull",
    "first",
    "firstOrNull",
    "flatMap",
    "forEach",
    "forEachIndexed",
    "getOrDefault",
    "getOrNull",
    "getOrPut",
    "groupBy",
    "isEmpty",
    "isNotEmpty",
    "isNullOrBlank",
    "isNullOrEmpty",
    "joinToString",
    "last",
    "lastOrNull",
    "let",
    "listOf",
    "map",
    "mapNotNull",
    "mapOf",
    "mapValues",
    "max",
    "maxOrNull",
    "min",
    "minOrNull",
    "none",
    "onEach",
    "orEmpty",
    "plus",
    "reversed",
    "run",
    "sorted",
    "sortedBy",
    "sortedByDescending",
    "sortedWith",
    "split",
    "step",
    "substring",
    "sumOf",
    "take",
    "takeIf",
    "takeUnless",
    "to",
    "toInt",
    "toList",
    "toLong",
    "toMap",
    "toMutableList",
    "toMutableMap",
    "toSet",
    "toString",
    "trim",
    "trimIndent",
    "until",
    "use",
    "with",
    "withIndex",
    "zip",
    "launch",
    "async",
    "emit",
    "collect",
    "lazy",
    "require",
    "check",
    "error",
];

/// The same for C and C++: the calls of the standard library, the C library
/// and the usual containers that a receiver of unknown type nearly always
/// means.
const COMMON_C_METHODS: &[&str] = &[
    "abort",
    "append",
    "assign",
    "assert",
    "at",
    "atoi",
    "back",
    "begin",
    "c_str",
    "cbegin",
    "cend",
    "clear",
    "close",
    "count",
    "data",
    "detach",
    "emplace",
    "emplace_back",
    "empty",
    "end",
    "eof",
    "erase",
    "exit",
    "fail",
    "fclose",
    "fflush",
    "fgets",
    "find",
    "first",
    "flush",
    "fopen",
    "fprintf",
    "fputs",
    "fread",
    "free",
    "front",
    "fwrite",
    "get",
    "good",
    "has_value",
    "init",
    "insert",
    "join",
    "length",
    "load",
    "lock",
    "malloc",
    "memcmp",
    "memcpy",
    "memmove",
    "memset",
    "notify_all",
    "notify_one",
    "open",
    "pop",
    "pop_back",
    "pop_front",
    "print",
    "printf",
    "push",
    "push_back",
    "push_front",
    "puts",
    "rbegin",
    "read",
    "realloc",
    "release",
    "rend",
    "reserve",
    "reset",
    "resize",
    "run",
    "second",
    "set",
    "size",
    "snprintf",
    "sprintf",
    "start",
    "stop",
    "store",
    "str",
    "strcat",
    "strcmp",
    "strcpy",
    "strlen",
    "strncmp",
    "strncpy",
    "substr",
    "swap",
    "top",
    "unlock",
    "update",
    "value",
    "value_or",
    "wait",
    "what",
    "write",
];

/// What a name stands for at the place it is written, in C and C++ code.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Want {
    /// An `#include`: a module.
    Module,
    Type,
    Call,
    Value,
}

fn same_language(a: Language, b: Language) -> bool {
    a == b || (a.is_c_family() && b.is_c_family())
}

pub struct Resolver<'a> {
    symbols: HashMap<&'a str, &'a Symbol>,
    by_name: HashMap<&'a str, Vec<&'a Symbol>>,
    /// module -> alias -> absolute path
    imports: HashMap<String, HashMap<String, Vec<String>>>,
    /// module -> absolute paths of glob-imported modules
    globs: HashMap<String, Vec<Vec<String>>>,
    crate_names: HashSet<&'a str>,
    /// Namespaces of C# code. Their `using` lines are no re-exports, so a
    /// name is never followed through them while a path is resolved.
    csharp_modules: HashSet<String>,
    /// Packages of Go code. An import there binds a name in one file and
    /// re-exports nothing, so a name is never followed through one.
    go_modules: HashSet<String>,
    /// Packages that hold Kotlin code. As in Go, an import there binds a
    /// name in one file and re-exports nothing; Kotlin files also import
    /// with `*` often, which would make the search through re-exports grow
    /// with every glob of every package on the way.
    kotlin_modules: HashSet<String>,
    /// crate -> the crates its manifest lets its code name, itself excluded
    crate_deps: HashMap<&'a str, HashSet<&'a str>>,
    /// The modules of C and C++ code: a header with its source. A name there
    /// is found in the module, in what it includes, and in what those
    /// include, and nowhere else (see `lookup_c`).
    c_modules: HashSet<String>,
    /// module -> the modules its files include directly.
    c_includes: HashMap<String, Vec<String>>,
    /// Every namespace some symbol was declared in, by segment.
    c_namespaces: HashSet<String>,
    /// module -> the modules it sees and how far (0 itself, 1 included
    /// directly, 2 and more through other headers). Worked out when first
    /// asked.
    c_seen: RefCell<HashMap<String, Rc<HashMap<String, u8>>>>,
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
            csharp_modules: HashSet::new(),
            go_modules: symbols
                .iter()
                .filter(|s| {
                    s.kind == SymbolKind::Module && Language::from_path(&s.file) == Language::Go
                })
                .map(|s| s.qualified.clone())
                .collect(),
            // The module symbol stands for a package in the file that came
            // first, which may be a Java file, so look at every file.
            kotlin_modules: files
                .iter()
                .filter(|(path, _)| Language::from_path(path) == Language::Kotlin)
                .flat_map(|(_, parsed)| &parsed.symbols)
                .filter(|s| s.kind == SymbolKind::Module)
                .map(|s| s.qualified.clone())
                .collect(),
            crate_deps: HashMap::new(),
            c_modules: HashSet::new(),
            c_includes: HashMap::new(),
            c_namespaces: HashSet::new(),
            c_seen: RefCell::new(HashMap::new()),
        };
        this.see_c_modules(symbols, files);
        this.see_enclosing_namespaces();
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

    /// Find the modules of C and C++ code, what each includes, and the
    /// namespaces that were declared.
    fn see_c_modules(&mut self, symbols: &[Symbol], files: &[(String, ParsedFile)]) {
        for s in symbols {
            if Language::from_path(&s.file).is_c_family() {
                if s.kind == SymbolKind::Module {
                    self.c_modules.insert(s.qualified.clone());
                }
                for attr in &s.attrs {
                    if let Some(namespace) = attr.strip_prefix(crate::c::NAMESPACE_ATTR) {
                        self.c_namespaces
                            .extend(namespace.split("::").map(str::to_string));
                    }
                }
            }
        }
        if self.c_modules.is_empty() {
            return;
        }
        for (_, parsed) in files {
            for u in &parsed.uses {
                if u.glob && self.c_modules.contains(&u.scope) {
                    let included = u.path.join("::");
                    if included != u.scope && self.c_modules.contains(&included) {
                        let list = self.c_includes.entry(u.scope.clone()).or_default();
                        if !list.contains(&included) {
                            list.push(included);
                        }
                    }
                }
            }
        }
    }

    /// The modules `module` sees, nearest first: itself, what its files
    /// include, and what those include. Transitive, as the preprocessor is.
    fn c_seen(&self, module: &str) -> Rc<HashMap<String, u8>> {
        if let Some(found) = self.c_seen.borrow().get(module) {
            return Rc::clone(found);
        }
        let mut seen: HashMap<String, u8> = HashMap::new();
        seen.insert(module.to_string(), 0);
        let mut frontier = vec![module.to_string()];
        let mut depth = 0u8;
        while !frontier.is_empty() && depth < 8 {
            depth = depth.saturating_add(1);
            let mut next = Vec::new();
            for current in frontier {
                for included in self.c_includes.get(&current).into_iter().flatten() {
                    if !seen.contains_key(included) {
                        seen.insert(included.clone(), depth.min(2));
                        next.push(included.clone());
                    }
                }
            }
            frontier = next;
        }
        let seen = Rc::new(seen);
        self.c_seen
            .borrow_mut()
            .insert(module.to_string(), Rc::clone(&seen));
        seen
    }

    /// The module a C or C++ symbol is declared in.
    fn c_home<'s>(&self, symbol: &'s Symbol) -> Option<&'s str> {
        let mut end = symbol.qualified.len();
        loop {
            let prefix = &symbol.qualified[..end];
            if self.c_modules.contains(prefix) {
                return Some(prefix);
            }
            end = prefix.rfind("::")?;
        }
    }

    /// Whether code in `module` can name `symbol`: it is in the module or in
    /// a header the module includes, and not private to another file.
    fn c_sees(&self, module: &str, symbol: &Symbol) -> bool {
        let Some(home) = self.c_home(symbol) else {
            return false;
        };
        if home == module {
            return true;
        }
        symbol.visibility == Visibility::Public && self.c_seen(module).contains_key(home)
    }

    /// Resolve a name written in a C or C++ module. The enclosing scopes of
    /// `from` come first (the class, then the module), then whatever the
    /// module sees, the nearest header first. A name that two headers at the
    /// same distance both declare names nothing.
    fn lookup_c(
        &self,
        path: &[String],
        module: &str,
        from: Option<&str>,
        want: Want,
    ) -> Option<&'a Symbol> {
        if want == Want::Module {
            return self
                .symbols
                .get(path.join("::").as_str())
                .filter(|s| s.kind == SymbolKind::Module)
                .copied();
        }
        let mut path = path;
        // `ns::f` names `f`: namespaces are no part of a symbol's name, but
        // `std::` is nobody's here.
        while path.len() > 1 && self.c_namespaces.contains(&path[0]) {
            path = &path[1..];
        }
        let name = path.last()?;
        let accepts = |s: &Symbol| match want {
            Want::Type => s.kind.is_type() || s.kind == SymbolKind::TypeAlias,
            Want::Call => {
                matches!(
                    s.kind,
                    SymbolKind::Function | SymbolKind::Method | SymbolKind::Macro
                ) || s.kind.is_type()
            }
            Want::Value => !matches!(s.kind, SymbolKind::Module),
            Want::Module => false,
        };
        let tail = path.join("::");
        if let Some(from) = from {
            let floor = module.split("::").count();
            let mut scope: Vec<&str> = from.split("::").collect();
            scope.pop();
            while scope.len() >= floor {
                let candidate = format!("{}::{tail}", scope.join("::"));
                if let Some(found) = self.symbols.get(candidate.as_str())
                    && accepts(found)
                {
                    return Some(found);
                }
                scope.pop();
            }
        }
        let suffix = format!("::{tail}");
        let seen = self.c_seen(module);
        let mut best: Option<(u8, &'a Symbol)> = None;
        let mut tie = false;
        for symbol in self.by_name.get(name.as_str())? {
            if !Language::from_path(&symbol.file).is_c_family()
                || !accepts(symbol)
                || !symbol.qualified.ends_with(&suffix)
            {
                continue;
            }
            // A bare name is a method only inside its own class.
            if path.len() == 1 && symbol.kind == SymbolKind::Method {
                continue;
            }
            let Some(home) = self.c_home(symbol) else {
                continue;
            };
            let Some(&depth) = seen.get(home) else {
                continue;
            };
            if home != module && symbol.visibility != Visibility::Public {
                continue;
            }
            match best {
                Some((d, b)) if d < depth || (d == depth && b.qualified == symbol.qualified) => {}
                Some((d, _)) if d == depth => tie = true,
                _ => {
                    best = Some((depth, symbol));
                    tie = false;
                }
            }
        }
        best.filter(|_| !tie).map(|(_, s)| s)
    }

    /// The function a call of `symbol` runs: a constructor for a type, the
    /// function or macro itself.
    fn c_callee(&self, symbol: &'a Symbol) -> Option<&'a Symbol> {
        if symbol.kind.is_type() {
            return self
                .symbols
                .get(format!("{}::{}", symbol.qualified, symbol.name).as_str())
                .copied()
                .filter(|s| s.kind == SymbolKind::Method);
        }
        Some(symbol)
    }

    /// C# code sees the types of the namespaces around its own without a
    /// `using`: in `A.B.C`, the types of `A.B` and `A`. Each namespace of a
    /// C# file opens its ancestors as glob imports, nearest first. These are
    /// no import of any file, so they give no import edge.
    fn see_enclosing_namespaces(&mut self) {
        let mut modules: Vec<&str> = self
            .symbols
            .values()
            .filter(|s| {
                s.kind == SymbolKind::Module && Language::from_path(&s.file) == Language::CSharp
            })
            .map(|s| s.qualified.as_str())
            .collect();
        modules.sort_unstable();
        self.csharp_modules = modules.iter().map(|m| m.to_string()).collect();
        for module in modules {
            let segments: Vec<&str> = module.split("::").collect();
            let ancestors: Vec<Vec<String>> = (1..segments.len())
                .rev()
                .map(|k| segments[..k].iter().map(|s| s.to_string()).collect())
                .collect();
            if !ancestors.is_empty() {
                self.globs
                    .entry(module.to_string())
                    .or_default()
                    .extend(ancestors);
            }
        }
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
            if self.csharp_modules.contains(&module)
                || self.go_modules.contains(&module)
                || self.kotlin_modules.contains(&module)
            {
                continue;
            }
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
        if self.c_modules.contains(module) {
            return self
                .lookup_c(path, module, None, Want::Module)
                .or_else(|| self.lookup_c(path, module, None, Want::Value));
        }
        let abs = self.absolutize(path, module, self_type)?;
        let explicit = matches!(path[0].as_str(), "crate" | "self" | "super" | "Self");
        if explicit {
            return self.resolve_abs(&abs, 0);
        }
        if self.csharp_modules.contains(module) {
            return self.lookup_csharp(path, module);
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

    /// [`Self::lookup`] for code in a C# namespace. A name there is a symbol
    /// at the place its alias, its namespace, a namespace around it or a
    /// `using` gives, so each candidate is one direct lookup.
    fn lookup_csharp(&self, path: &[String], module: &str) -> Option<&'a Symbol> {
        let tail = path.join("::");
        let direct = |prefix: &str| {
            let name = if prefix.is_empty() {
                tail.clone()
            } else {
                format!("{prefix}::{tail}")
            };
            self.symbols.get(name.as_str()).copied()
        };
        if let Some(target) = self.imports.get(module).and_then(|m| m.get(&path[0])) {
            let mut c = target.clone();
            c.extend(path[1..].iter().cloned());
            if let Some(found) = self.symbols.get(c.join("::").as_str()) {
                return Some(found);
            }
        }
        direct(module)
            .or_else(|| {
                self.crate_names
                    .contains(path[0].as_str())
                    .then(|| direct(""))
                    .flatten()
            })
            .or_else(|| {
                self.globs
                    .get(module)?
                    .iter()
                    .find_map(|g| direct(&g.join("::")))
            })
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
        if self.c_modules.contains(module) {
            return self.lookup_c(path, module, Some(from), Want::Value);
        }
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
            // a bare name there never means a sibling member. In C# it does
            // for the parts of a partial type, which the parser of one file
            // cannot see, but never for a constructor, which is named like
            // its class.
            let class = self.symbols.get(scope.join("::").as_str()).filter(|s| {
                matches!(s.kind, SymbolKind::Class | SymbolKind::Interface)
                    // A Go type is no scope either: a bare name in a method is
                    // a function of the package, never a sibling method.
                    || (s.kind.is_type() && Language::from_path(&s.file) == Language::Go)
            });
            let csharp = class.is_some_and(|s| Language::from_path(&s.file) == Language::CSharp);
            if class.is_some() && !csharp {
                scope.pop();
                continue;
            }
            let mut candidate: Vec<String> = scope.iter().map(|s| s.to_string()).collect();
            candidate.extend(path.iter().cloned());
            let class_name = scope.last().copied();
            if let Some(found) = self.resolve_abs(&candidate, 0)
                && !(csharp
                    && found.kind == SymbolKind::Method
                    && Some(found.name.as_str()) == class_name)
            {
                return Some(found);
            }
            scope.pop();
        }
        None
    }

    /// Tell the resolver which crates each Cargo package depends on, so that a
    /// name never matches a method in a crate the caller cannot reach.
    pub fn with_crates(mut self, crates: &'a [CrateInfo]) -> Self {
        self.crate_deps = crates
            .iter()
            .map(|c| {
                let deps = c.code_deps.iter().map(String::as_str).collect();
                (c.name.as_str(), deps)
            })
            .collect();
        self
    }

    /// Whether code in `module` can reach the crate `symbol` is declared in:
    /// its own crate, or one its manifest depends on. A crate no manifest
    /// describes reaches every crate.
    fn reaches(&self, module: &str, symbol: &Symbol) -> bool {
        let from = module.split("::").next().unwrap_or("");
        let to = symbol.qualified.split("::").next().unwrap_or("");
        from == to
            || self
                .crate_deps
                .get(from)
                .is_none_or(|deps| deps.contains(to))
    }

    /// Fall back to matching by bare name when path resolution fails.
    ///
    /// A name that only one method has is taken to mean it, unless the caller
    /// cannot see that method: a call such as `request.header(..)` on a type
    /// from a dependency must not bind to a private `header` elsewhere, and
    /// gpui's `div().size(..)` must not bind to a `size` in a crate the
    /// caller's crate does not depend on. When
    /// several methods share the name, a Rust caller gets the one declared in
    /// its own module, if there is just one. With `private_here`, the name is
    /// common enough that only a private method of the caller's own module is
    /// taken.
    fn by_bare_name(
        &self,
        name: &str,
        module: &str,
        language: Language,
        accept: impl Fn(&Symbol) -> bool,
        private_here: bool,
    ) -> Option<(&'a Symbol, Confidence)> {
        let rust = language == Language::Rust;
        let visible = |s: &Symbol| !rust || (visible_from(s, module) && self.reaches(module, s));
        let only = |found: Vec<&'a Symbol>| (found.len() == 1).then(|| found[0]);
        // A name never matches a symbol written in another language, Java
        // and Kotlin included: a receiver of unknown type calls `readLine`
        // or `add` of the platform far more often than a method of the other
        // language. Paths and imports do cross between the two.
        let all: Vec<&Symbol> = self
            .by_name
            .get(name)?
            .iter()
            .copied()
            .filter(|s| {
                accept(s)
                    && same_language(Language::from_path(&s.file), language)
                    && (!language.is_c_family() || self.c_sees(module, s))
            })
            .collect();
        let here = |found: &[&'a Symbol], private: bool| {
            only(
                found
                    .iter()
                    .copied()
                    .filter(|s| rust && home_module(s) == module && visible(s))
                    .filter(|s| !private || s.visibility == Visibility::Private)
                    .collect(),
            )
        };
        if private_here {
            return here(&all, true).map(|s| (s, Confidence::Low));
        }
        if all.len() == 1 {
            return visible(all[0]).then(|| (all[0], Confidence::Medium));
        }
        let krate = module.split("::").next().unwrap_or("");
        let prefix = format!("{krate}::");
        let same: Vec<&Symbol> = all
            .into_iter()
            .filter(|s| s.qualified.starts_with(&prefix))
            .collect();
        if same.len() == 1 {
            return visible(same[0]).then(|| (same[0], Confidence::Low));
        }
        // `page.id()` in `doc_view` means `doc_view::Page::id` sooner than a
        // `tray::Command::id`.
        here(&same, false).map(|s| (s, Confidence::Low))
    }

    /// Build every edge for the repository.
    pub fn resolve(&self, files: &[(String, ParsedFile)]) -> Resolved {
        // Each edge once, with the place of the first call it stands for.
        let mut edges: BTreeMap<(String, String, EdgeKind, Confidence), u32> = BTreeMap::new();
        let mut external: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
        // `at` is where a call stands in its file, and 0 for what is no call.
        let mut add = |src: String, dst: String, kind: EdgeKind, conf: Confidence, at: u32| {
            if src != dst {
                edges
                    .entry((src, dst, kind, conf))
                    .and_modify(|first| *first = (*first).min(at))
                    .or_insert(at);
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
                        0,
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
                            let taken = if language == Language::Rust {
                                let mut taken = u.path[1..].join("::");
                                if u.glob {
                                    if !taken.is_empty() {
                                        taken.push_str("::");
                                    }
                                    taken.push('*');
                                }
                                taken
                            } else {
                                u.external.clone()
                            };
                            let names = external
                                .entry((file.clone(), first.to_string()))
                                .or_default();
                            if !taken.is_empty() {
                                names.insert(taken);
                            }
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
                        0,
                    );
                }
            }

            for i in &parsed.impls {
                let Some(trait_path) = &i.trait_path else {
                    continue;
                };
                let (ty, tr) = if self.c_modules.contains(&i.module) {
                    (
                        self.lookup_c(&i.type_path, &i.module, None, Want::Type),
                        self.lookup_c(trait_path, &i.module, None, Want::Type),
                    )
                } else {
                    (
                        self.lookup(&i.type_path, &i.module, None),
                        self.lookup(trait_path, &i.module, None),
                    )
                };
                if let (Some(ty), Some(tr)) = (ty, tr) {
                    add(
                        ty.qualified.clone(),
                        tr.qualified.clone(),
                        EdgeKind::Implements,
                        Confidence::High,
                        0,
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
                            // On a receiver of unknown type, a common name
                            // nearly always means a standard library method.
                            let common = COMMON_METHODS.contains(&name)
                                || COMMON_CSHARP_METHODS.contains(&name)
                                || COMMON_GO_METHODS.contains(&name)
                                || COMMON_KOTLIN_METHODS.contains(&name)
                                || (language.is_c_family() && COMMON_C_METHODS.contains(&name));
                            self.by_bare_name(
                                name,
                                &c.module,
                                language,
                                |s| {
                                    s.kind == SymbolKind::Method
                                        || (s.kind == SymbolKind::Function
                                            && s.attrs.iter().any(|a| a == EXTENSION_ATTR))
                                },
                                common,
                            )
                            .map(|(s, _)| (s, Confidence::Low))
                        })
                } else {
                    // A path that does not resolve names something outside the
                    // repository (std, a dependency, a closure); never guess.
                    if self.c_modules.contains(&c.module) {
                        self.lookup_c(&c.path, &c.module, Some(&c.caller), Want::Call)
                            .and_then(|s| self.c_callee(s))
                            .map(|s| (s, Confidence::High))
                    } else {
                        self.lookup_from(&c.path, &c.module, self_type, &c.caller)
                            .filter(|s| callable(s.kind))
                            .map(|s| (s, Confidence::High))
                    }
                };
                if let Some((target, conf)) = hit {
                    add(
                        c.caller.clone(),
                        target.qualified.clone(),
                        EdgeKind::Calls,
                        conf,
                        c.at,
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
                    // A function-like macro named in code is a call of it.
                    SymbolKind::Macro if Language::from_path(&target.file).is_c_family() => {
                        if v.call {
                            EdgeKind::Calls
                        } else {
                            EdgeKind::References
                        }
                    }
                    k if callable(k) && v.call => EdgeKind::Calls,
                    // A function named without being called is passed as a
                    // value; a constant or static is read.
                    k if callable(k) => EdgeKind::References,
                    SymbolKind::Const | SymbolKind::Static => EdgeKind::References,
                    k if k.is_type() => EdgeKind::References,
                    _ => continue,
                };
                let at = if kind == EdgeKind::Calls { v.at } else { 0 };
                add(
                    v.from.clone(),
                    target.qualified.clone(),
                    kind,
                    Confidence::High,
                    at,
                );
            }

            for r in &parsed.type_refs {
                let hit = if self.c_modules.contains(&r.module) {
                    self.lookup_c(&r.path, &r.module, Some(&r.from), Want::Type)
                } else {
                    self.lookup_from(&r.path, &r.module, None, &r.from)
                }
                .filter(|s| s.kind.is_type() || s.kind == SymbolKind::TypeAlias)
                .map(|s| (s, Confidence::High));
                if let Some((target, conf)) = hit {
                    add(
                        r.from.clone(),
                        target.qualified.clone(),
                        EdgeKind::References,
                        conf,
                        0,
                    );
                }
            }
        }

        Resolved {
            edges: edges
                .into_iter()
                .map(|((src, dst, kind, confidence), site)| Edge {
                    src,
                    dst,
                    kind,
                    confidence,
                    site,
                })
                .collect(),
            external_uses: external
                .into_iter()
                .map(|((file, krate), names)| ExternalUse {
                    file,
                    krate,
                    names: leaves(names),
                })
                .collect(),
        }
    }
}
