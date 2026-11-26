---
tags: [core, cw-wiki, module]
---

# Coverage Module

> Test coverage imported from LCOV (`lcov.info`) and Cobertura XML reports.

## What it does

Test tools write coverage reports that say which lines ran. This module reads two common formats, LCOV and Cobertura XML. It matches the report's file names to the files in the index and caches the result. Then it ranks the files worth testing first: an uncovered file counts more when it changes often or many other files depend on it.

## Source

- `crates/cw-wiki/src/coverage.rs` — module `cw_wiki::coverage`

## How it works

`import` finds the report: the path given, or the first existing file in `DEFAULT_REPORTS` under the repository root. It reads the text and calls `import_text`.

`parse` tells the format from the first character: `<` means Cobertura, anything else LCOV. Both parsers produce a map of file name to line hit counts. LCOV records for the same file add up. In Cobertura, a line is keyed by number, so lines listed under both a class and its methods count once.

Reports name files in many ways, so `resolve` matches them to indexed paths. It tries the exact path, then the indexed path that is the longest suffix of the report path. A report path shorter than the indexed one matches only when exactly one file ends with it. Cobertura source directories are tried as prefixes. Entries that match nothing are counted in `unmatched`.

Per file, `import_text` counts found and hit lines, a percent, and the runs of uncovered lines (`uncovered_runs`). `save` stores the result as JSON in the index under `META_KEY`; `load` reads it back.

`report` ranks gaps. A file's score is its uncovered share, times a size factor that saturates at 50 uncovered lines, times a weight from its hotspot score and its summed pagerank. It also lists source files with functions that the report never mentions.

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

## Gotchas

- Windows paths and `//?/` prefixes are normalised to forward slashes before matching.
- A file with no lines and no `LF` total is dropped, so it never appears in the report.
- A file with zero instrumented lines counts as 100 percent.
- Negative or fractional LCOV hit counts are rounded: anything above zero means the line ran.
- If the cached JSON cannot be read back, `load` returns nothing instead of an error.
- An import that finds no `SF:` record or no `<coverage>` element is an error, not an empty report.

## See also

- [[_index]]
- [[cw-wiki]]
