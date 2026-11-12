//! Plumbing shared by the Python, TypeScript and Java extractors: the fact
//! sink, text helpers, the file-to-module mapping and import classification.
//!
//! Qualified names use `::` between segments in every language, so one
//! resolver serves all of them: `pkg::util::shout` for Python's
//! `pkg.util.shout`, `com::acme::Label` for Java's `com.acme.Label`, and
//! `src::util::text::shout` for a TypeScript export of `src/util/text.ts`.

use std::collections::HashSet;

use cw_core::{Language, Symbol, SymbolKind, Visibility};
use tree_sitter::Node;

use crate::rust::{CallSite, ImplFact, ModDecl, ParsedFile, TypeRef, UseImport, ValueRef};

const MAX_SIGNATURE_CHARS: usize = 300;

/// Top-level modules of the Python standard library. Imports of these are
/// part of the language, not integrations.
const PYTHON_STDLIB: [&str; 112] = [
    "__future__",
    "abc",
    "argparse",
    "array",
    "ast",
    "asyncio",
    "atexit",
    "base64",
    "binascii",
    "bisect",
    "builtins",
    "bz2",
    "calendar",
    "cmath",
    "codecs",
    "collections",
    "concurrent",
    "configparser",
    "contextlib",
    "contextvars",
    "copy",
    "csv",
    "ctypes",
    "dataclasses",
    "datetime",
    "decimal",
    "difflib",
    "dis",
    "email",
    "enum",
    "errno",
    "fnmatch",
    "fractions",
    "functools",
    "gc",
    "getpass",
    "gettext",
    "glob",
    "graphlib",
    "gzip",
    "hashlib",
    "heapq",
    "hmac",
    "html",
    "http",
    "importlib",
    "inspect",
    "io",
    "ipaddress",
    "itertools",
    "json",
    "keyword",
    "linecache",
    "locale",
    "logging",
    "lzma",
    "math",
    "mimetypes",
    "multiprocessing",
    "numbers",
    "operator",
    "os",
    "pathlib",
    "pickle",
    "pkgutil",
    "platform",
    "pprint",
    "queue",
    "random",
    "re",
    "secrets",
    "select",
    "selectors",
    "shlex",
    "shutil",
    "signal",
    "site",
    "smtplib",
    "socket",
    "socketserver",
    "sqlite3",
    "ssl",
    "stat",
    "statistics",
    "string",
    "struct",
    "subprocess",
    "sys",
    "sysconfig",
    "tarfile",
    "tempfile",
    "textwrap",
    "threading",
    "time",
    "timeit",
    "tkinter",
    "tokenize",
    "tomllib",
    "traceback",
    "types",
    "typing",
    "unicodedata",
    "unittest",
    "urllib",
    "uuid",
    "venv",
    "warnings",
    "weakref",
    "xml",
    "zipfile",
    "zlib",
    "zoneinfo",
];

/// Root packages of the Java platform.
const JAVA_PLATFORM: [&str; 4] = ["java", "javax", "jdk", "sun"];

/// Modules built into Node.js, importable with or without the `node:` prefix.
const NODE_BUILTINS: [&str; 34] = [
    "assert",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "crypto",
    "dgram",
    "dns",
    "events",
    "fs",
    "http",
    "http2",
    "https",
    "module",
    "net",
    "os",
    "path",
    "perf_hooks",
    "process",
    "querystring",
    "readline",
    "stream",
    "string_decoder",
    "test",
    "timers",
    "tls",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "worker_threads",
    "zlib",
    "inspector",
];

/// Extensions a TypeScript import specifier may leave out.
const TS_EXTENSIONS: [&str; 5] = ["ts", "tsx", "d.ts", "mts", "cts"];

/// Named children of `node`, collected so the tree can be walked while the
/// extractor is mutated.
pub(crate) fn named(node: Node) -> Vec<Node> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

/// Every child stored under the field `name`.
pub(crate) fn field_all<'t>(node: Node<'t>, name: &str) -> Vec<Node<'t>> {
    let mut cursor = node.walk();
    node.children_by_field_name(name, &mut cursor).collect()
}

