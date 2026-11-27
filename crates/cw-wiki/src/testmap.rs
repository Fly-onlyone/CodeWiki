//! Test map: which tests exist, what they exercise, and which to run after a
//! change.
//!
//! What a test exercises is read from the file dependency graph: a test file
//! exercises the files it depends on, directly or through further hops. No
//! coverage data is involved, so "exercises" means "can reach", not
//! "executes". That part is the same for every language.
//!
//! Two things differ between languages, and a [`TestLanguage`] holds both:
//! what a test is (which files hold tests, and which of their parts are
//! tests) and the command that runs it. Rust finds tests with a lexical scan
//! ([`crate::rustscan`]) because the structural parser leaves unit-test
//! modules out of the index; C# reads them from the attributes the parser
//! recorded; Go reads them from the names and signatures of the functions of
//! its `_test.go` files; C and C++ read them from the test macros the parser
//! recorded (`TEST`, `TEST_CASE`, `BOOST_AUTO_TEST_CASE`) and from what CMake
//! registers with CTest; Python reads the functions and methods the parser
//! recorded (`python.rs`); TypeScript and JavaScript read the titled calls
//! of `test`, `it` and `describe` from the file's text (`js.rs`). A language joins the map by adding one entry to
//! [`LANGUAGES`].

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::Path;

use cw_core::{Language, Result, Symbol, SymbolKind};
use cw_index::Index;
use cw_parse::cmake::{Cmake, Target, read_cmake};
use cw_parse::{CrateInfo, JsRepo, PythonRoots, c_module_map, module_path, owning_crate};
use serde::{Deserialize, Serialize};

use crate::gitintel::dependents_by_file;
use crate::rustscan::{Scanned, is_test_path};
mod js;
mod python;

use crate::units::{
    is_csharp_test_attribute, is_jvm_test_file, is_test_file, xml_attribute, xml_elements,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestKind {
    /// Tests inside a source file, usually in `#[cfg(test)] mod tests`.
    Unit,
    /// Tests in a file kept apart from the code: under a `tests/` directory,
    /// or in a test project.
    Integration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub line: u32,
}

/// A file that contains tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestFile {
    pub path: String,
    pub kind: TestKind,
    pub tests: Vec<TestCase>,
    /// Command that runs these tests; empty when none can be derived.
    pub command: String,
}

/// How one source file is tested.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileTests {
    pub path: String,
    /// Tests written inside the file itself.
    pub unit_tests: usize,
    /// Other test files that depend on this file directly.
    pub direct: Vec<String>,
    /// Test files that reach this file only through other files.
    pub indirect: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestMap {
    pub tests_total: usize,
    pub test_files: Vec<TestFile>,
    /// Every source file of a language the map knows, outside test code, by
    /// path.
    pub files: Vec<FileTests>,
    /// Source files that define functions, have no unit tests, and are
    /// reached by no test file.
    pub untested: Vec<String>,
}

/// What a language's test code is, found in a repository. Built once for a
/// run from the index and the checkout.
struct Env<'a> {
    root: &'a Path,
    crates: Vec<CrateInfo>,
    symbols_by_file: HashMap<String, Vec<Symbol>>,
    /// C# project files by their folder.
    projects: BTreeMap<String, String>,
    /// The C# project files that hold tests.
    test_projects: HashSet<String>,
    /// The folders of the Go modules, by `go.mod`.
    go_modules: BTreeSet<String>,
    /// Every file of the repository, for the build files of Gradle and Maven.
    files: HashSet<String>,
    /// The CMake targets and the tests registered with CTest.
    cmake: Cmake,
    /// What the JavaScript build files say: packages, workspaces, the
    /// package manager of each.
    js: JsRepo,
    /// The import roots and projects of Python.
    py_roots: PythonRoots,
    /// What each Python project says about its test runner, read when asked.
    py_projects: RefCell<python::Projects>,
}

impl Env<'_> {
    fn read(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(self.root.join(path)).ok()
    }

    fn symbols_of(&self, path: &str) -> &[Symbol] {
        self.symbols_by_file.get(path).map_or(&[], Vec::as_slice)
    }
}

/// The two things that differ between languages: what a test is, and the
/// command that runs one. Everything else about the map is reachability over
/// the dependency graph.
trait TestLanguage: Sync {
    fn language(&self) -> Language;

    /// True for a language whose tests this entry runs. Java and Kotlin
    /// share one: a Gradle or Maven module runs the tests of both with one
    /// command.
    fn covers(&self, language: Language) -> bool {
        language == self.language()
    }

    /// True for a file of test code kept apart from the code it tests. Such
    /// a file is no source file of the map.
    fn is_test_file(&self, env: &Env, path: &str) -> bool;

    /// The tests in a file, in source order.
    fn tests(&self, env: &Env, path: &str, text: &str) -> Vec<TestCase>;

    /// The command that runs the tests of `path`; empty when none can be
    /// derived.
    fn command(&self, env: &Env, path: &str, kind: TestKind, tests: &[TestCase]) -> String;

    /// The distinct commands that run every one of `files`, nearest first.
    /// A language whose runner takes several files in one run may fold them.
    fn commands(&self, files: &[&ImpactedTestFile]) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for file in files {
            if !file.command.is_empty() && !out.contains(&file.command) {
                out.push(file.command.clone());
            }
        }
        out
    }
}

/// The languages whose tests the map follows. Adding a language is adding an
/// entry here and the type that fills in [`TestLanguage`]: reachability,
/// the report, the MCP tool and the app need no change.
static LANGUAGES: [&dyn TestLanguage; 7] = [
    &RustTests,
    &CSharpTests,
    &GoTests,
    &JvmTests,
    &CTests,
    &python::PythonTests,
    &js::JsTests,
];

fn language_of(language: Language) -> Option<&'static dyn TestLanguage> {
    LANGUAGES.iter().copied().find(|l| l.covers(language))
}

struct RustTests;

impl TestLanguage for RustTests {
    fn language(&self) -> Language {
        Language::Rust
    }

    fn is_test_file(&self, _: &Env, path: &str) -> bool {
        is_test_path(path)
    }

    fn tests(&self, _: &Env, _: &str, text: &str) -> Vec<TestCase> {
        Scanned::new(text)
            .tests
            .into_iter()
            .map(|t| TestCase {
                name: t.name,
                line: t.line,
            })
            .collect()
    }

    fn command(&self, env: &Env, path: &str, kind: TestKind, _: &[TestCase]) -> String {
        command(&env.crates, path, kind)
    }
}

