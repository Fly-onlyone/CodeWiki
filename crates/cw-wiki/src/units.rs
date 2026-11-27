//! Units: the top-level parts of a repository, each with its own folder under
//! `01-Core`.
//!
//! A Cargo package is a unit. Code in other languages has no manifest the
//! scanner understands, so its units are derived here from the layout: a
//! Python top-level package, a JavaScript or TypeScript package directory, a
//! Java build module, a C# project, a Go module. Every unit has the same shape, so
//! planning and rendering treat them alike.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use cw_core::{EdgeKind, Language, Symbol, SymbolKind};
use cw_parse::cmake::{Cmake, TargetKind, read_cmake};
use cw_parse::{
    ENTRY_NAMES, MAIN_GUARD_ATTR, RepoFacts, TEST_ATTR, c_module_map, parse_go_mod, parse_go_work,
};

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
/// Files a C# project shares with its neighbours: package versions, and
/// properties every project below the folder takes.
const CSHARP_SHARED: [&str; 2] = ["Directory.Packages.props", "Directory.Build.props"];
/// The lock file a C# project writes beside itself.
const CSHARP_LOCK: &str = "packages.lock.json";
/// The sums of a Go module's dependencies, kept apart from the files a note
/// describes as the lock file of a C# project is.
const GO_LOCK: &str = "go.sum";
/// Where Gradle's default version catalog stands, below the folder that holds
/// `settings.gradle(.kts)`.
const GRADLE_CATALOG: &str = "gradle/libs.versions.toml";
/// Attributes that make a C# method a test, under any test framework.
const CSHARP_TEST_ATTRIBUTES: [&str; 11] = [
    "Fact",
    "Theory",
    "Test",
    "TestCase",
    "TestCaseSource",
    "TestMethod",
    "DataTestMethod",
    "TestFixture",
    "TestClass",
    "SkippableFact",
    "SkippableTheory",
];
/// Packages whose reference makes a C# project a test project.
const CSHARP_TEST_PACKAGES: [&str; 5] = [
    "xunit",
    "nunit",
    "mstest",
    "microsoft.net.test.sdk",
    "tunit",
];

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
    /// True when the unit is a project of tests, which has no notes about
    /// its code.
    pub tests: bool,
    /// The lock file of the unit's packages, kept out of `manifests` so that
    /// a lock does not count among the files a note describes. For a Gradle
    /// module with Kotlin files it is the version catalog.
    pub lock: Option<Manifest>,
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
    /// Files outside Rust whose code is test code, by their contents and
    /// their project, not by their name alone.
    tests: HashSet<String>,
    /// True when the repository's C or C++ code has tests: test files, or
    /// tests registered with CTest.
    c_tests: bool,
}

impl Units {
    /// True when the repository has tests written in C or C++.
    pub fn c_tests(&self) -> bool {
        self.c_tests
    }

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

    /// True for a source file the units found to be test code: one that
    /// holds a test, or sits in a project of tests.
    pub fn is_test(&self, file: &str) -> bool {
        self.tests.contains(file)
    }

    /// Unit of a source file outside Rust.
    pub fn of_file(&self, file: &str) -> Option<&Unit> {
        self.by_file.get(file).map(|i| &self.list[*i])
    }

    pub fn by_name(&self, name: &str) -> Option<&Unit> {
        self.list.iter().find(|u| u.name == name)
    }

    pub fn all_rust(&self) -> bool {
        // No unit at all is no Rust either.
        !self.list.is_empty() && self.list.iter().all(Unit::is_rust)
    }
}

/// Language family: one unit never mixes two of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Family {
    Python,
    Js,
    Java,
    CSharp,
    Go,
    /// C and C++ together: they share headers, and one CMake target holds both.
    C,
}

