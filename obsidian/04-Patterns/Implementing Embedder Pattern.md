---
tags: [pattern]
---

# Implementing Embedder Pattern

> Add a new `Embedder` by implementing the trait; 3 types already follow this shape.

## When to apply

When adding a type that must act as a `Embedder`. Existing implementations: [[LocalEmbedder]], `OllamaEmbedder`, and `OpenAiEmbedder`.

## The pattern

```rust
impl Embedder for NewType {
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        todo!()
    }
    fn name(&self) -> String {
        todo!()
    }
}
```

## See also

- [[_index]]
- [[Embedder]]
