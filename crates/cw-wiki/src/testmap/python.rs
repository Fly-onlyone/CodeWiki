//! Python tests: pytest, and unittest where a project names no pytest.
//!
//! A test file is `test_*.py`, `*_test.py`, or any file under a `tests` or
//! `test` folder; `conftest.py` is support and holds no test. A test is a
//! function `test_*`, a method `test_*` of a class `Test*` (pytest) and a
//! method `test_*` of a class that derives from a `unittest.TestCase` (any
//! base whose name ends in `TestCase`, also through a base class of the same
//! file).
//!
//! The runner is pytest when the project names it: `[tool.pytest.ini_options]`
//! or any `pytest` in its build files, `pytest.ini`, a `[pytest]` section in
//! `tox.ini`, `[tool:pytest]` in `setup.cfg`, a `conftest.py`, or the file
//! importing `pytest`; else `python -m unittest`. The project's own tool runs
//! it when its files say which: `uv run` (`uv.lock`), `poetry run`
//! (`poetry.lock` or `[tool.poetry]`), `pdm run` (`pdm.lock`), `hatch run`
//! (`[tool.hatch.envs`). Commands run from the repository root, or, for a
//! project below it, after `cd <project folder> &&`, with the paths as that
//! folder sees them. A `python -m unittest` name is the module's dotted name
//! as the scan gave it, so the package must be importable from that folder.

use std::collections::HashMap;

use cw_core::{Language, Symbol, SymbolKind};
use cw_parse::jsbuild::ancestors;

use super::{Env, ImpactedTestFile, TestCase, TestKind, TestLanguage};

/// More tests than this in one run and the run names the files instead; more
/// files than [`MAX_FILES`] and it runs the whole project.
const MAX_NAMES: usize = 8;
const MAX_FILES: usize = 8;

/// What a project says about how its tests run.
#[derive(Debug, Clone, Default)]
pub(super) struct PyProject {
    pytest: bool,
    /// `uv run `, `poetry run `, `pdm run ` or `hatch run `; empty for none.
    prefix: &'static str,
}

/// True for a file of Python test code: `test_*.py`, `*_test.py`, anything
/// under a folder named `tests` or `test`, and `conftest.py` (support).
pub(super) fn is_test_path(path: &str) -> bool {
    let mut parts: Vec<&str> = path.split('/').collect();
    let file = parts.pop().unwrap_or("");
    parts.iter().any(|d| matches!(*d, "tests" | "test"))
        || file == "conftest.py"
        || (file.ends_with(".py")
            && (file.starts_with("test_") || file.trim_end_matches(".py").ends_with("_test")))
}

fn word(text: &str, needle: &str) -> bool {
    let is_part = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-');
    text.match_indices(needle).any(|(at, _)| {
        !text[..at].chars().next_back().is_some_and(is_part)
            && !text[at + needle.len()..]
                .chars()
                .next()
                .is_some_and(is_part)
    })
}

/// What the files at and above `dir` say about the project's tests.
fn read_project(env: &Env, dir: &str) -> PyProject {
    let mut project = PyProject::default();
    for d in ancestors(dir) {
        let at = |name: &str| super::join_path(d, name);
        let text = |name: &str| env.read(&at(name)).unwrap_or_default();
        let pyproject = text("pyproject.toml");
        if env.files.contains(&at("pytest.ini"))
            || env.files.contains(&at(".pytest.ini"))
            || env.files.contains(&at("conftest.py"))
            || text("tox.ini").contains("[pytest]")
            || text("setup.cfg").contains("[tool:pytest]")
            || pyproject.contains("[tool.pytest")
            || word(&pyproject, "pytest")
            || word(&text("setup.cfg"), "pytest")
            || word(&text("setup.py"), "pytest")
            || word(&text("Pipfile"), "pytest")
            || [
                "requirements.txt",
                "requirements-dev.txt",
                "requirements-test.txt",
            ]
            .iter()
            .any(|name| word(&text(name), "pytest"))
        {
            project.pytest = true;
        }
        if project.prefix.is_empty() {
            project.prefix = if env.files.contains(&at("uv.lock")) {
                "uv run "
            } else if env.files.contains(&at("poetry.lock")) || pyproject.contains("[tool.poetry]")
            {
                "poetry run "
            } else if env.files.contains(&at("pdm.lock")) {
                "pdm run "
            } else if pyproject.contains("[tool.hatch.envs") {
                "hatch run "
            } else {
                ""
            };
        }
    }
    project
}

