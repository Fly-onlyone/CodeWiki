---
tags: [core, openui-lang, module]
---

# Parser Module

> Statement splitting and expression parsing.

## What it does

A program is a list of `identifier = expression` statements separated by newlines. Newlines inside brackets, and before a ternary `?` or `:`, do not end a statement. Expressions use a Pratt parser with the precedence of the reference implementation.

## Source

- `crates/openui-lang/src/parser.rs` — module `openui_lang::parser`

## How it works

1. `Node::display` in [[Node]] calls `format_number`.
2. `check` in [[cw-illustrate]] calls `parse`. `parse` calls `auto_close`, then `tokenize` in [[Lexer Module]], then `split`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub enum Expr` — public enum; used by `Evaluator::component`, `Evaluator::eval`, `Evaluator::function`, `ExprParser::call` and 5 more.
- `pub struct Statement` — public struct; used by `Program`, `Program::get`, `parse`.
- `pub struct Program` — public struct; used by `Evaluator`, `evaluate`, `parse`.
- `pub fn auto_close(input: &str) -> (String, bool)` — close unterminated strings and brackets so partial input still parses.
- `fn split(tokens: &[Token]) -> Vec<(String, Vec<Token>)>` — split tokens into `(id, expression tokens)` pairs.
- `const MAX_DEPTH: usize` — nesting beyond this is treated as malformed input, not parsed deeper.
- `pub fn format_number(n: f64) -> String` — render a number the way the language prints it: integers without `.0`.
- `pub fn parse(source: &str) -> Program` — parse a program. Never fails: malformed lines are skipped, as in the reference parser, so streamed or partly wrong input still yields a tree.

And 10 more private items with no comment.

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
