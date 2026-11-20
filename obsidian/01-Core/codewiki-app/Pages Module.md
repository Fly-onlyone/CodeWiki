---
tags: [core, codewiki-app, module]
---

# Pages Module

> The report pages: decisions, dead code, health, drift and the rest.

## What it does

Each turns what the backend sent into a `Doc`. The pages of the settings are in `settings_pages.rs`.

## Source

- `crates/codewiki-app/src/pages.rs` — module `codewiki_app::pages`

## How it works

What the functions of this module call:

1. `decisions_page` calls `Doc::new` in [[Doc]], then `code` in [[Doc Module]], then `string`, then `text` in [[Doc Module]], then `strings`, then `file` in [[Doc Module]], and 1 more.
2. `dead_code_page` calls `Doc::new` in [[Doc]], then `Table::new`, then `Column::new`, then `Column::numeric` (all in [[Doc Module]]), then `string`, then `Cell::chip` in [[Doc Module]], and 4 more.
3. `health_page` calls `Doc::new` in [[Doc]], then `Table::new`, then `Column::new` (both in [[Doc Module]]), then `list`, then `file` in [[Doc Module]], then `string`, and 1 more.
4. `drift_page` calls `Doc::new` in [[Doc]], then `number`, then `list`, then `Stat::new`, then `Table::new`, then `Column::new` (all in [[Doc Module]]), and 6 more.
5. `tests_page` calls `list`, then `strings`, then `Doc::new` in [[Doc]], then `number`, then `Stat::new`, then `Table::new` (both in [[Doc Module]]), and 8 more.

7 more entry points: `coverage_page`, `security_page`, `refactor_page`, `ownership_page`, `coupling_page`, `workspace_page`, and 1 more.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[codewiki-app]]
