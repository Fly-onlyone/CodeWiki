---
tags: [core, cw-wiki, module]
---

# Js Module

> TypeScript and JavaScript tests: vitest, jest, mocha, Playwright, ava, `node --test` and `bun test`.

## What it does

A test file is `*.test.*` or `*.spec.*` (any JavaScript or TypeScript extension), `*.cy.*`, or any source file under `__tests__`, `test`, `tests`, `e2e` or `cypress`. A test is a `test(...)`, `it(...)` or `describe(...)` call with a string title, also as `.only`, `.skip` and `.todo`; the unit of running is the file.

End-to-end specs (Playwright, Cypress) are test files like the others, but they reach the code through the running app, not through imports. The import graph sees only what such a spec imports (a page object, a helper), so a change to the app's own source impacts them only through that: no guess is made about which pages a change shows.

The runner is what the file imports (`vitest`, `@playwright/test`, `node:test`, `bun:test`, `@jest/globals`), else what the package's `test` script runs, else its dependencies; the package is the nearest `package.json`, and the workspace root's when that names no runner. The command goes through the package's manager (`pm.rs`): `pnpm exec vitest run a.test.ts`, `npx jest a.test.ts`, `yarn mocha a.test.ts`, `bunx ...`; a workspace package is named with the manager's filter, any other package below the root is entered with `cd`. Paths are those of the package's folder, except for `bun test`, which takes `./<path>` from the repository root. A runner that is not recognised runs the `test` script whole. Bun has no filter that runs a program in a workspace package, so there the test script runs whole too.

## Source

- `crates/cw-wiki/src/testmap/js.rs` — module `cw_wiki::testmap::js`

## How it works

What the functions of this module call:

1. `JsTests::is_test_file` calls `is_test_path`.
2. `JsTests::command` calls `parent_dir` in [[Testmap Module]], then `runner_in_imports`, then `JsTests::package_runner`, then `run_script`, then `arg`, then `exec` (all in [[Pm Module]]). `JsTests::package_runner` calls `runner_in`.
3. `JsTests::commands` calls `split_files`. `split_files` calls `words` in [[Python Module (testmap)]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const MAX_FILES: usize` — more files than this in one run and the run is the package's whole test run.
- `fn runner_in(text: &str) -> Option<Runner>` — the runner a piece of text (a script, a file) names.
- `pub(super) fn is_test_path(path: &str) -> bool` — true for a file of JavaScript or TypeScript test code.
- `fn find_tests(text: &str) -> Vec<TestCase>` — the tests a file's text writes: `test('title'`, `it("title"`, `describe(\`title\``, with `.only`, `.skip` and `.todo` after the name.
- `pub(super) struct JsTests` — internal struct; used by `LANGUAGES`.
- `fn runner_in_imports(text: &str) -> Option<Runner>` — the runner a test file's own imports name, which beats the package's.
- `fn split_files(command: &str) -> (String, Vec<String>)` — A command as its head (the program) and the test files at its end. A command with no file (a script run whole) is all head.

And 3 more private items with no comment.

## Depends on

- [[Testmap Module]] — uses `Env`, `ImpactedTestFile`, `TestCase`, `TestLanguage`
- [[Pm Module]] — uses `arg`, `exec`, `run_script`
- [[JsRepo]] — uses `manager_of`, `package_at`
- [[Python Module (testmap)]] — uses `words`
- [[Language]] — uses `Language`
- [[JsPackage]] — uses `json`
- [[TestKind]] — uses `TestKind`

## Used by

- [[Testmap Module]] — from `LANGUAGES`

## See also

- [[_index]]
- [[cw-wiki]]
