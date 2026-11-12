---
tags: [core, cw-wiki, module]
---

# Ci Module

> Findings for continuous integration, as text, SARIF or JUnit.

## Source

- `crates/cw-wiki/src/ci.rs` — module `cw_wiki::ci`

## How it works

Main items:

- `CheckFinding` — struct
- `check` — run every analysis and return findings sorted by file, line and rule.
- `to_junit` — JUnit XML: one test case per rule, failing when the rule has findings at or above `threshold`.
- `to_sarif` — SARIF 2.1.0, the format code-scanning services ingest.
- `to_text` — function
- `fails` — true when any finding is at or above `threshold`.
- `RULES` — what each rule means, shown by tools that list rules.
- `xml_escape` — function

## Depends on

- [[Health Module]] — uses `dead_code`, `doc_drift`, `file_health`
- [[cw-core]] — uses `VAULT_DIR`, `VERSION`
- [[Error Module]] — uses `Result`
- [[Decisions Module]] — uses `list`
- [[Security Module]] — uses `scan`
- [[Index]] — uses `Index`
- [[Level]] — uses `Level`

## Used by

- [[codewiki]] — from `run`
- [[cw-wiki]] — from `check_reports_security_findings_as_rules`

## See also

- [[_index]]
- [[cw-wiki]]