fn command(crates: &[CrateInfo], path: &str, kind: TestKind) -> String {
    let Some(krate) = owning_crate(crates, path) else {
        return String::new();
    };
    let rel = if krate.dir.is_empty() {
        path
    } else {
        &path[krate.dir.len() + 1..]
    };
    match kind {
        TestKind::Integration => {
            // Only files directly in `tests/` are test targets.
            match rel
                .strip_prefix("tests/")
                .and_then(|r| r.strip_suffix(".rs"))
            {
                Some(stem) if !stem.contains('/') => {
                    format!("cargo test -p {} --test {stem}", krate.package)
                }
                _ => String::new(),
            }
        }
        TestKind::Unit => {
            if rel == "src/main.rs" || rel.starts_with("src/bin/") {
                return format!("cargo test -p {} --bins", krate.package);
            }
            let target = if krate.has_lib { " --lib" } else { "" };
            let module = module_path(crates, path);
            match module.split_once("::") {
                Some((_, inner)) => {
                    format!("cargo test -p {}{target} {inner}::", krate.package)
                }
                None => format!("cargo test -p {}{target}", krate.package),
            }
        }
    }
}

struct CSharpTests;

impl CSharpTests {
    /// The project file of the folder nearest above `path`.
    fn project<'e>(env: &'e Env, path: &str) -> Option<&'e String> {
        let mut dir = path.rsplit_once('/').map_or("", |(d, _)| d);
        loop {
            if let Some(found) = env.projects.get(dir) {
                return Some(found);
            }
            if dir.is_empty() {
                return None;
            }
            dir = dir.rsplit_once('/').map_or("", |(d, _)| d);
        }
    }

    /// The types that stand outermost above the test methods of `path`: a
    /// filter on one matches the types nested in it too.
    fn test_types(env: &Env, path: &str) -> Vec<String> {
        let symbols = env.symbols_of(path);
        let mut out: Vec<String> = Vec::new();
        for symbol in symbols.iter().filter(|s| is_csharp_test(s)) {
            let mut owner = symbol.parent.as_deref();
            let mut outermost = None;
            while let Some(q) = owner {
                match env
                    .symbols_by_file
                    .get(path)
                    .and_then(|all| all.iter().find(|s| s.qualified == q))
                    .filter(|s| s.kind.is_type())
                {
                    Some(ty) => {
                        outermost = Some(ty.qualified.clone());
                        owner = ty.parent.as_deref();
                    }
                    None => break,
                }
            }
            if let Some(name) = outermost.map(|q| q.replace("::", "."))
                && !out.contains(&name)
            {
                out.push(name);
            }
        }
        out
    }
}

/// A test method: an attribute of a test framework on a method.
fn is_csharp_test(symbol: &Symbol) -> bool {
    symbol.kind == SymbolKind::Method && symbol.attrs.iter().any(|a| is_csharp_test_attribute(a))
}

impl TestLanguage for CSharpTests {
    fn language(&self) -> Language {
        Language::CSharp
    }

    /// A file of a test project, or one named or placed as tests are.
    fn is_test_file(&self, env: &Env, path: &str) -> bool {
        is_test_file(path)
            || Self::project(env, path).is_some_and(|project| env.test_projects.contains(project))
    }

    fn tests(&self, env: &Env, path: &str, _: &str) -> Vec<TestCase> {
        env.symbols_of(path)
            .iter()
            .filter(|s| is_csharp_test(s))
            .map(|s| TestCase {
                name: format!(
                    "{}.{}",
                    s.parent
                        .as_deref()
                        .and_then(|p| p.rsplit("::").next())
                        .unwrap_or(""),
                    s.name
                ),
                line: s.start_line,
            })
            .collect()
    }

    /// One run for each project, over every class of its impacted files; a
    /// file whose command names no class makes the run the whole project.
    fn commands(&self, files: &[&ImpactedTestFile]) -> Vec<String> {
        // Project command -> the classes to run, or `None` for all of them.
        let mut runs: Vec<(String, Option<Vec<String>>)> = Vec::new();
        for file in files.iter().filter(|f| !f.command.is_empty()) {
            let (base, filter) = match file.command.split_once(" --filter \"") {
                Some((base, rest)) => (
                    base.to_string(),
                    Some(
                        rest.trim_end_matches('"')
                            .split('|')
                            .map(str::to_string)
                            .collect::<Vec<_>>(),
                    ),
                ),
                None => (file.command.clone(), None),
            };
            match runs.iter_mut().find(|(known, _)| *known == base) {
                Some((_, known)) => {
                    *known = match (known.take(), filter) {
                        (Some(mut have), Some(more)) => {
                            for part in more {
                                if !have.contains(&part) {
                                    have.push(part);
                                }
                            }
                            Some(have)
                        }
                        _ => None,
                    };
                }
                None => runs.push((base, filter)),
            }
        }
        runs.into_iter()
            .map(|(base, filter)| match filter {
                Some(parts) => format!("{base} --filter \"{}\"", parts.join("|")),
                None => base,
            })
            .collect()
    }

    /// `dotnet test <project>`, narrowed to the test classes of the file
    /// when the project runs under VSTest, whose `--filter` is the same in
    /// xunit, NUnit and MSTest. A project on Microsoft.Testing.Platform has
    /// another filter syntax, so it runs whole.
    fn command(&self, env: &Env, path: &str, _: TestKind, _: &[TestCase]) -> String {
        let Some(project) = Self::project(env, path) else {
            return String::new();
        };
        let quoted = |p: &str| {
            if p.contains(' ') {
                format!("\"{p}\"")
            } else {
                p.to_string()
            }
        };
        let mut command = format!("dotnet test {}", quoted(project));
        let platform = env
            .read("global.json")
            .is_some_and(|text| text.contains("Microsoft.Testing.Platform"));
        let vstest = !platform
            && env.read(project).is_some_and(|text| {
                xml_elements(&text, "PackageReference")
                    .iter()
                    .filter_map(|(attributes, _)| xml_attribute(attributes, "Include"))
                    .any(|id| id.eq_ignore_ascii_case("Microsoft.NET.Test.Sdk"))
            });
        let types = Self::test_types(env, path);
        if vstest && !types.is_empty() {
            let filter: Vec<String> = types
                .iter()
                .map(|t| format!("FullyQualifiedName~{t}"))
                .collect();
            command.push_str(&format!(" --filter \"{}\"", filter.join("|")));
        }
        command
    }
}

/// More tests than this in one package, and the package runs whole: a list of
/// names would be longer than the run it saves.
const MAX_GO_NAMES: usize = 8;

struct GoTests;

/// What kind of test a Go function is, by the rules of `go test`: the name
/// starts with `Test`, `Benchmark`, `Fuzz` or `Example` and what follows does
/// not start with a lower-case letter, and the parameter says which: a
/// `*testing.T`, `*testing.B` or `*testing.F`, none for an example.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GoTest {
    Test,
    Benchmark,
    Fuzz,
    Example,
}