impl Family {
    fn of(language: Language) -> Option<Self> {
        match language {
            Language::Python => Some(Self::Python),
            Language::TypeScript | Language::JavaScript => Some(Self::Js),
            // A Gradle or Maven module holds both languages side by side.
            Language::Java | Language::Kotlin => Some(Self::Java),
            Language::CSharp => Some(Self::CSharp),
            Language::Go => Some(Self::Go),
            Language::C | Language::Cpp => Some(Self::C),
            _ => None,
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Self::Python => "python",
            Self::Js => "js",
            Self::Java => "java",
            Self::CSharp => "csharp",
            Self::Go => "go",
            Self::C => "c",
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
    // The Go tool ignores `testdata`: what lies there is fixtures of tests.
    if file.ends_with(".go") && dirs.contains(&"testdata") {
        return true;
    }
    let stem = file.split('.').next().unwrap_or(file);
    // A C# test project is a folder named for what it tests: `Core.Tests`.
    if file.ends_with(".cs")
        && dirs.iter().any(|d| {
            ["Tests", "Test", "UnitTests", "IntegrationTests"]
                .iter()
                .any(|suffix| d.len() > suffix.len() && d.ends_with(&format!(".{suffix}")))
        })
    {
        return true;
    }
    file.starts_with("test_")
        || is_c_test_name(file, stem)
        || file == "conftest.py"
        || file.contains(".test.")
        || file.contains(".spec.")
        || stem.ends_with("_test")
        || (file.ends_with(".java")
            && (stem.ends_with("Test") || stem.ends_with("Tests") || stem.ends_with("IT")))
        || (file.ends_with(".cs") && (stem.ends_with("Test") || stem.ends_with("Tests")))
        // `Spec` is the name Kotest and Spek give a test class.
        || (file.ends_with(".kt")
            && (stem.ends_with("Test")
                || stem.ends_with("Tests")
                || stem.ends_with("IT")
                || stem.ends_with("Spec")
                || in_jvm_test_set(&dirs)))
}

/// A C or C++ file named like a test: `list_test.c`, `test_list.cpp`,
/// `list_tests.cc`, `ListTest.cpp`, `ListTests.cpp`. A file in a `test` or
/// `tests` folder is caught by its folder.
fn is_c_test_name(file: &str, stem: &str) -> bool {
    let ext = file.rsplit_once('.').map_or("", |(_, e)| e);
    matches!(ext, "c" | "cc" | "cpp" | "cxx" | "c++")
        && (stem.ends_with("_tests")
            || (stem.len() > 4 && (stem.ends_with("Test") || stem.ends_with("Tests"))))
}

/// A C or C++ program's way in: `main`, `wmain`, `WinMain` or `wWinMain` in
/// a source file. `DllMain` is not one: a library calls it, not a program.
fn is_c_entry(symbol: &Symbol) -> bool {
    symbol.kind == SymbolKind::Function
        && ENTRY_NAMES.contains(&symbol.name.as_str())
        && symbol.parent.as_deref().is_some_and(|p| {
            // A free function: its parent is the module, which is no class.
            symbol.qualified == format!("{p}::{}", symbol.name)
        })
        && !cw_parse::is_c_header(&symbol.file)
        && !is_test_file(&symbol.file)
        && !symbol.attrs.iter().any(|a| a == TEST_ATTR)
}

/// Libraries a CMake target links to run tests with.
const C_TEST_LIBRARIES: [&str; 12] = [
    "gtest",
    "gtest_main",
    "gmock",
    "gmock_main",
    "catch2",
    "catch2withmain",
    "doctest",
    "unit_test_framework",
    "cmocka",
    "criterion",
    "check",
    "unity",
];

/// True when a target links a test framework: `GTest::gtest_main`,
/// `Catch2::Catch2WithMain`, `doctest::doctest`.
pub(crate) fn links_a_test_framework(links: &[String]) -> bool {
    links.iter().any(|l| {
        let name = l
            .rsplit("::")
            .next()
            .unwrap_or(l)
            .to_ascii_lowercase()
            .replace(['-', '_'], "");
        C_TEST_LIBRARIES
            .iter()
            .any(|t| name == t.replace('_', "") || name == format!("{}main", t.replace('_', "")))
    })
}

/// The folder unit a module belongs to when no CMake target says: the project
/// folder under a container (`libs/net`), else its first name (`net` for
/// `net::socket`), else the root.
fn c_folder_unit(module: &str) -> String {
    let segments: Vec<&str> = module.split("::").collect();
    if segments.len() >= 3
        && [
            "libs", "apps", "packages", "projects", "modules", "services", "plugins", "tests",
            "test", "samples", "examples",
        ]
        .contains(&segments[0].to_ascii_lowercase().as_str())
    {
        format!("{}/{}", segments[0], segments[1])
    } else if segments.len() >= 2 {
        segments[0].to_string()
    } else {
        String::new()
    }
}

/// True for a folder of a Gradle source set of tests: `src/test`, and the
/// ones that name their target, such as Kotlin Multiplatform's
/// `src/commonTest`, `src/jvmTest` and Android's `src/androidTest`.
fn in_jvm_test_set(dirs: &[&str]) -> bool {
    dirs.windows(2).any(|pair| {
        pair[0] == "src"
            && (pair[1] == "test"
                || (pair[1].len() > "Test".len()
                    && (pair[1].ends_with("Test") || pair[1].ends_with("Tests"))))
    })
}

/// True for a Java or Kotlin file of test code, the source sets of tests
/// included. [`is_test_file`] leaves the sets of a Java file alone, since a
/// Java repository's notes must not change; the test map looks further.
pub(crate) fn is_jvm_test_file(path: &str) -> bool {
    let dirs: Vec<&str> = path.split('/').collect();
    is_test_file(path) || in_jvm_test_set(&dirs[..dirs.len().saturating_sub(1)])
}

/// A Kotlin program's `main`: a function at the top of a file, or an
/// `@JvmStatic` one of an `object` or a companion, taking no parameter or
/// the arguments (`Array<String>`, or `vararg String`).
pub(crate) fn is_kotlin_entry(symbol: &Symbol) -> bool {
    if symbol.name != "main"
        || !symbol.file.ends_with(".kt")
        || is_test_file(&symbol.file)
        || symbol.visibility == cw_core::Visibility::Private
    {
        return false;
    }
    let placed = match symbol.kind {
        SymbolKind::Function => true,
        SymbolKind::Method => symbol.attrs.iter().any(|a| a == "JvmStatic"),
        _ => false,
    };
    let parameters = symbol
        .signature
        .split_once("main(")
        .and_then(|(_, rest)| rest.split_once(')'))
        .map_or("", |(parameters, _)| parameters.trim());
    placed
        && (parameters.is_empty()
            || parameters.contains("Array<String>")
            || parameters.starts_with("vararg"))
}

/// True when `attribute` (as written: `Fact`, `Xunit.Fact`, `FactAttribute`)
/// marks a C# test.
pub(crate) fn is_csharp_test_attribute(attribute: &str) -> bool {
    let name = attribute.rsplit('.').next().unwrap_or(attribute);
    let name = name.strip_suffix("Attribute").unwrap_or(name);
    CSHARP_TEST_ATTRIBUTES.contains(&name)
}

/// Text of the first `<tag>` element of `xml`, trimmed.
pub(crate) fn xml_element<'a>(xml: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&format!("</{tag}>"))?;
    Some(xml[start..start + end].trim())
}

/// Every element named `tag` in `xml` with its attribute text and, for one
/// that is not self-closing, its inner text.
pub(crate) fn xml_elements<'a>(xml: &'a str, tag: &str) -> Vec<(&'a str, &'a str)> {
    let open = format!("<{tag}");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(at) = rest.find(&open) {
        let after = &rest[at + open.len()..];
        // `<PackageReferenceX` is another element.
        if !after.starts_with([' ', '\t', '\r', '\n', '/', '>']) {
            rest = after;
            continue;
        }
        let Some(end) = after.find('>') else {
            break;
        };
        let head = &after[..end];
        if let Some(attributes) = head.strip_suffix('/') {
            out.push((attributes, ""));
            rest = &after[end + 1..];
            continue;
        }
        let body = &after[end + 1..];
        let close = format!("</{tag}>");
        match body.find(&close) {
            Some(stop) => {
                out.push((head, &body[..stop]));
                rest = &body[stop + close.len()..];
            }
            None => {
                out.push((head, ""));
                rest = body;
            }
        }
    }
    out
}

/// The value of attribute `name` in the attribute text of an element.
pub(crate) fn xml_attribute(attributes: &str, name: &str) -> Option<String> {
    let mut rest = attributes;
    while let Some(at) = rest.find(name) {
        let before_ok = rest[..at]
            .chars()
            .next_back()
            .is_none_or(|c| c.is_whitespace());
        let after = rest[at + name.len()..].trim_start();
        if before_ok && let Some(value) = after.strip_prefix('=') {
            let value = value.trim_start();
            let quote = value.chars().next().filter(|c| matches!(c, '"' | '\''))?;
            let inner = &value[1..];
            return inner.find(quote).map(|end| inner[..end].to_string());
        }
        rest = &rest[at + name.len()..];
    }
    None
}

