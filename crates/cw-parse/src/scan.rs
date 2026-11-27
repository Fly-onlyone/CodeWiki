//! Walk a repository and produce every fact the index needs.

use std::collections::HashSet;
use std::path::Path;

use cw_core::{
    Edge, Error, FileRecord, Language, Result, Symbol, SymbolKind, VAULT_DIR, Visibility,
    sha256_hex,
};
use ignore::WalkBuilder;

use crate::csharp::parse_csharp;
use crate::java::parse_java;
use crate::lang::{link_imports, nuget_ids, python_module, resolve_ts_import, ts_module};
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
            !(is_dir
                && (SKIP_DIRS.contains(&name.as_ref()) || (e.depth() == 1 && name == VAULT_DIR)))
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

    for path in &paths {
        let full = root.join(path);
        let meta = std::fs::metadata(&full).map_err(|e| Error::io(&full, e))?;
        let language = Language::from_path(path);
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
                Language::CSharp => parse_csharp(path, &source)?,
                _ => continue,
            };
            if file_parsed.has_errors {
                facts.files_with_errors.push(path.clone());
            }
            for symbol in &file_parsed.symbols {
                // Overloads, stubs, the files of one Java package and the
                // parts of a C# partial type share a name; keep the first.
                if seen.insert(symbol.qualified.clone()) {
                    facts.symbols.push(symbol.clone());
                } else if language == Language::CSharp {
                    merge_partial(&mut facts.symbols, symbol);
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
    link_imports(&mut parsed, &nuget);
    // The first segment of every Python, TypeScript, Java and C# module is a
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
}