/// A path as one shell argument.
fn quote(path: &str) -> String {
    if path.contains(' ') {
        format!("\"{path}\"")
    } else {
        path.to_string()
    }
}

/// True when the class `name` of the file's symbols derives from a
/// `unittest.TestCase`, directly or through a class of the same file.
fn is_unittest_class(symbols: &[Symbol], class: &Symbol, depth: usize) -> bool {
    let bases = class
        .signature
        .split_once('(')
        .and_then(|(_, rest)| rest.rsplit_once(')'))
        .map_or("", |(bases, _)| bases);
    bases.split(',').map(str::trim).any(|base| {
        let last = base.rsplit('.').next().unwrap_or(base);
        last.ends_with("TestCase")
            || (depth < 4
                && symbols.iter().any(|s| {
                    s.kind == SymbolKind::Class
                        && s.name == last
                        && is_unittest_class(symbols, s, depth + 1)
                }))
    })
}

pub(super) struct PythonTests;

impl PythonTests {
    /// The folder of the project that owns `path`; empty for the repository.
    fn project_dir<'e>(env: &'e Env, path: &str) -> &'e str {
        env.py_roots.project_of(path).unwrap_or("")
    }

    fn project(env: &Env, dir: &str) -> PyProject {
        env.py_projects
            .borrow_mut()
            .entry(dir.to_string())
            .or_insert_with(|| read_project(env, dir))
            .clone()
    }

    /// `Class::method` or `function`, the name of a test as pytest gives it.
    fn name_of(symbols: &[Symbol], symbol: &Symbol) -> Option<String> {
        if !symbol.name.starts_with("test_") {
            return None;
        }
        match symbol.kind {
            SymbolKind::Function => Some(symbol.name.clone()),
            SymbolKind::Method => {
                let class = symbols
                    .iter()
                    .find(|s| Some(s.qualified.as_str()) == symbol.parent.as_deref())
                    .filter(|s| s.kind == SymbolKind::Class)?;
                (class.name.starts_with("Test") || is_unittest_class(symbols, class, 0))
                    .then(|| format!("{}::{}", class.name, symbol.name))
            }
            _ => None,
        }
    }
}

impl TestLanguage for PythonTests {
    fn language(&self) -> Language {
        Language::Python
    }

    fn is_test_file(&self, _: &Env, path: &str) -> bool {
        is_test_path(path)
    }

    fn tests(&self, env: &Env, path: &str, _: &str) -> Vec<TestCase> {
        if path.ends_with("conftest.py") {
            return Vec::new();
        }
        let symbols = env.symbols_of(path);
        symbols
            .iter()
            .filter_map(|s| {
                Some(TestCase {
                    name: Self::name_of(symbols, s)?,
                    line: s.start_line,
                })
            })
            .collect()
    }

    /// `pytest path/test_x.py::test_a path/test_x.py::TestC::test_m`, or
    /// `python -m unittest pkg.test_x.TestC.test_m`.
    fn command(&self, env: &Env, path: &str, _: TestKind, tests: &[TestCase]) -> String {
        let dir = Self::project_dir(env, path);
        let project = Self::project(env, dir);
        let imports_pytest = env
            .read(path)
            .is_some_and(|text| text.contains("import pytest") || text.contains("from pytest"));
        let rel = path
            .strip_prefix(dir)
            .map_or(path, |r| r.trim_start_matches('/'));
        let from = if dir.is_empty() {
            String::new()
        } else {
            format!("cd {} && ", quote(dir))
        };
        if project.pytest || imports_pytest {
            let ids: Vec<String> = tests
                .iter()
                .map(|t| quote(&format!("{rel}::{}", t.name)))
                .collect();
            let args = if ids.is_empty() || ids.len() > MAX_NAMES {
                quote(rel)
            } else {
                ids.join(" ")
            };
            return format!("{from}{}pytest {args}", project.prefix);
        }
        let Some(module) = env
            .symbols_of(path)
            .iter()
            .find(|s| s.kind == SymbolKind::Module)
            .map(|s| s.qualified.replace("::", "."))
        else {
            return String::new();
        };
        // unittest runs methods of TestCase classes, nothing else.
        let ids: Vec<String> = tests
            .iter()
            .filter_map(|t| t.name.split_once("::"))
            .map(|(class, method)| format!("{module}.{class}.{method}"))
            .collect();
        let args = if ids.is_empty() || ids.len() > MAX_NAMES {
            module
        } else {
            ids.join(" ")
        };
        format!("{from}{}python -m unittest {args}", project.prefix)
    }

