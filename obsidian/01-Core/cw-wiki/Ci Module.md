---
tags: [core, cw-wiki, module]
---

# Ci Module

> Findings for continuous integration, as text, SARIF or JUnit.

## What it does

A CI system needs one list of problems it can fail a build on or show on a pull request. This module collects the results of the other analyses into that list and prints it as plain text, SARIF (for code-scanning services) or JUnit XML (for test dashboards).

## Source

- `crates/cw-wiki/src/ci.rs` — module `cw_wiki::ci`

## How it works

`check` runs each analysis and maps every result to a `CheckFinding` with a rule id, level, message, file and line:

- Dead code: only symbols whose certainty is `Likely`, as warnings.
- File health: each finding becomes `health/<detector>`; a penalty of 2 or more is a warning, less is a note. Dead-code findings are skipped here since they are reported per symbol.
- Doc drift: missing source and dead path are warnings, a changed source is a note, a broken link is an error.
- Stale decisions: warnings.
- Security scan results are copied with their own rule and level.

The list is sorted by file, line, rule, then message, so output is stable between runs.

`fails` is true when any finding is at or above the threshold [[Level]]. `to_text` prints one line per finding and a summary count. `to_sarif` writes SARIF 2.1.0 with all rules listed. `to_junit` writes one test case per rule in `RULES`; a case fails when a finding of that rule reaches the threshold.

## Items

- `pub struct CheckFinding` — public struct; used by `check`, `fails`, `to_junit`, `to_sarif` and 1 more.
- `pub(crate) const RULES: [(&str, &str); 22]` — what each rule means, shown by tools that list rules.
- `pub fn check(root: &Path, index: &Index) -> Result<Vec<CheckFinding>>` — run every analysis and return findings sorted by file, line and rule.
- `pub fn fails(findings: &[CheckFinding], threshold: Level) -> bool` — true when any finding is at or above `threshold`.
- `pub fn to_text(findings: &[CheckFinding]) -> String` — public function, returns `String`; called by `run`.
- `pub fn to_sarif(findings: &[CheckFinding]) -> Value` — SARIF 2.1.0, the format code-scanning services ingest.
- `fn xml_escape(text: &str) -> String` — private function, returns `String`; called by `to_junit`.
- `pub fn to_junit(findings: &[CheckFinding], threshold: Level) -> String` — JUnit XML: one test case per rule, failing when the rule has findings at or above `threshold`. Rules with no findings pass, so the report shows what was checked.

## Depends on

- [[Security Module]] — uses `COMMAND_INJECTION`, `ENV_SET_VAR`, `HARDCODED_SECRET`, `INSECURE_HTTP`
- [[Health Module]] — uses `dead_code`, `doc_drift`, `file_health`
- [[cw-core]] — uses `VAULT_DIR`, `VERSION`
- [[Error Module]] — uses `Result`
- [[Decisions Module]] — uses `list`
- [[Index]] — uses `Index`
- [[Level]] — uses `Level`

## Used by

- [[codewiki]] — from `run`
- [[cw-wiki]] — from `check_reports_security_findings_as_rules`

## Gotchas

- `RULES` must list every rule id that `check` can emit. JUnit and SARIF only show rules from that table.
- Line 0 means the finding is about the whole file; SARIF then omits the region.
- A rule with findings below the threshold still passes in JUnit.
- Doc-drift files are reported under the vault folder path, not the repo source path.
- `xml_escape` replaces control characters with spaces because XML 1.0 cannot hold them.

## See also

- [[_index]]
- [[cw-wiki]]
