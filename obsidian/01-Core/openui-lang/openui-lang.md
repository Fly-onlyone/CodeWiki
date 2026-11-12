---
tags: [core, openui-lang]
---

# openui-lang

> Parser and static evaluator for OpenUI Lang.

## Source

- `crates/openui-lang/Cargo.toml` — package manifest
- `crates/openui-lang/src/lib.rs` — crate root

## How it works

Builds a library.

Modules:

- [[Eval Module]] — static evaluation of a program into a tree of elements.
- [[Lexer Module]] — tokenizer for OpenUI Lang.
- [[Parser Module]] — statement splitting and expression parsing.

## Depends on

- [[Parser Module]] — uses `Expr`, `parse`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
