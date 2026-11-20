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

Builds a library.

## Items

Modules:

- [[Eval Module]] — static evaluation of a program into a tree of elements.
- [[Lexer Module]] — tokenizer for OpenUI Lang.
- [[Parser Module]] — statement splitting and expression parsing.

## Depends on

- [[Parser Module]] — uses `Expr`, `parse`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
