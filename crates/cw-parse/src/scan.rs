//! Walk a repository and produce every fact the index needs.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::Path;

use cw_core::{
    Edge, Error, FileRecord, Language, Result, Symbol, SymbolKind, VAULT_DIR, Visibility,
    sha256_hex,
};
use ignore::WalkBuilder;

use crate::c::{CInput, parse_c};
use crate::cfamily::{CPlan, merge as merge_c_family};
use crate::csharp::parse_csharp;
use crate::go::{package_name, parse_go};
use crate::gomod::parse_go_mod;
use crate::java::parse_java;
use crate::kotlin::parse_kotlin;
use crate::lang::{GoBuild, link_imports, nuget_ids, python_module, resolve_ts_import, ts_module};
use crate::python::parse_python;
use crate::resolve::{ExternalUse, Resolver};
use crate::rust::{ParsedFile, parse_rust};
use crate::typescript::parse_typescript;
use crate::workspace::{CrateInfo, module_path, parse_manifest};

/// Files larger than this are recorded but not read as source.
const MAX_SOURCE_BYTES: u64 = 1_500_000;

/// Directories never worth indexing, on top of what `.gitignore` excludes.
const SKIP_DIRS: [&str; 6] = [
    ".git",
    ".idea",
    ".vscode",
    "target",
    "node_modules",
    ".venv",
];

#[derive(Debug, Default)]
pub struct RepoFacts {
    pub crates: Vec<CrateInfo>,
    pub files: Vec<FileRecord>,
    pub symbols: Vec<Symbol>,
    pub edges: Vec<Edge>,
    pub external_uses: Vec<ExternalUse>,
    /// Files whose syntax tree contained errors; their facts may be partial.
    pub files_with_errors: Vec<String>,
}

/// Repo-relative path with forward slashes.
fn relative(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let s = rel.to_str()?.replace('\\', "/");
    (!s.is_empty()).then_some(s)
}

/// List indexable files under `root`, sorted, honoring `.gitignore`.
pub fn list_files(root: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .require_git(false)
        .add_custom_ignore_filename(".codewikiignore")
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            let is_dir = e.file_type().is_some_and(|t| t.is_dir());
            // Go's `vendor` folder is a copy of the dependencies, which
            // `go mod vendor` describes in `modules.txt`.
            let vendored = is_dir && name == "vendor" && e.path().join("modules.txt").is_file();
            !(is_dir
                && (SKIP_DIRS.contains(&name.as_ref()) || (e.depth() == 1 && name == VAULT_DIR)))
                && !vendored
        })
        .build();
    for entry in walker {
        let entry = entry.map_err(|e| Error::external("walking the repository", e))?;
        if entry.file_type().is_some_and(|t| t.is_file())
            && let Some(rel) = relative(root, entry.path())
        {
            out.push(rel);
        }
    }
    out.sort();
    Ok(out)
}

