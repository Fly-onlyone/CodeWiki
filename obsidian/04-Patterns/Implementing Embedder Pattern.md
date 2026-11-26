---
tags: [pattern]
---

# Implementing Embedder Pattern

> Add a new `Embedder` by implementing the trait; 5 types already follow this shape.

## When to apply

When adding a type that must act as a `Embedder`. Existing implementations: [[LocalEmbedder]], [[OllamaEmbedder]], [[OpenAiEmbedder]], `Failing`, and `StandIn`.

## The pattern

```rust
impl Embedder for NewType {
    fn batch_size(&self) -> usize {
        todo!()
    }
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        todo!()
    }
    fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        todo!()
    }
    fn name(&self) -> String {
        todo!()
    }
    fn probe(&self) -> Result<usize> {
        todo!()
    }
}
```

## Why

Semantic search must work with many sources of vectors: an in-process hash, Ollama, and any OpenAI-style server. The rest of the code sees only the `Embedder` trait, so a new source plugs in without a change elsewhere.

Only `name` and `embed` are required. The other methods have defaults, and a type overrides one only when it needs to:

- `embed_query` embeds one text like a document. Override it when the model wants a different prefix for a query, as the Ollama and OpenAI-style embedders do.
- `batch_size` defaults to 64. A slow embedder names a smaller number, because between two batches a long run reports progress and can be stopped.
- `probe` embeds one word and returns the vector length. The Settings page uses it for its Test button. A server that needs a long time to load its model may override it to set its own limit.

`name` matters most. It is stored with each vector, and vectors from different embedders are never compared. A new name re-embeds everything. So the name must change when the vectors change, for example when the model or a prefix rule changes. The two HTTP embedders put the model and the prefix rule id in it.

## Don't

- Do not return vectors of different lengths in one `embed` call. Every vector must have the same length, and the count must match the number of texts, in order.
- Do not keep a name when the model behind it changes. Old vectors would be compared with new ones.
- Do not return a zero-length vector. Real embedders scale each vector to unit length with `normalize`, which the similarity function expects. A zero vector stays zero.
- Do not make the hash or features unstable. The local embedder uses a fixed hash so stored vectors stay valid across runs and platforms.
- Do not wait without a limit on the network. The HTTP embedders set a timeout, and give the first call longer so a server can load its model.
- Do not build a new type without adding it to `from_config` and the `PROVIDERS` list, or users cannot choose it.

## See also

- [[_index]]
- [[Embedder]]
