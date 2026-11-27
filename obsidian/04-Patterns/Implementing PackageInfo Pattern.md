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
    fn describe_in( &self, language: Language, package: &str, version: Option<&str>, _dirs: &[&str], ) -> Option<String> {
        todo!()
    }
}
```

## Why

The repository's build files say which package version is asked for and locked. They do not say what the package is. That answer lives in files a machine may or may not have, such as cargo's registry cache or `node_modules`. The `PackageInfo` trait puts that lookup behind one method, `describe`, so the wiki code does not care where the text comes from.

`Machine` reads the local files and never uses the network. `NoPackageInfo` knows nothing, so tests and runs on a clean machine still work. A test type such as `OnlyAxum` can answer for one package.

The free function `describe` calls the trait and keeps the answers in `packages.json` in the vault. What the machine finds wins. What it does not find comes from the saved file. This way a machine without the cache writes the same notes as one with it.

## Don't

- Do not read the network from `describe`. The answer must not depend on being online.
- Do not return an empty string for an unknown package. Return `None`, so the saved description is kept.
- Do not return more than one line. The text goes into a note as a short summary.
- Do not write to the vault from the implementation. Only the free `describe` function saves `packages.json`.
- Do not assume a version is present. `version` is `None` when there is no lock file.

## See also

- [[_index]]
- [[Manifests Module]]
