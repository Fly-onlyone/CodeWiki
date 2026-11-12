---
tags: [core, openui-lang, module]
---

# Lexer Module

> Tokenizer for OpenUI Lang.

## Source

- `crates/openui-lang/src/lexer.rs` — module `openui_lang::lexer`

## How it works

Main items:

- `Token` — enum
- `tokenize` — function
- `is_word` — function
- `is_word_start` — function
- `unescape_json` — decode a double-quoted string body with JSON escapes.

## Used by

- [[Parser Module]] — from `ExprParser`, `cur`, `eat`, `infix_prec`

## See also

- [[_index]]
- [[openui-lang]]
