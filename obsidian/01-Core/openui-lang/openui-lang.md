---
tags: [core, openui-lang]
---

# openui-lang

> Parser and static evaluator for OpenUI Lang.

## What it does

OpenUI Lang (<https://github.com/thesysdev/openui>, MIT) is a compact language in which a model describes a user interface as a list of `name = Component(args)` statements. This crate is an independent Rust implementation of the language's syntax, written against the published 1.0 specification and the behavior of the reference parser. It has no UI dependencies: rendering the resulting tree is the caller's job.

```rust
use openui_lang::{Kind, Library, Node, evaluate, parse};

let library = Library::new(Some("Card"))
    .component("Card", &[("children", Kind::List, true)])
    .component("TextContent", &[("text", Kind::Text, true)]);
let document = evaluate(&parse("root = Card([t])\nt = TextContent(\"Hi\")"), &library);
assert!(document.is_valid());
assert!(matches!(document.root, Some(Node::Element { .. })));
```

## Source

- `crates/openui-lang/Cargo.toml` — package manifest
- `crates/openui-lang/src/lib.rs` — crate root

## How it works

A caller follows four steps. It builds a `Library` of allowed components. It calls `parse` on the model's text to get a `Program`. It calls `evaluate` with the program and the library to get a `Document`. It renders `Document.root`, which is a `Node` tree.

`evaluate` starts at the `root` statement, or at the library's root component if there is none. It follows references, computes expressions and checks each component against the library. Problems go into `Document.errors` with codes from the specification. A bad component becomes `Null` and the rest still renders. `is_valid` is true when there is a root and no error-level problem.

The crate root only re-exports types from the [[Parser Module]], the [[Lexer Module]] and the eval module.

## Items

Modules:

- [[Eval Module]] — static evaluation of a program into a tree of elements.
- [[Lexer Module]] — tokenizer for OpenUI Lang.
- [[Parser Module]] — statement splitting and expression parsing.

## Depends on

- [[Parser Module]] — uses `Expr`, `parse`

## Gotchas

- Only warnings do not stop rendering. Any error-level code makes `is_valid` false even when part of the tree renders.
- The language is static here: queries, mutations and actions are reported as warnings and produce nothing.
- Partial input is normal. Check `Document.incomplete` to know that the text was auto-closed.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
