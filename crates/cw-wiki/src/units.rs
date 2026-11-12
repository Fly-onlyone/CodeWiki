//! Units: the top-level parts of a repository, each with its own folder under
//! `01-Core`.
//!
//! A Cargo package is a unit. Code in other languages has no manifest the
//! scanner understands, so its units are derived here from the layout: a
//! Python top-level package, a JavaScript or TypeScript package directory, a
//! Java build module. Every unit has the same shape, so planning and
//! rendering treat them alike.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use cw_core::{EdgeKind, Language, Symbol, SymbolKind};
use cw_parse::{MAIN_GUARD_ATTR, RepoFacts};

/// Directories a bundler writes to; `main` in `package.json` often points
/// into one while the source sits under `src`.
const JS_OUTPUT_DIRS: [&str; 4] = ["dist", "build", "lib", "out"];
const JS_EXTENSIONS: [&str; 8] = ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];
const PYTHON_MANIFESTS: [&str; 5] = [
    "pyproject.toml",
    "setup.py",
    "setup.cfg",
    "requirements.txt",
    "Pipfile",
];
const JAVA_MANIFESTS: [&str; 3] = ["pom.xml", "build.gradle", "build.gradle.kts"];

/// A build or dependency file that belongs to a unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// Repo-relative path.
    pub path: String,
    /// Contents, empty when the file was not read.
    pub text: String,
}

/// A symbol where execution starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Qualified name of the function or module that runs first.
    pub symbol: String,
    /// True when the entry is the unit's one obvious way in: a Rust `main`,
    /// `__main__.py`, an entry named in `package.json`, or the only Java
    /// `main` method.
    pub declared: bool,
}

/// One top-level part of the repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// Stable key, unique in the repository; note ids are built from it.
    pub name: String,
    /// Name shown to readers.
    pub package: String,
    /// Folder under `01-Core` and tag; safe in a path and in frontmatter.
    pub slug: String,
    /// Repo-relative directory, empty for the repository root.
    pub dir: String,
    pub description: String,
    /// Names of the units and outside packages this unit depends on.
    pub deps: Vec<String>,
    pub has_lib: bool,
    pub has_bin: bool,
    pub language: Language,
    /// Build files, the primary one first.
    pub manifests: Vec<Manifest>,
    /// Qualified name of the module every other module of the unit nests in.
    pub root: Option<String>,
    /// Leading segments of a qualified name that say nothing inside the unit.
    pub strip: usize,
    pub entries: Vec<Entry>,
    /// Script names from `package.json`.
    pub scripts: Vec<String>,
}

impl Unit {
    pub fn is_rust(&self) -> bool {
        self.language == Language::Rust
    }

    /// True when a build file of the unit mentions `package` as a whole word.
    pub fn declares(&self, package: &str) -> Vec<&str> {
        let needle = package.to_lowercase();
        let part = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-');
        self.manifests
            .iter()
            .filter(|m| {
                let text = m.text.to_lowercase();
                text.match_indices(&needle).any(|(at, _)| {
                    !text[..at].chars().next_back().is_some_and(part)
                        && !text[at + needle.len()..].chars().next().is_some_and(part)
                })
            })
            .map(|m| m.path.as_str())
            .collect()
    }
}

/// Every unit of a repository and the lookup from code to unit.
#[derive(Debug, Default)]
pub struct Units {
    pub list: Vec<Unit>,
    /// Crate name as used in paths -> unit.
    rust: HashMap<String, usize>,
    /// Source file of another language -> unit.
    by_file: HashMap<String, usize>,
}

impl Units {
    /// Index of the unit that holds `symbol`.
    pub fn index_of(&self, symbol: &Symbol) -> Option<usize> {
        if Language::from_path(&symbol.file) == Language::Rust {
            self.rust_index(&symbol.qualified)
        } else {
            self.by_file.get(&symbol.file).copied()
        }
    }

