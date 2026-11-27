//! TypeScript and JavaScript tests: vitest, jest, mocha, Playwright, ava,
//! `node --test` and `bun test`.
//!
//! A test file is `*.test.*` or `*.spec.*` (any JavaScript or TypeScript
//! extension), `*.cy.*`, or any source file under `__tests__`, `test`,
//! `tests`, `e2e` or `cypress`. A test is a `test(...)`, `it(...)` or
//! `describe(...)` call with a string title, also as `.only`, `.skip` and
//! `.todo`; the unit of running is the file.
//!
//! End-to-end specs (Playwright, Cypress) are test files like the others,
//! but they reach the code through the running app, not through imports. The
//! import graph sees only what such a spec imports (a page object, a helper),
//! so a change to the app's own source impacts them only through that: no
//! guess is made about which pages a change shows.
//!
//! The runner is what the file imports (`vitest`, `@playwright/test`,
//! `node:test`, `bun:test`, `@jest/globals`), else what the package's `test`
//! script runs, else its dependencies; the package is the nearest
//! `package.json`, and the workspace root's when that names no runner. The
//! command goes through the package's manager (`pm.rs`): `pnpm exec vitest
//! run a.test.ts`, `npx jest a.test.ts`, `yarn mocha a.test.ts`, `bunx ...`;
//! a workspace package is named with the manager's filter, any other package
//! below the root is entered with `cd`. Paths are those of the package's
//! folder, except for `bun test`, which takes `./<path>` from the repository
//! root. A runner that is not recognised runs the `test` script whole.
//! Bun has no filter that runs a program in a workspace package, so there
//! the test script runs whole too.

use cw_core::Language;
use cw_parse::Manager;
use serde_json::Value;

use super::python::words;
use super::{Env, ImpactedTestFile, TestCase, TestKind, TestLanguage, parent_dir};

/// More files than this in one run and the run is the package's whole test
/// run.
const MAX_FILES: usize = 8;

const SOURCE_EXTENSIONS: [&str; 8] = ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Runner {
    Vitest,
    Jest,
    Mocha,
    Playwright,
    Ava,
    NodeTest,
    BunTest,
    /// A runner the map does not know: the `test` script runs whole.
    Unknown,
}

impl Runner {
    /// The program and the words before the files.
    fn program(self) -> Option<&'static str> {
        Some(match self {
            Self::Vitest => "vitest run",
            Self::Jest => "jest",
            Self::Mocha => "mocha",
            Self::Playwright => "playwright test",
            Self::Ava => "ava",
            Self::NodeTest => "node --test",
            Self::BunTest => "bun test",
            Self::Unknown => return None,
        })
    }
}

/// The runner a piece of text (a script, a file) names.
fn runner_in(text: &str) -> Option<Runner> {
    // The order matters where one name holds another.
    const NAMES: [(&str, Runner); 10] = [
        ("@playwright/test", Runner::Playwright),
        ("playwright test", Runner::Playwright),
        ("vitest", Runner::Vitest),
        ("@jest/globals", Runner::Jest),
        ("jest", Runner::Jest),
        ("mocha", Runner::Mocha),
        ("node:test", Runner::NodeTest),
        ("node --test", Runner::NodeTest),
        ("bun:test", Runner::BunTest),
        ("bun test", Runner::BunTest),
    ];
    NAMES
        .iter()
        .find(|(name, _)| text.contains(name))
        .map(|(_, runner)| *runner)
        .or_else(|| {
            // `ava` is a short word: only as a whole word.
            let found = text.match_indices("ava").any(|(at, _)| {
                !text[..at]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '/'))
                    && !text[at + 3..]
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
            });
            found.then_some(Runner::Ava)
        })
}

/// True for a file of JavaScript or TypeScript test code.
pub(super) fn is_test_path(path: &str) -> bool {
    let mut parts: Vec<&str> = path.split('/').collect();
    let file = parts.pop().unwrap_or("");
    let ext = file.rsplit_once('.').map_or("", |(_, e)| e);
    if !SOURCE_EXTENSIONS.contains(&ext) {
        return false;
    }
    let stem = file.rsplit_once('.').map_or(file, |(s, _)| s);
    // `a.test.ts`, `a.spec.js`, `a.cy.ts`; a `.d.ts` is no test.
    let marked = stem
        .rsplit_once('.')
        .is_some_and(|(_, tag)| matches!(tag, "test" | "spec" | "cy"));
    marked
        || parts
            .iter()
            .any(|d| matches!(*d, "__tests__" | "test" | "tests" | "e2e" | "cypress"))
}

/// The tests a file's text writes: `test('title'`, `it("title"`,
/// `describe(\`title\``, with `.only`, `.skip` and `.todo` after the name.
fn find_tests(text: &str) -> Vec<TestCase> {
    let mut out = Vec::new();
    let mut in_block = false;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if in_block {
            if let Some(end) = trimmed.find("*/") {
                in_block = false;
                scan_line(&trimmed[end + 2..], index, &mut out);
            }
            continue;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("/*") {
            if let Some(end) = rest.find("*/") {
                scan_line(&rest[end + 2..], index, &mut out);
            } else {
                in_block = true;
            }
            continue;
        }
        scan_line(line, index, &mut out);
    }
    out
}

