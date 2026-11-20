---
tags: [pattern]
---

# Implementing PackageInfo Pattern

> Add a new `PackageInfo` by implementing the trait; 3 types already follow this shape.

## When to apply

When adding a type that must act as a `PackageInfo`. Existing implementations: [[Machine]], `NoPackageInfo`, and `KnowsSerde`.

## The pattern

```rust
impl PackageInfo for NewType {
    fn describe(&self, language: Language, package: &str, version: Option<&str>) -> Option<String> {
        todo!()
    }
}
```

## Why

Says what a package from outside the repository is.

## See also

- [[_index]]
- [[Manifests Module]]