/// The repo-relative path that the relative `target` names from `dir`, with
/// its `..` and `.` segments resolved and either slash accepted; `None`
/// when it climbs out of the repository.
pub(crate) fn resolve_relative(dir: &str, target: &str) -> Option<String> {
    let mut parts: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
    for segment in target.split(['/', '\\']) {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

/// Lowercase name safe as a folder and as a tag.
pub(crate) fn slug(name: &str) -> String {
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

/// True for a C# project or folder named for its tests: `Core.Tests`.
fn is_test_name(name: &str) -> bool {
    ["Tests", "Test", "UnitTests", "IntegrationTests", "Specs"]
        .iter()
        .any(|suffix| name.len() > suffix.len() && name.ends_with(&format!(".{suffix}")))
}

/// True when a project file references a test framework or says it is a test
/// project.
pub(crate) fn references_a_test_framework(project: &str) -> bool {
    project.to_lowercase().contains("<istestproject>true")
        || xml_elements(project, "PackageReference")
            .iter()
            .filter_map(|(attributes, _)| xml_attribute(attributes, "Include"))
            .any(|id| {
                let id = id.to_lowercase();
                CSHARP_TEST_PACKAGES.iter().any(|p| id.starts_with(p))
            })
}

/// True for a C# program's way in: a static `Main` method that takes no
/// arguments or the command line, or the top-level statements of a file.
fn is_csharp_entry(symbol: &Symbol) -> bool {
    match symbol.kind {
        SymbolKind::Function => symbol.name == "Main" && symbol.signature == "top-level statements",
        SymbolKind::Method => {
            if symbol.name != "Main" || !symbol.signature.split_whitespace().any(|w| w == "static")
            {
                return false;
            }
            let params = symbol
                .signature
                .split_once("Main(")
                .and_then(|(_, rest)| rest.split_once(')'))
                .map_or("", |(inside, _)| inside)
                .trim();
            params.is_empty()
                || !params.contains(',')
                    && params
                        .strip_prefix("string")
                        .or_else(|| params.strip_prefix("String"))
                        .is_some_and(|rest| rest.trim_start().starts_with("[]"))
        }
        _ => false,
    }
}

/// True for a Go program's way in: `func main()` of a package called `main`.
fn is_go_entry(symbol: &Symbol, symbols: &HashMap<&str, &Symbol>) -> bool {
    symbol.kind == SymbolKind::Function
        && symbol.name == "main"
        && !is_test_file(&symbol.file)
        && symbol
            .parent
            .as_deref()
            .and_then(|parent| symbols.get(parent))
            .is_some_and(|module| module.signature == "package main")
}

/// What a Go module is called: the last element of its path, leaving out a
/// major-version element (`github.com/acme/app/v2` is `app`).
fn go_module_name(path: &str) -> String {
    let mut segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() > 1
        && segments.last().is_some_and(|last| {
            last.strip_prefix('v')
                .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        })
    {
        segments.pop();
    }
    segments.last().copied().unwrap_or("").to_string()
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
    /// The CMake target of a C or C++ unit, and whether it is a library.
    c_target: Option<(String, bool)>,
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
            manifests: vec![{
                let path = join(&krate.dir, "Cargo.toml");
                Manifest {
                    text: read(&path).unwrap_or_default(),
                    path,
                }
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
            tests: false,
            lock: None,
        });
    }

    let files: HashSet<&str> = facts.files.iter().map(|f| f.path.as_str()).collect();
    let lang_of: HashMap<&str, Language> = facts
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.language))
        .collect();
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

    // The C# projects of the repository, by folder.
    let mut projects: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for path in &files {
        if path.ends_with(".csproj") {
            projects.entry(parent_dir(path)).or_default().push(path);
        }
    }
    for found in projects.values_mut() {
        found.sort_unstable();
    }
    let nearest_project = |dir: &str| -> Option<String> {
        ancestors(dir)
            .into_iter()
            .find(|d| projects.contains_key(d))
            .map(str::to_string)
    };

    // The folders of the Go modules, which every `.go` file below belongs to.
    let go_modules: HashSet<&str> = files
        .iter()
        .filter(|p| p.rsplit('/').next() == Some("go.mod"))
        .map(|p| parent_dir(p))
        .collect();

    // C and C++: the CMake targets, and which module each file belongs to.
    let all_paths: Vec<String> = facts.files.iter().map(|f| f.path.clone()).collect();
    let has_c = facts.files.iter().any(|f| f.language.is_c_family());
    let cmake: Cmake = if has_c {
        read_cmake(&all_paths, read)
    } else {
        Cmake::default()
    };
    let c_modules: HashMap<String, String> = if has_c {
        c_module_map(&all_paths)
    } else {
        HashMap::new()
    };

    // A CMake target of tests: CTest runs it, or it links a test framework,
    // or every file it holds is test code.
    let c_tests_target = |group: &Group| -> bool {
        let target = group.c_target.as_ref().map(|(n, _)| n.as_str());
        // A program that CTest runs as a smoke test is still a program: only
        // a discovery command (`gtest_discover_tests`) says its tests are
        // tests of other code.
        let registered = target.is_some_and(|n| {
            cmake
                .tests
                .iter()
                .any(|t| t.discovered.is_some() && t.target.as_deref() == Some(n))
        });
        let links = target
            .and_then(|n| cmake.targets.iter().find(|t| t.name == n))
            .is_some_and(|t| links_a_test_framework(&t.links));
        registered
            || links
            || (!group.files.is_empty() && group.files.iter().all(|f| is_test_file(f)))
    };

    let mut groups: BTreeMap<(Family, String), Group> = BTreeMap::new();
    for file in &facts.files {
        let path = file.path.as_str();
        let Some(family) = Family::of(file.language) else {
            continue;
        };
        if family == Family::C {
            // A file of a vendored tree has no module and no unit.
            let Some(module) = c_modules.get(path) else {
                continue;
            };
            let (key, manifest_dir, target) = match cmake.target_of(path) {
                Some(target) => (
                    format!("cmake:{}:{}", target.dir, target.name),
                    Some(target.dir.clone()),
                    Some((target.name.clone(), target.kind == TargetKind::Library)),
                ),
                None => (format!("dir:{}", c_folder_unit(module)), None, None),
            };
            let group = groups.entry((family, key)).or_default();
            group.manifest_dir = manifest_dir;
            group.c_target = target;
            group.files.push(path);
            continue;
        }
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
            Family::CSharp => match nearest_project(parent_dir(path)) {
                Some(dir) => (format!("dir:{dir}"), Some(dir)),
                None => (format!("pkg:{first}"), None),
            },
            // A module holds every package below its `go.mod`, the `cmd`
            // folders of its programs among them.
            Family::Go => match ancestors(parent_dir(path))
                .into_iter()
                .find(|d| go_modules.contains(d))
            {
                Some(dir) => (format!("dir:{dir}"), Some(dir.to_string())),
                None => (format!("pkg:{first}"), None),
            },
            Family::C => unreachable!("C files are grouped above"),
        };
        let group = groups.entry((family, key)).or_default();
        group.manifest_dir = manifest_dir;
        group.files.push(path);
    }

    // Keys and folders are taken separately: a unit's key may equal its folder.
    let mut used: HashSet<String> = units.list.iter().map(|u| u.name.to_lowercase()).collect();
    let mut used_slugs: HashSet<String> =
        units.list.iter().map(|u| u.slug.to_lowercase()).collect();
    // The module path of each Go unit, by unit index.
    let mut go_unit_paths: Vec<(usize, String)> = Vec::new();
    // The unit of each CMake target, by the target's name.
    let mut c_unit_of_target: HashMap<String, usize> = HashMap::new();
    for ((family, key), group) in &groups {
        let members: Vec<&Symbol> = group
            .files
            .iter()
            .filter_map(|f| in_file.get(f))
            .flatten()
            .copied()
            .collect();
        // Configuration files and test suites alone do not make a unit. A C#
        // project of tests is a project all the same.
        if !members.iter().any(|s| {
            s.kind != SymbolKind::Module && (*family == Family::CSharp || !is_test_file(&s.file))
        }) {
            // The files of tests are test code, though they make no unit.
            if *family == Family::C {
                let all = c_tests_target(group);
                units.tests.extend(
                    group
                        .files
                        .iter()
                        .filter(|f| all || is_test_file(f))
                        .map(|f| f.to_string()),
                );
            }
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
            Family::CSharp => {
                let mut found: Vec<String> = group
                    .manifest_dir
                    .iter()
                    .flat_map(|d| projects.get(d.as_str()).into_iter().flatten())
                    .map(|p| p.to_string())
                    .collect();
                if !found.is_empty() {
                    for shared in CSHARP_SHARED {
                        if let Some(at) = nearest_with(&files, &dir, &[shared]) {
                            found.push(join(at, shared));
                        }
                    }
                }
                found
            }
            Family::Go => {
                let mut found: Vec<String> = group
                    .manifest_dir
                    .iter()
                    .map(|d| join(d, "go.mod"))
                    .collect();
                // The workspace file that lists the module, nearest above.
                if let Some(at) = nearest_with(&files, &dir, &["go.work"])
                    && !found.is_empty()
                {
                    let work = join(at, "go.work");
                    let listed = read(&work).is_some_and(|text| {
                        parse_go_work(&text)
                            .iter()
                            .any(|used| resolve_relative(at, used).as_deref() == Some(dir.as_str()))
                    });
                    if listed {
                        found.push(work);
                    }
                }
                found
            }
            Family::C => {
                let mut found: Vec<String> = group
                    .c_target
                    .as_ref()
                    .and_then(|(name, _)| cmake.targets.iter().find(|t| &t.name == name))
                    .map(|t| vec![t.file.clone()])
                    .unwrap_or_default();
                for names in [
                    &["CMakePresets.json"][..],
                    &["vcpkg.json"][..],
                    &["conanfile.txt", "conanfile.py"][..],
                ] {
                    if let Some(at) = nearest_with(&files, &dir, names) {
                        for name in names {
                            let path = join(at, name);
                            if files.contains(path.as_str()) && !found.contains(&path) {
                                found.push(path);
                            }
                        }
                    }
                }
                found
            }
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
        let project_file = manifests
            .first()
            .filter(|m| m.path.ends_with(".csproj"))
            .map(|m| m.path.rsplit('/').next().unwrap_or(&m.path).to_string());
        let go_path = match family {
            Family::Go => manifests
                .first()
                .map(|m| parse_go_mod(&m.text).module)
                .unwrap_or_default(),
            _ => String::new(),
        };
        let own_name = match family {
            Family::Go if !go_path.is_empty() => go_module_name(&go_path),
            Family::Go => prefix.replace("::", "."),
            Family::CSharp => match &project_file {
                Some(file) => file.strip_suffix(".csproj").unwrap_or(file).to_string(),
                None => prefix.replace("::", "."),
            },
            Family::C => match &group.c_target {
                Some((name, _)) => name.clone(),
                None => key
                    .strip_prefix("dir:")
                    .unwrap_or("")
                    .rsplit('/')
                    .next()
                    .unwrap_or("")
                    .to_string(),
            },
            Family::Python => key.clone(),
            Family::Js if !package_json.name.is_empty() => package_json.name.clone(),
            Family::Js if group.manifest_dir.is_none() => {
                key.strip_prefix("top:").unwrap_or("").to_string()
            }
            Family::Js => dir_name.to_string(),
            Family::Java if group.manifest_dir.is_some() => dir_name.to_string(),
            Family::Java => prefix.replace("::", "."),
        };
        let lock = match family {
            Family::CSharp => group
                .manifest_dir
                .as_deref()
                .map(|d| join(d, CSHARP_LOCK))
                .filter(|p| files.contains(p.as_str()))
                .map(|path| Manifest {
                    text: read(&path).unwrap_or_default(),
                    path,
                }),
            Family::Go => group
                .manifest_dir
                .as_deref()
                .map(|d| join(d, GO_LOCK))
                .filter(|p| files.contains(p.as_str()))
                .map(|path| Manifest {
                    text: read(&path).unwrap_or_default(),
                    path,
                }),
            Family::C => nearest_with(&files, &dir, &["conan.lock"])
                .map(|d| join(d, "conan.lock"))
                .map(|path| Manifest {
                    text: read(&path).unwrap_or_default(),
                    path,
                }),
            // A Kotlin module takes its libraries from the version catalog
            // more often than not: the nearest `gradle/libs.versions.toml`
            // above it stands where a lock file does for the others. A
            // module of Java alone is left as it was.
            Family::Java
                if group
                    .files
                    .iter()
                    .any(|f| Language::from_path(f) == Language::Kotlin) =>
            {
                nearest_with(&files, &dir, &[GRADLE_CATALOG])
                    .map(|d| join(d, GRADLE_CATALOG))
                    .map(|path| Manifest {
                        text: read(&path).unwrap_or_default(),
                        path,
                    })
            }
            _ => None,
        };
        let c_test_project = *family == Family::C && c_tests_target(group);
        let test_project = c_test_project
            || *family == Family::CSharp
                && (is_test_name(&own_name)
                    || is_test_name(dir_name)
                    || manifests.iter().any(|m| {
                        m.path.ends_with(".csproj") && references_a_test_framework(&m.text)
                    }));
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
            Family::CSharp => {
                entries.extend(
                    members
                        .iter()
                        .filter(|s| is_csharp_entry(s) && !is_test_file(&s.file) && !test_project)
                        .map(|s| s.qualified.clone()),
                );
                entries.sort();
            }
            Family::Go => {
                entries.extend(
                    members
                        .iter()
                        .filter(|s| is_go_entry(s, &symbols))
                        .map(|s| s.qualified.clone()),
                );
                entries.sort();
            }
            Family::C => {
                entries.extend(
                    members
                        .iter()
                        .filter(|s| is_c_entry(s) && !test_project)
                        .map(|s| s.qualified.clone()),
                );
                entries.sort();
            }
            Family::Java => {
                entries.extend(
                    members
                        .iter()
                        .filter(|s| {
                            (s.kind == SymbolKind::Method
                                && s.name == "main"
                                && s.signature.contains("static")
                                && s.signature.contains("String")
                                && !is_test_file(&s.file))
                                || is_kotlin_entry(s)
                        })
                        .map(|s| s.qualified.clone()),
                );
                entries.sort();
            }
        }
        // A lone `main` is the module's way in; a lone script is still
        // just a script and keeps its own name.
        if matches!(
            family,
            Family::Java | Family::CSharp | Family::Go | Family::C
        ) && entries.len() == 1
        {
            declared = 1;
        }

        let mut counts: BTreeMap<&str, (usize, Language)> = BTreeMap::new();
        for file in &group.files {
            // A `.h` is C or C++ as the scan decided.
            let language = lang_of
                .get(file)
                .copied()
                .unwrap_or_else(|| Language::from_path(file));
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
        if *family == Family::CSharp {
            let holds_test = |file: &str| {
                test_project
                    || is_test_file(file)
                    || in_file.get(file).is_some_and(|symbols| {
                        symbols
                            .iter()
                            .any(|s| s.attrs.iter().any(|a| is_csharp_test_attribute(a)))
                    })
            };
            units.tests.extend(
                group
                    .files
                    .iter()
                    .filter(|f| holds_test(f))
                    .map(|f| f.to_string()),
            );
        }
        if *family == Family::C {
            units.tests.extend(
                group
                    .files
                    .iter()
                    .filter(|f| {
                        test_project
                            || is_test_file(f)
                            || in_file.get(*f).is_some_and(|symbols| {
                                symbols
                                    .iter()
                                    .any(|s| s.attrs.iter().any(|a| a == TEST_ATTR))
                            })
                    })
                    .map(|f| f.to_string()),
            );
            if let Some((name, _)) = &group.c_target {
                c_unit_of_target.insert(name.clone(), index);
            }
        }
        let description = match family {
            Family::CSharp => manifests
                .iter()
                .filter(|m| m.path.ends_with(".csproj"))
                .find_map(|m| {
                    xml_element(&m.text, "Description")
                        .or_else(|| xml_element(&m.text, "PackageDescription"))
                })
                .unwrap_or("")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
            _ => package_json.description,
        };
        if *family == Family::Go && !go_path.is_empty() {
            go_unit_paths.push((index, go_path.clone()));
        }
        units.list.push(Unit {
            name,
            package,
            slug,
            dir,
            description,
            deps: Vec::new(),
            has_lib: group.c_target.as_ref().is_some_and(|(_, lib)| *lib),
            has_bin: !entries.is_empty() || group.c_target.as_ref().is_some_and(|(_, lib)| !*lib),
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
            tests: test_project,
            lock,
        });
    }

    // A C# project depends on the projects it references. One that names
    // none, or whose file was not read, is read from its imports like the
    // units of other languages.
    let by_project: HashMap<String, usize> = units
        .list
        .iter()
        .enumerate()
        .filter_map(|(i, u)| Some((u.manifests.first()?.path.clone(), i)))
        .filter(|(path, _)| path.ends_with(".csproj"))
        .collect();
    let mut referencing: HashSet<usize> = HashSet::new();
    let mut project_deps: Vec<(usize, Vec<String>)> = Vec::new();
    for (index, unit) in units.list.iter().enumerate() {
        let Some(project) = unit
            .manifests
            .first()
            .filter(|m| m.path.ends_with(".csproj"))
        else {
            continue;
        };
        let dir = parent_dir(&project.path);
        let mut names: Vec<String> = Vec::new();
        for (attributes, _) in xml_elements(&project.text, "ProjectReference") {
            let Some(target) = xml_attribute(attributes, "Include")
                .and_then(|include| resolve_relative(dir, &include))
            else {
                continue;
            };
            if let Some(&to) = by_project.get(&target)
                && to != index
                && !names.contains(&units.list[to].name)
            {
                names.push(units.list[to].name.clone());
            }
        }
        if !names.is_empty() {
            referencing.insert(index);
            project_deps.push((index, names));
        }
    }
    // A Go module depends on the modules of the repository its `go.mod`
    // requires, or replaces by a path of this machine.
    let by_path: HashMap<&str, usize> = go_unit_paths
        .iter()
        .map(|(i, path)| (path.as_str(), *i))
        .collect();
    let by_dir: HashMap<String, usize> = go_unit_paths
        .iter()
        .map(|(i, _)| (units.list[*i].dir.clone(), *i))
        .collect();
    let mut go_deps: Vec<(usize, Vec<String>)> = Vec::new();
    for (index, _) in &go_unit_paths {
        let unit = &units.list[*index];
        let Some(go_mod) = unit
            .manifests
            .first()
            .filter(|m| m.path.ends_with("go.mod"))
            .map(|m| parse_go_mod(&m.text))
        else {
            continue;
        };
        let mut targets: Vec<usize> = go_mod
            .requires
            .iter()
            .filter_map(|r| by_path.get(r.path.as_str()).copied())
            .collect();
        for replace in &go_mod.replaces {
            if let Some(&to) = by_path.get(replace.old.as_str()) {
                targets.push(to);
            }
            if replace.new_version.is_empty()
                && let Some(dir) = resolve_relative(&unit.dir, &replace.new)
                && let Some(&to) = by_dir.get(&dir)
            {
                targets.push(to);
            }
        }
        let mut names: Vec<String> = Vec::new();
        for to in targets {
            let name = units.list[to].name.clone();
            if to != *index && !names.contains(&name) {
                names.push(name);
            }
        }
        if !names.is_empty() {
            go_deps.push((*index, names));
        }
    }
    units.c_tests = !cmake.tests.is_empty()
        || units
            .tests
            .iter()
            .any(|f| lang_of.get(f.as_str()).is_some_and(|l| l.is_c_family()));
    // A target depends on the targets of the repository it links.
    let mut c_deps: Vec<(usize, Vec<String>)> = Vec::new();
    for target in &cmake.targets {
        let Some(&from) = c_unit_of_target.get(&target.name) else {
            continue;
        };
        let mut names: Vec<String> = Vec::new();
        for link in &target.links {
            if let Some(&to) = c_unit_of_target.get(link)
                && to != from
                && !names.contains(&units.list[to].name)
            {
                names.push(units.list[to].name.clone());
            }
        }
        if !names.is_empty() {
            c_deps.push((from, names));
        }
    }
    for (index, names) in project_deps.into_iter().chain(go_deps).chain(c_deps) {
        referencing.insert(index);
        units.list[index].deps = names;
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
            && !referencing.contains(&from)
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
            tests: false,
            lock: None,
        };
        assert_eq!(
            unit("Requests>=2\n").declares("requests"),
            ["requirements.txt"]
        );
        assert!(unit("requests-oauthlib\n").declares("requests").is_empty());
    }

    #[test]
    fn recognises_csharp_test_files() {
        for path in [
            "tests/Core.Tests/EngineTests.cs",
            "src/Core.Tests/Engine.cs",
            "src/Core.UnitTests/Sub/Engine.cs",
            "src/Core/EngineTest.cs",
            "src/Core/EngineTests.cs",
        ] {
            assert!(is_test_file(path), "{path}");
        }
        for path in [
            "src/Core/Engine.cs",
            "src/Core.Testing/Engine.cs",
            "src/Tests.cs.txt",
            "src/Latest/Engine.cs",
        ] {
            assert!(!is_test_file(path), "{path}");
        }
        for name in [
            "Fact",
            "Xunit.Fact",
            "TheoryAttribute",
            "TestMethod",
            "Test",
        ] {
            assert!(is_csharp_test_attribute(name), "{name}");
        }
        for name in ["Obsolete", "Factory", "TestData"] {
            assert!(!is_csharp_test_attribute(name), "{name}");
        }
    }

    #[test]
    fn reads_the_xml_of_project_files() {
        let xml = r#"<Project><ItemGroup>
  <PackageReference Include="Serilog" Version="4.2.0" />
  <PackageReferenceGroup Include="no" />
  <PackageReference Include='xunit.v3'>
    <Version>3.2.2</Version>
  </PackageReference>
</ItemGroup><Description> A  thing. </Description></Project>"#;
        let found = xml_elements(xml, "PackageReference");
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(
            xml_attribute(found[0].0, "Include").as_deref(),
            Some("Serilog")
        );
        assert_eq!(
            xml_attribute(found[0].0, "Version").as_deref(),
            Some("4.2.0")
        );
        assert_eq!(
            xml_attribute(found[1].0, "Include").as_deref(),
            Some("xunit.v3")
        );
        assert_eq!(xml_element(found[1].1, "Version"), Some("3.2.2"));
        assert_eq!(xml_element(xml, "Description"), Some("A  thing."));
        // `Include` is not `IncludeAssets`.
        assert_eq!(
            xml_attribute(r#" IncludeAssets="all" Include="X""#, "Include").as_deref(),
            Some("X")
        );
        assert_eq!(
            resolve_relative("tests/Core.Tests", r"..\..\src\Core\Core.csproj").as_deref(),
            Some("src/Core/Core.csproj")
        );
        assert_eq!(resolve_relative("a", "../../b"), None);
    }

    fn discovery(files: &[(&str, &str)]) -> Units {
        let dir = tempfile::tempdir().unwrap();
        for (rel, text) in files {
            let path = dir.path().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let facts = cw_parse::scan_repo(dir.path()).unwrap();
        let root = dir.path().to_path_buf();
        discover(&facts, "repo", &|path| {
            std::fs::read_to_string(root.join(path)).ok()
        })
    }

    #[test]
    fn a_csharp_project_is_a_unit() {
        let units = discovery(&[
            (
                "Directory.Packages.props",
                "<Project><ItemGroup><PackageVersion Include=\"Serilog\" Version=\"4.2.0\" /></ItemGroup></Project>",
            ),
            (
                "src/Core/Core.csproj",
                "<Project><PropertyGroup><Description>The core.</Description></PropertyGroup><ItemGroup><PackageReference Include=\"Serilog\" /></ItemGroup></Project>",
            ),
            ("src/Core/packages.lock.json", "{}"),
            (
                "src/Core/Engine.cs",
                "namespace Acme.Core;\n\npublic class Engine { public void Run() { } }\n",
            ),
            (
                "src/App/App.csproj",
                "<Project><ItemGroup><ProjectReference Include=\"..\\Core\\Core.csproj\" /></ItemGroup></Project>",
            ),
            (
                "src/App/Program.cs",
                "using Acme.Core;\n\nnamespace Acme.App;\n\ninternal static class Program\n{\n    public static int Main(string[] args)\n    {\n        new Engine().Run();\n        return 0;\n    }\n}\n",
            ),
            ("src/Other/Other.csproj", "<Project />"),
            (
                "src/Other/Program.cs",
                "using Acme.Core;\nnew Engine().Run();\n",
            ),
            ("src/Many/Many.csproj", "<Project />"),
            (
                "src/Many/Entries.cs",
                "namespace Acme.Many;\n\npublic class A { static void Main() { } }\npublic class B { public static async Task<int> Main(string[] args) => 0; }\npublic class C { private static void Main(string[] a, int b) { } }\n",
            ),
            (
                "tests/Core.Tests/Core.Tests.csproj",
                "<Project><ItemGroup><ProjectReference Include=\"..\\..\\src\\Core\\Core.csproj\" /><PackageReference Include=\"xunit.v3\" /></ItemGroup></Project>",
            ),
            (
                "tests/Core.Tests/EngineTests.cs",
                "namespace Acme.Core.Tests;\n\npublic class EngineTests\n{\n    [Fact]\n    public void Runs() { new Acme.Core.Engine().Run(); }\n}\n",
            ),
            (
                "src/Core/Spec.cs",
                "namespace Acme.Core;\n\npublic class Spec { [Fact] public void Holds() { } }\n",
            ),
        ]);
        let names: Vec<&str> = units.list.iter().map(|u| u.name.as_str()).collect();
        assert_eq!(names, ["app", "core", "many", "other", "core-tests"]);

        let core = units.by_name("core").unwrap();
        assert_eq!(core.language, Language::CSharp);
        assert_eq!(core.package, "Core");
        assert_eq!(core.dir, "src/Core");
        assert_eq!(core.description, "The core.");
        assert!(!core.tests && core.entries.is_empty() && !core.has_bin);
        let manifests: Vec<&str> = core.manifests.iter().map(|m| m.path.as_str()).collect();
        assert_eq!(
            manifests,
            ["src/Core/Core.csproj", "Directory.Packages.props"]
        );
        assert_eq!(
            core.lock.as_ref().unwrap().path,
            "src/Core/packages.lock.json"
        );

        // A project depends on the projects it references.
        assert_eq!(units.by_name("app").unwrap().deps, ["core"]);
        assert_eq!(units.by_name("core-tests").unwrap().deps, ["core"]);
        assert!(core.deps.is_empty());

        // A static `Main` of either shape, or top-level statements.
        let app = units.by_name("app").unwrap();
        assert_eq!(
            app.entries,
            [Entry {
                symbol: "Acme::App::Program::Main".into(),
                declared: true
            }]
        );
        let other = units.by_name("other").unwrap();
        assert_eq!(other.entries[0].symbol, "src::Other::Main");
        let many = units.by_name("many").unwrap();
        let many_entries: Vec<&str> = many.entries.iter().map(|e| e.symbol.as_str()).collect();
        assert_eq!(many_entries, ["Acme::Many::A::Main", "Acme::Many::B::Main"]);
        assert!(!many.entries[0].declared);

        // A project of tests is a unit, and its code is test code, so is the
        // file of a test in a project that is no test project.
        assert!(units.by_name("core-tests").unwrap().tests);
        assert!(units.by_name("core-tests").unwrap().entries.is_empty());
        assert!(units.is_test("tests/Core.Tests/EngineTests.cs"));
        assert!(units.is_test("src/Core/Spec.cs"));
        assert!(!units.is_test("src/Core/Engine.cs"));
        assert_eq!(units.of_file("src/App/Program.cs").unwrap().name, "app");
    }

    #[test]
    fn csharp_files_without_a_project_are_grouped_by_namespace() {
        let units = discovery(&[
            (
                "a/One.cs",
                "namespace Acme.Util;\n\npublic class One { void Go() { } }\n",
            ),
            (
                "b/Two.cs",
                "namespace Acme.Util;\n\npublic class Two { void Go() { } }\n",
            ),
        ]);
        assert_eq!(units.list.len(), 1);
        assert_eq!(units.list[0].package, "Acme.Util");
        assert!(units.list[0].manifests.is_empty());
    }

    #[test]
    fn a_project_that_names_no_reference_depends_on_what_it_imports() {
        let units = discovery(&[
            ("src/Core/Core.csproj", "<Project />"),
            (
                "src/Core/Engine.cs",
                "namespace Acme.Core;\n\npublic class Engine { public void Run() { } }\n",
            ),
            ("src/App/App.csproj", "<Project />"),
            (
                "src/App/Use.cs",
                "using Acme.Core;\nnamespace Acme.App;\n\npublic class Use { void Go() { new Engine().Run(); } }\n",
            ),
        ]);
        assert_eq!(units.by_name("app").unwrap().deps, ["core"]);
    }

    #[test]
    fn recognises_go_test_files() {
        for path in [
            "internal/store/store_test.go",
            "store_test.go",
            "internal/store/testdata/fixture.go",
        ] {
            assert!(is_test_file(path), "{path}");
        }
        for path in [
            "internal/store/store.go",
            "internal/store/testing_helpers.go",
            "internal/contest.go",
        ] {
            assert!(!is_test_file(path), "{path}");
        }
        // `testdata` is the Go tool's, no other language's.
        assert!(!is_test_file("data/testdata/fixture.py"));
    }

    #[test]
    fn a_go_module_is_a_unit() {
        let units = discovery(&[
            (
                "go.mod",
                "module github.com/acme/app/v2

go 1.22

require github.com/acme/shared v1.0.0
require github.com/spf13/cobra v1.8.0
",
            ),
            (
                "go.sum",
                "github.com/spf13/cobra v1.8.0 h1:x=
",
            ),
            (
                "go.work",
                "go 1.22
use (
	.
	./shared
)
",
            ),
            (
                "cmd/app/main.go",
                "package main

import \"github.com/acme/app/v2/internal/store\"

func main() { store.New() }
",
            ),
            (
                "cmd/worker/main.go",
                "package main

func main() {}

func helper() {}
",
            ),
            (
                "cmd/worker/main_test.go",
                "package main

func main2() {}
",
            ),
            (
                "internal/store/store.go",
                "package store

func New() int { return 1 }

func main() {}
",
            ),
            (
                "internal/store/store_test.go",
                "package store

func TestNew() {}
",
            ),
            (
                "internal/store/testdata/fixture.go",
                "package fixture

func Fixture() {}
",
            ),
            (
                "shared/go.mod",
                "module github.com/acme/shared
",
            ),
            (
                "shared/shared.go",
                "package shared

func Shared() {}
",
            ),
            (
                "tools/gen/go.mod",
                "module example.com/gen

require github.com/acme/app/v2 v2.0.0

replace github.com/acme/app/v2 => ../..
",
            ),
            (
                "tools/gen/main.go",
                "package main

func main() {}
",
            ),
            (
                "other/go.mod",
                "module example.com/other
",
            ),
            (
                "other/other.go",
                "package other

import \"github.com/acme/shared\"

func Other() { shared.Shared() }
",
            ),
        ]);
        let names: Vec<&str> = units.list.iter().map(|u| u.name.as_str()).collect();
        assert_eq!(names, ["app", "other", "shared", "gen"]);

        let app = units.by_name("app").unwrap();
        // Named from the module path, without its major version.
        assert_eq!(app.package, "app");
        assert_eq!(app.language, Language::Go);
        assert_eq!(app.dir, "");
        let manifests: Vec<&str> = app.manifests.iter().map(|m| m.path.as_str()).collect();
        assert_eq!(manifests, ["go.mod", "go.work"]);
        assert_eq!(app.lock.as_ref().unwrap().path, "go.sum");
        assert_eq!(app.description, "");
        // The `cmd` folders belong to the module's unit.
        assert!(
            units
                .of_file("cmd/app/main.go")
                .is_some_and(|u| u.name == "app")
        );
        assert!(
            units
                .of_file("internal/store/store.go")
                .is_some_and(|u| u.name == "app")
        );
        // No `.go` file at the module's root: no package stands for the module.
        assert_eq!(app.root, None);
        assert_eq!(app.strip, 4);

        // Two programs, neither one the module's own way in; a `main` that
        // is not in `package main`, and one in a test file, start nothing.
        let entries: Vec<(&str, bool)> = app
            .entries
            .iter()
            .map(|e| (e.symbol.as_str(), e.declared))
            .collect();
        assert_eq!(
            entries,
            [
                ("github_com::acme::app::v2::cmd::app::main", false),
                ("github_com::acme::app::v2::cmd::worker::main", false),
            ]
        );
        assert!(app.has_bin);
        let generator = units.by_name("gen").unwrap();
        assert_eq!(generator.entries.len(), 1);
        assert!(generator.entries[0].declared);
        assert!(units.by_name("shared").unwrap().entries.is_empty());
        // The workspace file is a build file of the modules it lists, and of
        // no other.
        let shared: Vec<&str> = units
            .by_name("shared")
            .unwrap()
            .manifests
            .iter()
            .map(|m| m.path.as_str())
            .collect();
        assert_eq!(shared, ["shared/go.mod", "go.work"]);
        assert_eq!(generator.manifests.len(), 1);

        // `require` and a `replace` by a folder name another module of the
        // repository; a module that names none is read from its imports.
        assert_eq!(app.deps, ["shared"]);
        assert_eq!(generator.deps, ["app"]);
        assert_eq!(units.by_name("other").unwrap().deps, ["shared"]);

        // The fixtures of the Go tool are test code.
        assert!(is_test_file("internal/store/testdata/fixture.go"));
    }

    #[test]
    fn go_files_without_a_module_are_grouped_by_their_top_folder() {
        let units = discovery(&[
            (
                "tools/gen/main.go",
                "package main

func main() {}
",
            ),
            (
                "tools/gen/util.go",
                "package main

func util() {}
",
            ),
        ]);
        assert_eq!(units.list.len(), 1);
        assert_eq!(units.list[0].package, "tools.gen");
        assert!(units.list[0].manifests.is_empty());
        assert_eq!(units.list[0].entries.len(), 1);
    }

    #[test]
    fn recognises_kotlin_test_files() {
        for path in [
            "src/test/kotlin/com/acme/AppTest.kt",
            "src/commonTest/kotlin/com/acme/Shared.kt",
            "src/jvmTest/kotlin/com/acme/Shared.kt",
            "app/src/androidTest/kotlin/Instrumented.kt",
            "src/main/kotlin/com/acme/AppTests.kt",
            "src/main/kotlin/com/acme/AppIT.kt",
            "src/main/kotlin/com/acme/UserSpec.kt",
        ] {
            assert!(is_test_file(path), "{path}");
            assert!(is_jvm_test_file(path), "{path}");
        }
        for path in [
            "src/main/kotlin/com/acme/App.kt",
            "src/commonMain/kotlin/com/acme/App.kt",
            "src/jvmMain/kotlin/com/acme/App.kt",
            "src/main/kotlin/com/acme/Contest.kt",
            "src/Test/kotlin/com/acme/App.kt",
        ] {
            assert!(!is_test_file(path), "{path}");
        }
        // A Java repository's notes do not change: the source sets of tests
        // are the test map's to know for a Java file.
        assert!(!is_test_file("src/integrationTest/java/com/acme/Flow.java"));
        assert!(is_jvm_test_file(
            "src/integrationTest/java/com/acme/Flow.java"
        ));
        assert!(is_jvm_test_file("src/test/java/com/acme/AppTest.java"));
        assert!(!is_jvm_test_file("src/main/java/com/acme/App.java"));
    }

    #[test]
    fn a_gradle_module_of_java_and_kotlin_is_one_unit() {
        let units = discovery(&[
            ("settings.gradle.kts", "include(\"app\", \"lib\")\n"),
            ("gradle/libs.versions.toml", "[libraries]\n"),
            ("app/build.gradle.kts", "plugins { kotlin(\"jvm\") }\n"),
            (
                "app/src/main/kotlin/com/acme/app/Main.kt",
                "package com.acme.app

import com.acme.store.Cart

fun main(args: Array<String>) {
    Cart().checkout()
}
",
            ),
            (
                "app/src/main/kotlin/com/acme/app/Tool.kt",
                "package com.acme.app

object Tool {
    @JvmStatic
    fun main(args: Array<String>) {}

    fun run() {}
}
",
            ),
            (
                "app/src/main/kotlin/com/acme/app/Other.kt",
                "package com.acme.other

fun main(x: Int) {}

fun helper() {}
",
            ),
            (
                "app/src/main/kotlin/com/acme/app/Hidden.kt",
                "package com.acme.hidden

private fun main() {}
",
            ),
            (
                "app/src/main/java/com/acme/store/Cart.java",
                "package com.acme.store;

public class Cart {
    public void checkout() {}
}
",
            ),
            (
                "app/src/test/kotlin/com/acme/app/MainTest.kt",
                "package com.acme.app.tests

fun main() {}
",
            ),
            ("lib/build.gradle", "plugins { id 'java' }\n"),
            (
                "lib/src/main/java/com/acme/lib/Util.java",
                "package com.acme.lib;

public class Util {
    public static String shout(String s) { return s; }
}
",
            ),
        ]);
        assert_eq!(
            units.list.len(),
            2,
            "{:?}",
            units.list.iter().map(|u| &u.name).collect::<Vec<_>>()
        );
        let app = units.by_name("app").unwrap();
        // Two Kotlin files for every Java file: the unit is Kotlin.
        assert_eq!(app.language, Language::Kotlin);
        assert_eq!(app.dir, "app");
        assert_eq!(
            app.manifests
                .iter()
                .map(|m| m.path.as_str())
                .collect::<Vec<_>>(),
            ["app/build.gradle.kts"]
        );
        assert_eq!(
            units
                .of_file("app/src/main/java/com/acme/store/Cart.java")
                .map(|u| u.name.as_str()),
            Some("app")
        );
        // The version catalog stands where a lock file does.
        assert_eq!(app.lock.as_ref().unwrap().path, "gradle/libs.versions.toml");
        // A top-level `main`, an `@JvmStatic` one, and none that takes
        // another parameter, is private or sits in a test.
        let entries: Vec<&str> = app.entries.iter().map(|e| e.symbol.as_str()).collect();
        assert_eq!(
            entries,
            ["com::acme::app::Tool::main", "com::acme::app::main"]
        );
        assert!(!app.entries[0].declared);
        // A module of Java alone is as it was: no catalog, Java.
        let lib = units.by_name("lib").unwrap();
        assert_eq!(lib.language, Language::Java);
        assert!(lib.lock.is_none());
    }

    #[test]
    fn recognises_c_and_cpp_test_files() {
        for path in [
            "tests/list.c",
            "test/unit/a.cpp",
            "src/list_test.c",
            "src/list_test.cc",
            "src/test_list.cpp",
            "src/list_tests.cpp",
            "src/ListTest.cpp",
            "src/ListTests.cxx",
        ] {
            assert!(is_test_file(path), "{path}");
        }
        for path in [
            "src/list.c",
            "src/latest.cpp",
            "src/Test.cpp",
            // Headers are named for what they declare; a header called
            // `...Test.h` is a helper of tests, not a test.
            "src/ListTest.h",
            "src/attest.h",
        ] {
            assert!(!is_test_file(path), "{path}");
        }
    }

    #[test]
    fn cmake_targets_are_the_units_of_c_code() {
        let units = discovery(&[
            (
                "CMakeLists.txt",
                "project(demo)\nadd_subdirectory(core)\nadd_subdirectory(app)\nadd_subdirectory(tests)\nfind_package(ZLIB)\n",
            ),
            (
                "core/CMakeLists.txt",
                "add_library(core src/list.c src/map.c)\ntarget_include_directories(core PUBLIC include)\n",
            ),
            ("core/include/core/list.h", "int list_len(void);\n"),
            (
                "core/src/list.c",
                "#include \"core/list.h\"\nint list_len(void) { return 0; }\n",
            ),
            ("core/src/map.c", "int map_len(void) { return 1; }\n"),
            (
                "app/CMakeLists.txt",
                "add_executable(demo main.c)\ntarget_link_libraries(demo core)\n",
            ),
            (
                "app/main.c",
                "#include \"core/list.h\"\nint main(void) { return list_len(); }\n",
            ),
            (
                "tests/CMakeLists.txt",
                "add_executable(core_tests list_test.cpp)\ntarget_link_libraries(core_tests core GTest::gtest_main)\ngtest_discover_tests(core_tests)\n",
            ),
            (
                "tests/list_test.cpp",
                "#include \"core/list.h\"\nTEST(List, Len) { list_len(); }\n",
            ),
            ("vcpkg.json", "{\"dependencies\": [\"zlib\"]}"),
            ("conanfile.txt", "[requires]\nzlib/1.3.1\n"),
            ("conan.lock", "{\"requires\": [\"zlib/1.3.1#aa\"]}"),
            ("CMakePresets.json", "{\"version\": 3}"),
        ]);
        let names: Vec<&str> = units.list.iter().map(|u| u.name.as_str()).collect();
        assert_eq!(names, ["demo", "core"]);
        let core = units.by_name("core").unwrap();
        assert_eq!(core.language, Language::C);
        assert!(core.has_lib && !core.has_bin);
        assert_eq!(core.dir, "core");
        // The header sits in the unit of the source that includes it.
        assert_eq!(
            units
                .of_file("core/include/core/list.h")
                .map(|u| u.name.as_str()),
            Some("core")
        );
        let manifests: Vec<&str> = core.manifests.iter().map(|m| m.path.as_str()).collect();
        assert_eq!(
            manifests,
            [
                "core/CMakeLists.txt",
                "CMakePresets.json",
                "vcpkg.json",
                "conanfile.txt"
            ]
        );
        assert_eq!(core.lock.as_ref().unwrap().path, "conan.lock");
        // A program, its entry, and the library it links.
        let app = units.by_name("demo").unwrap();
        assert!(app.has_bin && !app.has_lib);
        assert_eq!(app.deps, ["core"]);
        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].symbol, "app::main::main");
        assert!(app.entries[0].declared);
        // The test executable is registered with CTest: its files are test
        // code, and it has no notes about its code.
        assert!(units.is_test("tests/list_test.cpp"));
        assert!(!units.is_test("core/src/list.c"));
        assert!(
            units.list.iter().all(|u| u.name != "core_tests")
                || units.by_name("core_tests").unwrap().tests
        );
    }

    #[test]
    fn c_files_without_cmake_are_grouped_by_folder() {
        let units = discovery(&[
            ("src/main.c", "int main(void) { return 0; }\n"),
            (
                "src/net/socket.c",
                "#include \"socket.h\"\nint open_socket(void) { return 1; }\n",
            ),
            ("src/net/socket.h", "int open_socket(void);\n"),
            ("src/net/pool.c", "int pool(void) { return 2; }\n"),
            ("src/db/store.c", "int store(void) { return 3; }\n"),
            ("libs/zip/zip.c", "int zip(void) { return 4; }\n"),
            ("libs/zip/zip.h", "int zip(void);\n"),
            ("libs/zip/zip_extra.c", "int extra(void) { return 5; }\n"),
        ]);
        let mut names: Vec<&str> = units.list.iter().map(|u| u.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, ["db", "net", "root-c", "zip"]);
        // A header and its source are in one unit.
        assert_eq!(
            units.of_file("src/net/socket.h").map(|u| u.name.as_str()),
            Some("net")
        );
        assert_eq!(
            units.of_file("src/net/pool.c").map(|u| u.name.as_str()),
            Some("net")
        );
        assert_eq!(units.of_file("src/main.c").unwrap().entries.len(), 1);
        assert_eq!(units.by_name("net").unwrap().manifests, []);
        assert!(!units.by_name("net").unwrap().has_bin);
    }

    #[test]
    fn a_unit_may_hold_both_c_and_cpp() {
        let units = discovery(&[
            ("CMakeLists.txt", "add_library(mixed a.c b.cpp)\n"),
            ("a.c", "int a(void) { return 1; }\n"),
            ("b.cpp", "int b() { return 2; }\n"),
            ("b2.cpp", "int b2() { return 3; }\n"),
        ]);
        let unit = units.by_name("mixed").unwrap();
        // The language with more files names the unit.
        assert_eq!(unit.language, Language::Cpp);
        assert_eq!(units.list.len(), 1);
        assert_eq!(
            units.of_file("b2.cpp").map(|u| u.name.as_str()),
            Some("mixed")
        );
    }

    #[test]
    fn a_vendored_tree_has_no_unit() {
        let units = discovery(&[
            ("src/a.c", "int a(void) { return 1; }\n"),
            ("third_party/zlib/z.c", "int z(void) { return 2; }\n"),
        ]);
        assert!(units.of_file("third_party/zlib/z.c").is_none());
        assert!(units.of_file("src/a.c").is_some());
    }
}
