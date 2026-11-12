---
tags: [core, openui-lang, module]
---

# Eval Module

> Static evaluation of a program into a tree of elements.

## Source

- `crates/openui-lang/src/eval.rs` — module `openui_lang::eval`

## How it works

Main items:

- [[Node]] — enum
- [[Library]] — the set of components a program may use.
- [[Kind]] — enum
- `Param` — struct
- `Severity` — enum
- `Component` — struct
- `LangError` — struct
- `Document` — struct

## Depends on

- [[Node]] — uses `Node`, `as_list`, `display`, `truthy`
- [[Parser Module]] — uses `Expr`, `Program`
- [[Kind]] — uses `Kind`
- [[Library]] — uses `Library`

## Used by

- [[Library]] — from `Library`, `component`, `get`
- [[codewiki-app]] — from `open_note`
- [[cw-illustrate]] — from `check`
- [[Generate Module]] — from `generate`

## See also

- [[_index]]
- [[openui-lang]]