fn go_test_kind(symbol: &Symbol) -> Option<GoTest> {
    if symbol.kind != SymbolKind::Function || !symbol.file.ends_with("_test.go") {
        return None;
    }
    let after = |prefix: &str| -> bool {
        symbol
            .name
            .strip_prefix(prefix)
            .is_some_and(|rest| !rest.starts_with(char::is_lowercase))
    };
    let takes = |ty: &str| symbol.signature.contains(&format!("testing.{ty}"));
    if after("Test") && takes("T") {
        Some(GoTest::Test)
    } else if after("Benchmark") && takes("B") {
        Some(GoTest::Benchmark)
    } else if after("Fuzz") && takes("F") {
        Some(GoTest::Fuzz)
    } else if after("Example") && symbol.signature.trim_end().ends_with("()") {
        Some(GoTest::Example)
    } else {
        None
    }
}

impl GoTests {
    /// The module folder of `dir` and the package path `go test` is given
    /// from there: `./internal/store`, or `.` for the module's root.
    fn package(env: &Env, dir: &str) -> (String, String) {
        let module = env
            .go_modules
            .iter()
            .filter(|m| m.is_empty() || dir == m.as_str() || dir.starts_with(&format!("{m}/")))
            .max_by_key(|m| m.len())
            .cloned()
            .unwrap_or_default();
        let below = dir
            .strip_prefix(module.as_str())
            .unwrap_or(dir)
            .trim_start_matches('/');
        let package = if below.is_empty() {
            ".".to_string()
        } else {
            format!("./{below}")
        };
        (module, package)
    }
}

/// The parts of a command [`GoTests::command`] wrote: what runs the package,
/// the names `-run` selects and the names `-bench` selects.
fn split_go_command(command: &str) -> (String, Vec<String>, Vec<String>) {
    let names = |flag: &str| -> Option<(usize, Vec<String>)> {
        let at = command.find(&format!(" {flag} '"))?;
        let rest = &command[at + flag.len() + 3..];
        let inner = &rest[..rest.find('\'')?];
        let list = inner.trim_start_matches('^').trim_end_matches('$');
        let list = list
            .strip_prefix('(')
            .and_then(|l| l.strip_suffix(')'))
            .unwrap_or(list);
        Some((
            at,
            list.split('|')
                .filter(|n| !n.is_empty())
                .map(str::to_string)
                .collect(),
        ))
    };
    let run = names("-run");
    let bench = names("-bench");
    let end = [run.as_ref(), bench.as_ref()]
        .into_iter()
        .flatten()
        .map(|(at, _)| *at)
        .min()
        .unwrap_or(command.len());
    (
        command[..end].to_string(),
        run.map(|(_, n)| n).unwrap_or_default(),
        bench.map(|(_, n)| n).unwrap_or_default(),
    )
}

impl TestLanguage for GoTests {
    fn language(&self) -> Language {
        Language::Go
    }

    /// A `_test.go` file, or one under a `testdata` folder.
    fn is_test_file(&self, _: &Env, path: &str) -> bool {
        is_test_file(path)
    }

    fn tests(&self, env: &Env, path: &str, _: &str) -> Vec<TestCase> {
        env.symbols_of(path)
            .iter()
            .filter(|s| go_test_kind(s).is_some())
            .map(|s| TestCase {
                name: s.name.clone(),
                line: s.start_line,
            })
            .collect()
    }

    /// `go test <package> -run '^(TestA|TestB)$'`, from the module's folder
    /// (`-C`) when it is not the repository root. Tests, fuzz targets and
    /// examples go to `-run`, benchmarks to `-bench`; a file of benchmarks
    /// alone runs no test (`-run '^$'`).
    fn command(&self, env: &Env, path: &str, _: TestKind, tests: &[TestCase]) -> String {
        let dir = path.rsplit_once('/').map_or("", |(d, _)| d);
        let (module, package) = Self::package(env, dir);
        let quoted = |p: &str| {
            if p.contains(' ') {
                format!("\"{p}\"")
            } else {
                p.to_string()
            }
        };
        let mut command = String::from("go test");
        if !module.is_empty() {
            command.push_str(&format!(" -C {}", quoted(&module)));
        }
        command.push_str(&format!(" {}", quoted(&package)));
        let kinds: HashMap<&str, GoTest> = env
            .symbols_of(path)
            .iter()
            .filter_map(|s| Some((s.name.as_str(), go_test_kind(s)?)))
            .collect();
        let (benches, runs): (Vec<&str>, Vec<&str>) = tests
            .iter()
            .map(|t| t.name.as_str())
            .partition(|n| kinds.get(n) == Some(&GoTest::Benchmark));
        if runs.is_empty() {
            command.push_str(" -run '^$'");
        } else {
            command.push_str(&format!(" -run '^({})$'", runs.join("|")));
        }
        if !benches.is_empty() {
            command.push_str(&format!(" -bench '^({})$'", benches.join("|")));
        }
        command
    }

    /// One run for each package, over the tests of every impacted file in
    /// it. Past [`MAX_GO_NAMES`] names the package runs whole.
    fn commands(&self, files: &[&ImpactedTestFile]) -> Vec<String> {
        // Package command -> the names `-run` takes, and the ones `-bench` takes.
        let mut runs: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
        for file in files.iter().filter(|f| !f.command.is_empty()) {
            let (base, run, bench) = split_go_command(&file.command);
            let at = match runs.iter().position(|(known, _, _)| *known == base) {
                Some(at) => at,
                None => {
                    runs.push((base, Vec::new(), Vec::new()));
                    runs.len() - 1
                }
            };
            for name in run {
                if !runs[at].1.contains(&name) {
                    runs[at].1.push(name);
                }
            }
            for name in bench {
                if !runs[at].2.contains(&name) {
                    runs[at].2.push(name);
                }
            }
        }
        runs.into_iter()
            .map(|(base, run, bench)| {
                let mut command = base;
                // Past the limit: every test of the package, no filter.
                if run.len() <= MAX_GO_NAMES {
                    if run.is_empty() {
                        command.push_str(" -run '^$'");
                    } else {
                        command.push_str(&format!(" -run '^({})$'", run.join("|")));
                    }
                }
                if !bench.is_empty() {
                    command.push_str(&format!(" -bench '^({})$'", bench.join("|")));
                }
                command
            })
            .collect()
    }
}

/// More test classes than this in one module and the module's tests run
/// whole: a list of names would be longer than the run it saves.
const MAX_JVM_CLASSES: usize = 8;

/// The build files that make a folder a Java or Kotlin module.
const JVM_BUILD_FILES: [&str; 3] = ["build.gradle", "build.gradle.kts", "pom.xml"];

/// Annotations that make a method a test: JUnit 4 and 5, TestNG and
/// `kotlin.test` all call theirs `Test`, and JUnit 5 adds the parameterized,
/// repeated, factory and template forms.
const JVM_TEST_ANNOTATIONS: [&str; 5] = [
    "Test",
    "ParameterizedTest",
    "RepeatedTest",
    "TestFactory",
    "TestTemplate",
];

