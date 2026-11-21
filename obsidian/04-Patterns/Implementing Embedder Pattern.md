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

## See also

- [[_index]]
- [[Embedder]]