    /// Index of the crate a Rust path starts in.
    pub fn rust_index(&self, qualified: &str) -> Option<usize> {
        self.rust
            .get(qualified.split("::").next().unwrap_or(""))
            .copied()
    }

    pub fn by_name(&self, name: &str) -> Option<&Unit> {
        self.list.iter().find(|u| u.name == name)
    }

    pub fn all_rust(&self) -> bool {
        self.list.iter().all(Unit::is_rust)
    }
}

/// Language family: one unit never mixes two of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Family {
    Python,
    Js,
    Java,
}

impl Family {
    fn of(language: Language) -> Option<Self> {
        match language {
            Language::Python => Some(Self::Python),
            Language::TypeScript | Language::JavaScript => Some(Self::Js),
            Language::Java => Some(Self::Java),
            _ => None,
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Js => "js",
            Self::Java => "java",
        }
    }
}

/// True for a test file of a language other than Rust.
pub fn is_test_file(path: &str) -> bool {
    let mut dirs: Vec<&str> = path.split('/').collect();
    let file = dirs.pop().unwrap_or("");
    if dirs.iter().any(|d| {
        matches!(
            *d,
            "tests" | "test" | "__tests__" | "spec" | "e2e" | "testing"
        )
    }) {
        return true;
    }
    let stem = file.split('.').next().unwrap_or(file);
    file.starts_with("test_")
        || file == "conftest.py"
        || file.contains(".test.")
        || file.contains(".spec.")
        || stem.ends_with("_test")
        || (file.ends_with(".java")
            && (stem.ends_with("Test") || stem.ends_with("Tests") || stem.ends_with("IT")))
}

/// Lowercase name safe as a folder and as a tag.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() || c == '_' {
            out.extend(c.to_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "code".to_string()
    } else {
        out
    }
}

fn parent_dir(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

fn join(dir: &str, file: &str) -> String {
    if dir.is_empty() {
        file.to_string()
    } else {
        format!("{dir}/{file}")
    }
}

/// `dir` and each of its ancestors, nearest first, ending at the root (`""`).
fn ancestors(dir: &str) -> Vec<&str> {
    let mut out = vec![dir];
    let mut current = dir;
    while let Some((parent, _)) = current.rsplit_once('/') {
        out.push(parent);
        current = parent;
    }
    if !dir.is_empty() {
        out.push("");
    }
    out
}

/// Nearest directory at or above `dir` that holds one of `names`.
fn nearest_with<'a>(files: &HashSet<&str>, dir: &'a str, names: &[&str]) -> Option<&'a str> {
    ancestors(dir).into_iter().find(|d| {
        names
            .iter()
            .any(|name| files.contains(join(d, name).as_str()))
    })
}

/// Source file a `package.json` path names. Published packages point at
/// build output, so the matching file under `src` is tried as well.
fn resolve_js_entry(files: &HashSet<&str>, dir: &str, spec: &str) -> Option<String> {
    let spec = spec.trim().trim_start_matches("./");
    if spec.is_empty() || spec.starts_with("..") {
        return None;
    }
    let stem = JS_EXTENSIONS
        .iter()
        .find_map(|ext| spec.strip_suffix(&format!(".{ext}")))
        .unwrap_or(spec);
    let mut stems = vec![stem.to_string()];
    if let Some((first, rest)) = stem.split_once('/')
        && JS_OUTPUT_DIRS.contains(&first)
    {
        stems.push(format!("src/{rest}"));
    }
    stems
        .iter()
        .flat_map(|s| {
            JS_EXTENSIONS
                .iter()
                .map(move |ext| join(dir, &format!("{s}.{ext}")))
        })
        .find(|candidate| files.contains(candidate.as_str()))
}

/// What a `package.json` says about its package.
#[derive(Default)]
struct PackageJson {
    name: String,
    description: String,
    /// Paths named by `main`, `bin` and the `start` script, in that order.
    entries: Vec<String>,
    scripts: Vec<String>,
}