/// A test: a method with one of the annotations (as written: `Test`,
/// `org.junit.Test`), or a class that extends one of Kotest's spec styles,
/// whose tests are written in its constructor and carry no annotation.
fn is_jvm_test(symbol: &Symbol) -> bool {
    match symbol.kind {
        SymbolKind::Method => symbol
            .attrs
            .iter()
            .any(|a| JVM_TEST_ANNOTATIONS.contains(&a.rsplit('.').next().unwrap_or(a))),
        SymbolKind::Class => symbol.attrs.iter().any(|a| a == cw_parse::KOTEST_ATTR),
        _ => false,
    }
}

fn parent_dir(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

fn join_path(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

/// Java and Kotlin tests, run by Gradle or Maven. A file is a test file when
/// it is test code by name or by its Gradle source set (`src/test`,
/// `src/commonTest`, `src/jvmTest`). One command runs a module's tests of
/// both languages, so the two share this entry.
struct JvmTests;

impl JvmTests {
    /// The folder of the module that owns `path`: the nearest one above it
    /// that holds a build file.
    fn module(env: &Env, path: &str) -> Option<String> {
        let mut dir = parent_dir(path);
        loop {
            if JVM_BUILD_FILES
                .iter()
                .any(|name| env.files.contains(&join_path(dir, name)))
            {
                return Some(dir.to_string());
            }
            if dir.is_empty() {
                return None;
            }
            dir = parent_dir(dir);
        }
    }

    fn is_gradle(env: &Env, module: &str) -> bool {
        ["build.gradle", "build.gradle.kts"]
            .iter()
            .any(|name| env.files.contains(&join_path(module, name)))
    }

    /// The classes a run is told to filter on: the type that stands
    /// outermost above each test, in the language's own name
    /// (`com.acme.AppTest`), with `*` after it when a nested class holds
    /// tests, which a filter on the outer class alone would miss.
    fn classes(env: &Env, path: &str) -> Vec<String> {
        let symbols = env.symbols_of(path);
        let mut out: Vec<String> = Vec::new();
        for test in symbols.iter().filter(|s| is_jvm_test(s)) {
            let owner = if test.kind == SymbolKind::Class {
                Some(test)
            } else {
                symbols
                    .iter()
                    .find(|s| Some(s.qualified.as_str()) == test.parent.as_deref())
            };
            let Some(mut current) = owner else {
                continue;
            };
            let mut nested = false;
            while let Some(parent) = symbols.iter().find(|s| {
                Some(s.qualified.as_str()) == current.parent.as_deref() && s.kind.is_type()
            }) {
                current = parent;
                nested = true;
            }
            let name = format!(
                "{}{}",
                current.qualified.replace("::", "."),
                if nested { "*" } else { "" }
            );
            if !out.contains(&name) {
                out.push(name);
            }
        }
        // `Outer*` already runs `Outer`.
        let starred: Vec<String> = out
            .iter()
            .filter_map(|c| c.strip_suffix('*').map(str::to_string))
            .collect();
        out.retain(|c| !starred.contains(c));
        out
    }

    /// The Gradle command that runs `classes` of the module at `module`.
    fn gradle(env: &Env, module: &str, path: &str, classes: &[String]) -> String {
        // The folder of `settings.gradle(.kts)`, nearest above the module,
        // is the root of the build; a project path is the module's folder
        // below it, with `:` for `/`. A `projectDir` set in the settings is
        // not read.
        let mut root = module;
        let root = loop {
            if ["settings.gradle", "settings.gradle.kts"]
                .iter()
                .any(|name| env.files.contains(&join_path(root, name)))
            {
                break root;
            }
            if root.is_empty() {
                break module;
            }
            root = parent_dir(root);
        };
        let below = module
            .strip_prefix(root)
            .unwrap_or("")
            .trim_start_matches('/');
        let quoted = |p: &str| {
            if p.contains(' ') {
                format!("\"{p}\"")
            } else {
                p.to_string()
            }
        };
        let tool = if env.files.contains(&join_path(root, "gradlew")) {
            if root.is_empty() {
                "./gradlew".to_string()
            } else {
                format!("./{}/gradlew -p {}", quoted(root), quoted(root))
            }
        } else if root.is_empty() {
            "gradle".to_string()
        } else {
            format!("gradle -p {}", quoted(root))
        };
        // The task of the source set the file stands in. Kotlin
        // Multiplatform names a set after its target (`jvmTest`) and runs
        // the shared tests (`commonTest`) through `allTests`, which takes no
        // filter.
        let set = path
            .strip_prefix(&join_path(module, "src/"))
            .and_then(|rest| rest.split('/').next());
        let (task, filterable) = match set {
            Some("commonTest") => ("allTests", false),
            Some(set) if set != "test" && set.ends_with("Test") => (set, true),
            _ => ("test", true),
        };
        let project = if below.is_empty() {
            String::new()
        } else {
            format!(":{}:", below.replace('/', ":"))
        };
        let mut command = format!("{tool} {project}{task}");
        if filterable && !classes.is_empty() && classes.len() <= MAX_JVM_CLASSES {
            for class in classes {
                command.push_str(&format!(" --tests \"{class}\""));
            }
        }
        command
    }

    /// The Maven command that runs `classes` of the module at `module`. A
    /// module below an aggregator `pom.xml` (one with `<modules>`) is picked
    /// with `-pl` from the aggregator's folder; a module of a build with no
    /// aggregator is the whole build and needs none.
    fn maven(env: &Env, module: &str, classes: &[String]) -> String {
        let mut reactor: Option<&str> = None;
        let mut dir = module;
        while !dir.is_empty() {
            dir = parent_dir(dir);
            let pom = join_path(dir, "pom.xml");
            if env.files.contains(&pom) && env.read(&pom).is_some_and(|t| t.contains("<modules>")) {
                reactor = Some(dir);
            }
        }
        let quoted = |p: &str| {
            if p.contains(' ') {
                format!("\"{p}\"")
            } else {
                p.to_string()
            }
        };
        let mut command = match reactor {
            Some(root) => {
                let below = module
                    .strip_prefix(root)
                    .unwrap_or(module)
                    .trim_start_matches('/');
                let from = if root.is_empty() {
                    String::new()
                } else {
                    format!(" -f {}", quoted(&join_path(root, "pom.xml")))
                };
                format!("mvn{from} -pl {} test", quoted(below))
            }
            None if module.is_empty() => "mvn test".to_string(),
            None => format!("mvn -f {} test", quoted(&join_path(module, "pom.xml"))),
        };
        if !classes.is_empty() && classes.len() <= MAX_JVM_CLASSES {
            let names: Vec<String> = classes
                .iter()
                .map(|c| {
                    let simple = c.rsplit('.').next().unwrap_or(c);
                    simple.to_string()
                })
                .collect();
            command.push_str(&format!(" -Dtest={}", names.join(",")));
        }
        command
    }
}

/// The parts of a command [`JvmTests::command`] wrote: what runs the module,
/// and the classes it is filtered to (`None` for the whole module).
fn split_jvm_command(command: &str) -> (String, Option<Vec<String>>, bool) {
    if let Some(at) = command.find(" --tests \"") {
        let classes = command[at..]
            .split(" --tests \"")
            .skip(1)
            .map(|c| c.trim_end_matches('"').to_string())
            .collect();
        return (command[..at].to_string(), Some(classes), true);
    }
    if let Some(at) = command.find(" -Dtest=") {
        let classes = command[at + " -Dtest=".len()..]
            .split(',')
            .map(str::to_string)
            .collect();
        return (command[..at].to_string(), Some(classes), false);
    }
    (command.to_string(), None, true)
}

impl TestLanguage for JvmTests {
    fn language(&self) -> Language {
        Language::Java
    }

    fn covers(&self, language: Language) -> bool {
        matches!(language, Language::Java | Language::Kotlin)
    }

    fn is_test_file(&self, _: &Env, path: &str) -> bool {
        is_jvm_test_file(path)
    }

    fn tests(&self, env: &Env, path: &str, _: &str) -> Vec<TestCase> {
        env.symbols_of(path)
            .iter()
            .filter(|s| is_jvm_test(s))
            .map(|s| TestCase {
                name: if s.kind == SymbolKind::Class {
                    s.name.clone()
                } else {
                    format!(
                        "{}.{}",
                        s.parent
                            .as_deref()
                            .and_then(|p| p.rsplit("::").next())
                            .unwrap_or(""),
                        s.name
                    )
                },
                line: s.start_line,
            })
            .collect()
    }

    /// Gradle: `./gradlew :<project>:test --tests "<package.Class>"`, the
    /// option once for each class. Maven: `mvn -pl <module> test
    /// -Dtest=<Class>,<Class>`. The build tool is the one the file's module
    /// has a build file for, Gradle first; past [`MAX_JVM_CLASSES`] classes
    /// the module runs whole.
    fn command(&self, env: &Env, path: &str, _: TestKind, _: &[TestCase]) -> String {
        let Some(module) = Self::module(env, path) else {
            return String::new();
        };
        let classes = Self::classes(env, path);
        if Self::is_gradle(env, &module) {
            Self::gradle(env, &module, path, &classes)
        } else {
            Self::maven(env, &module, &classes)
        }
    }

    /// One run for each module and task, over every class of its impacted
    /// files; a file whose command names no class, and more than
    /// [`MAX_JVM_CLASSES`] classes, make the run the whole module.
    fn commands(&self, files: &[&ImpactedTestFile]) -> Vec<String> {
        // Module command -> its classes (`None` for all of them), and
        // whether the filter is Gradle's.
        let mut runs: Vec<(String, Option<Vec<String>>, bool)> = Vec::new();
        for file in files.iter().filter(|f| !f.command.is_empty()) {
            let (base, filter, gradle) = split_jvm_command(&file.command);
            match runs.iter_mut().find(|(known, _, _)| *known == base) {
                Some((_, known, _)) => {
                    *known = match (known.take(), filter) {
                        (Some(mut have), Some(more)) => {
                            for class in more {
                                if !have.contains(&class) {
                                    have.push(class);
                                }
                            }
                            Some(have)
                        }
                        _ => None,
                    };
                }
                None => runs.push((base, filter, gradle)),
            }
        }
        runs.into_iter()
            .map(|(base, filter, gradle)| match filter {
                Some(classes) if classes.len() <= MAX_JVM_CLASSES => {
                    if gradle {
                        let tests: String = classes
                            .iter()
                            .map(|c| format!(" --tests \"{c}\""))
                            .collect();
                        format!("{base}{tests}")
                    } else {
                        format!("{base} -Dtest={}", classes.join(","))
                    }
                }
                _ => base,
            })
            .collect()
    }
}

/// A test that reaches what a header declares reaches the source that
/// defines it, so a change to the source impacts it too. The header and the
/// source of one name are one module; every file of a module gets the files
/// that depend on any other file of it.
fn add_c_pair_dependents(files: &[String], dependents: &mut HashMap<String, BTreeSet<String>>) {
    let modules = c_module_map(files);
    if modules.is_empty() {
        return;
    }
    let mut by_module: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (file, module) in &modules {
        by_module.entry(module).or_default().push(file);
    }
    for members in by_module.values().filter(|m| m.len() > 1) {
        let mut all: BTreeSet<String> = BTreeSet::new();
        for member in members {
            all.extend(dependents.get(*member).into_iter().flatten().cloned());
        }
        for member in members {
            let own = dependents.entry((*member).to_string()).or_default();
            own.extend(all.iter().filter(|f| f.as_str() != *member).cloned());
        }
    }
}

/// More tests than this in one run and the run is the whole executable, or
/// every test CTest knows: a list of names would be longer than the run it
/// saves.
const MAX_C_TESTS: usize = 8;

/// The test frameworks of C and C++ code that the parser reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CFilter {
    /// `ctest -R`.
    Ctest,
    Gtest,
    Catch,
    Boost,
    Doctest,
}

/// How CMake registers the tests of an executable with CTest.
enum Registration {
    /// `gtest_discover_tests` and the like: CTest knows each test by its own
    /// name, which is `Suite.Name` for GoogleTest.
    Discovered,
    /// `add_test(NAME x COMMAND exe)`: CTest knows the whole executable as `x`.
    Plain(Vec<String>),
    None,
}

/// `text` as a regular expression that matches it and nothing else.
fn regex_escape(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if "\\^$.|?*+()[]{}".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

struct CTests;

impl CTests {
    /// The CMake target that lists `path`.
    fn target<'e>(env: &'e Env, path: &str) -> Option<&'e Target> {
        env.cmake
            .targets
            .iter()
            .find(|t| t.sources.iter().any(|s| s == path))
    }

    fn registration(env: &Env, target: &Target) -> Registration {
        let mut names = Vec::new();
        for test in &env.cmake.tests {
            if test.target.as_deref() != Some(target.name.as_str()) && test.name != target.name {
                continue;
            }
            if test.discovered.is_some() {
                return Registration::Discovered;
            }
            names.push(test.name.clone());
        }
        if names.is_empty() {
            Registration::None
        } else {
            Registration::Plain(names)
        }
    }

    /// The test frameworks' own names for a test: `Suite.Name` for GoogleTest.
    fn name_of(symbol: &Symbol) -> Option<&str> {
        symbol
            .attrs
            .iter()
            .find_map(|a| a.strip_prefix(cw_parse::TEST_NAME_ATTR))
    }

    fn framework(symbol: &Symbol) -> Option<&str> {
        symbol
            .attrs
            .iter()
            .find_map(|a| a.strip_prefix(cw_parse::TEST_FRAMEWORK_ATTR))
    }

    /// A CTest regex for one test.
    fn ctest_alternative(symbol: &Symbol, name: &str) -> String {
        let escaped = regex_escape(name);
        // A parameterized or typed test is known as `Prefix/Suite.Name/N`
        // or `Suite/0.Name`.
        if symbol.signature.starts_with("TEST_P") || symbol.signature.starts_with("TYPED_TEST") {
            match name.split_once('.') {
                Some((suite, case)) => {
                    format!("{}[./]{}", regex_escape(suite), regex_escape(case))
                }
                None => escaped,
            }
        } else {
            format!("^{escaped}$")
        }
    }
}