pub(crate) fn split_qualified(qualified: &str) -> Vec<String> {
    qualified
        .split("::")
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Segments of a dotted name such as `pkg.util.shout`.
pub(crate) fn dotted(text: &str) -> Vec<String> {
    text.split('.')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

/// Declaration text with whitespace collapsed and the body opener removed.
pub(crate) fn collapse(raw: &str) -> String {
    let mut sig = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    loop {
        let keep = sig.trim_end().trim_end_matches([';', '{', ':']).len();
        // `const LIMIT: number =` loses the `=`; an arrow's `=>` stays.
        let keep = match sig[..keep].trim_end() {
            s if s.ends_with('=') && !s.ends_with("==") => s.len() - 1,
            s => s.len(),
        };
        if keep == sig.len() {
            break;
        }
        sig.truncate(keep);
    }
    if sig.chars().count() > MAX_SIGNATURE_CHARS {
        sig = sig
            .chars()
            .take(MAX_SIGNATURE_CHARS - 3)
            .collect::<String>()
            + "...";
    }
    sig
}

/// Remove the indentation shared by the lines after the first.
pub(crate) fn dedent(text: &str) -> String {
    let mut lines = text.lines();
    let first = lines.next().unwrap_or("").trim();
    let rest: Vec<&str> = lines.collect();
    let indent = rest
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    let mut out = vec![first.to_string()];
    out.extend(
        rest.iter()
            .map(|l| l.get(indent..).unwrap_or("").trim_end().to_string()),
    );
    out.join("\n").trim().to_string()
}

/// Text of a `/** ... */` comment with the markers and leading stars removed.
/// Returns `None` for any other comment.
pub(crate) fn block_doc(comment: &str) -> Option<String> {
    let inner = comment.strip_prefix("/**")?.strip_suffix("*/")?;
    // `/**/` is an empty ordinary comment, not documentation.
    let lines: Vec<&str> = inner
        .lines()
        .map(|l| {
            let l = l.trim();
            let l = l.strip_prefix('*').unwrap_or(l);
            l.strip_prefix(' ').unwrap_or(l).trim_end()
        })
        .collect();
    Some(lines.join("\n").trim().to_string())
}

/// Make a path component usable as a name segment.
pub(crate) fn sanitize(segment: &str) -> String {
    segment
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn starts_uppercase(name: &str) -> bool {
    name.chars().next().is_some_and(char::is_uppercase)
}

/// Single uppercase letters are almost always generic parameters.
pub(crate) fn is_generic_param(name: &str) -> bool {
    name.len() == 1 && name.chars().all(|c| c.is_ascii_uppercase())
}

/// Name of a decorator or annotation: the text after `@` up to its arguments.
pub(crate) fn marker_name(text: &str) -> String {
    text.trim_start_matches('@')
        .trim_start()
        .chars()
        .take_while(|c| c.is_alphanumeric() || matches!(c, '_' | '.'))
        .collect()
}

/// Collects the facts of one file.
pub(crate) struct Sink<'a> {
    pub file: &'a str,
    pub src: &'a [u8],
    /// Qualified module the file defines.
    pub module: String,
    pub out: ParsedFile,
    values_seen: HashSet<(String, Vec<String>)>,
    types_seen: HashSet<(String, Vec<String>)>,
}

impl<'a> Sink<'a> {
    pub fn new(file: &'a str, source: &'a str, module: &str, has_errors: bool) -> Self {
        Self {
            file,
            src: source.as_bytes(),
            module: module.to_string(),
            out: ParsedFile {
                has_errors,
                ..ParsedFile::default()
            },
            values_seen: HashSet::new(),
            types_seen: HashSet::new(),
        }
    }

    pub fn text(&self, node: Node) -> &'a str {
        node.utf8_text(self.src).unwrap_or("")
    }

    pub fn between(&self, start: usize, end: usize) -> &'a str {
        self.src
            .get(start..end)
            .and_then(|b| std::str::from_utf8(b).ok())
            .unwrap_or("")
    }

    /// Record the module the file defines, spanning `start_line..=end_line`.
    pub fn module_symbol(
        &mut self,
        signature: String,
        doc: String,
        start_line: u32,
        end_line: u32,
    ) {
        let module = self.module.clone();
        let (parent, name) = match module.rsplit_once("::") {
            Some((parent, name)) => (Some(parent.to_string()), name.to_string()),
            None => (None, module.clone()),
        };
        self.out.symbols.push(Symbol {
            qualified: module,
            name,
            kind: SymbolKind::Module,
            visibility: Visibility::Public,
            file: self.file.to_string(),
            start_line: start_line.max(1),
            end_line: end_line.max(start_line).max(1),
            signature,
            doc,
            parent,
            attrs: Vec::new(),
        });
    }

    /// Add a public symbol named `name` under `scope` covering `span`, and
    /// return it so the caller can fill in the remaining fields.
    pub fn symbol(&mut self, scope: &str, name: &str, kind: SymbolKind, span: Node) -> &mut Symbol {
        self.out.symbols.push(Symbol {
            qualified: format!("{scope}::{name}"),
            name: name.to_string(),
            kind,
            visibility: Visibility::Public,
            file: self.file.to_string(),
            start_line: span.start_position().row as u32 + 1,
            end_line: span.end_position().row as u32 + 1,
            signature: String::new(),
            doc: String::new(),
            parent: Some(scope.to_string()),
            attrs: Vec::new(),
        });
        let last = self.out.symbols.len() - 1;
        &mut self.out.symbols[last]
    }

    /// A name used as a value inside `owner`.
    pub fn value(&mut self, owner: &str, path: Vec<String>) {
        if path.is_empty() || !self.values_seen.insert((owner.to_string(), path.clone())) {
            return;
        }
        self.out.value_refs.push(ValueRef {
            from: owner.to_string(),
            module: self.module.clone(),
            self_type: None,
            path,
            call: false,
        });
    }

    /// A type mentioned inside `owner`.
    pub fn type_ref(&mut self, owner: &str, path: Vec<String>) {
        let Some(last) = path.last() else {
            return;
        };
        if is_generic_param(last) || !self.types_seen.insert((owner.to_string(), path.clone())) {
            return;
        }
        self.out.type_refs.push(TypeRef {
            from: owner.to_string(),
            module: self.module.clone(),
            path,
        });
    }

    /// A call made by `owner`. `method` marks `receiver.name(...)` where the
    /// receiver's type is unknown; `self_type` is set only when the receiver
    /// is the instance itself.
    pub fn call(&mut self, owner: &str, path: Vec<String>, method: bool, self_type: Option<&str>) {
        if path.is_empty() {
            return;
        }
        self.out.calls.push(CallSite {
            caller: owner.to_string(),
            module: self.module.clone(),
            self_type: self_type.map(str::to_string),
            path,
            method,
        });
    }

    /// A name bound by an import. An empty `alias` binds nothing and only
    /// states that the file depends on `path`.
    pub fn import(&mut self, path: Vec<String>, alias: &str, glob: bool, public: bool) {
        if path.is_empty() {
            return;
        }
        self.out.uses.push(UseImport {
            scope: self.module.clone(),
            path,
            alias: alias.to_string(),
            glob,
            public,
        });
    }

    /// `qualified` extends or implements the type written as `parent`.
    pub fn inherits(&mut self, qualified: &str, parent: Vec<String>) {
        let prefix = format!("{}::", self.module);
        let Some(relative) = qualified.strip_prefix(&prefix) else {
            return;
        };
        if parent.is_empty() {
            return;
        }
        self.out.impls.push(ImplFact {
            module: self.module.clone(),
            type_path: split_qualified(relative),
            trait_path: Some(parent),
        });
    }
}

