---
tags: [pattern]
---

# Implementing TestLanguage Pattern

> Add a new `TestLanguage` by implementing the trait; 7 types already follow this shape.

## When to apply

When adding a type that must act as a `TestLanguage`. Existing implementations: `CSharpTests`, `CTests`, `GoTests`, `JvmTests`, `RustTests`, `JsTests`, and [[PythonTests]].

## The pattern

```rust
impl TestLanguage for NewType {
    fn command(&self, env: &Env, path: &str, kind: TestKind, tests: &[TestCase]) -> String {
        todo!()
    }
    fn commands(&self, files: &[&ImpactedTestFile]) -> Vec<String> {
        todo!()
    }
    fn covers(&self, language: Language) -> bool {
        todo!()
    }
    fn is_test_file(&self, env: &Env, path: &str) -> bool {
        todo!()
    }
    fn language(&self) -> Language {
        todo!()
    }
    fn tests(&self, env: &Env, path: &str, text: &str) -> Vec<TestCase> {
        todo!()
    }
}
```

## Why

The two things that differ between languages: what a test is, and the command that runs one. Everything else about the map is reachability over the dependency graph.

## See also

- [[_index]]
- [[Testmap Module]]