impl TestLanguage for CTests {
    fn language(&self) -> Language {
        Language::Cpp
    }

    fn covers(&self, language: Language) -> bool {
        language.is_c_family()
    }

    /// A file in a test folder or named like a test, and the sources of a
    /// CMake target that CTest runs or that links a test framework.
    fn is_test_file(&self, env: &Env, path: &str) -> bool {
        is_test_file(path)
            || Self::target(env, path).is_some_and(|t| {
                matches!(Self::registration(env, t), Registration::Discovered)
                    || crate::units::links_a_test_framework(&t.links)
            })
    }

    /// The test macros the parser found, and for a program CTest runs as a
    /// whole, its `main`.
    fn tests(&self, env: &Env, path: &str, _: &str) -> Vec<TestCase> {
        let found: Vec<TestCase> = env
            .symbols_of(path)
            .iter()
            .filter(|s| s.attrs.iter().any(|a| a == cw_parse::TEST_ATTR))
            .filter_map(|s| {
                Some(TestCase {
                    name: Self::name_of(s)?.to_string(),
                    line: s.start_line,
                })
            })
            .collect();
        if !found.is_empty() {
            return found;
        }
        let Some(target) = Self::target(env, path) else {
            return Vec::new();
        };
        let Registration::Plain(names) = Self::registration(env, target) else {
            return Vec::new();
        };
        // A program CTest runs as a smoke test is no file of tests.
        if !self.is_test_file(env, path) {
            return Vec::new();
        }
        env.symbols_of(path)
            .iter()
            .find(|s| s.kind == SymbolKind::Function && s.name == "main")
            .map(|main| {
                vec![TestCase {
                    name: names[0].clone(),
                    line: main.start_line,
                }]
            })
            .unwrap_or_default()
    }

