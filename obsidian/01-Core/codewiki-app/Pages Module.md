---
tags: [core, codewiki-app, module]
---

# Pages Module

> The report pages: decisions, dead code, health, drift, settings and the rest.

## Source

- `crates/codewiki-app/src/pages.rs` — module `codewiki_app::pages`

## How it works

Each turns what the backend sent into a `Doc`.

Main items:

- `settings_page` — the settings page: appearance, where data comes from, updates and the keyboard shortcuts.
- `contracts_page` — HTTP endpoints across the workspace: who provides what each repository calls, and what nothing provides or calls.
- `coupling_page` — files that tend to change in the same commit.
- `coverage_page` — imported line coverage, least covered first.
- `dead_code_page` — private symbols that nothing refers to.
- `decisions_page` — the decision records with their status and staleness.
- `drift_page` — documentation drift and coverage.
- `health_page` — file health scores, worst first.

## Depends on

- [[Doc Module]] — uses `Choice`, `Line`, `chip`, `code`
- [[Doc]] — uses `Doc`, `heading`, `lead`, `new`
- [[MotionPref]] — uses `ALL`, `MotionPref`
- [[Update Module]] — uses `State`
- [[Block]] — uses `Block`

## Used by

- [[codewiki-app]] — from `open_settings`, `run_command`

## See also

- [[_index]]
- [[codewiki-app]]
