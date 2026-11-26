---
tags: [core, openui-lang, module]
---

# Parser Module

> Statement splitting and expression parsing.

## What it does

The parser turns OpenUI Lang text into a `Program`: a list of `identifier = expression` statements, each holding an `Expr` tree. It is built to accept streamed or half-written model output, so it never fails. The evaluator then resolves the `Program`. Newlines end a statement, except inside brackets and before a ternary `?` or `:`. Expressions use a Pratt parser (each operator has a binding strength) with the precedence of the reference implementation.

## Source

- `crates/openui-lang/src/parser.rs` — module `openui_lang::parser`

## How it works

`parse` runs three steps. `auto_close` appends the missing quote and closing brackets for unfinished input and reports that it did so; the result is stored as `incomplete`. `tokenize` from the [[Lexer Module]] makes tokens. `split` cuts the tokens into statements and keeps a newline inside brackets or a ternary.

For each statement, `ExprParser::expr` parses with a minimum precedence. `prefix` reads literals, arrays, objects, calls, references and unary operators. `infix` reads binary operators, the ternary, `.field` and `[index]`. Precedence rises from ternary through or, and, equality, comparison, add and multiply to unary and member access.

A PascalCase word followed by `(` is a component call. `@Name(` is a function call. Without parentheses both are plain references.

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

## Gotchas

- A line that is not `name = expr` is dropped without any error.
- A later statement with the same name wins; `Program::get` searches from the end.
- Nesting deeper than 200 gives `Null` and stops that expression, so hostile input cannot overflow the stack.
- A token that cannot start an expression is consumed as `Null`, so loops always advance.
- `format_number` prints whole numbers without `.0`, but only below 1e15.
- Left-hand names may be PascalCase or `$state` names, not only lowercase ones.

## See also

- [[_index]]
- [[openui-lang]]
