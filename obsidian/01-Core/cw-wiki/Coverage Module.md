---
tags: [core, cw-wiki, module]
---

# Coverage Module

> Test coverage imported from LCOV (`lcov.info`) and Cobertura XML reports.

## Source

- `crates/cw-wiki/src/coverage.rs` — module `cw_wiki::coverage`

## How it works

Main items:

- `Format` — enum
- `FileCoverage` — struct
- `import` — read and import a report file.
- `import_text` — parse report `text` and match its files to the index.
- `CoverageData` — coverage as imported, with paths matched to the index.
- `report` — summarise imported coverage and rank the gaps.
- `META_KEY` — meta key under which imported coverage is cached in the index.
- `Gap` — A poorly covered file and why it matters.

## Depends on

- [[Index]] — uses `Index`, `files`, `meta`, `metrics`
- [[Error]] — uses `external`, `invalid`, `io`, `not_found`
- [[Gitintel Module]] — uses `hotspots`, `load`
- [[Error Module]] — uses `Result`
- [[Model Module]] — uses `FileRecord`
- [[Rustscan Module]] — uses `is_test_path`
- [[Language]] — uses `has_symbol_support`

## Used by

- [[cw-wiki]] — from `imports_cobertura_with_the_same_numbers`, `imports_lcov_and_ranks_gaps_by_what_depends_on_them`, `missing_and_malformed_reports_are_errors`
- [[Api Module]] — from `coverage`, `import_coverage`
- [[codewiki]] — from `run`

## See also

- [[_index]]
- [[cw-wiki]]
