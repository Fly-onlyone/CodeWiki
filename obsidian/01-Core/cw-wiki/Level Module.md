---
tags: [core, cw-wiki, module]
---

# Level Module

> Severity of a finding, shared by the analyses and the CI report.

## What it does

This tiny module holds one type, [[Level]], the severity of a finding. The health checks, the security scan and the CI report all use it, so a finding means the same thing everywhere. It sorts from note to error, so a threshold comparison works.

## Source

- `crates/cw-wiki/src/level.rs` — module `cw_wiki::level`

## How it works

`Level` has three values in order: note, warning, error. Its derived ordering follows that order, so code can ask whether a finding is at or above a threshold with a plain comparison. `as_str` gives the lowercase word, and `parse` reads it back. Serialisation uses the same lowercase words.

## Items

- [[Level]] — enum

## Gotchas

- `parse` is exact. It accepts only the lowercase words `note`, `warning` and `error`, and returns nothing for anything else.
- The order of the variants in the source matters. Changing it changes every threshold check.

## See also

- [[_index]]
- [[cw-wiki]]
