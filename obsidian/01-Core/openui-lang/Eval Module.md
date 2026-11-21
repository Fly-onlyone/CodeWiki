---
tags: [core, openui-lang, module]
---

# Eval Module

> Static evaluation of a program into a tree of elements.

## What it does

Illustrations in CodeWiki are static documents: there are no queries, mutations or user actions. Evaluation resolves references from the `root` statement, computes expressions, and checks every component call against a library. Problems are collected with the error codes of the OpenUI Lang 1.0 specification and never abort rendering of the rest.

## Source

- `crates/openui-lang/src/eval.rs` — module `openui_lang::eval`

## How it works

1. `check` in [[cw-illustrate]] calls `evaluate`.

## Items

- [[Node]] — enum
- [[Kind]] — enum
- `pub struct Param` — public struct; used by `Component`, `Library::component`.
- `pub struct Component` — public struct; used by `Library`, `Library::component`, `Library::get`.
- [[Library]] — the set of components a program may use.
- `pub enum Severity` — public enum; used by `Evaluator::error`, `LangError`.
- `pub struct LangError` — public struct; used by `Document`, `Evaluator`, `Evaluator::error`.
- `pub struct Document` — public struct; used by `check`, `evaluate`.
- `const MAX_REF_DEPTH: usize` — references deeper than this are a cycle or a pathological program.
- `fn member(object: &Node, field: &str) -> Node` — `object.field`; on a list it plucks the field from every element.
- `pub fn evaluate(program: &Program, library: &Library) -> Document` — evaluate a parsed program against a library.

And 3 more private items with no comment.

## Depends on

- [[Node]] — uses `Node`, `as_list`, `display`, `truthy`
- [[Parser Module]] — uses `Expr`, `Program`
- [[Kind]] — uses `Kind`
- [[Library]] — uses `Library`

## Used by

- [[Library]] — from `Library`, `component`, `get`
- [[cw-illustrate]] — from `check`

## See also

- [[_index]]
- [[openui-lang]]
