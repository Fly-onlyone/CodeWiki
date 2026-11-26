---
tags: [core, openui-lang]
---

# Node

> `Node` enum defined in `openui_lang::eval` with 4 methods.

## What it does

`Node` is a value in an evaluated illustration. It is `Null`, a bool, a number, text, a list, a map, or an `Element`, which is a named component with its props. The tree of `Node` values is what the evaluator hands to the renderer. The crate itself draws nothing: callers such as [[cw-illustrate]] and the [[Illustration Module]] walk the tree.

## Source

- `crates/openui-lang/src/eval.rs` — definition of `Node`

## How it works

The evaluator builds a `Node` for every expression. Literals map directly. A component call becomes `Element` with props keyed by parameter name.

Helpers read nodes. `as_text` returns the string of a `Text` node. `as_list` returns the items of a `List`, or an empty slice for any other node, so callers need no match.

Two private helpers serve the evaluator. `truthy` decides conditions: `Null`, `false`, `0`, NaN and empty text are false, and everything else is true. `display` gives the text used when `+` joins strings; numbers print through `format_number`.

## Items

Declared as `pub enum Node`.

Members:

- `pub fn as_text(&self) -> Option<&str>` — public method, returns `Option<&str>`; called by `text`.
- `pub fn as_list(&self) -> &[Node]` — public method, returns `&[Node]`; called by `list_of`, `review`.
- `fn display(&self) -> String` — text form used by string concatenation.

And 1 more private item with no comment.

## Depends on

- [[Parser Module]] — uses `format_number`

## Used by

- [[Eval Module]] — from `Document`, `binary`, `component`, `eval`
- [[cw-illustrate]] — from `Props`, `element`, `review`, `text`
- [[codewiki-app]] — from `AppView`, `Page`, `open_note_as`
- [[Illustration Module]] — from `Props`, `list_of`, `render`

## Gotchas

- `display` returns an empty string for lists, maps and elements, so joining one of them into text adds nothing.
- An empty list, an empty map and any element count as true.
- `Node` derives `PartialEq` on `f64`, so two NaN numbers are not equal.

## See also

- [[_index]]
- [[Eval Module]]
