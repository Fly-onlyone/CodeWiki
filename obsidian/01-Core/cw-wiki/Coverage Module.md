---
tags: [core, cw-wiki, module]
---

# Coverage Module

> Test coverage imported from LCOV (`lcov.info`) and Cobertura XML reports.

## What it does

A report is parsed, its paths are matched to indexed files, and the result is cached in the index. Reporting then ranks the gaps: an uncovered file matters more when it changes often (hotspot) or much depends on it (rank).

Both formats are read by small parsers written against their public descriptions: the `geninfo` tracefile format (`SF`, `DA`, `LF`, `LH`, `end_of_record`) and the Cobertura `coverage-04.dtd` element layout (`sources/source`, `class@filename`, `line@number@hits`).

## Source

- `crates/cw-wiki/src/coverage.rs` — module `cw_wiki::coverage`

## How it works

1. `run` in [[codewiki]] and `coverage` in [[Api Module]], and 1 more, call `import`. `import` calls `Error::not_found`, then `Error::io` (both in [[Error]]), then `import_text`.
2. `run` in [[codewiki]] and `import_coverage` in [[Api Module]] call `save`. `save` calls `Error::external` in [[Error]].
3. `run` in [[codewiki]] and `coverage` in [[Api Module]] call `load`.
4. `run` in [[codewiki]] and `coverage` in [[Api Module]], and 1 more, call `report`. `report` calls `load`, then `hotspots` (both in [[Gitintel Module]]), then `is_test_path` in [[Rustscan Module]].
5. `import_coverage` in [[Api Module]] calls `import_text`. `import_text` calls `parse`, then `normalize`, then `resolve`, then `percent`, then `uncovered_runs`. `parse` calls `parse_cobertura`, then `parse_lcov`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const META_KEY: &str` — meta key under which imported coverage is cached in the index.
- `pub const DEFAULT_REPORTS: [&str; 12]` — where a report is looked for when no path is given, relative to the repository root.
- `pub enum Format` — public enum; used by `CoverageData`, `CoverageReport`, `RawReport`.
- `fn parse_lcov(text: &str) -> Result<RawReport>` — private function, returns `Result<RawReport>`; called by `parse`.
- `struct Tag<'a>` — one XML tag: `<name a="b">`, `</name>` or `<name/>`.
- `fn parse_cobertura(text: &str) -> Result<RawReport>` — private function, returns `Result<RawReport>`; called by `parse`.
- `fn parse(text: &str) -> Result<RawReport>` — parse a report, telling the format from its first character.
- `fn normalize(path: &str) -> String` — private function, returns `String`; called by `import_text`.
- `fn resolve<'a>(candidates: &[String], known: &'a [String]) -> Option<&'a str>` — the indexed file a report path refers to. Reports carry absolute paths, paths relative to the repository, or paths relative to a package directory. The indexed path that shares the longest run of trailing segments wins; a report path shorter than the indexed one only matches when exactly one file ends with it.
- `pub struct FileCoverage` — public struct; used by `CoverageData`, `CoverageReport`, `import_text`.
- `pub struct CoverageData` — coverage as imported, with paths matched to the index.
- `fn percent(hit: u32, found: u32) -> f64` — private function, returns `f64`; called by `import_text`.
- `fn uncovered_runs(lines: &BTreeMap<u32, u64>) -> Vec<[u32; 2]>` — private function, returns `Vec<[u32; 2]>`; called by `import_text`.
- `pub fn import_text(index: &Index, text: &str, source: &str) -> Result<CoverageData>` — parse report `text` and match its files to the index.
- `pub fn import(root: &Path, index: &Index, report: Option<&Path>) -> Result<CoverageData>` — read and import a report file. With no `report`, the first of `DEFAULT_REPORTS` that exists under `root` is used.
- `pub fn save(index: &Index, data: &CoverageData) -> Result<()>` — cache imported coverage in the index.
- `pub fn load(index: &Index) -> Result<Option<CoverageData>>` — coverage cached in the index, if any was imported.
- `pub struct Gap` — A poorly covered file and why it matters.
- `pub struct CoverageReport` — public struct; used by `coverage`, `import_coverage`, `report`.
- `pub fn report(index: &Index, data: &CoverageData, limit: usize) -> Result<CoverageReport>` — summarise imported coverage and rank the gaps.

And 6 more items.

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
