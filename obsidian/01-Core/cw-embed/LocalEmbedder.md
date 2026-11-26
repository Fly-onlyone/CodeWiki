---
tags: [core, cw-embed]
---

# LocalEmbedder

> The word-form embedder: signed feature hashing, no model and no network.

## What it does

`LocalEmbedder` is the default embedder. It needs no model file, no download and no network, so semantic search works on a fresh install. It turns text into a vector of 384 numbers by hashing word forms. Texts that share words, word stems or identifier parts get close vectors. It does not know that two different words mean the same; a neural model served by Ollama or another server does.

## Source

- `crates/cw-embed/src/lib.rs` — definition of `LocalEmbedder`

## How it works

`embed_one` does this:

1. `words` splits the text into lowercase words, breaking identifiers at underscores and camel-case humps.
2. Common words such as "the" are dropped.
3. Each word is counted. Its weight is 1 plus the natural log of its count, so a word said ten times is not ten times as important.
4. For each word, the stem goes into a slot picked by a hash. For words longer than 3 characters, each trigram of letters goes in too, at 0.35 of the weight.
5. Each pair of neighbouring stems goes in at 0.6.
6. The result is scaled to unit length.

The hash gives both the slot and a plus or minus sign, taken from different bits, so collisions cancel as often as they add. A different salt per feature kind keeps words, trigrams and pairs apart. `embed` simply calls `embed_one` for each text and never fails.

## Items

Declared as `pub struct LocalEmbedder`.

Members:

- `fn default() -> Self` — public method, returns `Self`; called by `from_config`.
- `pub fn embed_one(&self, text: &str) -> Vec<f32>` — public method, returns `Vec<f32>`; called by `LocalEmbedder::embed`.
- `fn name(&self) -> String` — public method, returns `String`.
- `fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>` — public method, returns `Result<Vec<Vec<f32>>>`.

And 1 more private item with no comment.

## Depends on

- [[cw-embed]] — uses `STOP_WORDS`, `hash`, `normalize`, `stem`
- [[Error Module]] — uses `Result`
- [[Embedder]] — uses `Embedder`

## Used by

- [[cw-server]] — from `StandIn`, `held`, `stand_in`
- [[cw-embed]] — from `a_probe_embeds_one_word_and_reports_the_vector_length`, `from_config`

## Gotchas

- The hash is FNV-1a with fixed salts, and `name` is `local-hash-v1:384`. Stored vectors depend on this exact recipe. If the recipe or size changes, the name must change too, so old vectors are rebuilt.
- Empty text, or text with only stop words, gives a zero vector. It scores 0 against everything.
- The stemmer is crude on purpose; it only has to be consistent.
- It matches form, not meaning: "car" and "automobile" are unrelated to it.

## See also

- [[_index]]
- [[cw-embed]]
