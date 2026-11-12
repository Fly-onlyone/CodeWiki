---
tags: [core, codewiki-app, module]
---

# Pages Module

> Markdown for the report pages: decisions, dead code, health, drift, settings.

## Source

- `crates/codewiki-app/src/pages.rs` — module `codewiki_app::pages`

## How it works

Main items:

- `settings_page` — the settings page: where data comes from, and the keyboard shortcuts.
- `contracts_page` — render HTTP endpoints across the workspace: who provides what each repository calls, and what nothing provides or calls.
- `coupling_page` — render files that tend to change in the same commit.
- `coverage_page` — render imported line coverage, least covered first.
- `dead_code_page` — render private symbols that nothing refers to.
- `decisions_page` — render the decision records with their status and staleness.
- `drift_page` — render documentation drift and coverage.
- `health_page` — render file health scores, worst first.

## Depends on

- [[Markdown Module]] — uses `NOTE_SCHEME`, `THEME_SCHEME`, `encode`, `file_link`

## Used by

- [[codewiki-app]] — from `open_settings`, `run_command`

## See also

- [[_index]]
- [[codewiki-app]]
