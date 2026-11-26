---
tags: [core, cw-embed, module]
---

# Prefix Module

> The text some embedding models expect before a query and before a document.

## What it does

Some embedding models were trained with a short instruction in front of the text, and a different one for a search question than for a document. Leave it out and the model still answers, but finds less. This module keeps a table of known model families and what each wants. It matches by model name, so the same rule applies whichever server serves the model.

## Source

- `crates/cw-embed/src/prefix.rs` — module `cw_embed::prefix`

## How it works

`RULES` holds six families: gemma, qwen3, nomic, mxbai, bge and e5. Each `Rule` has an id, name parts, a query prefix and a document prefix. `rule` lowercases the model name and returns the first rule that `matches`. A rule matches when its first part starts a word in the name (at the start, or after a character that is not a letter or digit) and every other part appears anywhere in the name. So `bge-` plus `-v1.5` matches the bge small v1.5 models. [[OllamaEmbedder]] and [[OpenAiEmbedder]] call `rule` when they are made, put the prefix in front of each text, and add `+id` to the vector name.

## Items

- `pub struct Rule` — what one family of models wants put in front of a text.
- `pub const RULES: &[Rule]` — the known families. The first match wins.
- `pub fn rule(model: &str) -> Option<&'static Rule>` — the rule for a model name as a server gives it, if the model is known.
- `fn matches(rule: &Rule, name: &str) -> bool` — private function, returns `bool`; called by `rule`.

And 1 more private item with no comment.

## Used by

- [[OllamaEmbedder]] — from `OllamaEmbedder`, `new`
- [[OpenAiEmbedder]] — from `OpenAiEmbedder`, `new`
- [[cw-embed]] — from `named`

## Gotchas

- The first matching rule wins, so order in `RULES` matters.
- Matching is deliberately narrow. `bge-m3`, `text-embedding-3-small` and `gte-modernbert-base` get no rule, and a name like `subge-small-v1.5` does not match.
- The `+id` in the vector name is on purpose: vectors made with and without a prefix must never be compared. Changing a prefix string changes what the vectors mean without changing the name, so existing vectors would be stale.
- Each prefix comes from the model's own card. Do not edit one from memory.
- The qwen3 query prefix includes a newline.

## See also

- [[_index]]
- [[cw-embed]]
