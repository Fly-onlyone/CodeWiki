---
tags: [pattern]
---

# Implementing LlmProvider Pattern

> Add a new `LlmProvider` by implementing the trait; 4 types already follow this shape.

## When to apply

When adding a type that must act as a `LlmProvider`. Existing implementations: [[AgentCli]], [[Anthropic]], `Ollama`, and `OpenAiCompatible`.

## The pattern

```rust
impl LlmProvider for NewType {
    fn complete(&self, system: &str, user: &str) -> Result<Completion> {
        todo!()
    }
    fn name(&self) -> String {
        todo!()
    }
}
```

## See also

- [[_index]]
- [[LlmProvider]]