    /// With CMake: `ctest --test-dir build -R '<tests>'` for what CTest
    /// knows by name, else the test executable with its framework's filter
    /// (`./build/<folder>/<target> --gtest_filter=A.B`), else every test:
    /// `ctest --test-dir build`. The folder `build` is a guess: it is where
    /// `cmake -S . -B build` puts a build. Without CMake there is no
    /// executable to name, and no command.
    fn command(&self, env: &Env, path: &str, _: TestKind, tests: &[TestCase]) -> String {
        let Some(target) = Self::target(env, path) else {
            return String::new();
        };
        let symbols: Vec<&Symbol> = env
            .symbols_of(path)
            .iter()
            .filter(|s| s.attrs.iter().any(|a| a == cw_parse::TEST_ATTR))
            .collect();
        let named: Vec<(&Symbol, &str)> = symbols
            .iter()
            .filter_map(|s| Some((*s, Self::name_of(s)?)))
            .filter(|(_, name)| tests.iter().any(|t| t.name == *name))
            .collect();
        match Self::registration(env, target) {
            Registration::Discovered if !named.is_empty() => format!(
                "ctest --test-dir build -R '{}'",
                named
                    .iter()
                    .map(|(s, n)| Self::ctest_alternative(s, n))
                    .collect::<Vec<_>>()
                    .join("|")
            ),
            Registration::Plain(names) => format!(
                "ctest --test-dir build -R '{}'",
                names
                    .iter()
                    .map(|n| format!("^{}$", regex_escape(n)))
                    .collect::<Vec<_>>()
                    .join("|")
            ),
            _ => {
                let program = if target.dir.is_empty() {
                    format!("./build/{}", target.name)
                } else {
                    format!("./build/{}/{}", target.dir, target.name)
                };
                let framework = named.first().and_then(|(s, _)| Self::framework(s));
                let names: Vec<&str> = named.iter().map(|(_, n)| *n).collect();
                match framework {
                    Some("gtest") if !names.is_empty() => {
                        format!("{program} --gtest_filter={}", names.join(":"))
                    }
                    Some("boost") if !names.is_empty() => {
                        format!("{program} --run_test={}", names.join(","))
                    }
                    Some("doctest") if !names.is_empty() => {
                        format!("{program} --test-case=\"{}\"", names.join(","))
                    }
                    // Catch2 and doctest read the same macro. A name with a
                    // comma or a quote cannot be written in the filter.
                    Some("catch")
                        if !names.is_empty()
                            && names.iter().all(|n| !n.contains([',', '"', '[', ']'])) =>
                    {
                        format!(
                            "{program} {}",
                            names
                                .iter()
                                .map(|n| format!("\"{n}\""))
                                .collect::<Vec<_>>()
                                .join(",")
                        )
                    }
                    _ => "ctest --test-dir build".to_string(),
                }
            }
        }
    }