fn scan_line(line: &str, index: usize, out: &mut Vec<TestCase>) {
    let bytes = line.as_bytes();
    let is_name = |b: u8| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'$' | b'.');
    for name in ["test", "it", "describe"] {
        let mut from = 0;
        while let Some(found) = line[from..].find(name) {
            let at = from + found;
            from = at + name.len();
            if at > 0 && is_name(bytes[at - 1]) {
                continue;
            }
            let mut rest = &line[at + name.len()..];
            // `.only`, `.skip`, `.todo` between the name and the call.
            for modifier in [".only", ".skip", ".todo"] {
                if let Some(after) = rest.strip_prefix(modifier) {
                    rest = after;
                    break;
                }
            }
            let Some(rest) = rest.trim_start().strip_prefix('(') else {
                continue;
            };
            let rest = rest.trim_start();
            let Some(quote) = rest
                .chars()
                .next()
                .filter(|c| matches!(c, '\'' | '"' | '`'))
            else {
                continue;
            };
            let Some(end) = rest[1..].find(quote) else {
                continue;
            };
            let title = &rest[1..=end];
            if !title.is_empty() {
                out.push(TestCase {
                    name: title.to_string(),
                    line: index as u32 + 1,
                });
            }
        }
    }
    // Calls of one line stand in source order.
    out.sort_by_key(|t| t.line);
}

pub(super) struct JsTests;

impl JsTests {
    /// The runner of the package in `dir`, from its `package.json` and, when
    /// that names none, the workspace root's.
    fn package_runner(env: &Env, dir: &str, root: &str) -> Runner {
        let of = |d: &str| -> Option<Runner> {
            let package = env.js.package_at(d)?;
            let json = package.json();
            if let Some(script) = json
                .get("scripts")
                .and_then(|s| s.get("test"))
                .and_then(Value::as_str)
                && let Some(runner) = runner_in(script)
            {
                return Some(runner);
            }
            for table in ["devDependencies", "dependencies"] {
                let names: Vec<&str> = json
                    .get(table)
                    .and_then(Value::as_object)
                    .map(|m| m.keys().map(String::as_str).collect())
                    .unwrap_or_default();
                for (name, runner) in [
                    ("vitest", Runner::Vitest),
                    ("jest", Runner::Jest),
                    ("mocha", Runner::Mocha),
                    ("@playwright/test", Runner::Playwright),
                    ("ava", Runner::Ava),
                ] {
                    if names.contains(&name) {
                        return Some(runner);
                    }
                }
            }
            None
        };
        of(dir).or_else(|| of(root)).unwrap_or(Runner::Unknown)
    }
}

impl TestLanguage for JsTests {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn covers(&self, language: Language) -> bool {
        matches!(language, Language::TypeScript | Language::JavaScript)
    }

    fn is_test_file(&self, _: &Env, path: &str) -> bool {
        is_test_path(path)
    }

    fn tests(&self, _: &Env, path: &str, text: &str) -> Vec<TestCase> {
        if is_test_path(path) {
            find_tests(text)
        } else {
            // Only a file of test code is a test file; a source file that
            // happens to call `test(` is not one.
            Vec::new()
        }
    }

    fn command(&self, env: &Env, path: &str, _: TestKind, _: &[TestCase]) -> String {
        let mut dir = parent_dir(path);
        while !dir.is_empty() && env.js.package_at(dir).is_none() {
            dir = parent_dir(dir);
        }
        let pm = env.js.manager_of(dir);
        let text = env.read(path).unwrap_or_default();
        let runner =
            runner_in_imports(&text).unwrap_or_else(|| Self::package_runner(env, dir, &pm.root));
        let rel = path
            .strip_prefix(dir)
            .map_or(path, |r| r.trim_start_matches('/'));
        let file = |p: &str| {
            if p.contains(' ') {
                format!("\"{p}\"")
            } else {
                p.to_string()
            }
        };
        let whole = || crate::pm::run_script(&pm, dir, "test");
        match runner {
            Runner::Unknown => {
                if env.js.package_at(dir).is_some_and(|p| {
                    p.json()
                        .get("scripts")
                        .and_then(|s| s.get("test"))
                        .is_some()
                }) {
                    whole()
                } else {
                    String::new()
                }
            }
            // `bun test` takes paths from where it runs, with `./` to tell a
            // path from a filter.
            Runner::BunTest => format!("bun test {}", file(&format!("./{path}"))),
            Runner::NodeTest => {
                let command = format!("node --test {}", file(rel));
                if dir.is_empty() {
                    command
                } else {
                    format!("cd {} && {command}", crate::pm::arg(dir))
                }
            }
            runner => {
                let program = format!("{} {}", runner.program().unwrap_or(""), file(rel));
                match crate::pm::exec(&pm, dir, &program) {
                    Some(command) => command,
                    None if pm.manager == Manager::Bun => whole(),
                    None => String::new(),
                }
            }
        }
    }

