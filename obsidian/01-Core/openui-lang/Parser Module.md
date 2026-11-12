---
tags: [core, openui-lang, module]
---

# Parser Module

> Statement splitting and expression parsing.

## Source

- `crates/openui-lang/src/parser.rs` — module `openui_lang::parser`

## How it works

Main items:

- `Expr` — enum
- `Statement` — struct
- `format_number` — render a number the way the language prints it: integers without `.0`.
- `Program` — struct
- `parse` — parse a program.
- `auto_close` — close unterminated strings and brackets so partial input still parses.
- `ExprParser` — struct
- `split` — split tokens into `(id, expression tokens)` pairs.

## Depends on

- [[Lexer Module]] — uses `Token`, `tokenize`

## Used by

- [[Eval Module]] — from `Evaluator`, `component`, `eval`, `evaluate`
- [[openui-lang]] — from `json`, `main`
- [[cw-illustrate]] — from `check`
- [[Node]] — from `display`

## See also

- [[_index]]
- [[openui-lang]]
