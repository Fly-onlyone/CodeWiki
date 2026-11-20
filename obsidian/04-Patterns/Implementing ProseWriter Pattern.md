---
tags: [pattern]
---

# Implementing ProseWriter Pattern

> Add a new `ProseWriter` by implementing the trait; 3 types already follow this shape.

## When to apply

When adding a type that must act as a `ProseWriter`. Existing implementations: [[LlmWriter]], `Draws`, and `Scripted`.

## The pattern

```rust
impl ProseWriter for NewType {
    fn illustrate( &self, _note: &Planned, _text: &str, _structural: Option<&str>, _context: &WriterContext, ) -> Option<String> {
        todo!()
    }
    fn name(&self) -> String {
        todo!()
    }
    fn rewrite(&self, note: &Planned, draft: &str, context: &WriterContext) -> Rewrite {
        todo!()
    }
    fn usage(&self) -> WriterUsage {
        todo!()
    }
}
```

## Why

Replaces the structural prose of a note, typically with an LLM. Anything but `Rewrite::Text` keeps the structural draft.

## See also

- [[_index]]
- [[ProseWriter]]