    /// One run for each project and runner, over the tests of every impacted
    /// file in it: past [`MAX_NAMES`] names the files, past [`MAX_FILES`]
    /// files the whole project.
    fn commands(&self, files: &[&ImpactedTestFile]) -> Vec<String> {
        // Head of the command (up to the runner) -> its arguments, in order.
        let mut runs: Vec<(String, bool, Vec<String>)> = Vec::new();
        for file in files.iter().filter(|f| !f.command.is_empty()) {
            let Some((head, unittest, args)) = split_command(&file.command) else {
                continue;
            };
            let at = match runs.iter().position(|(h, _, _)| *h == head) {
                Some(at) => at,
                None => {
                    runs.push((head, unittest, Vec::new()));
                    runs.len() - 1
                }
            };
            for arg in args {
                if !runs[at].2.contains(&arg) {
                    runs[at].2.push(arg);
                }
            }
        }
        runs.into_iter()
            .map(|(head, unittest, mut args)| {
                if unittest {
                    if args.len() > MAX_NAMES {
                        args = unique(args.iter().map(|a| unittest_module(a)).collect());
                    }
                } else {
                    // A file named whole covers the ids inside it.
                    let whole: Vec<String> =
                        args.iter().filter(|a| !a.contains("::")).cloned().collect();
                    args.retain(|a| !a.contains("::") || !whole.contains(&file_of(a)));
                    if args.len() > MAX_NAMES {
                        args = unique(args.iter().map(|a| file_of(a)).collect());
                    }
                }
                if args.len() > MAX_FILES {
                    args.clear();
                }
                if args.is_empty() {
                    head
                } else {
                    format!("{head} {}", args.join(" "))
                }
            })
            .collect()
    }
}

fn unique(items: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in items {
        if !out.contains(&item) {
            out.push(item);
        }
    }
    out
}

/// The file of a pytest id, as written (quotes kept).
fn file_of(arg: &str) -> String {
    let unquoted = arg.trim_matches('"');
    let file = unquoted.split("::").next().unwrap_or(unquoted);
    if arg.starts_with('"') && file.contains(' ') {
        format!("\"{file}\"")
    } else {
        file.to_string()
    }
}

/// The module of a unittest id `pkg.mod.Class.method`.
fn unittest_module(arg: &str) -> String {
    let mut parts: Vec<&str> = arg.split('.').collect();
    if parts.len() > 2 {
        parts.truncate(parts.len() - 2);
    }
    parts.join(".")
}

/// The words of a command, a double-quoted word kept whole with its quotes.
pub(super) fn words(command: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for c in command.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                current.push(c);
            }
            ' ' if !quoted => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// A command written by [`PythonTests::command`] as its head (everything up
/// to and including the runner), whether the runner is unittest, and its
/// arguments.
fn split_command(command: &str) -> Option<(String, bool, Vec<String>)> {
    let words = words(command);
    let at = words
        .iter()
        .position(|w| w == "pytest" || w == "unittest")?;
    let unittest = words[at] == "unittest";
    Some((words[..=at].join(" "), unittest, words[at + 1..].to_vec()))
}

/// What [`Env::py_projects`] keeps: a project folder and what it says.
pub(super) type Projects = HashMap<String, PyProject>;
