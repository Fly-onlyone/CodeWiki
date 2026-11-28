---
tags: [core, cw-wiki, module]
---

# Js Module

> TypeScript and JavaScript tests: vitest, jest, mocha, Playwright, ava, `node --test` and `bun test`.

## What it does

Teaches the test map about JavaScript and TypeScript tests: vitest, jest, mocha, Playwright, ava, `node --test` and `bun test`. It says which files are tests, which tests they hold, and which command runs a file.

A test file is `*.test.*`, `*.spec.*` or `*.cy.*` with any JS or TS extension, or any source file under `__tests__`, `test`, `tests`, `e2e` or `cypress`. A test is a `test`, `it` or `describe` call with a string title, also with `.only`, `.skip` or `.todo`. The unit of running is the file.

End-to-end specs are test files too, but they reach the app through a browser, not through imports. So the import graph links them only to what they import, such as a page object. No guess is made about which pages a change shows.

## Source

- `crates/cw-wiki/src/testmap/js.rs` — module `cw_wiki::testmap::js`

## How it works

`is_test_path` checks the file name and folders. `find_tests` scans the text line by line, skips comments, and `scan_line` looks for the three names followed by an opening bracket and a quoted title.

`JsTests::command` walks up to the nearest folder with a `package.json`, then asks [[JsRepo]] for the package manager of that folder. It picks the runner in this order: what the file imports (`runner_in_imports`), else what the package's `test` script runs, else its dependencies (`package_runner`, falling back to the workspace root's package). It then writes the command through [[Pm Module]], for example `pnpm exec vitest run a.test.ts`.

`JsTests::commands` folds the files of many impacted tests into one run per package and runner. It uses `split_files` to separate the program from the files at the end.

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

## Gotchas

- An unknown runner runs the package's `test` script whole, and gives an empty command when there is no such script.
- Bun has no filter that runs a program in a workspace package, so the test script runs whole there.
- `bun test` takes a path from the repository root, written `./<path>`, while other runners take paths as the package folder sees them.
- More than 8 files in one run turns into the package's whole test run.
- A source file that merely calls `test(` is not a test file, so `tests` gives nothing for it.
- Names are matched in order, because one name can hold another, such as `@playwright/test` and `playwright test`. `ava` counts only as a whole word.

## See also

- [[_index]]
- [[cw-wiki]]
