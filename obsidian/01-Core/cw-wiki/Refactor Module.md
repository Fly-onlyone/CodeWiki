---
tags: [core, cw-wiki, module]
---

# Refactor Module

> Refactoring plan: suggestions derived from facts the index already holds.

## Source

- `crates/cw-wiki/src/refactor.rs` — module `cw_wiki::refactor`

## How it works

Main items:

- `Kind` — enum
- `Suggestion` — struct
- `plan` — every suggestion, highest priority first.
- `Facts` — facts shared by the detectors.
- `MAX_LISTED` — const
- `GENERIC_PREFIXES` — Method-name prefixes that describe access, not a responsibility.
- `dir_of` — function
- `priority` — function

## Depends on

- [[Index]] — uses `Index`, `crates`, `edges`, `symbols`
- [[Gitintel Module]] — uses `dependents_by_file`, `hotspots`, `load`
- [[Health Module]] — uses `MANY_METHODS`, `PASSIVE_ATTRS`, `file_health`
- [[Error Module]] — uses `Result`
- [[Metrics Module]] — uses `strongly_connected_components`
- [[Analyze Module]] — uses `DEPENDENCY_KINDS`
- [[Rustscan Module]] — uses `is_test_path`
- [[Symbol]] — uses `Symbol`

## Used by

- [[cw-wiki]] — from `a_healthy_repository_gets_no_suggestions`, `of`, `suggests_one_refactoring_per_problem_with_evidence_and_steps`
- [[codewiki]] — from `run`
- [[Api Module]] — from `refactor`

## See also

- [[_index]]
- [[cw-wiki]]
