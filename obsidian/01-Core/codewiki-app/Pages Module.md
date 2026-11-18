---
tags: [core, codewiki-app, module]
---

# Pages Module

> The report pages: decisions, dead code, health, drift and the rest.

## Source

- `crates/codewiki-app/src/pages.rs` — module `codewiki_app::pages`

## How it works

Each turns what the backend sent into a `Doc`. The pages of the settings are in `settings_pages.rs`.

Main items:

- `contracts_page` — HTTP endpoints across the workspace: who provides what each repository calls, and what nothing provides or calls.
- `coupling_page` — files that tend to change in the same commit.
- `coverage_page` — imported line coverage, least covered first.
- `dead_code_page` — private symbols that nothing refers to.
- `decisions_page` — the decision records with their status and staleness.
- `drift_page` — documentation drift and coverage.
- `health_page` — file health scores, worst first.
- `ownership_page` — who knows the code: authors by the files they lead, and files only one person has worked on.

## Depends on

- [[Doc Module]] — uses `Line`, `chip`, `code`, `file`
- [[Doc]] — uses `Doc`, `heading`, `lead`, `new`
- [[Block]] — uses `Block`

## Used by

- [[codewiki-app]] — from `run_command`

## See also

- [[_index]]
- [[codewiki-app]]
