---
tags: [core, openui-lang, module]
---

# Eval Module

> Static evaluation of a program into a tree of elements.

## What it does

Turns a parsed OpenUI Lang program into a tree of elements that a renderer can draw, and checks the program against a library of allowed components. Pictures in notes are static, so queries, mutations and actions do not exist here. Problems found along the way are collected with the error codes of the OpenUI Lang 1.0 specification. They never stop the rest of the document from rendering.

## Source

- `crates/openui-lang/src/eval.rs` — module `openui_lang::eval`

## How it works

`evaluate` takes a `Program` and a [[Library]]. It picks the entry: the statement named `root`. Without one, it takes the first statement that calls the library's root component and records a warning. If nothing fits, it records an error. `resolve` then evaluates the entry statement. A reference to another statement is resolved the same way, with a stack of the statements being evaluated, to find cycles. References deeper than 64 levels count as a cycle.

`eval` computes each expression into a [[Node]]: text, number, bool, list, map or element. It handles unary and binary operators (with short-circuit `&&` and `||`), ternaries, member access and indexing. A few built-in functions work on lists: `Count`, `Sum`, `Avg`, `Min`, `Max`, `First`, `Last`, `Round`, `Abs`, `Floor`, `Ceil`.

For a component call, `component` finds it in the library and maps positional arguments to parameters by order. A missing, null or wrongly typed required value is an error, and that component is dropped as null. The rest still renders. The result is a `Document` with the root node, the errors, and an `incomplete` flag for partial input.

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

## Gotchas

- A reference to a statement that does not exist evaluates to null without an error. A streamed program may simply not be finished.
- The entry must produce a component. Otherwise there is a `no-root` error and no root.
- Extra arguments are ignored with a warning. A text parameter also accepts a number.
- `Query`, `Mutation` and `Action` are refused with a warning. Unknown `@functions` give a warning and null.
- The same error is reported once per statement, not once per reference.
- Division and modulo by zero give null. `+` with text on either side joins text.

## See also

- [[_index]]
- [[openui-lang]]