    /// One command for each test executable, over the tests of every impacted
    /// file of it; every CTest regex folds into one `ctest` run. Past
    /// [`MAX_C_TESTS`] names the executable or CTest runs whole.
    fn commands(&self, files: &[&ImpactedTestFile]) -> Vec<String> {
        // base -> (filter, names); no names and no filter: the whole run.
        let mut runs: Vec<(String, Option<CFilter>, Vec<String>)> = Vec::new();
        for file in files.iter().filter(|f| !f.command.is_empty()) {
            let (base, filter, names) = split_c_command(&file.command);
            let at = match runs.iter().position(|(known, _, _)| *known == base) {
                Some(at) => at,
                None => {
                    runs.push((base, filter, Vec::new()));
                    runs.len() - 1
                }
            };
            match filter {
                Some(_) => {
                    for name in names {
                        if !runs[at].2.contains(&name) {
                            runs[at].2.push(name);
                        }
                    }
                }
                // Part of the run is unfiltered: run it all.
                None => runs[at].1 = None,
            }
            if filter.is_none() {
                runs[at].2.clear();
            }
        }
        runs.into_iter()
            .map(|(base, filter, names)| match filter {
                Some(_) if names.len() > MAX_C_TESTS => {
                    // Whole: every test of the executable, or of the build.
                    base
                }
                Some(CFilter::Ctest) => format!("{base} -R '{}'", names.join("|")),
                Some(CFilter::Gtest) => format!("{base} --gtest_filter={}", names.join(":")),
                Some(CFilter::Boost) => format!("{base} --run_test={}", names.join(",")),
                Some(CFilter::Doctest) => format!("{base} --test-case=\"{}\"", names.join(",")),
                Some(CFilter::Catch) => format!(
                    "{base} {}",
                    names
                        .iter()
                        .map(|n| format!("\"{n}\""))
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                None => base,
            })
            .collect()
    }
}

/// The parts of a command [`CTests::command`] wrote: what runs, how the tests
/// are chosen, and the names chosen.
fn split_c_command(command: &str) -> (String, Option<CFilter>, Vec<String>) {
    if let Some(rest) = command.strip_prefix("ctest --test-dir build -R '")
        && let Some(inner) = rest.strip_suffix('\'')
    {
        // Alternatives are split where a `|` is not escaped.
        let mut names: Vec<String> = Vec::new();
        let mut current = String::new();
        let mut escaped = false;
        for c in inner.chars() {
            match c {
                '\\' if !escaped => {
                    escaped = true;
                    current.push(c);
                    continue;
                }
                '|' if !escaped => names.push(std::mem::take(&mut current)),
                _ => current.push(c),
            }
            escaped = false;
        }
        names.push(current);
        return (
            "ctest --test-dir build".to_string(),
            Some(CFilter::Ctest),
            names,
        );
    }
    let split = |marker: &str, filter: CFilter, separator: &str, quoted: bool| {
        let at = command.find(marker)?;
        let mut tail = &command[at + marker.len()..];
        if quoted {
            tail = tail.strip_suffix('"').unwrap_or(tail);
        }
        Some((
            command[..at].to_string(),
            Some(filter),
            tail.split(separator).map(str::to_string).collect(),
        ))
    };
    if let Some(found) = split(" --gtest_filter=", CFilter::Gtest, ":", false)
        .or_else(|| split(" --run_test=", CFilter::Boost, ",", false))
        .or_else(|| split(" --test-case=\"", CFilter::Doctest, ",", true))
    {
        return found;
    }
    if let Some(at) = command.find(" \"") {
        let names = command[at + 2..]
            .trim_end_matches('"')
            .split("\",\"")
            .map(str::to_string)
            .collect();
        return (command[..at].to_string(), Some(CFilter::Catch), names);
    }
    (command.to_string(), None, Vec::new())
}

/// A Go test file builds with every file of its package, without importing
/// it: the tests of `package store` and of `package store_test` in one folder
/// both run for a change to any non-test file of that folder. Each such file
/// gets those test files as dependents. The edges stay out of the index, where
/// a package of hundreds of files would make tens of thousands of them.
fn add_go_package_dependents(files: &[String], dependents: &mut HashMap<String, BTreeSet<String>>) {
    let mut by_folder: BTreeMap<&str, (Vec<&String>, Vec<&String>)> = BTreeMap::new();
    for file in files.iter().filter(|f| f.ends_with(".go")) {
        let folder = file.rsplit_once('/').map_or("", |(dir, _)| dir);
        let entry = by_folder.entry(folder).or_default();
        if file.ends_with("_test.go") {
            entry.0.push(file);
        } else if !is_test_file(file) {
            entry.1.push(file);
        }
    }
    for (tests, sources) in by_folder.values() {
        for source in sources {
            dependents
                .entry((*source).clone())
                .or_default()
                .extend(tests.iter().map(|t| (*t).clone()));
        }
    }
}

/// Everything the analyses below need, gathered once.
struct Facts {
    test_files: Vec<TestFile>,
    /// Source files outside test code, of the languages the map knows.
    sources: Vec<String>,
    /// file -> files that depend on it.
    dependents: HashMap<String, BTreeSet<String>>,
}

fn gather(root: &Path, index: &Index) -> Result<Facts> {
    let records = index.files()?;
    let mut symbols_by_file: HashMap<String, Vec<Symbol>> = HashMap::new();
    for symbol in index.symbols()? {
        if language_of(Language::from_path(&symbol.file))
            .is_some_and(|l| l.language() != Language::Rust)
        {
            symbols_by_file
                .entry(symbol.file.clone())
                .or_default()
                .push(symbol);
        }
    }
    for symbols in symbols_by_file.values_mut() {
        symbols.sort_by_key(|s| (s.start_line, s.qualified.clone()));
    }
    let files: Vec<String> = records.iter().map(|f| f.path.clone()).collect();
    let mut projects: BTreeMap<String, String> = BTreeMap::new();
    for path in files.iter().filter(|f| f.ends_with(".csproj")) {
        let dir = path.rsplit_once('/').map_or("", |(d, _)| d);
        projects
            .entry(dir.to_string())
            .or_insert_with(|| path.clone());
    }
    let test_projects: HashSet<String> = projects
        .values()
        .filter(|path| {
            std::fs::read_to_string(root.join(path))
                .is_ok_and(|text| crate::units::references_a_test_framework(&text))
        })
        .cloned()
        .collect();
    let go_modules: BTreeSet<String> = files
        .iter()
        .filter(|f| f.rsplit('/').next() == Some("go.mod"))
        .map(|f| f.rsplit_once('/').map_or("", |(d, _)| d).to_string())
        .collect();
    let env = Env {
        root,
        crates: index.crates()?,
        symbols_by_file,
        projects,
        test_projects,
        go_modules,
        files: files.iter().cloned().collect(),
        cmake: if records.iter().any(|f| f.language.is_c_family()) {
            read_cmake(&files, &|path| {
                std::fs::read_to_string(root.join(path)).ok()
            })
        } else {
            Cmake::default()
        },
        js: JsRepo::read(&files, &|path| {
            std::fs::read_to_string(root.join(path)).ok()
        }),
        py_roots: PythonRoots::read(&files, &|path| {
            std::fs::read_to_string(root.join(path)).ok()
        }),
        py_projects: RefCell::new(HashMap::new()),
    };
    let mut test_files = Vec::new();
    let mut sources = Vec::new();
    for file in records {
        let Some(language) = language_of(file.language) else {
            continue;
        };
        let in_test_dir = language.is_test_file(&env, &file.path);
        if !in_test_dir {
            sources.push(file.path.clone());
        }
        let Ok(text) = std::fs::read_to_string(root.join(&file.path)) else {
            continue;
        };
        let tests = language.tests(&env, &file.path, &text);
        if tests.is_empty() {
            continue;
        }
        let kind = if in_test_dir {
            TestKind::Integration
        } else {
            TestKind::Unit
        };
        test_files.push(TestFile {
            command: language.command(&env, &file.path, kind, &tests),
            path: file.path,
            kind,
            tests,
        });
    }
    test_files.sort_by(|a, b| a.path.cmp(&b.path));
    sources.sort();
    let mut dependents = dependents_by_file(index)?;
    add_go_package_dependents(&files, &mut dependents);
    add_c_pair_dependents(&files, &mut dependents);
    Ok(Facts {
        test_files,
        sources,
        dependents,
    })
}

/// Breadth-first distances from `start` along `next`, `start` itself at 0.
fn distances<'a>(
    start: &'a str,
    next: &'a HashMap<String, BTreeSet<String>>,
) -> BTreeMap<&'a str, u32> {
    let mut seen: BTreeMap<&str, u32> = BTreeMap::from([(start, 0)]);
    let mut queue: VecDeque<&str> = VecDeque::from([start]);
    while let Some(current) = queue.pop_front() {
        let hop = seen[current] + 1;
        for node in next.get(current).into_iter().flatten() {
            if !seen.contains_key(node.as_str()) {
                seen.insert(node, hop);
                queue.push_back(node);
            }
        }
    }
    seen
}