fn parse_package_json(text: &str) -> PackageJson {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return PackageJson::default();
    };
    let text_of = |key: &str| {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    };
    let mut entries = vec![text_of("main")];
    match value.get("bin") {
        Some(serde_json::Value::String(path)) => entries.push(path.clone()),
        Some(serde_json::Value::Object(map)) => {
            entries.extend(map.values().filter_map(|v| v.as_str()).map(str::to_string));
        }
        _ => {}
    }
    let scripts = value.get("scripts").and_then(|s| s.as_object());
    if let Some(start) = scripts
        .and_then(|s| s.get("start"))
        .and_then(|s| s.as_str())
    {
        entries.extend(
            start
                .split_whitespace()
                .filter(|token| {
                    JS_EXTENSIONS
                        .iter()
                        .any(|ext| token.ends_with(&format!(".{ext}")))
                })
                .map(str::to_string),
        );
    }
    entries.retain(|e| !e.is_empty());
    PackageJson {
        name: text_of("name"),
        description: text_of("description"),
        entries,
        scripts: scripts
            .map(|s| s.keys().cloned().collect())
            .unwrap_or_default(),
    }
}

/// Files of one unit that is still being put together.
#[derive(Default)]
struct Group<'a> {
    /// Directory of the manifest that defines the unit, when one does.
    manifest_dir: Option<String>,
    files: Vec<&'a str>,
}

