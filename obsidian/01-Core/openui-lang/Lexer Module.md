---
tags: [core, openui-lang, module]
---

# Lexer Module

> Tokenizer for OpenUI Lang.

## What it does

Follows the token rules of the reference implementation (`@openuidev/lang-core`, MIT): newlines are significant, `//` and `#` start line comments, PascalCase words are component names, `$name` is a state variable, `@Name` is a function call, and unknown characters are skipped.

## Source

- `crates/openui-lang/src/lexer.rs` — module `openui_lang::lexer`

## How it works

1. `parse` in [[Parser Module]] calls `tokenize`. `tokenize` calls `unescape_json`, then `is_word`, then `is_word_start`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub enum Token` — public enum; used by `ExprParser`, `ExprParser::cur`, `ExprParser::eat`, `ExprParser::infix_prec` and 3 more.
- `fn is_word(c: char) -> bool` — private function, returns `bool`; called by `tokenize`.
- `fn is_word_start(c: char) -> bool` — private function, returns `bool`; called by `tokenize`.
- `fn unescape_json(body: &str) -> Option<String>` — decode a double-quoted string body with JSON escapes. Returns `None` when an escape is malformed, in which case the caller keeps the raw text.
- `pub fn tokenize(src: &str) -> Vec<Token>` — public function, returns `Vec<Token>`; called by `parse`.

## Used by

- [[Parser Module]] — from `ExprParser`, `cur`, `eat`, `infix_prec`

## See also

- [[_index]]
- [[openui-lang]]