    /// One run for each package and runner over the files of every impacted
    /// test in it; past [`MAX_FILES`] files the package's runner runs whole.
    fn commands(&self, files: &[&ImpactedTestFile]) -> Vec<String> {
        let mut runs: Vec<(String, Vec<String>)> = Vec::new();
        for file in files.iter().filter(|f| !f.command.is_empty()) {
            let (head, args) = split_files(&file.command);
            let at = match runs.iter().position(|(h, _)| *h == head) {
                Some(at) => at,
                None => {
                    runs.push((head, Vec::new()));
                    runs.len() - 1
                }
            };
            for arg in args {
                if !runs[at].1.contains(&arg) {
                    runs[at].1.push(arg);
                }
            }
        }
        runs.into_iter()
            .map(|(head, args)| {
                if args.is_empty() || args.len() > MAX_FILES {
                    head
                } else {
                    format!("{head} {}", args.join(" "))
                }
            })
            .collect()
    }
}

/// The runner a test file's own imports name, which beats the package's.
fn runner_in_imports(text: &str) -> Option<Runner> {
    let head: String = text.lines().take(60).collect::<Vec<_>>().join("\n");
    [
        ("'@playwright/test'", Runner::Playwright),
        ("\"@playwright/test\"", Runner::Playwright),
        ("'vitest'", Runner::Vitest),
        ("\"vitest\"", Runner::Vitest),
        ("'@jest/globals'", Runner::Jest),
        ("\"@jest/globals\"", Runner::Jest),
        ("'node:test'", Runner::NodeTest),
        ("\"node:test\"", Runner::NodeTest),
        ("'bun:test'", Runner::BunTest),
        ("\"bun:test\"", Runner::BunTest),
    ]
    .iter()
    .find(|(name, _)| head.contains(name))
    .map(|(_, runner)| *runner)
}

/// A command as its head (the program) and the test files at its end. A
/// command with no file (a script run whole) is all head.
fn split_files(command: &str) -> (String, Vec<String>) {
    let words = words(command);
    let is_file = |w: &String| {
        let bare = w.trim_matches('"');
        bare.rsplit_once('.')
            .is_some_and(|(_, ext)| SOURCE_EXTENSIONS.contains(&ext))
    };
    match words.iter().position(is_file) {
        Some(at) => (words[..at].join(" "), words[at..].to_vec()),
        None => (words.join(" "), Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_test_files() {
        for path in [
            "src/a.test.ts",
            "src/a.spec.jsx",
            "web/__tests__/a.ts",
            "e2e/login.spec.ts",
            "test/a.mjs",
            "src/a.cy.ts",
        ] {
            assert!(is_test_path(path), "{path}");
        }
        for path in [
            "src/a.ts",
            "src/contest.ts",
            "src/test.ts",
            "tests/readme.md",
        ] {
            assert!(!is_test_path(path), "{path}");
        }
    }

    #[test]
    fn finds_titled_calls_outside_comments() {
        let text = "import { test, describe } from 'vitest';\n// test('commented', () => {})\ndescribe('cart', () => {\n  it('adds', () => {});\n  it.skip(\"removes\", () => {});\n  test(`has ${x}`, () => {});\n  /* it('block', () => {}) */\n  const latest = () => {};\n  expect(test_helper('x')).toBe(1);\n});\n";
        let names: Vec<String> = find_tests(text).into_iter().map(|t| t.name).collect();
        assert_eq!(names, ["cart", "adds", "removes", "has ${x}"]);
    }

    #[test]
    fn names_the_runner_from_scripts_and_imports() {
        assert_eq!(runner_in("vitest run --coverage"), Some(Runner::Vitest));
        assert_eq!(runner_in("jest --ci"), Some(Runner::Jest));
        assert_eq!(runner_in("node --test test/"), Some(Runner::NodeTest));
        assert_eq!(runner_in("ava"), Some(Runner::Ava));
        assert_eq!(runner_in("tsc && lava"), None);
        assert_eq!(runner_in("echo none"), None);
        assert_eq!(
            runner_in_imports("import { test } from '@playwright/test';\n"),
            Some(Runner::Playwright)
        );
    }

    #[test]
    fn folds_files_into_the_head() {
        assert_eq!(
            split_files("pnpm exec vitest run src/a.test.ts \"x y/b.test.ts\""),
            (
                "pnpm exec vitest run".to_string(),
                vec!["src/a.test.ts".to_string(), "\"x y/b.test.ts\"".to_string()]
            )
        );
        assert_eq!(
            split_files("pnpm run test"),
            ("pnpm run test".to_string(), vec![])
        );
    }
}
