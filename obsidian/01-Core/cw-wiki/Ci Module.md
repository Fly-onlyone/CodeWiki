---
tags: [core, cw-wiki, module]
---

# Ci Module

> Findings for continuous integration, as text, SARIF or JUnit.

## What it does

`check` gathers what the other analyses report (dead code, file health, documentation drift, stale decisions, security patterns) into one list a CI system can gate on or annotate a pull request with.

## Source

- `crates/cw-wiki/src/ci.rs` — module `cw_wiki::ci`

## How it works

1. `run` in [[codewiki]] calls `check`. `check` calls `dead_code`, then `file_health`, then `doc_drift` (all in [[Health Module]]), then `list` in [[Decisions Module]], then `scan` in [[Security Module]].
2. `run` in [[codewiki]] calls `fails`.
3. `run` in [[codewiki]] calls `to_text`.
4. `run` in [[codewiki]] calls `to_sarif`.
5. `run` in [[codewiki]] calls `to_junit`. `to_junit` calls `xml_escape`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

## See also

- [[_index]]
- [[cw-wiki]]