/// Module a Python file defines, and whether the file is a package
/// (`__init__.py`).
///
/// The name starts at the outermost directory that has an `__init__.py`, so
/// it matches what `import` statements write. A file outside any package is
/// named by its path from the repository root, minus a leading `src` or `lib`.
pub fn python_module(files: &HashSet<&str>, path: &str) -> (String, bool) {
    let stem_path = path.rsplit_once('.').map_or(path, |(stem, _)| stem);
    let mut dirs: Vec<&str> = stem_path.split('/').collect();
    let stem = dirs.pop().unwrap_or("");
    let is_package = stem == "__init__";
    let has_init = |depth: usize| {
        let dir = dirs[..=depth].join("/");
        ["py", "pyi"]
            .iter()
            .any(|ext| files.contains(format!("{dir}/__init__.{ext}").as_str()))
    };
    let start = (0..dirs.len())
        .find(|&i| has_init(i))
        .unwrap_or_else(|| usize::from(dirs.first().is_some_and(|d| matches!(*d, "src" | "lib"))));
    let mut segments: Vec<String> = dirs[start..].iter().map(|d| sanitize(d)).collect();
    if !is_package {
        segments.push(sanitize(stem));
    }
    if segments.is_empty() {
        segments.push("__init__".to_string());
    }
    (segments.join("::"), is_package)
}

/// Module a TypeScript file defines: its path without the extension.
pub fn ts_module(path: &str) -> String {
    let stem = path.rsplit_once('.').map_or(path, |(stem, _)| stem);
    stem.split('/')
        .filter(|s| !s.is_empty())
        .map(sanitize)
        .collect::<Vec<_>>()
        .join("::")
}

/// JavaScript source extensions, tried after the TypeScript ones.
const JS_EXTENSIONS: [&str; 4] = ["js", "jsx", "mjs", "cjs"];