/// Tests per file and the files nothing tests.
pub fn test_map(root: &Path, index: &Index) -> Result<TestMap> {
    let facts = gather(root, index)?;
    let own: HashMap<&str, usize> = facts
        .test_files
        .iter()
        .map(|t| (t.path.as_str(), t.tests.len()))
        .collect();
    let files: Vec<FileTests> = facts
        .sources
        .iter()
        .map(|path| {
            let mut direct = Vec::new();
            let mut indirect = Vec::new();
            for (file, hops) in distances(path, &facts.dependents) {
                if hops == 0 || !own.contains_key(file) {
                    continue;
                }
                if hops == 1 {
                    direct.push(file.to_string());
                } else {
                    indirect.push(file.to_string());
                }
            }
            FileTests {
                unit_tests: own.get(path.as_str()).copied().unwrap_or(0),
                path: path.clone(),
                direct,
                indirect,
            }
        })
        .collect();
    // A file of declarations and re-exports has nothing to test.
    let with_code: BTreeSet<String> = index
        .symbols()?
        .into_iter()
        .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
        .map(|s| s.file)
        .collect();
    let untested = files
        .iter()
        .filter(|f| f.unit_tests == 0 && f.direct.is_empty() && f.indirect.is_empty())
        .filter(|f| with_code.contains(&f.path))
        .map(|f| f.path.clone())
        .collect();
    Ok(TestMap {
        tests_total: facts.test_files.iter().map(|t| t.tests.len()).sum(),
        test_files: facts.test_files,
        files,
        untested,
    })
}

/// A test file worth running after a change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpactedTestFile {
    pub path: String,
    pub kind: TestKind,
    pub tests: Vec<String>,
    /// Dependency hops from the nearest changed file; 0 when the test file
    /// itself changed.
    pub distance: u32,
    /// Changed files that reach this test file.
    pub because: Vec<String>,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Impact {
    pub changed: Vec<String>,
    /// Nearest first.
    pub test_files: Vec<ImpactedTestFile>,
    pub tests_total: usize,
    /// Distinct commands that run every impacted test, nearest first.
    pub commands: Vec<String>,
    /// Changed source files that no test reaches.
    pub untested_changes: Vec<String>,
}

/// Tests to run when `changed` files change: tests in the changed files, and
/// tests in every file that depends on them, however many hops away.
pub fn impacted_tests(root: &Path, index: &Index, changed: &[String]) -> Result<Impact> {
    let facts = gather(root, index)?;
    let by_path: HashMap<&str, &TestFile> = facts
        .test_files
        .iter()
        .map(|t| (t.path.as_str(), t))
        .collect();
    let mut changed: Vec<String> = changed.iter().map(|c| c.replace('\\', "/")).collect();
    changed.sort();
    changed.dedup();

    let mut hit: BTreeMap<&str, (u32, Vec<String>)> = BTreeMap::new();
    let mut untested_changes = Vec::new();
    for file in &changed {
        let mut reached = false;
        for (path, hops) in distances(file, &facts.dependents) {
            if by_path.contains_key(path) {
                reached = true;
                let entry = hit.entry(path).or_insert((hops, Vec::new()));
                entry.0 = entry.0.min(hops);
                entry.1.push(file.clone());
            }
        }
        if !reached && facts.sources.binary_search(file).is_ok() {
            untested_changes.push(file.clone());
        }
    }
    let mut test_files: Vec<ImpactedTestFile> = hit
        .into_iter()
        .map(|(path, (distance, because))| {
            let file = by_path[path];
            ImpactedTestFile {
                path: path.to_string(),
                kind: file.kind,
                tests: file.tests.iter().map(|t| t.name.clone()).collect(),
                distance,
                because,
                command: file.command.clone(),
            }
        })
        .collect();
    test_files.sort_by(|a, b| a.distance.cmp(&b.distance).then(a.path.cmp(&b.path)));
    let mut commands: Vec<String> = Vec::new();
    for language in LANGUAGES {
        let own: Vec<&ImpactedTestFile> = test_files
            .iter()
            .filter(|f| language.covers(Language::from_path(&f.path)))
            .collect();
        commands.extend(language.commands(&own));
    }
    Ok(Impact {
        tests_total: test_files.iter().map(|t| t.tests.len()).sum(),
        changed,
        test_files,
        commands,
        untested_changes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn krate(package: &str, dir: &str, has_lib: bool) -> CrateInfo {
        CrateInfo {
            name: package.replace('-', "_"),
            package: package.into(),
            dir: dir.into(),
            description: String::new(),
            deps: Vec::new(),
            code_deps: Vec::new(),
            has_lib,
            has_bin: !has_lib,
        }
    }

    #[test]
    fn derives_the_command_that_runs_a_test_file() {
        let crates = [
            krate("cw-wiki", "crates/cw-wiki", true),
            krate("tool", "", false),
        ];
        let run = |path: &str, kind| command(&crates, path, kind);
        assert_eq!(
            run("crates/cw-wiki/tests/health.rs", TestKind::Integration),
            "cargo test -p cw-wiki --test health"
        );
        // Helper modules under `tests/` are not test targets.
        assert_eq!(
            run("crates/cw-wiki/tests/common/mod.rs", TestKind::Integration),
            ""
        );
        assert_eq!(
            run("crates/cw-wiki/src/health.rs", TestKind::Unit),
            "cargo test -p cw-wiki --lib health::"
        );
        assert_eq!(
            run("crates/cw-wiki/src/lib.rs", TestKind::Unit),
            "cargo test -p cw-wiki --lib"
        );
        assert_eq!(
            run("src/main.rs", TestKind::Unit),
            "cargo test -p tool --bins"
        );
        assert_eq!(
            run("src/parse.rs", TestKind::Unit),
            "cargo test -p tool parse::"
        );
    }

    #[test]
    fn distances_count_hops_and_survive_cycles() {
        let mut next: HashMap<String, BTreeSet<String>> = HashMap::new();
        for (from, to) in [("a", "b"), ("b", "c"), ("c", "a"), ("b", "d")] {
            next.entry(from.into()).or_default().insert(to.into());
        }
        let seen = distances("a", &next);
        assert_eq!(
            seen,
            BTreeMap::from([("a", 0), ("b", 1), ("c", 2), ("d", 2)])
        );
        assert_eq!(distances("zzz", &next), BTreeMap::from([("zzz", 0)]));
    }
}
