---
tags: [core, openui-lang, module]
---

# Lexer Module

> Tokenizer for OpenUI Lang.

## What it does

The lexer turns OpenUI Lang source text into a flat list of `Token` values. OpenUI Lang is the small language a model uses to describe a user interface. The [[Parser Module]] reads this list instead of raw text. The token rules copy the reference implementation: newlines count as tokens, `//` and `#` start line comments, PascalCase words are component names, `$name` is a state variable, `@Name` is a function call. Characters it does not know are skipped.

## Source

- `crates/openui-lang/src/lexer.rs` — module `openui_lang::lexer`

## How it works

`tokenize` walks the characters once. Single characters map straight to a token. `=`, `!`, `>` and `<` look one character ahead to choose between `Equals` and `EqEq`, and so on. A single `&` or `|` is accepted as `And` or `Or`.

Words are read next. `true`, `false` and `null` become keyword tokens. A word that starts with an uppercase letter becomes `Type`, anything else becomes `Ident`.

Strings in double quotes are decoded by `unescape_json`, which handles the JSON escapes and surrogate pairs. Strings in single quotes support only `\n`, `\t` and a quoted character.

A `-` is part of a number only when a digit follows and the previous token is not a value. Otherwise it is `Minus`. The list always ends with `Eof`.

## Items

- `pub enum Token` — public enum; used by `ExprParser`, `ExprParser::cur`, `ExprParser::eat`, `ExprParser::infix_prec` and 3 more.
- `fn is_word(c: char) -> bool` — private function, returns `bool`; called by `tokenize`.
- `fn is_word_start(c: char) -> bool` — private function, returns `bool`; called by `tokenize`.
- `fn unescape_json(body: &str) -> Option<String>` — decode a double-quoted string body with JSON escapes. Returns `None` when an escape is malformed, in which case the caller keeps the raw text.
- `pub fn tokenize(src: &str) -> Vec<Token>` — public function, returns `Vec<Token>`; called by `parse`.

## Used by

- [[Parser Module]] — from `ExprParser`, `cur`, `eat`, `infix_prec`

## Gotchas

- A string with a malformed escape keeps its raw text; it is not an error.
- An unterminated string runs to the end of the input. This lets streamed, half-finished text still tokenize.
- `a -5` is `a`, `Minus`, `5`: the minus is subtraction because it follows a value. `[-1]` gives the number `-1`.
- `1.` gives a number and then a `Dot`: a decimal point needs a digit after it.
- Unknown characters, such as emoji, vanish without any message.
- Only ASCII letters and digits make up a word.

## See also

- [[_index]]
- [[openui-lang]]