/// File a relative TypeScript import specifier names, as written in `from`.
/// Tries the specifier itself, then each source extension, then an `index`
/// file in a directory of that name. Non-relative specifiers return `None`.
pub fn resolve_ts_import(files: &HashSet<&str>, from: &str, spec: &str) -> Option<String> {
    if !spec.starts_with('.') {
        return None;
    }
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop();
    for segment in spec.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    let base = parts.join("/");
    let is_source = |p: &str| {
        matches!(
            Language::from_path(p),
            Language::TypeScript | Language::JavaScript
        )
    };
    if is_source(&base) && files.contains(base.as_str()) {
        return Some(base);
    }
    // `./text.js` is how ESM TypeScript names `./text.ts`.
    let stem = [".js", ".jsx", ".mjs", ".cjs"]
        .iter()
        .find_map(|ext| base.strip_suffix(ext))
        .unwrap_or(&base);
    let index = if base.is_empty() {
        "index".to_string()
    } else {
        format!("{base}/index")
    };
    [stem, index.as_str()]
        .iter()
        .filter(|candidate| !candidate.is_empty())
        .flat_map(|candidate| {
            TS_EXTENSIONS
                .iter()
                .chain(JS_EXTENSIONS.iter())
                .map(move |ext| format!("{candidate}.{ext}"))
        })
        .find(|candidate| files.contains(candidate.as_str()))
}

/// Package a non-relative TypeScript import specifier names, or `None` when
/// it is a Node.js builtin or a path alias whose target is not known.
pub(crate) fn ts_package(spec: &str) -> Option<String> {
    if spec.starts_with("node:") || spec.starts_with(['.', '/', '~', '#']) || spec.starts_with("@/")
    {
        return None;
    }
    let mut parts = spec.split('/');
    let first = parts.next().filter(|s| !s.is_empty())?;
    if NODE_BUILTINS.contains(&first) {
        return None;
    }
    if first.starts_with('@') {
        let second = parts.next()?;
        return Some(format!("{first}/{second}"));
    }
    Some(first.to_string())
}

/// Module used for a Java file with no `package` declaration: its directory.
pub(crate) fn java_default_package(path: &str) -> String {
    let mut dirs: Vec<String> = path.split('/').map(sanitize).collect();
    dirs.pop();
    if dirs.is_empty() {
        return "default_package".to_string();
    }
    dirs.join("::")
}

/// True when the first `k` segments of `path` name a module, for some
/// `k >= min_len`.
fn has_module_prefix(modules: &HashSet<String>, path: &[String], min_len: usize) -> bool {
    (min_len.max(1)..=path.len()).any(|k| modules.contains(&path[..k].join("::")))
}

fn modules_of(parsed: &[(String, ParsedFile)], language: Language) -> HashSet<String> {
    parsed
        .iter()
        .filter(|(file, _)| Language::from_path(file) == language)
        .flat_map(|(_, p)| p.symbols.iter())
        .filter(|s| s.kind == SymbolKind::Module)
        .map(|s| s.qualified.clone())
        .collect()
}

/// An import that leaves the repository: keep one segment naming the package.
fn external(import: UseImport, package: String) -> UseImport {
    UseImport {
        path: vec![package],
        alias: String::new(),
        glob: false,
        ..import
    }
}

fn link_python(mut import: UseImport, modules: &HashSet<String>) -> Option<UseImport> {
    if has_module_prefix(modules, &import.path, 1) {
        return Some(import);
    }
    // A script outside any package also finds the modules next to it, since
    // its own directory is on the import path.
    let mut parent = split_qualified(&import.scope);
    parent.pop();
    if !parent.is_empty() && !modules.contains(&parent.join("::")) {
        let depth = parent.len();
        parent.extend(import.path.iter().cloned());
        if has_module_prefix(modules, &parent, depth + 1) {
            import.path = parent;
            return Some(import);
        }
    }
    let root = import.path.first()?.clone();
    if PYTHON_STDLIB.contains(&root.as_str()) {
        return None;
    }
    Some(external(import, root))
}

fn link_java(import: UseImport, packages: &HashSet<String>) -> Option<UseImport> {
    if has_module_prefix(packages, &import.path, 1) {
        return Some(import);
    }
    let root = import.path.first()?;
    if JAVA_PLATFORM.contains(&root.as_str()) {
        return None;
    }
    // `org.junit.Test` belongs to `org.junit`; the class name is not part of
    // the library's identity.
    let len = import.path.len();
    let keep = if import.glob { len } else { len - 1 }.clamp(1, 2);
    let package = import.path[..keep].join(".");
    Some(external(import, package))
}