/// Find the units of a repository. `repo_name` names a unit that is the
/// repository itself; `read` returns the text of a repo-relative file.
pub fn discover(
    facts: &RepoFacts,
    repo_name: &str,
    read: &dyn Fn(&str) -> Option<String>,
) -> Units {
    let mut units = Units::default();
    let symbols: HashMap<&str, &Symbol> = facts
        .symbols
        .iter()
        .map(|s| (s.qualified.as_str(), s))
        .collect();
    for krate in &facts.crates {
        let entry = [
            format!("{}::main", krate.name),
            format!("{}::main::main", krate.name),
        ]
        .into_iter()
        .find(|q| {
            symbols
                .get(q.as_str())
                .is_some_and(|s| s.kind == SymbolKind::Function)
        });
        units.rust.insert(krate.name.clone(), units.list.len());
        units.list.push(Unit {
            name: krate.name.clone(),
            package: krate.package.clone(),
            slug: krate.package.clone(),
            dir: krate.dir.clone(),
            description: krate.description.clone(),
            deps: krate.deps.clone(),
            has_lib: krate.has_lib,
            has_bin: krate.has_bin,
            language: Language::Rust,
            manifests: vec![Manifest {
                path: join(&krate.dir, "Cargo.toml"),
                text: String::new(),
            }],
            root: symbols
                .contains_key(krate.name.as_str())
                .then(|| krate.name.clone()),
            strip: 1,
            entries: entry
                .filter(|_| krate.has_bin)
                .map(|symbol| Entry {
                    symbol,
                    declared: true,
                })
                .into_iter()
                .collect(),
            scripts: Vec::new(),
        });
    }

    let files: HashSet<&str> = facts.files.iter().map(|f| f.path.as_str()).collect();
    // The module a file defines; Java files of one package share a module, so
    // any symbol of the file stands in for it.
    let mut module_of: HashMap<&str, &Symbol> = HashMap::new();
    let mut any_of: HashMap<&str, &Symbol> = HashMap::new();
    let mut in_file: HashMap<&str, Vec<&Symbol>> = HashMap::new();
    for s in &facts.symbols {
        if s.kind == SymbolKind::Module {
            module_of.entry(s.file.as_str()).or_insert(s);
        }
        any_of.entry(s.file.as_str()).or_insert(s);
        in_file.entry(s.file.as_str()).or_default().push(s);
    }

    let mut groups: BTreeMap<(Family, String), Group> = BTreeMap::new();
    for file in &facts.files {
        let path = file.path.as_str();
        let Some(family) = Family::of(file.language) else {
            continue;
        };
        let Some(symbol) = module_of.get(path).or_else(|| any_of.get(path)) else {
            continue;
        };
        let first = symbol.qualified.split("::").next().unwrap_or("");
        let top_dir = path.split_once('/').map_or("", |(dir, _)| dir);
        let (key, manifest_dir) = match family {
            Family::Python => {
                let lone = !symbol.qualified.contains("::") && !path.ends_with("__init__.py");
                (
                    if lone {
                        String::new()
                    } else {
                        first.to_string()
                    },
                    None,
                )
            }
            Family::Js => match nearest_with(&files, parent_dir(path), &["package.json"]) {
                Some(dir) => (format!("dir:{dir}"), Some(dir.to_string())),
                None => (format!("top:{top_dir}"), None),
            },
            Family::Java => match nearest_with(&files, parent_dir(path), &JAVA_MANIFESTS) {
                Some(dir) => (format!("dir:{dir}"), Some(dir.to_string())),
                None => (format!("pkg:{first}"), None),
            },
        };
        let group = groups.entry((family, key)).or_default();
        group.manifest_dir = manifest_dir;
        group.files.push(path);
    }

    // Keys and folders are taken separately: a unit's key may equal its folder.
    let mut used: HashSet<String> = units.list.iter().map(|u| u.name.to_lowercase()).collect();
    let mut used_slugs: HashSet<String> =
        units.list.iter().map(|u| u.slug.to_lowercase()).collect();
    for ((family, key), group) in &groups {
        let members: Vec<&Symbol> = group
            .files
            .iter()
            .filter_map(|f| in_file.get(f))
            .flatten()
            .copied()
            .collect();
        // Configuration files and test suites alone do not make a unit.
        if !members
            .iter()
            .any(|s| s.kind != SymbolKind::Module && !is_test_file(&s.file))
        {
            continue;
        }
        let modules: Vec<&Symbol> = members
            .iter()
            .copied()
            .filter(|s| s.kind == SymbolKind::Module)
            .collect();
        let mut common: Vec<&str> = modules
            .first()
            .map(|m| m.qualified.split("::").collect())
            .unwrap_or_default();
        for m in &modules {
            let shared = common
                .iter()
                .zip(m.qualified.split("::"))
                .take_while(|(a, b)| **a == *b)
                .count();
            common.truncate(shared);
        }
        let prefix = common.join("::");
        let root = modules
            .iter()
            .find(|m| m.qualified == prefix)
            .map(|m| m.qualified.clone());

        let mut shared_dir: Option<&str> = None;
        for file in &group.files {
            let mut dir = parent_dir(file);
            if let Some(current) = shared_dir {
                while !(dir.is_empty() || current == dir || current.starts_with(&format!("{dir}/")))
                {
                    dir = parent_dir(dir);
                }
            }
            shared_dir = Some(dir);
        }
        let dir = group
            .manifest_dir
            .clone()
            .unwrap_or_else(|| shared_dir.unwrap_or("").to_string());

        let manifest_paths: Vec<String> = match family {
            Family::Js => group
                .manifest_dir
                .iter()
                .map(|d| join(d, "package.json"))
                .collect(),
            Family::Java => group
                .manifest_dir
                .iter()
                .flat_map(|d| JAVA_MANIFESTS.iter().map(move |name| join(d, name)))
                .filter(|p| files.contains(p.as_str()))
                .collect(),
            Family::Python => nearest_with(&files, &dir, &PYTHON_MANIFESTS)
                .map(|d| {
                    let mut found: Vec<String> = PYTHON_MANIFESTS
                        .iter()
                        .map(|name| join(d, name))
                        .filter(|p| files.contains(p.as_str()))
                        .collect();
                    let mut extra: Vec<String> = files
                        .iter()
                        .filter(|p| {
                            parent_dir(p) == d
                                && p.ends_with(".txt")
                                && p.rsplit('/')
                                    .next()
                                    .is_some_and(|n| n.starts_with("requirements"))
                                && !found.iter().any(|f| f == *p)
                        })
                        .map(|p| p.to_string())
                        .collect();
                    extra.sort();
                    found.extend(extra);
                    found
                })
                .unwrap_or_default(),
        };
        let manifests: Vec<Manifest> = manifest_paths
            .into_iter()
            .map(|path| Manifest {
                text: read(&path).unwrap_or_default(),
                path,
            })
            .collect();
        let package_json = match family {
            Family::Js => manifests
                .first()
                .map(|m| parse_package_json(&m.text))
                .unwrap_or_default(),
            _ => PackageJson::default(),
        };

        let dir_name = dir.rsplit('/').next().unwrap_or("");
        let own_name = match family {
            Family::Python => key.clone(),
            Family::Js if !package_json.name.is_empty() => package_json.name.clone(),
            Family::Js if group.manifest_dir.is_none() => {
                key.strip_prefix("top:").unwrap_or("").to_string()
            }
            Family::Js => dir_name.to_string(),
            Family::Java if group.manifest_dir.is_some() => dir_name.to_string(),
            Family::Java => prefix.replace("::", "."),
        };
        let is_repo = own_name.is_empty();
        let package = if is_repo {
            repo_name.to_string()
        } else {
            own_name
        };
        let unique = |base: String, used: &HashSet<String>| -> String {
            let mut candidate = base.clone();
            let mut n = 1;
            while used.contains(&candidate.to_lowercase()) {
                candidate = match n {
                    1 => format!("{base}-{}", family.suffix()),
                    n => format!("{base}-{}-{n}", family.suffix()),
                };
                n += 1;
            }
            candidate
        };
        // The key of a unit that is the repository itself must survive a
        // rename of the checkout folder.
        let name = unique(
            if is_repo {
                format!("root-{}", family.suffix())
            } else {
                slug(&package)
            },
            &used,
        );
        used.insert(name.to_lowercase());
        let slug = unique(slug(&package), &used_slugs);
        used_slugs.insert(slug.to_lowercase());

        let mut entries: Vec<String> = Vec::new();
        let mut declared = 0;
        match family {
            Family::Python => {
                let mut scripts: Vec<&Symbol> = modules
                    .iter()
                    .copied()
                    .filter(|m| {
                        !is_test_file(&m.file)
                            && (m.name == "__main__"
                                || m.attrs.iter().any(|a| a == MAIN_GUARD_ATTR))
                    })
                    .collect();
                scripts.sort_by_key(|m| (m.name != "__main__", m.qualified.as_str()));
                declared = scripts.iter().filter(|m| m.name == "__main__").count();
                entries.extend(scripts.iter().map(|m| m.qualified.clone()));
            }
            Family::Js => {
                for spec in &package_json.entries {
                    if let Some(module) = resolve_js_entry(&files, &dir, spec)
                        .and_then(|file| module_of.get(file.as_str()))
                        && !entries.contains(&module.qualified)
                    {
                        entries.push(module.qualified.clone());
                    }
                }
                declared = entries.len();
            }
            Family::Java => {
                entries.extend(
                    members
                        .iter()
                        .filter(|s| {
                            s.kind == SymbolKind::Method
                                && s.name == "main"
                                && s.signature.contains("static")
                                && s.signature.contains("String")
                                && !is_test_file(&s.file)
                        })
                        .map(|s| s.qualified.clone()),
                );
                entries.sort();
            }
        }
        // A lone `main` is the module's way in; a lone script is still
        // just a script and keeps its own name.
        if *family == Family::Java && entries.len() == 1 {
            declared = 1;
        }

        let mut counts: BTreeMap<&str, (usize, Language)> = BTreeMap::new();
        for file in &group.files {
            let language = Language::from_path(file);
            counts.entry(language.as_str()).or_insert((0, language)).0 += 1;
        }
        let language = counts
            .values()
            .max_by_key(|(count, _)| *count)
            .map_or(Language::Other, |(_, language)| *language);

        let index = units.list.len();
        for file in &group.files {
            units.by_file.insert(file.to_string(), index);
        }
        units.list.push(Unit {
            name,
            package,
            slug,
            dir,
            description: package_json.description,
            deps: Vec::new(),
            has_lib: false,
            has_bin: !entries.is_empty(),
            language,
            manifests,
            root,
            strip: common.len(),
            entries: entries
                .into_iter()
                .enumerate()
                .map(|(i, symbol)| Entry {
                    symbol,
                    declared: i < declared,
                })
                .collect(),
            scripts: package_json.scripts,
        });
    }

    // A unit depends on the units whose files its own files import.
    let mut deps: BTreeSet<(usize, usize)> = BTreeSet::new();
    for edge in facts.edges.iter().filter(|e| e.kind == EdgeKind::Imports) {
        let unit = |node: &str| {
            node.strip_prefix("file:")
                .and_then(|f| units.by_file.get(f))
                .copied()
        };
        if let (Some(from), Some(to)) = (unit(&edge.src), unit(&edge.dst))
            && from != to
        {
            deps.insert((from, to));
        }
    }
    for (from, to) in deps {
        let name = units.list[to].name.clone();
        units.list[from].deps.push(name);
    }
    units
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_test_files() {
        for path in [
            "tests/unit/a.py",
            "pkg/test_text.py",
            "pkg/conftest.py",
            "web/src/app.test.tsx",
            "web/src/__tests__/app.ts",
            "src/main/java/com/acme/LabelTest.java",
        ] {
            assert!(is_test_file(path), "{path}");
        }
        for path in ["pkg/text.py", "web/src/app.tsx", "pkg/contest.py"] {
            assert!(!is_test_file(path), "{path}");
        }
    }

    #[test]
    fn names_are_safe_in_paths_and_tags() {
        assert_eq!(slug("My Bot - Python"), "my-bot-python");
        assert_eq!(slug("@scope/pkg"), "scope-pkg");
        assert_eq!(slug("..."), "code");
    }

    #[test]
    fn finds_the_source_behind_a_package_entry() {
        let files: HashSet<&str> = ["web/src/index.ts", "web/cli.js"].into_iter().collect();
        let r = |spec: &str| resolve_js_entry(&files, "web", spec);
        assert_eq!(r("./dist/index.js").as_deref(), Some("web/src/index.ts"));
        assert_eq!(r("cli.js").as_deref(), Some("web/cli.js"));
        assert_eq!(r("../other.js"), None);
        assert_eq!(r("missing.js"), None);
    }

    #[test]
    fn reads_package_json() {
        let p = parse_package_json(
            r#"{"name":"shop","description":"A shop.","main":"dist/index.js",
                "bin":{"shop":"./cli.js"},"scripts":{"start":"node server.js","test":"vitest"}}"#,
        );
        assert_eq!(p.name, "shop");
        assert_eq!(p.entries, ["dist/index.js", "./cli.js", "server.js"]);
        assert_eq!(p.scripts, ["start", "test"]);
        assert!(parse_package_json("not json").name.is_empty());
    }

    #[test]
    fn matches_declared_packages_as_whole_words() {
        let unit = |text: &str| Unit {
            name: "u".into(),
            package: "u".into(),
            slug: "u".into(),
            dir: String::new(),
            description: String::new(),
            deps: Vec::new(),
            has_lib: false,
            has_bin: false,
            language: Language::Python,
            manifests: vec![Manifest {
                path: "requirements.txt".into(),
                text: text.into(),
            }],
            root: None,
            strip: 0,
            entries: Vec::new(),
            scripts: Vec::new(),
        };
        assert_eq!(
            unit("Requests>=2\n").declares("requests"),
            ["requirements.txt"]
        );
        assert!(unit("requests-oauthlib\n").declares("requests").is_empty());
    }
}
