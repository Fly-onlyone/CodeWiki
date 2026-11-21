---
tags: [core, cw-embed, module]
---

# Prefix Module

> The text some embedding models expect before a query and before a document.

## What it does

Without it they still answer, but retrieval is worse. A model is known by its name, whichever server serves it.

## Source

- `crates/cw-embed/src/prefix.rs` — module `cw_embed::prefix`

## How it works

1. `OllamaEmbedder::new` in [[OllamaEmbedder]] and `OpenAiEmbedder::new` in [[OpenAiEmbedder]] call `rule`. `rule` calls `matches`.

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

## See also

- [[_index]]
- [[cw-embed]]
