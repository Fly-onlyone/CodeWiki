//! Test map: which tests exist, what they exercise, and which to run after a
//! change.
//!
//! Tests are found with a lexical scan ([`crate::rustscan`]) because the
//! structural parser leaves unit-test modules out of the index. What a test
//! exercises is read from the file dependency graph: a test file exercises the
//! files it depends on, directly or through further hops. No coverage data is
//! involved, so "exercises" means "can reach", not "executes".

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::path::Path;

use cw_core::{Language, Result, SymbolKind};
use cw_index::Index;
use cw_parse::{CrateInfo, module_path, owning_crate};
use serde::{Deserialize, Serialize};

use crate::gitintel::dependents_by_file;
use crate::rustscan::{Scanned, is_test_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestKind {
    /// Tests inside a source file, usually in `#[cfg(test)] mod tests`.
    Unit,
    /// Tests in a file under a `tests/` directory.
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
    /// Every Rust source file outside test directories, by path.
    pub files: Vec<FileTests>,
    /// Source files that define functions, have no unit tests, and are
    /// reached by no test file.
    pub untested: Vec<String>,
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

/// Everything the analyses below need, gathered once.
struct Facts {
    test_files: Vec<TestFile>,
    /// Rust files outside test directories.
    sources: Vec<String>,
    /// file -> files that depend on it.
    dependents: HashMap<String, BTreeSet<String>>,
}

fn gather(root: &Path, index: &Index) -> Result<Facts> {
    let crates = index.crates()?;
    let mut test_files = Vec::new();
    let mut sources = Vec::new();
    for file in index.files()? {
        if file.language != Language::Rust {
            continue;
        }
        let in_test_dir = is_test_path(&file.path);
        if !in_test_dir {
            sources.push(file.path.clone());
        }
        let Ok(text) = std::fs::read_to_string(root.join(&file.path)) else {
            continue;
        };
        let tests: Vec<TestCase> = Scanned::new(&text)
            .tests
            .into_iter()
            .map(|t| TestCase {
                name: t.name,
                line: t.line,
            })
            .collect();
        if tests.is_empty() {
            continue;
        }
        let kind = if in_test_dir {
            TestKind::Integration
        } else {
            TestKind::Unit
        };
        test_files.push(TestFile {
            command: command(&crates, &file.path, kind),
            path: file.path,
            kind,
            tests,
        });
    }
    test_files.sort_by(|a, b| a.path.cmp(&b.path));
    sources.sort();
    Ok(Facts {
        test_files,
        sources,
        dependents: dependents_by_file(index)?,
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
    /// Changed Rust source files that no test reaches.
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
    for file in &test_files {
        if !file.command.is_empty() && !commands.contains(&file.command) {
            commands.push(file.command.clone());
        }
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