/// Leading `//!` documentation of a file.
fn inner_doc(source: &str) -> String {
    source
        .lines()
        .map(str::trim)
        .skip_while(|l| l.is_empty())
        .take_while(|l| l.starts_with("//!"))
        .map(|l| l[3..].strip_prefix(' ').unwrap_or(&l[3..]))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// The parts of a C# partial type are one symbol. The part that says most
/// about it stands for it: the one with the documentation comment, then the
/// one that lists the base types. What another part carries, its attributes
/// and a comment the first lacks, is added.
fn merge_partial(symbols: &mut [Symbol], part: &Symbol) {
    if !part.kind.is_type() {
        return;
    }
    let Some(first) = symbols
        .iter_mut()
        .rev()
        .find(|s| s.qualified == part.qualified)
    else {
        return;
    };
    let says = |s: &Symbol| (!s.doc.is_empty(), s.signature.len());
    let mut attrs = first.attrs.clone();
    for attr in &part.attrs {
        if !attrs.contains(attr) {
            attrs.push(attr.clone());
        }
    }
    if says(part) > says(first) {
        let doc = if part.doc.is_empty() {
            first.doc.clone()
        } else {
            part.doc.clone()
        };
        *first = part.clone();
        first.doc = doc;
    } else if first.doc.is_empty() {
        first.doc.clone_from(&part.doc);
    }
    first.attrs = attrs;
}

/// The parts of a Go package are one module. The file that stands for it is
/// the first one that is not a test file, and its documentation is the first
/// comment any file of the package has above its `package` clause.
fn merge_package(symbols: &mut [Symbol], at: usize, part: &Symbol) {
    let first = &mut symbols[at];
    if first.file.ends_with("_test.go") && !part.file.ends_with("_test.go") {
        let doc = if part.doc.is_empty() {
            first.doc.clone()
        } else {
            part.doc.clone()
        };
        *first = part.clone();
        first.doc = doc;
    } else if first.doc.is_empty() {
        first.doc.clone_from(&part.doc);
    }
}

/// Scan the whole repository at `root`.
pub fn scan_repo(root: &Path) -> Result<RepoFacts> {
    let paths = list_files(root)?;

    let mut crates = Vec::new();
    for path in paths.iter().filter(|p| p.ends_with("Cargo.toml")) {
        let dir = path
            .strip_suffix("Cargo.toml")
            .unwrap()
            .trim_end_matches('/');
        let full = root.join(path);
        let text = std::fs::read_to_string(&full).map_err(|e| Error::io(&full, e))?;
        // A broken manifest must not abort indexing of the rest of the repo.
        if let Ok(Some(info)) = parse_manifest(dir, &text, &paths) {
            crates.push(info);
        }
    }

    let mut facts = RepoFacts {
        crates,
        ..RepoFacts::default()
    };
    let mut parsed: Vec<(String, ParsedFile)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let file_set: HashSet<&str> = paths.iter().map(String::as_str).collect();

    // The Go modules of the repository name the packages inside them; the
    // names the packages give themselves tell what an import is called.
    let mut go = GoBuild::default();
    for path in paths
        .iter()
        .filter(|p| p.rsplit('/').next() == Some("go.mod"))
    {
        let Ok(text) = std::fs::read_to_string(root.join(path)) else {
            continue;
        };
        let parsed_mod = parse_go_mod(&text);
        if !parsed_mod.module.is_empty() {
            let dir = path.rsplit_once('/').map_or("", |(dir, _)| dir);
            go.modules.list.push((dir.to_string(), parsed_mod.module));
        }
        go.requires
            .extend(parsed_mod.requires.into_iter().map(|r| r.path));
        go.requires
            .extend(parsed_mod.replaces.into_iter().map(|r| r.old));
    }
    go.requires.sort();
    go.requires.dedup();
    let mut go_names: HashMap<String, String> = HashMap::new();
    for path in paths
        .iter()
        .filter(|p| p.ends_with(".go") && !p.ends_with("_test.go"))
    {
        let dir = path.rsplit_once('/').map_or("", |(dir, _)| dir);
        if go_names.contains_key(dir) {
            continue;
        }
        // The package clause stands above the first declaration: the top of
        // the file is enough.
        let head = std::fs::File::open(root.join(path)).ok().and_then(|file| {
            let mut head = Vec::new();
            file.take(16 * 1024).read_to_end(&mut head).ok()?;
            Some(String::from_utf8_lossy(&head).into_owned())
        });
        if let Some(name) = head.as_deref().and_then(package_name) {
            go_names.insert(dir.to_string(), name.to_string());
        }
    }
    let go_import_name =
        |import: &str| -> Option<String> { go_names.get(&go.modules.folder_of(import)?).cloned() };
    let mut go_module_at: HashMap<String, usize> = HashMap::new();

    // The language of every C and C++ file, which header and source are one
    // module, and what an `#include` names: all need the other files.
    let c_plan = CPlan::build(&paths, &|path| {
        std::fs::read_to_string(root.join(path)).ok()
    });

    for path in &paths {
        let full = root.join(path);
        let meta = std::fs::metadata(&full).map_err(|e| Error::io(&full, e))?;
        let language = c_plan
            .as_ref()
            .and_then(|plan| plan.language.get(path).copied())
            .unwrap_or_else(|| Language::from_path(path));
        if meta.len() > MAX_SOURCE_BYTES {
            facts.files.push(FileRecord {
                path: path.clone(),
                language,
                hash: String::new(),
                bytes: meta.len(),
                lines: 0,
            });
            continue;
        }
        let bytes = std::fs::read(&full).map_err(|e| Error::io(&full, e))?;
        let text = String::from_utf8(bytes.clone()).ok();
        facts.files.push(FileRecord {
            path: path.clone(),
            language,
            hash: sha256_hex(&bytes),
            bytes: meta.len(),
            lines: text
                .as_deref()
                .map(|t| t.lines().count() as u32)
                .unwrap_or(0),
        });
        let Some(source) = text else {
            continue;
        };
        if language != Language::Rust {
            // These extractors emit the module symbol themselves.
            let file_parsed = match language {
                Language::Python => {
                    let (module, is_package) = python_module(&file_set, path);
                    parse_python(path, &module, is_package, &source)?
                }
                // JavaScript is parsed with the TSX grammar, which accepts it.
                Language::TypeScript | Language::JavaScript => {
                    let resolve = |spec: &str| {
                        resolve_ts_import(&file_set, path, spec).map(|file| ts_module(&file))
                    };
                    parse_typescript(path, &ts_module(path), &source, &resolve)?
                }
                Language::Java => parse_java(path, &source)?,
                Language::Kotlin => parse_kotlin(path, &source)?,
                Language::CSharp => parse_csharp(path, &source)?,
                Language::Go => {
                    parse_go(path, &source, &go.modules.package_of(path), &go_import_name)?
                }
                Language::C | Language::Cpp => {
                    // A vendored tree has no module: it is not parsed.
                    let Some((plan, module)) = c_plan
                        .as_ref()
                        .and_then(|plan| Some((plan, plan.module.get(path)?)))
                    else {
                        continue;
                    };
                    let dirs = plan.include_dirs(path);
                    let include =
                        |spec: &str, quoted: bool| plan.include(path, &dirs, spec, quoted);
                    let input = CInput {
                        module,
                        cpp: language == Language::Cpp,
                        include: &include,
                    };
                    parse_c(path, &source, &input)?
                }
                _ => continue,
            };
            if file_parsed.has_errors {
                facts.files_with_errors.push(path.clone());
            }
            // The symbols of C and C++ files are put together afterwards:
            // a declaration and its definition stand in different files.
            if language.is_c_family() {
                parsed.push((path.clone(), file_parsed));
                continue;
            }
            for symbol in &file_parsed.symbols {
                // Overloads, stubs, the files of one Java package and the
                // parts of a C# partial type share a name; keep the first.
                if seen.insert(symbol.qualified.clone()) {
                    if language == Language::Go && symbol.kind == SymbolKind::Module {
                        go_module_at.insert(symbol.qualified.clone(), facts.symbols.len());
                    }
                    facts.symbols.push(symbol.clone());
                } else if language == Language::CSharp {
                    merge_partial(&mut facts.symbols, symbol);
                } else if language == Language::Go
                    && symbol.kind == SymbolKind::Module
                    && let Some(at) = go_module_at.get(&symbol.qualified)
                {
                    merge_package(&mut facts.symbols, *at, symbol);
                }
            }
            parsed.push((path.clone(), file_parsed));
            continue;
        }
        let module = module_path(&facts.crates, path);
        let file_parsed = parse_rust(path, &module, &source)?;
        if file_parsed.has_errors {
            facts.files_with_errors.push(path.clone());
        }
        if seen.insert(module.clone()) {
            let name = module.rsplit("::").next().unwrap_or(&module).to_string();
            let parent = module.rsplit_once("::").map(|(p, _)| p.to_string());
            facts.symbols.push(Symbol {
                qualified: module.clone(),
                name,
                kind: SymbolKind::Module,
                visibility: Visibility::Public,
                file: path.clone(),
                start_line: 1,
                end_line: source.lines().count().max(1) as u32,
                signature: format!("mod {}", module.rsplit("::").next().unwrap_or(&module)),
                doc: inner_doc(&source),
                parent,
                attrs: Vec::new(),
            });
        }
        for symbol in &file_parsed.symbols {
            // `#[cfg]` twins and inline/file module pairs share a name; keep the first.
            if seen.insert(symbol.qualified.clone()) {
                facts.symbols.push(symbol.clone());
            }
        }
        parsed.push((path.clone(), file_parsed));
    }

    if let Some(plan) = &c_plan {
        for symbol in merge_c_family(&mut parsed, plan) {
            if seen.insert(symbol.qualified.clone()) {
                facts.symbols.push(symbol);
            }
        }
    }

    // The packages the repository's projects reference tell which package a
    // C# namespace outside the repository belongs to.
    let mut nuget: Vec<String> = Vec::new();
    for path in paths
        .iter()
        .filter(|p| p.ends_with(".csproj") || p.ends_with(".props"))
    {
        if let Ok(text) = std::fs::read_to_string(root.join(path)) {
            nuget.extend(nuget_ids(&text));
        }
    }
    nuget.sort();
    nuget.dedup();
    link_imports(&mut parsed, &nuget, &go);
    // The first segment of every Python, TypeScript, Java, Kotlin, C# and Go module is a
    // root that paths may start at, as crate names are for Rust.
    let mut roots: Vec<&str> = facts.crates.iter().map(|c| c.name.as_str()).collect();
    roots.extend(
        facts
            .symbols
            .iter()
            .filter(|s| {
                s.kind == SymbolKind::Module && Language::from_path(&s.file) != Language::Rust
            })
            .filter_map(|s| s.qualified.split("::").next()),
    );
    let resolved = Resolver::new(&facts.symbols, &parsed, roots)
        .with_crates(&facts.crates)
        .resolve(&parsed);
    facts.edges = resolved.edges;
    facts.external_uses = resolved.external_uses;
    facts.symbols.sort_by(|a, b| a.qualified.cmp(&b.qualified));
    Ok(facts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cw_core::{Confidence, EdgeKind};
    use std::fs;

    fn write(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        write(r, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
        write(
            r,
            "crates/util/Cargo.toml",
            "[package]\nname = \"my-util\"\n[dependencies]\nserde = \"1\"\n",
        );
        write(
            r,
            "crates/util/src/lib.rs",
            "//! Utilities.\npub mod text;\npub use text::{Label, shout};\npub trait Named { fn name(&self) -> String; }\n",
        );
        write(
            r,
            "crates/util/src/text.rs",
            "use serde::Serialize;\npub fn shout(s: &str) -> String { s.to_uppercase() }\npub struct Label;\n",
        );
        write(
            r,
            "crates/app/Cargo.toml",
            "[package]\nname = \"app\"\n[dependencies]\nmy-util = { path = \"../util\" }\n",
        );
        write(
            r,
            "crates/app/src/main.rs",
            "use my_util::{Label, Named, shout};\nstruct App { label: Label }\nimpl Named for App { fn name(&self) -> String { shout(\"app\") } }\nfn main() { let a = App { label: Label }; a.name(); }\n",
        );
        write(r, "obsidian/_HOME.md", "# Home\n");
        write(r, "target/debug/junk.rs", "fn junk() {}\n");
        write(r, "README.md", "# Demo\n");
        dir
    }

    #[test]
    fn scans_fixture_repository() {
        let dir = fixture();
        let facts = scan_repo(dir.path()).unwrap();

        let paths: Vec<&str> = facts.files.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"README.md"));
        assert!(
            !paths
                .iter()
                .any(|p| p.starts_with("obsidian/") || p.starts_with("target/"))
        );

        let names: Vec<&str> = facts.crates.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["app", "my_util"]);

        let has = |q: &str| facts.symbols.iter().any(|s| s.qualified == q);
        assert!(has("my_util") && has("my_util::text") && has("my_util::text::shout"));
        assert!(has("app::App") && has("app::App::name") && has("app::main"));
        let root = facts
            .symbols
            .iter()
            .find(|s| s.qualified == "my_util")
            .unwrap();
        assert_eq!(root.doc, "Utilities.");

        let edge = |src: &str, dst: &str, kind: EdgeKind| {
            facts
                .edges
                .iter()
                .find(|e| e.src == src && e.dst == dst && e.kind == kind)
                .map(|e| e.confidence)
        };
        // `shout` is reached through the `pub use` re-export in lib.rs.
        assert_eq!(
            edge("app::App::name", "my_util::text::shout", EdgeKind::Calls),
            Some(Confidence::High)
        );
        assert_eq!(
            edge("app::App", "my_util::Named", EdgeKind::Implements),
            Some(Confidence::High)
        );
        assert_eq!(
            edge("app::App", "my_util::text::Label", EdgeKind::References),
            Some(Confidence::High)
        );
        assert!(edge("app::main", "app::App::name", EdgeKind::Calls).is_some());
        assert!(
            edge(
                "file:crates/app/src/main.rs",
                "file:crates/util/src/text.rs",
                EdgeKind::Imports
            )
            .is_some()
        );
        assert!(edge("my_util", "my_util::text", EdgeKind::Declares).is_some());

        assert_eq!(
            facts.external_uses,
            vec![ExternalUse {
                file: "crates/util/src/text.rs".into(),
                krate: "serde".into(),
                names: vec!["Serialize".into()],
            }]
        );
    }

    #[test]
    fn scan_is_deterministic() {
        let dir = fixture();
        let a = scan_repo(dir.path()).unwrap();
        let b = scan_repo(dir.path()).unwrap();
        assert_eq!(a.symbols, b.symbols);
        assert_eq!(a.edges, b.edges);
        assert_eq!(a.files, b.files);
    }

    /// Scan `files` as a repository and return the facts with two lookups.
    fn scan_files(files: &[(&str, &str)]) -> RepoFacts {
        let dir = tempfile::tempdir().unwrap();
        for (rel, text) in files {
            write(dir.path(), rel, text);
        }
        scan_repo(dir.path()).unwrap()
    }

    fn edge_of(facts: &RepoFacts, src: &str, dst: &str, kind: EdgeKind) -> Option<Confidence> {
        facts
            .edges
            .iter()
            .find(|e| e.src == src && e.dst == dst && e.kind == kind)
            .map(|e| e.confidence)
    }

    fn kind_of(facts: &RepoFacts, qualified: &str) -> Option<SymbolKind> {
        facts
            .symbols
            .iter()
            .find(|s| s.qualified == qualified)
            .map(|s| s.kind)
    }

    #[test]
    fn scans_python_packages() {
        let facts = scan_files(&[
            (
                "src/pkg/__init__.py",
                "\"\"\"Demo package.\"\"\"\nfrom .text import shout\n",
            ),
            (
                "src/pkg/text.py",
                "import os\nimport requests\n\n\nclass Base:\n    def render(self):\n        return self.name()\n\n    def name(self):\n        return os.sep\n\n\ndef shout(s):\n    return s.upper()\n",
            ),
            (
                "src/pkg/app.py",
                "from pkg import shout\nfrom pkg.text import Base\nfrom . import text\n\n\nclass App(Base):\n    def run(self):\n        obj = make()\n        obj.render()\n        return text.shout(self.title())\n\n    def title(self):\n        return shout(\"x\")\n\n\ndef make():\n    return App()\n",
            ),
            (
                "tools/gen.py",
                "import helper\n\n\ndef main():\n    helper.emit()\n",
            ),
            ("tools/helper.py", "def emit():\n    pass\n"),
        ]);
        assert!(facts.files_with_errors.is_empty());

        assert_eq!(kind_of(&facts, "pkg"), Some(SymbolKind::Module));
        assert_eq!(kind_of(&facts, "pkg::text"), Some(SymbolKind::Module));
        assert_eq!(
            kind_of(&facts, "pkg::text::shout"),
            Some(SymbolKind::Function)
        );
        assert_eq!(kind_of(&facts, "pkg::app::App"), Some(SymbolKind::Class));
        assert_eq!(
            kind_of(&facts, "pkg::app::App::run"),
            Some(SymbolKind::Method)
        );
        assert_eq!(
            kind_of(&facts, "tools::gen::main"),
            Some(SymbolKind::Function)
        );
        let package = facts.symbols.iter().find(|s| s.qualified == "pkg").unwrap();
        assert_eq!(package.doc, "Demo package.");
        assert_eq!(package.file, "src/pkg/__init__.py");

        let edge = |src: &str, dst: &str, kind| edge_of(&facts, src, dst, kind);
        // Through the re-export in `__init__.py`.
        assert_eq!(
            edge("pkg::app::App::title", "pkg::text::shout", EdgeKind::Calls),
            Some(Confidence::High)
        );
        // Through the imported module.
        assert_eq!(
            edge("pkg::app::App::run", "pkg::text::shout", EdgeKind::Calls),
            Some(Confidence::High)
        );
        assert_eq!(
            edge("pkg::app::App::run", "pkg::app::make", EdgeKind::Calls),
            Some(Confidence::High)
        );
        // `self.title()` is dynamic dispatch on a known class.
        assert_eq!(
            edge(
                "pkg::app::App::run",
                "pkg::app::App::title",
                EdgeKind::Calls
            ),
            Some(Confidence::Medium)
        );
        // `obj.render()` has a receiver of unknown type: a name match only.
        assert_eq!(
            edge(
                "pkg::app::App::run",
                "pkg::text::Base::render",
                EdgeKind::Calls
            ),
            Some(Confidence::Low)
        );
        assert_eq!(
            edge("pkg::app::App", "pkg::text::Base", EdgeKind::Implements),
            Some(Confidence::High)
        );
        assert_eq!(
            edge("pkg::app::make", "pkg::app::App", EdgeKind::References),
            Some(Confidence::High)
        );
        assert!(
            edge(
                "file:src/pkg/app.py",
                "file:src/pkg/text.py",
                EdgeKind::Imports
            )
            .is_some()
        );
        assert!(edge("pkg", "pkg::text", EdgeKind::Declares).is_some());
        // A script finds the module next to it.
        assert_eq!(
            edge("tools::gen::main", "tools::helper::emit", EdgeKind::Calls),
            Some(Confidence::High)
        );

        // The standard library is not an integration.
        assert_eq!(
            facts.external_uses,
            vec![ExternalUse {
                file: "src/pkg/text.py".into(),
                krate: "requests".into(),
                // `import requests` names the package alone.
                names: vec![],
            }]
        );
    }

    #[test]
    fn scans_javascript_modules() {
        let facts = scan_files(&[
            (
                "site/lib/text.js",
                "/** Make it loud. */\nexport function shout(s) {\n  return s.toUpperCase();\n}\nexport class Label {\n  show() {\n    return shout(this.text);\n  }\n}\n",
            ),
            (
                "site/app.jsx",
                "import { shout } from \"./lib/text.js\";\n\nexport function App() {\n  return <b>{shout(\"hi\")}</b>;\n}\n",
            ),
            (
                "site/mixed.ts",
                "import { shout } from \"./lib/text\";\nexport function loud(): string {\n  return shout(\"x\");\n}\n",
            ),
        ]);
        assert!(facts.files_with_errors.is_empty());
        let t = "site::lib::text";
        assert_eq!(
            kind_of(&facts, &format!("{t}::shout")),
            Some(SymbolKind::Function)
        );
        assert_eq!(
            kind_of(&facts, &format!("{t}::Label")),
            Some(SymbolKind::Class)
        );
        assert_eq!(
            kind_of(&facts, &format!("{t}::Label::show")),
            Some(SymbolKind::Method)
        );
        // A JSX file calling into a JavaScript module by its `.js` name.
        assert_eq!(
            edge_of(
                &facts,
                "site::app::App",
                &format!("{t}::shout"),
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        // TypeScript importing JavaScript without an extension.
        assert_eq!(
            edge_of(
                &facts,
                "site::mixed::loud",
                &format!("{t}::shout"),
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
    }

    #[test]
    fn scans_typescript_modules() {
        let facts = scan_files(&[
            (
                "web/src/util/text.ts",
                "export function shout(s: string): string {\n  return s.toUpperCase();\n}\nexport class Label {\n  constructor(public text: string) {}\n  show(): string {\n    return shout(this.text);\n  }\n}\nexport interface Named {\n  name(): string;\n}\nexport const LIMIT = 3;\n",
            ),
            ("web/src/util/index.ts", "export * from \"./text\";\n"),
            (
                "web/src/app.tsx",
                "import express from \"express\";\nimport { readFile } from \"node:fs\";\nimport { Label, Named, shout, LIMIT } from \"./util\";\nimport * as text from \"./util/text\";\n\nexport class App implements Named {\n  name(): string {\n    return shout(\"app\") + LIMIT;\n  }\n  view() {\n    const l = new Label(this.name());\n    return <b>{text.shout(l.show())}</b>;\n  }\n}\n",
            ),
        ]);
        assert!(facts.files_with_errors.is_empty());

        let t = "web::src::util::text";
        assert_eq!(kind_of(&facts, t), Some(SymbolKind::Module));
        assert_eq!(
            kind_of(&facts, &format!("{t}::shout")),
            Some(SymbolKind::Function)
        );
        assert_eq!(
            kind_of(&facts, &format!("{t}::Label")),
            Some(SymbolKind::Class)
        );
        assert_eq!(
            kind_of(&facts, &format!("{t}::Named")),
            Some(SymbolKind::Interface)
        );
        assert_eq!(
            kind_of(&facts, &format!("{t}::LIMIT")),
            Some(SymbolKind::Const)
        );
        assert_eq!(
            kind_of(&facts, "web::src::app::App::view"),
            Some(SymbolKind::Method)
        );

        let edge = |src: &str, dst: &str, kind| edge_of(&facts, src, dst, kind);
        // Through `export *` in the directory's index file.
        assert_eq!(
            edge(
                "web::src::app::App::name",
                &format!("{t}::shout"),
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        // Through the namespace import.
        assert_eq!(
            edge(
                "web::src::app::App::view",
                &format!("{t}::shout"),
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "web::src::app::App::view",
                &format!("{t}::Label::constructor"),
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "web::src::app::App::view",
                "web::src::app::App::name",
                EdgeKind::Calls
            ),
            Some(Confidence::Medium)
        );
        assert_eq!(
            edge(
                "web::src::app::App::view",
                &format!("{t}::Label::show"),
                EdgeKind::Calls
            ),
            Some(Confidence::Low)
        );
        assert_eq!(
            edge(
                "web::src::app::App",
                &format!("{t}::Named"),
                EdgeKind::Implements
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "web::src::app::App::name",
                &format!("{t}::LIMIT"),
                EdgeKind::References
            ),
            Some(Confidence::High)
        );
        for target in ["web/src/util/text.ts", "web/src/util/index.ts"] {
            assert!(
                edge(
                    "file:web/src/app.tsx",
                    &format!("file:{target}"),
                    EdgeKind::Imports
                )
                .is_some()
            );
        }

        // Node's own modules are not an integration.
        assert_eq!(
            facts.external_uses,
            vec![ExternalUse {
                file: "web/src/app.tsx".into(),
                krate: "express".into(),
                names: vec!["default".into()],
            }]
        );
    }

    #[test]
    fn scans_java_packages() {
        let text = "src/main/java/com/acme/text";
        let facts = scan_files(&[
            (
                &format!("{text}/Words.java"),
                "package com.acme.text;\n\nimport java.util.Locale;\n\npublic class Words {\n    public static String shout(String s) {\n        return s.toUpperCase(Locale.ROOT);\n    }\n}\n",
            ),
            (
                &format!("{text}/Named.java"),
                "package com.acme.text;\n\npublic interface Named {\n    String name();\n}\n",
            ),
            (
                &format!("{text}/Label.java"),
                "package com.acme.text;\n\npublic class Label implements Named {\n    private final String text;\n\n    public Label(String text) {\n        this.text = text;\n    }\n\n    public String name() {\n        return Words.shout(text);\n    }\n}\n",
            ),
            (
                "src/main/java/com/acme/app/App.java",
                "package com.acme.app;\n\nimport com.acme.text.Label;\nimport com.acme.text.Named;\nimport static com.acme.text.Words.shout;\nimport org.slf4j.Logger;\n\npublic class App implements Named {\n    public String name() {\n        return shout(title());\n    }\n\n    String title() {\n        Label l = new Label(\"x\");\n        return l.name();\n    }\n}\n",
            ),
        ]);
        assert!(facts.files_with_errors.is_empty());

        assert_eq!(kind_of(&facts, "com::acme::text"), Some(SymbolKind::Module));
        assert_eq!(
            kind_of(&facts, "com::acme::text::Label"),
            Some(SymbolKind::Class)
        );
        assert_eq!(
            kind_of(&facts, "com::acme::text::Named"),
            Some(SymbolKind::Interface)
        );
        assert_eq!(
            kind_of(&facts, "com::acme::text::Words::shout"),
            Some(SymbolKind::Method)
        );
        assert_eq!(
            kind_of(&facts, "com::acme::app::App::title"),
            Some(SymbolKind::Method)
        );

        let edge = |src: &str, dst: &str, kind| edge_of(&facts, src, dst, kind);
        // Through the static import.
        assert_eq!(
            edge(
                "com::acme::app::App::name",
                "com::acme::text::Words::shout",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        // A bare call to a member of the same class.
        assert_eq!(
            edge(
                "com::acme::app::App::name",
                "com::acme::app::App::title",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        // A class of the same package needs no import.
        assert_eq!(
            edge(
                "com::acme::text::Label::name",
                "com::acme::text::Words::shout",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "com::acme::app::App::title",
                "com::acme::text::Label::Label",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "com::acme::app::App",
                "com::acme::text::Named",
                EdgeKind::Implements
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "com::acme::text::Label",
                "com::acme::text::Named",
                EdgeKind::Implements
            ),
            Some(Confidence::High)
        );
        // `l.name()` could be any of three `name` methods: no edge.
        assert!(
            !facts
                .edges
                .iter()
                .any(|e| e.src == "com::acme::app::App::title"
                    && e.kind == EdgeKind::Calls
                    && e.dst.ends_with("::name"))
        );
        for target in ["Label.java", "Words.java"] {
            assert!(
                edge(
                    "file:src/main/java/com/acme/app/App.java",
                    &format!("file:{text}/{target}"),
                    EdgeKind::Imports
                )
                .is_some()
            );
        }

        // `java.*` is the platform, not an integration.
        assert_eq!(
            facts.external_uses,
            vec![ExternalUse {
                file: "src/main/java/com/acme/app/App.java".into(),
                krate: "org.slf4j".into(),
                names: vec!["Logger".into()],
            }]
        );
    }

    #[test]
    fn scans_kotlin_and_java_files_of_one_package() {
        let store = "src/main/kotlin/com/acme/store";
        let facts = scan_files(&[
            (
                &format!("{store}/Cart.kt"),
                "package com.acme.store

class Cart(val items: List<Item>) : Priced {
    override fun total(): Int = items.sumOf { it.price }

    fun checkout() {
        Tax.apply(total())
        audit()
    }
}

fun audit() {}
",
            ),
            (
                &format!("{store}/Tax.kt"),
                "package com.acme.store

interface Priced {
    fun total(): Int
}

object Tax {
    const val RATE = 3

    fun rate(): Int = RATE

    fun apply(n: Int): Int = n * rate()
}
",
            ),
            // A Java file in a Kotlin folder, and a package the folder does
            // not name: the declaration wins.
            (
                "src/main/java/elsewhere/Item.java",
                "package com.acme.store;

public class Item {
    public int price;

    public int taxed() {
        return Tax.rate();
    }

    public Cart cart() {
        return new Cart(null);
    }
}
",
            ),
            (
                "src/main/kotlin/com/acme/util/Print.kt",
                "package com.acme.util

fun println(s: String) {}

fun map(): Int = 1
",
            ),
            (
                "src/main/kotlin/com/acme/app/Main.kt",
                "package com.acme.app

import com.acme.store.Cart
import com.acme.store.Tax as T
import com.acme.store.audit
import kotlinx.coroutines.launch
import kotlin.math.max
import java.util.Locale

fun main() {
    val cart = Cart(listOf())
    cart.checkout()
    audit()
    T.rate()
    println(\"x\")
    listOf(1).map { it }
    launch { }
}
",
            ),
        ]);
        assert!(
            facts.files_with_errors.is_empty(),
            "{:?}",
            facts.files_with_errors
        );

        // One module for the package, whichever language wrote it first.
        let modules = facts
            .symbols
            .iter()
            .filter(|s| s.qualified == "com::acme::store")
            .count();
        assert_eq!(modules, 1);
        assert_eq!(
            kind_of(&facts, "com::acme::store::Cart"),
            Some(SymbolKind::Class)
        );
        assert_eq!(
            kind_of(&facts, "com::acme::store::Item"),
            Some(SymbolKind::Class)
        );
        assert_eq!(
            kind_of(&facts, "com::acme::store::audit"),
            Some(SymbolKind::Function)
        );
        assert_eq!(
            kind_of(&facts, "com::acme::store::Tax::RATE"),
            Some(SymbolKind::Const)
        );

        let edge = |src: &str, dst: &str, kind| edge_of(&facts, src, dst, kind);
        // The same package needs no import: a type, a call on an object, a
        // top-level function, a member of the enclosing object.
        assert_eq!(
            edge(
                "com::acme::store::Cart",
                "com::acme::store::Priced",
                EdgeKind::Implements
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "com::acme::store::Cart",
                "com::acme::store::Item",
                EdgeKind::References
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "com::acme::store::Cart::checkout",
                "com::acme::store::Tax::apply",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "com::acme::store::Cart::checkout",
                "com::acme::store::audit",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "com::acme::store::Cart::checkout",
                "com::acme::store::Cart::total",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "com::acme::store::Tax::apply",
                "com::acme::store::Tax::rate",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "com::acme::store::Tax::rate",
                "com::acme::store::Tax::RATE",
                EdgeKind::References
            ),
            Some(Confidence::High)
        );
        // Java calls Kotlin and refers to it.
        assert_eq!(
            edge(
                "com::acme::store::Item::taxed",
                "com::acme::store::Tax::rate",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "com::acme::store::Item::cart",
                "com::acme::store::Cart",
                EdgeKind::References
            ),
            Some(Confidence::High)
        );
        // Through an import, an alias and a receiver of unknown type.
        let main = "com::acme::app::main";
        assert_eq!(
            edge(main, "com::acme::store::audit", EdgeKind::Calls),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(main, "com::acme::store::Tax::rate", EdgeKind::Calls),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(main, "com::acme::store::Cart::checkout", EdgeKind::Calls),
            Some(Confidence::Low)
        );
        assert_eq!(
            edge(main, "com::acme::store::Cart", EdgeKind::References),
            Some(Confidence::High)
        );
        // A default import is no repository function: `println` and `map`
        // are the platform's, though another package has one of each.
        assert!(
            !facts
                .edges
                .iter()
                .any(|e| e.src == main && e.dst.starts_with("com::acme::util")),
            "{:?}",
            facts
                .edges
                .iter()
                .filter(|e| e.src == main)
                .collect::<Vec<_>>()
        );
        for target in ["Cart.kt", "Tax.kt"] {
            assert!(
                edge(
                    "file:src/main/kotlin/com/acme/app/Main.kt",
                    &format!("file:{store}/{target}"),
                    EdgeKind::Imports
                )
                .is_some(),
                "{target}"
            );
        }

        // `kotlin.*` and `java.*` are the platform, `kotlinx.*` a library.
        assert_eq!(
            facts.external_uses,
            vec![ExternalUse {
                file: "src/main/kotlin/com/acme/app/Main.kt".into(),
                krate: "kotlinx.coroutines".into(),
                names: vec!["launch".into()],
            }]
        );
    }

    #[test]
    fn scans_csharp_namespaces() {
        let facts = scan_files(&[
            (
                "src/Text/Words.cs",
                "using System;
using Microsoft.Win32;

namespace Acme.Text;

/// <summary>Word helpers.</summary>
public static class Words
{
    public static string Shout(string s) => s.ToUpperInvariant();
}
",
            ),
            (
                "src/Text/Named.cs",
                "namespace Acme.Text;

public interface INamed
{
    string Name();
}
",
            ),
            (
                "src/Text/Label.cs",
                "namespace Acme.Text;

public partial class Label : INamed
{
    public string Name() => Words.Shout(\"x\");
}
",
            ),
            (
                "src/Text/Label.Extra.cs",
                "namespace Acme.Text;

/// <summary>A label.</summary>
[Serializable]
public partial class Label
{
    public string Extra() => Name();
}
",
            ),
            (
                "src/App/Settings.cs",
                "namespace Acme.App;

public class Settings
{
    public int Level => 1;
}
",
            ),
            (
                "src/App/Sub/Deep.cs",
                "using Acme.Text;
using static Acme.Text.Words;
using Microsoft.Extensions.Logging;
using Serilog.Sinks.File;
using Alias = Newtonsoft.Json.JsonConvert;

namespace Acme.App.Sub;

public class Deep : Settings
{
    public string Run(Label label)
    {
        var s = new Settings();
        label.Extra();
        return Shout(label.Name()) + label.ToString() + Name();
    }

    public string Name() => \"deep\";

    public override string ToString() => \"d\";
}
",
            ),
            (
                "src/App/Globals.cs",
                "global using Acme.Text;
global using Xunit;
",
            ),
            (
                "src/App/Other.cs",
                "namespace Acme.App;

public class Other
{
    public string Go() => Words.Shout(\"y\");
}
",
            ),
        ]);
        assert!(facts.files_with_errors.is_empty());

        assert_eq!(kind_of(&facts, "Acme::Text"), Some(SymbolKind::Module));
        assert_eq!(
            kind_of(&facts, "Acme::Text::Label"),
            Some(SymbolKind::Class)
        );
        assert_eq!(
            kind_of(&facts, "Acme::Text::INamed"),
            Some(SymbolKind::Interface)
        );
        assert_eq!(
            kind_of(&facts, "Acme::Text::Words::Shout"),
            Some(SymbolKind::Method)
        );
        // The parts of a partial type are one symbol, which keeps what any
        // part says about it.
        let labels: Vec<&Symbol> = facts
            .symbols
            .iter()
            .filter(|s| s.qualified == "Acme::Text::Label")
            .collect();
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].file, "src/Text/Label.Extra.cs");
        assert_eq!(labels[0].doc, "A label.");
        assert_eq!(labels[0].attrs, ["Serializable"]);
        assert_eq!(
            kind_of(&facts, "Acme::Text::Label::Extra"),
            Some(SymbolKind::Method)
        );

        let edge = |src: &str, dst: &str, kind| edge_of(&facts, src, dst, kind);
        // A class of the same namespace needs no using.
        assert_eq!(
            edge(
                "Acme::Text::Label::Name",
                "Acme::Text::Words::Shout",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "Acme::Text::Label",
                "Acme::Text::INamed",
                EdgeKind::Implements
            ),
            Some(Confidence::High)
        );
        // A member of the other part, through `this`.
        assert!(
            edge(
                "Acme::Text::Label::Extra",
                "Acme::Text::Label::Name",
                EdgeKind::Calls
            )
            .is_some()
        );
        // The namespaces around one's own are visible without a using.
        assert_eq!(
            edge(
                "Acme::App::Sub::Deep",
                "Acme::App::Settings",
                EdgeKind::Implements
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "Acme::App::Sub::Deep::Run",
                "Acme::App::Settings",
                EdgeKind::References
            ),
            Some(Confidence::High)
        );
        // `using static` and a bare call.
        assert_eq!(
            edge(
                "Acme::App::Sub::Deep::Run",
                "Acme::Text::Words::Shout",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "Acme::App::Sub::Deep::Run",
                "Acme::App::Sub::Deep::Name",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        // `label.ToString()` is the object's own, not `Deep.ToString`.
        assert!(
            !facts
                .edges
                .iter()
                .any(|e| e.dst == "Acme::App::Sub::Deep::ToString" && e.kind == EdgeKind::Calls)
        );
        // A `global using` reaches the other namespaces.
        assert_eq!(
            edge(
                "Acme::App::Other::Go",
                "Acme::Text::Words::Shout",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert!(
            edge(
                "file:src/App/Sub/Deep.cs",
                "file:src/Text/Label.cs",
                EdgeKind::Imports
            )
            .is_some()
                || edge(
                    "file:src/App/Sub/Deep.cs",
                    "file:src/Text/Words.cs",
                    EdgeKind::Imports
                )
                .is_some()
        );

        // `System` and `Microsoft.Win32` are the platform; the rest of
        // `Microsoft` is a package.
        let uses: Vec<(&str, &str, Vec<&str>)> = facts
            .external_uses
            .iter()
            .map(|u| {
                (
                    u.file.as_str(),
                    u.krate.as_str(),
                    u.names.iter().map(String::as_str).collect(),
                )
            })
            .collect();
        assert_eq!(
            uses,
            vec![
                ("src/App/Globals.cs", "Xunit", vec!["*"]),
                (
                    "src/App/Sub/Deep.cs",
                    "Microsoft.Extensions.Logging",
                    vec!["*"]
                ),
                (
                    "src/App/Sub/Deep.cs",
                    "Newtonsoft.Json",
                    vec!["JsonConvert"]
                ),
                ("src/App/Sub/Deep.cs", "Serilog.Sinks", vec!["File.*"]),
            ]
        );
    }

    #[test]
    fn scans_go_modules() {
        let facts = scan_files(&[
            (
                "go.mod",
                "module example.com/app\n\ngo 1.22\n\nrequire (\n\tgithub.com/spf13/cobra v1.8.0\n\tgolang.org/x/sync v0.7.0\n)\n",
            ),
            (
                "internal/store/doc.go",
                "// Package store keeps things.\npackage store\n",
            ),
            (
                "internal/store/store.go",
                "package store\n\nimport (\n\t\"fmt\"\n\t\"example.com/app/internal/words\"\n\t\"golang.org/x/sync/errgroup\"\n\t\"github.com/spf13/cobra\"\n\t\"github.com/unknown/lib/v2/sub\"\n)\n\ntype Reader interface {\n\tRead(key string) string\n}\n\ntype Store struct {\n\ttext.Base\n\titems map[string]string\n}\n\nvar _ Reader = (*Store)(nil)\n\nfunc New() *Store { return &Store{} }\n\nfunc (s *Store) Read(key string) string {\n\tfmt.Println(key)\n\tvar g errgroup.Group\n\t_ = g\n\tcobra.Execute()\n\treturn text.Shout(s.items[key]) + s.pad(key) + helper()\n}\n\nfunc (s *Store) pad(key string) string { return key }\n\nfunc helper() string { return \"\" }\n",
            ),
            (
                "internal/store/store_test.go",
                "package store\n\nimport \"testing\"\n\nfunc TestRead(t *testing.T) {\n\ts := New()\n\ts.Read(\"a\")\n}\n",
            ),
            (
                "internal/store/api_test.go",
                "package store_test\n\nimport (\n\t\"testing\"\n\t\"example.com/app/internal/store\"\n)\n\nfunc TestApi(t *testing.T) { _ = store.New() }\n",
            ),
            // The package is called `text`, whatever its folder says.
            (
                "internal/words/words.go",
                "package text\n\nimport \"strings\"\n\ntype Base struct{}\n\nfunc Shout(s string) string { return strings.ToUpper(s) }\n",
            ),
            (
                "cmd/app/main.go",
                "package main\n\nimport (\n\t\"example.com/app/internal/store\"\n\t\"example.com/app/internal/words\"\n\t. \"example.com/app/internal/words\"\n\t_ \"embed\"\n)\n\nfunc main() {\n\ts := store.New()\n\t_ = text.Shout(\"x\")\n\t_ = s\n}\n",
            ),
            // A second module of the repository, with its own path.
            (
                "tools/gen/go.mod",
                "module example.com/gen\n\nrequire example.com/app v0.0.0\n",
            ),
            (
                "tools/gen/main.go",
                "package main\n\nimport \"example.com/app/internal/words\"\n\nfunc main() { text.Shout(\"gen\") }\n",
            ),
            // The vendored copy is not the repository's code.
            ("vendor/modules.txt", "# github.com/spf13/cobra v1.8.0\n"),
            (
                "vendor/github.com/spf13/cobra/cobra.go",
                "package cobra\n\nfunc Execute() {}\n",
            ),
        ]);
        assert!(
            facts.files_with_errors.is_empty(),
            "{:?}",
            facts.files_with_errors
        );
        assert!(
            facts.files.iter().all(|f| !f.path.starts_with("vendor/")),
            "vendor is skipped"
        );

        let store = "example_com::app::internal::store";
        let words = "example_com::app::internal::words";
        assert_eq!(kind_of(&facts, store), Some(SymbolKind::Module));
        assert_eq!(
            kind_of(&facts, "example_com::app::cmd::app"),
            Some(SymbolKind::Module)
        );
        assert_eq!(
            kind_of(&facts, "example_com::gen"),
            Some(SymbolKind::Module)
        );
        assert_eq!(
            kind_of(&facts, &format!("{store}::Store")),
            Some(SymbolKind::Struct)
        );
        assert_eq!(
            kind_of(&facts, &format!("{store}::Store::Read")),
            Some(SymbolKind::Method)
        );
        // The package stands for its non-test file, with the first doc.
        let package = facts.symbols.iter().find(|s| s.qualified == store).unwrap();
        assert_eq!(package.file, "internal/store/doc.go");
        assert_eq!(package.doc, "Package store keeps things.");
        assert_eq!(
            facts
                .symbols
                .iter()
                .filter(|s| s.kind == SymbolKind::Module && s.file.ends_with("_test.go"))
                .count(),
            1,
            "only the external test package is a module of its own"
        );
        assert_eq!(
            kind_of(&facts, &format!("{store}_test")),
            Some(SymbolKind::Module)
        );

        let edge = |src: &str, dst: &str, kind| edge_of(&facts, src, dst, kind);
        // Through an import whose package calls itself `text`.
        assert_eq!(
            edge(
                &format!("{store}::Store::Read"),
                &format!("{words}::Shout"),
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "example_com::app::cmd::app::main",
                &format!("{store}::New"),
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        // `text.Shout` there: the package is called `text`, its folder `words`.
        assert_eq!(
            edge(
                "example_com::app::cmd::app::main",
                &format!("{words}::Shout"),
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        // The module of another `go.mod` of the repository, by its path.
        assert_eq!(
            edge(
                "example_com::gen::main",
                &format!("{words}::Shout"),
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        // The receiver's own method, a package function.
        assert_eq!(
            edge(
                &format!("{store}::Store::Read"),
                &format!("{store}::Store::pad"),
                EdgeKind::Calls
            ),
            Some(Confidence::Medium)
        );
        assert_eq!(
            edge(
                &format!("{store}::Store::Read"),
                &format!("{store}::helper"),
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        // Embedding and the compile-time assertion are the inheritance.
        assert_eq!(
            edge(
                &format!("{store}::Store"),
                &format!("{words}::Base"),
                EdgeKind::Implements
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                &format!("{store}::Store"),
                &format!("{store}::Reader"),
                EdgeKind::Implements
            ),
            Some(Confidence::High)
        );
        // `s.Read` on a variable: the name alone, and a common one at that.
        assert!(
            edge(
                &format!("{store}::TestRead"),
                &format!("{store}::Store::Read"),
                EdgeKind::Calls
            )
            .is_none()
        );
        // Files: an import of a package reaches its file.
        assert!(
            edge(
                "file:cmd/app/main.go",
                "file:internal/store/doc.go",
                EdgeKind::Imports
            )
            .is_some()
        );

        // The standard library is no integration; a module is named by the
        // requirement that holds the import, else by its host.
        let uses: Vec<(&str, &str, Vec<&str>)> = facts
            .external_uses
            .iter()
            .map(|u| {
                (
                    u.file.as_str(),
                    u.krate.as_str(),
                    u.names.iter().map(String::as_str).collect(),
                )
            })
            .collect();
        assert_eq!(
            uses,
            vec![
                ("internal/store/store.go", "github.com/spf13/cobra", vec![]),
                (
                    "internal/store/store.go",
                    "github.com/unknown/lib/v2",
                    vec!["sub"]
                ),
                (
                    "internal/store/store.go",
                    "golang.org/x/sync",
                    vec!["errgroup"]
                ),
            ]
        );
    }

    #[test]
    fn a_dot_import_opens_a_package_to_bare_names() {
        let facts = scan_files(&[
            (
                "go.mod",
                "module example.com/app
",
            ),
            (
                "words/words.go",
                "package words

func Shout() {}
",
            ),
            (
                "main.go",
                "package main

import . \"example.com/app/words\"

func main() { Shout() }
",
            ),
        ]);
        assert_eq!(
            edge_of(
                &facts,
                "example_com::app::main",
                "example_com::app::words::Shout",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
    }

    #[test]
    fn a_go_file_without_a_module_is_named_by_its_folder() {
        let facts = scan_files(&[
            (
                "tools/gen/main.go",
                "package main\n\nfunc main() { helper() }\n",
            ),
            ("tools/gen/helper.go", "package main\n\nfunc helper() {}\n"),
        ]);
        assert_eq!(kind_of(&facts, "tools::gen"), Some(SymbolKind::Module));
        assert_eq!(
            edge_of(
                &facts,
                "tools::gen::main",
                "tools::gen::helper",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
    }

    #[test]
    fn names_do_not_match_across_languages() {
        let facts = scan_files(&[
            ("Cargo.toml", "[package]\nname = \"demo\"\n"),
            (
                "src/lib.rs",
                "pub fn run(x: &Thing) { x.refresh(); }\npub struct Thing;\n",
            ),
            (
                "scripts/job.py",
                "class Job:\n    def refresh(self):\n        pass\n",
            ),
        ]);
        assert_eq!(
            kind_of(&facts, "scripts::job::Job::refresh"),
            Some(SymbolKind::Method)
        );
        assert!(
            !facts
                .edges
                .iter()
                .any(|e| e.src == "demo::run" && e.kind == EdgeKind::Calls)
        );
    }

    /// What `file` imports from `package`.
    fn imported<'a>(facts: &'a RepoFacts, file: &str, package: &str) -> Vec<&'a str> {
        facts
            .external_uses
            .iter()
            .find(|u| u.file == file && u.krate == package)
            .unwrap_or_else(|| panic!("{file} does not use {package}"))
            .names
            .iter()
            .map(String::as_str)
            .collect()
    }

    #[test]
    fn an_outside_package_keeps_the_names_a_file_takes_from_it() {
        let facts = scan_files(&[
            (
                "Cargo.toml",
                "[package]\nname = \"demo\"\n[dependencies]\nserde = \"1\"\naxum = \"0\"\ntokio = \"1\"\n",
            ),
            (
                "src/lib.rs",
                "use axum::extract::{Path, State};\nuse axum::Router as App;\nuse serde::de::{self, Visitor};\nuse serde::Serialize;\nuse tokio;\nuse tokio::sync::*;\n",
            ),
            (
                "tools/fetch.py",
                "import requests\nfrom requests import Session\nfrom requests.adapters import HTTPAdapter as Adapter\nfrom yaml import *\n",
            ),
            (
                "web/app.ts",
                "import React, { useState, useEffect as effect } from \"react\";\nimport * as path from \"lodash\";\nimport \"reflect-metadata\";\nexport { z } from \"zod\";\nimport { Client } from \"@scope/sdk/client\";\n",
            ),
            (
                "src/main/java/p/A.java",
                "package p;\n\nimport org.slf4j.Logger;\nimport org.slf4j.LoggerFactory;\nimport com.google.common.collect.*;\nimport static org.junit.Assert.assertEquals;\n\nclass A {}\n",
            ),
        ]);
        // Rust: the path after the crate, the written name and not its
        // alias; a module is left out when a name inside it is listed.
        assert_eq!(
            imported(&facts, "src/lib.rs", "axum"),
            ["Router", "extract::Path", "extract::State"]
        );
        assert_eq!(
            imported(&facts, "src/lib.rs", "serde"),
            ["Serialize", "de::Visitor"]
        );
        assert_eq!(imported(&facts, "src/lib.rs", "tokio"), ["sync::*"]);

        assert_eq!(
            imported(&facts, "tools/fetch.py", "requests"),
            ["Session", "adapters.HTTPAdapter"]
        );
        assert_eq!(imported(&facts, "tools/fetch.py", "yaml"), ["*"]);

        assert_eq!(
            imported(&facts, "web/app.ts", "react"),
            ["default", "useEffect", "useState"]
        );
        assert_eq!(imported(&facts, "web/app.ts", "lodash"), ["*"]);
        assert!(imported(&facts, "web/app.ts", "reflect-metadata").is_empty());
        assert_eq!(imported(&facts, "web/app.ts", "zod"), ["z"]);
        assert_eq!(imported(&facts, "web/app.ts", "@scope/sdk"), ["Client"]);

        let java = "src/main/java/p/A.java";
        assert_eq!(
            imported(&facts, java, "org.slf4j"),
            ["Logger", "LoggerFactory"]
        );
        assert_eq!(imported(&facts, java, "com.google"), ["common.collect.*"]);
        assert_eq!(imported(&facts, java, "org.junit"), ["Assert.assertEquals"]);
    }

    #[test]
    fn a_call_edge_keeps_the_place_of_the_first_call() {
        let facts = scan_files(&[
            (
                "Cargo.toml",
                "[package]
name = \"demo\"
",
            ),
            (
                "src/lib.rs",
                "pub fn run() {
    last(first());
    middle();
    first();
}
fn first() -> u8 { 0 }
fn middle() {}
fn last(_: u8) {}
pub const LIMIT: u8 = 1;
pub fn read() -> u8 { LIMIT }
",
            ),
        ]);
        let mut calls: Vec<(u32, &str)> = facts
            .edges
            .iter()
            .filter(|e| e.src == "demo::run" && e.kind == EdgeKind::Calls)
            .map(|e| (e.site, e.dst.as_str()))
            .collect();
        calls.sort();
        // `first` is called twice and stands where it is called first.
        let order: Vec<&str> = calls.iter().map(|(_, dst)| *dst).collect();
        assert_eq!(order, ["demo::last", "demo::first", "demo::middle"]);
        assert!(calls.iter().all(|(site, _)| *site > 0));
        // What is no call has no place.
        let read = facts
            .edges
            .iter()
            .find(|e| e.src == "demo::read" && e.dst == "demo::LIMIT")
            .unwrap();
        assert_eq!((read.kind, read.site), (EdgeKind::References, 0));
    }

    #[test]
    fn a_name_matches_a_method_only_in_a_crate_the_caller_depends_on() {
        let call = "pub fn draw(d: gpui::Div) -> gpui::Div { d.size(4.0) }\n";
        let facts = scan_files(&[
            ("Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n"),
            ("crates/models/Cargo.toml", "[package]\nname = \"models\"\n"),
            (
                "crates/models/src/lib.rs",
                "pub struct Entry;\nimpl Entry {\n    pub fn size(&self) -> u64 { 0 }\n}\n",
            ),
            // `app` does not depend on `models`: its `size` is gpui's.
            (
                "crates/app/Cargo.toml",
                "[package]\nname = \"app\"\n[dependencies]\ngpui = \"0\"\n",
            ),
            ("crates/app/src/lib.rs", call),
            // `ui` does, under a target table.
            (
                "crates/ui/Cargo.toml",
                "[package]\nname = \"ui\"\n[target.'cfg(windows)'.dependencies]\nmodels = { path = \"../models\" }\n",
            ),
            ("crates/ui/src/lib.rs", call),
        ]);
        let size = "models::Entry::size";
        assert_eq!(edge_of(&facts, "app::draw", size, EdgeKind::Calls), None);
        assert_eq!(
            edge_of(&facts, "ui::draw", size, EdgeKind::Calls),
            Some(Confidence::Low)
        );
    }

    fn symbol_of<'f>(facts: &'f RepoFacts, qualified: &str) -> &'f Symbol {
        facts
            .symbols
            .iter()
            .find(|s| s.qualified == qualified)
            .unwrap_or_else(|| {
                let all: Vec<&str> = facts.symbols.iter().map(|s| s.qualified.as_str()).collect();
                panic!("no {qualified} in {all:#?}")
            })
    }

    #[test]
    fn scans_a_c_project_with_header_and_source_pairs() {
        let facts = scan_files(&[
            (
                "CMakeLists.txt",
                "project(demo)\nfind_package(ZLIB REQUIRED)\nadd_library(net src/net/socket.c)\ntarget_include_directories(net PUBLIC include)\nadd_executable(app src/main.c)\ntarget_link_libraries(app net)\n",
            ),
            (
                "include/net/socket.h",
                "#ifndef NET_SOCKET_H\n#define NET_SOCKET_H\n#include <stdint.h>\n\n#define SOCKET_MAX 64\n#define SOCKET_OK(s) ((s) != NULL)\n\n/* A connection. */\ntypedef struct socket {\n    int fd;\n} socket_t;\n\ntypedef enum { SOCK_TCP, SOCK_UDP } sock_kind;\n\n/* Open a socket. */\nsocket_t *socket_open(int port);\nint socket_close(socket_t *s);\nextern int socket_count;\n#endif\n",
            ),
            (
                "src/net/socket.c",
                "#include \"net/socket.h\"\n#include <zlib.h>\n#include <stdio.h>\n\nint socket_count = 0;\n\nstatic int next_fd(void) { return socket_count++; }\n\nsocket_t *socket_open(int port) {\n    socket_t *s = 0;\n    next_fd();\n    printf(\"%d\", port);\n    return s;\n}\n\nint socket_close(socket_t *s) {\n    return SOCKET_OK(s) ? 0 : SOCKET_MAX;\n}\n",
            ),
            (
                "src/main.c",
                "#include \"net/socket.h\"\n\nint main(int argc, char **argv) {\n    socket_t *s = socket_open(80);\n    sock_kind k = SOCK_TCP;\n    return socket_close(s);\n}\n",
            ),
        ]);
        assert!(
            facts.files_with_errors.is_empty(),
            "{:?}",
            facts.files_with_errors
        );
        // The header and the source are one module, standing at the header.
        let module = symbol_of(&facts, "net::socket");
        assert_eq!(module.kind, SymbolKind::Module);
        assert_eq!(module.file, "include/net/socket.h");
        assert_eq!(
            facts
                .symbols
                .iter()
                .filter(|s| s.qualified == "net::socket")
                .count(),
            1
        );
        // A declaration and its definition are one symbol at the header, and
        // name the file that defines it.
        let open = symbol_of(&facts, "net::socket::socket_open");
        assert_eq!(open.kind, SymbolKind::Function);
        assert_eq!(open.file, "include/net/socket.h");
        assert_eq!(open.doc, "Open a socket.");
        assert_eq!(open.visibility, Visibility::Public);
        assert!(open.attrs.contains(&"impl:src/net/socket.c".to_string()));
        assert!(!open.attrs.iter().any(|a| a == "declaration"));
        // `static` is private; a definition no header declares is internal.
        assert_eq!(
            symbol_of(&facts, "net::socket::next_fd").visibility,
            Visibility::Private
        );
        let count = symbol_of(&facts, "net::socket::socket_count");
        assert_eq!(
            (count.kind, count.file.as_str()),
            (SymbolKind::Static, "include/net/socket.h")
        );
        // A typedef'd struct is one struct; the enum has constants.
        assert_eq!(
            kind_of(&facts, "net::socket::socket_t"),
            Some(SymbolKind::Struct)
        );
        assert_eq!(kind_of(&facts, "net::socket::socket"), None);
        assert_eq!(
            kind_of(&facts, "net::socket::sock_kind"),
            Some(SymbolKind::Enum)
        );
        assert_eq!(
            kind_of(&facts, "net::socket::sock_kind::SOCK_UDP"),
            Some(SymbolKind::Const)
        );
        // A constant macro and a function-like macro are symbols; the
        // include guard is none.
        assert_eq!(
            kind_of(&facts, "net::socket::SOCKET_MAX"),
            Some(SymbolKind::Const)
        );
        assert_eq!(
            kind_of(&facts, "net::socket::SOCKET_OK"),
            Some(SymbolKind::Macro)
        );
        assert_eq!(kind_of(&facts, "net::socket::NET_SOCKET_H"), None);
        // `main` of a program of its own.
        assert_eq!(kind_of(&facts, "main::main"), Some(SymbolKind::Function));

        let edge = |src: &str, dst: &str, kind| edge_of(&facts, src, dst, kind);
        // Through the include: the header's names are the file's.
        assert_eq!(
            edge("main::main", "net::socket::socket_open", EdgeKind::Calls),
            Some(Confidence::High)
        );
        assert_eq!(
            edge("main::main", "net::socket::socket_close", EdgeKind::Calls),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "main::main",
                "net::socket::sock_kind::SOCK_TCP",
                EdgeKind::References
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge("main::main", "net::socket::socket_t", EdgeKind::References),
            Some(Confidence::High)
        );
        // Within the module, and a macro invoked like a function.
        assert_eq!(
            edge(
                "net::socket::socket_open",
                "net::socket::next_fd",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "net::socket::socket_close",
                "net::socket::SOCKET_OK",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "net::socket::socket_close",
                "net::socket::SOCKET_MAX",
                EdgeKind::References
            ),
            Some(Confidence::High)
        );
        // `printf` is nobody's.
        assert!(!facts.edges.iter().any(|e| e.dst.ends_with("printf")));
        assert!(
            edge(
                "file:src/main.c",
                "file:include/net/socket.h",
                EdgeKind::Imports
            )
            .is_some()
        );
        assert!(
            edge(
                "file:src/net/socket.c",
                "file:include/net/socket.h",
                EdgeKind::Imports
            )
            .is_some()
        );
        // The standard library is no integration; `zlib.h` is the library the
        // repository asks for.
        assert_eq!(
            facts.external_uses,
            vec![ExternalUse {
                file: "src/net/socket.c".into(),
                krate: "ZLIB".into(),
                names: vec![],
            }]
        );
    }

    #[test]
    fn scans_a_cpp_project_with_a_class_across_header_and_source() {
        let facts = scan_files(&[
            (
                "src/shapes/shape.hpp",
                "#pragma once\nnamespace shapes {\n\n/// Anything with an area.\nclass Shape {\npublic:\n    virtual ~Shape() = default;\n    virtual double area() const = 0;\n    virtual const char* name() const = 0;\n};\n\n}\n",
            ),
            (
                "src/shapes/circle.hpp",
                "#pragma once\n#include \"shape.hpp\"\n#include <vector>\nnamespace shapes {\n\ntemplate <typename T>\nT square(T x);\n\n/// A circle.\nclass Circle : public Shape {\npublic:\n    explicit Circle(double r);\n    double area() const override;\n    const char* name() const override { return \"circle\"; }\n    static Circle unit();\nprivate:\n    double scale() const;\n    double r_;\n};\n\n}\n",
            ),
            (
                "src/shapes/circle.cpp",
                "#include \"circle.hpp\"\n\nnamespace shapes {\n\nCircle::Circle(double r) : r_(r) {}\n\ndouble Circle::area() const { return square(r_) * scale(); }\n\ndouble Circle::scale() const { return 3.14; }\n\nCircle Circle::unit() { return Circle(1.0); }\n\ntemplate <typename T>\nT square(T x) { return x * x; }\n\n}\n",
            ),
            (
                "src/app/main.cpp",
                "#include \"../shapes/circle.hpp\"\n#include <iostream>\n\nstatic double total(const shapes::Shape& s) { return s.area(); }\n\nint main() {\n    shapes::Circle c(2.0);\n    auto u = shapes::Circle::unit();\n    std::cout << total(c) << u.area();\n    shapes::Shape* p = new shapes::Circle(3.0);\n    return p->name() == nullptr;\n}\n",
            ),
        ]);
        assert!(
            facts.files_with_errors.is_empty(),
            "{:?}",
            facts.files_with_errors
        );
        // The pure virtual class is the nearest thing to an interface.
        let shape = symbol_of(&facts, "shapes::shape::Shape");
        assert_eq!(shape.kind, SymbolKind::Interface);
        assert_eq!(shape.doc, "Anything with an area.");
        assert!(shape.attrs.contains(&"namespace:shapes".to_string()));
        let circle = symbol_of(&facts, "shapes::circle::Circle");
        assert_eq!(circle.kind, SymbolKind::Class);
        assert_eq!(circle.file, "src/shapes/circle.hpp");
        // The out-of-line definitions are the class's methods, one symbol
        // each, at the header, and say where they are defined.
        let area = symbol_of(&facts, "shapes::circle::Circle::area");
        assert_eq!(area.kind, SymbolKind::Method);
        assert_eq!(area.file, "src/shapes/circle.hpp");
        assert!(
            area.attrs
                .contains(&"impl:src/shapes/circle.cpp".to_string())
        );
        assert_eq!(
            symbol_of(&facts, "shapes::circle::Circle::scale").visibility,
            Visibility::Private
        );
        assert_eq!(
            kind_of(&facts, "shapes::circle::Circle::Circle"),
            Some(SymbolKind::Method)
        );
        assert_eq!(
            kind_of(&facts, "shapes::circle::Circle::unit"),
            Some(SymbolKind::Method)
        );
        assert_eq!(
            facts
                .symbols
                .iter()
                .filter(|s| s.qualified == "shapes::circle::Circle::area")
                .count(),
            1
        );
        // A template function is one symbol, its definition in the source.
        let square = symbol_of(&facts, "shapes::circle::square");
        assert_eq!(square.kind, SymbolKind::Function);
        assert!(square.attrs.contains(&"template".to_string()));
        // Inheritance, through the include.
        assert_eq!(
            edge_of(
                &facts,
                "shapes::circle::Circle",
                "shapes::shape::Shape",
                EdgeKind::Implements
            ),
            Some(Confidence::High)
        );
        let edge = |src: &str, dst: &str, kind| edge_of(&facts, src, dst, kind);
        // A call inside a method finds a sibling, and a template.
        assert_eq!(
            edge(
                "shapes::circle::Circle::area",
                "shapes::circle::Circle::scale",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "shapes::circle::Circle::area",
                "shapes::circle::square",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        // `Namespace::Class::method()` and a constructor.
        assert_eq!(
            edge(
                "app::main::main",
                "shapes::circle::Circle::unit",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge(
                "app::main::main",
                "shapes::circle::Circle::Circle",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        // `s.area()` on a parameter: two classes have an `area`, so nothing
        // is claimed.
        assert!(
            edge(
                "app::main::total",
                "shapes::circle::Circle::area",
                EdgeKind::Calls
            )
            .is_none()
        );
        // The standard library never binds.
        assert!(!facts.edges.iter().any(|e| e.dst.contains("cout")));
        assert_eq!(facts.external_uses, vec![]);
    }

    #[test]
    fn a_mixed_repository_keeps_c_and_cpp_apart_and_together() {
        let facts = scan_files(&[
            (
                "core/buf.h",
                "#ifdef __cplusplus\nextern \"C\" {\n#endif\nint buf_len(const char *b);\n#ifdef __cplusplus\n}\n#endif\n",
            ),
            (
                "core/buf.c",
                "#include \"buf.h\"\nint buf_len(const char *b) { return 0; }\n",
            ),
            (
                "app/wrap.hpp",
                "#include \"../core/buf.h\"\nclass Wrap { public: int size() const; };\n",
            ),
            (
                "app/wrap.cpp",
                "#include \"wrap.hpp\"\nint Wrap::size() const { return buf_len(\"x\"); }\n",
            ),
        ]);
        let language = |path: &str| {
            facts
                .files
                .iter()
                .find(|f| f.path == path)
                .map(|f| f.language)
                .unwrap()
        };
        assert_eq!(language("core/buf.h"), Language::C);
        assert_eq!(language("core/buf.c"), Language::C);
        assert_eq!(language("app/wrap.hpp"), Language::Cpp);
        assert_eq!(language("app/wrap.cpp"), Language::Cpp);
        // C++ calls the C function through the header, and the C and C++
        // files of the repository are mapped as one.
        assert_eq!(
            edge_of(
                &facts,
                "app::wrap::Wrap::size",
                "core::buf::buf_len",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(kind_of(&facts, "app::wrap::Wrap"), Some(SymbolKind::Class));
    }

    #[test]
    fn a_dot_h_with_cpp_in_it_is_parsed_as_cpp() {
        let facts = scan_files(&[
            (
                "inc/shape.h",
                "namespace g { class Shape { public: virtual ~Shape(); int id() const; }; }\n",
            ),
            ("inc/plain.h", "int plain(int x);\n"),
        ]);
        let language = |path: &str| {
            facts
                .files
                .iter()
                .find(|f| f.path == path)
                .map(|f| f.language)
                .unwrap()
        };
        assert_eq!(language("inc/shape.h"), Language::Cpp);
        assert_eq!(language("inc/plain.h"), Language::C);
        assert_eq!(kind_of(&facts, "shape::Shape"), Some(SymbolKind::Class));
        assert_eq!(
            kind_of(&facts, "shape::Shape::id"),
            Some(SymbolKind::Method)
        );
        assert_eq!(kind_of(&facts, "plain::plain"), Some(SymbolKind::Function));
    }

    #[test]
    fn a_vendored_tree_is_not_parsed_beside_the_projects_own_code() {
        let facts = scan_files(&[
            (
                "src/app.c",
                "#include <png/png.h>\nint app(void) { return png_ok(); }\n",
            ),
            ("third_party/png/png.h", "int png_ok(void);\n"),
            ("third_party/png/png.c", "int png_ok(void) { return 1; }\n"),
        ]);
        assert!(
            !facts
                .symbols
                .iter()
                .any(|s| s.file.starts_with("third_party/"))
        );
        assert_eq!(kind_of(&facts, "app::app"), Some(SymbolKind::Function));
        assert!(!facts.edges.iter().any(|e| e.dst.contains("png_ok")));
        // The folder names the library the include reaches.
        assert_eq!(
            facts.external_uses,
            vec![ExternalUse {
                file: "src/app.c".into(),
                krate: "png".into(),
                names: vec!["png/png.h".into()],
            }]
        );
        // The files are known all the same.
        assert!(
            facts
                .files
                .iter()
                .any(|f| f.path == "third_party/png/png.c")
        );
    }

    #[test]
    fn a_function_defined_in_another_module_joins_its_declaration() {
        let facts = scan_files(&[
            (
                "api.h",
                "/* Run it. */\nint api_run(int n);\nint api_other(void);\n",
            ),
            (
                "impl_linux.c",
                "#include \"api.h\"\n\nstatic int helper(int n) { return n + 1; }\n\nint api_run(int n) { return helper(n); }\n",
            ),
            (
                "impl_other.c",
                "static int helper(int n) { return n + 2; }\nint unrelated(int n) { return helper(n); }\n",
            ),
            (
                "user.c",
                "#include \"api.h\"\nint use_it(void) { return api_run(1) + api_other(); }\n",
            ),
        ]);
        // One symbol at the header, with the file that defines it.
        let run = symbol_of(&facts, "api::api_run");
        assert_eq!(run.file, "api.h");
        assert_eq!(run.doc, "Run it.");
        assert!(run.attrs.contains(&"impl:impl_linux.c".to_string()));
        assert!(
            !facts
                .symbols
                .iter()
                .any(|s| s.qualified == "impl_linux::api_run")
        );
        // What the definition's body calls belongs to the merged symbol, and
        // each static `helper` is the one of its own module.
        assert_eq!(
            edge_of(
                &facts,
                "api::api_run",
                "impl_linux::helper",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge_of(
                &facts,
                "impl_other::unrelated",
                "impl_other::helper",
                EdgeKind::Calls
            ),
            Some(Confidence::High)
        );
        assert_eq!(
            edge_of(
                &facts,
                "impl_other::unrelated",
                "impl_linux::helper",
                EdgeKind::Calls
            ),
            None
        );
        // A call through the header finds it, and a function nothing defines
        // is still the header's.
        assert_eq!(
            edge_of(&facts, "user::use_it", "api::api_run", EdgeKind::Calls),
            Some(Confidence::High)
        );
        assert_eq!(
            edge_of(&facts, "user::use_it", "api::api_other", EdgeKind::Calls),
            Some(Confidence::High)
        );
    }

    #[test]
    fn two_headers_that_declare_a_name_leave_it_unbound() {
        let facts = scan_files(&[
            ("a.h", "int shared_name(void);\n"),
            ("b.h", "int shared_name(void);\n"),
            (
                "use.c",
                "#include \"a.h\"\n#include \"b.h\"\nint f(void) { return shared_name(); }\n",
            ),
            (
                "only_a.c",
                "#include \"a.h\"\nint g(void) { return shared_name(); }\n",
            ),
        ]);
        // Declared by two headers at one distance: no claim.
        assert_eq!(
            edge_of(&facts, "use::f", "a::shared_name", EdgeKind::Calls),
            None
        );
        assert_eq!(
            edge_of(&facts, "use::f", "b::shared_name", EdgeKind::Calls),
            None
        );
        // One header sees only its own.
        assert_eq!(
            edge_of(&facts, "only_a::g", "a::shared_name", EdgeKind::Calls),
            Some(Confidence::High)
        );
    }
}
