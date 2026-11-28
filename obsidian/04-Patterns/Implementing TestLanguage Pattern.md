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

Between languages only two things differ: what a test is, and the command that runs one. Everything else about the test map is reachability over the dependency graph, which is the same for all. The trait keeps those two things in one small place per language. A new language is one type plus one entry in the `LANGUAGES` list. The reachability, the report, the MCP tool and the app need no change, as the comment on `LANGUAGES` says. Java and Kotlin share one entry through `covers`, because one Gradle or Maven command runs the tests of both.

## Don't

- Don't add language checks to the reachability code. Put them in the trait methods.
- Don't forget the entry in `LANGUAGES`. A type that is not listed is never asked.
- Don't return a made-up command. `command` should give an empty string when none can be derived.
- Don't count a source file that merely calls a test function as a test file. `is_test_file` is for test code kept apart from the code it tests.
- Don't rely on the default `commands` when the runner takes several files in one run. Override it to fold them.

## See also

- [[_index]]
- [[Testmap Module]]
