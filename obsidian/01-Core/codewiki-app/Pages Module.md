---
tags: [core, codewiki-app, module]
---

# Pages Module

> The report pages: decisions, dead code, health, drift and the rest.

## What it does

These are the report pages of the app: decisions, dead code, code health, documentation drift, tests, coverage, security, refactoring, ownership, change coupling, workspace and API contracts. The backend sends plain JSON. Each `*_page` function here turns that JSON into a [[Doc]], a small page model that the app draws. The pages for settings live in a different file, `settings_pages.rs`. No page does any I/O: JSON in, `Doc` out, which is why they are easy to test.

## Source

- `crates/codewiki-app/src/pages.rs` — module `codewiki_app::pages`

## How it works

Every page follows one shape. It reads the JSON with three small helpers (`list` for arrays, `strings` for string arrays, `string` for text), builds a `Doc` with a title and a lead sentence, then pushes blocks: stat tiles, tables, cards or lists. When there is nothing to show, it pushes an empty-state block with a hint and returns early.

Some pages sort first: `health_page` drops files with no findings and puts the lowest score first; `security_page` orders error, warning, then the rest; `coverage_page` shows the least covered files. Scores map to a tone: 0 to 4 is error, 5 to 7 is warning, the rest is success.

`ownership_page` and `coupling_page` both read the commit history through `history_files`. Ownership counts files led per author and lists files with a bus factor of 1, meaning only one person has worked on them. Coupling lists each pair of files that change together once, and ranks pairs by strength.

Numbers come out through `number`, which prints a dash for a missing value instead of failing.

## Items

- `fn strings<'a>(value: &'a Value, key: &str) -> Vec<&'a str>` — private function, returns `Vec<&'a str>`; called by `coverage_page`, `decisions_page`, `drift_page`, `refactor_page` and 2 more.
- `fn list<'a>(value: &'a Value, key: &str) -> Vec<&'a Value>` — private function, returns `Vec<&'a Value>`; called by `contracts_page`, `coupling_page`, `coverage_page`, `drift_page` and 3 more.
- `fn string<'a>(value: &'a Value, key: &str) -> &'a str` — private function, returns `&'a str`; called by `contracts_page`, `coupling_page`, `coverage_page`, `dead_code_page` and 8 more.
- `fn number(value: &Value) -> String` — A number as the backend sent it; a dash when it sent none.
- `fn file_list(paths: &[&str], limit: usize) -> Block` — A list of file links, one to a line.
- `pub fn decisions_page(rows: &Value) -> Doc` — the decision records with their status and staleness.
- `pub fn dead_code_page(rows: &Value) -> Doc` — private symbols that nothing refers to.
- `pub fn health_page(rows: &Value) -> Doc` — file health scores, worst first. Files without findings are left out.
- `pub fn drift_page(report: &Value) -> Doc` — documentation drift and coverage.
- `pub fn tests_page(map: &Value) -> Doc` — the test map: where the tests are and which files have none.
- `pub fn coverage_page(report: &Value) -> Doc` — imported line coverage, least covered first.
- `pub fn security_page(rows: &Value) -> Doc` — security findings, most severe first.
- `pub fn refactor_page(rows: &Value) -> Doc` — refactoring suggestions with their evidence and steps.
- `pub fn ownership_page(history: &Value) -> Doc` — who knows the code: authors by the files they lead, and files only one person has worked on.
- `pub fn coupling_page(history: &Value) -> Doc` — files that tend to change in the same commit.
- `pub fn workspace_page(workspace: &Value) -> Doc` — every registered repository side by side.
- `pub fn contracts_page(contracts: &Value) -> Doc` — HTTP endpoints across the workspace: who provides what each repository calls, and what nothing provides or calls.

And 1 more private item with no comment.

## Depends on

- [[Doc Module]] — uses `Line`, `chip`, `code`, `file`
- [[Doc]] — uses `Doc`, `heading`, `lead`, `new`
- [[Block]] — uses `Block`

## Used by

- [[codewiki-app]] — from `run_command`

## Gotchas

- A missing or mistyped key never fails. The helpers return an empty string or list, so a backend field rename shows up as a blank cell, not an error.
- Lists are capped: file lists at 60 or 80, solo files at 40, coupling pairs at 80, matched contract calls at 200. A long report is silently cut.
- Coupling pairs are stored on both files. The page keeps only the direction where the path sorts first, so each pair appears once.
- The dead code page does not hide anything: entries marked possible stay in the table with a neutral tone. Only likely gets a warning tone.
- Tests check pages through `words()`, `tables()` and `actions()` on the `Doc`, so a changed label breaks a test on purpose.

## See also

- [[_index]]
- [[codewiki-app]]