/// Decide, for every Python and Java import, whether it names repository
/// code, an outside package, or the language's own library. Imports of the
/// last kind are dropped. Also records which Python package holds which
/// module.
pub(crate) fn link_imports(parsed: &mut [(String, ParsedFile)]) {
    let python = modules_of(parsed, Language::Python);
    let java = modules_of(parsed, Language::Java);
    for (file, facts) in parsed.iter_mut() {
        let uses = std::mem::take(&mut facts.uses);
        match Language::from_path(file) {
            Language::Python => {
                facts.uses = uses
                    .into_iter()
                    .filter_map(|u| link_python(u, &python))
                    .collect();
                let own = facts
                    .symbols
                    .iter()
                    .find(|s| s.kind == SymbolKind::Module)
                    .and_then(|s| s.qualified.rsplit_once("::"));
                if let Some((parent, name)) = own
                    && python.contains(parent)
                {
                    facts.mods.push(ModDecl {
                        parent: parent.to_string(),
                        name: name.to_string(),
                        inline: false,
                    });
                }
            }
            Language::Java => {
                facts.uses = uses
                    .into_iter()
                    .filter_map(|u| link_java(u, &java))
                    .collect();
            }
            _ => facts.uses = uses,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set<'a>(files: &[&'a str]) -> HashSet<&'a str> {
        files.iter().copied().collect()
    }

    #[test]
    fn maps_python_files_to_modules() {
        let files = set(&[
            "src/pkg/__init__.py",
            "src/pkg/util.py",
            "src/pkg/sub/deep.py",
            "tools/gen.py",
            "main.py",
        ]);
        let m = |p: &str| python_module(&files, p);
        assert_eq!(m("src/pkg/__init__.py"), ("pkg".to_string(), true));
        assert_eq!(m("src/pkg/util.py"), ("pkg::util".to_string(), false));
        assert_eq!(m("src/pkg/sub/deep.py").0, "pkg::sub::deep");
        assert_eq!(m("tools/gen.py").0, "tools::gen");
        assert_eq!(m("main.py").0, "main");
    }

    #[test]
    fn resolves_typescript_specifiers() {
        let files = set(&[
            "src/app.ts",
            "src/util/text.ts",
            "src/util/index.ts",
            "src/ui/Button.tsx",
            "src/types.d.ts",
        ]);
        let r = |spec: &str| resolve_ts_import(&files, "src/app.ts", spec);
        assert_eq!(r("./util/text").as_deref(), Some("src/util/text.ts"));
        assert_eq!(r("./util/text.js").as_deref(), Some("src/util/text.ts"));
        assert_eq!(r("./util").as_deref(), Some("src/util/index.ts"));
        assert_eq!(r("./ui/Button").as_deref(), Some("src/ui/Button.tsx"));
        assert_eq!(r("./types").as_deref(), Some("src/types.d.ts"));
        assert_eq!(r("./missing"), None);
        assert_eq!(r("../../outside"), None);
        assert_eq!(r("react"), None);
        assert_eq!(ts_module("src/util/text.ts"), "src::util::text");
        assert_eq!(ts_module("src/my-view.tsx"), "src::my_view");
    }

    #[test]
    fn names_typescript_packages() {
        assert_eq!(ts_package("react").as_deref(), Some("react"));
        assert_eq!(ts_package("lodash/merge").as_deref(), Some("lodash"));
        assert_eq!(ts_package("@scope/pkg/sub").as_deref(), Some("@scope/pkg"));
        assert_eq!(ts_package("node:fs"), None);
        assert_eq!(ts_package("path"), None);
        assert_eq!(ts_package("@/components/x"), None);
    }

    #[test]
    fn cleans_text() {
        assert_eq!(collapse("def f(a,\n      b) -> int:"), "def f(a, b) -> int");
        assert_eq!(collapse("const LIMIT: number = "), "const LIMIT: number");
        assert_eq!(
            collapse("const f = (a: string) =>"),
            "const f = (a: string) =>"
        );
        assert_eq!(collapse("class A extends B {"), "class A extends B");
        assert_eq!(
            block_doc("/**\n * First.\n *\n * Second.\n */").as_deref(),
            Some("First.\n\nSecond.")
        );
        assert_eq!(block_doc("/* plain */"), None);
        assert_eq!(
            dedent("First.\n\n    Second.\n      Third.\n    "),
            "First.\n\nSecond.\n  Third."
        );
    }
}
