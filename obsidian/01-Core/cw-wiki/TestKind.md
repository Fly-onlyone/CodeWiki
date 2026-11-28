---
tags: [core, cw-wiki]
---

# TestKind

> `TestKind` enum defined in `cw_wiki::testmap`.

## What it does

Says what sort of file a test sits in. `Unit` means the tests are inside a source file, usually in a `#[cfg(test)] mod tests`. `Integration` means the tests are in a file kept apart from the code: under a `tests/` directory, or in a test project. Each `TestFile` in the test map carries one.

## Source

- `crates/cw-wiki/src/testmap.rs` — definition of `TestKind`

## How it works

The language readers in [[Testmap Module]] set the kind while they scan a file. The kind then decides how the command that runs the tests is written, as in the JavaScript and Python readers. It is saved with the map, as the lowercase words `unit` and `integration`.

## Items

Declared as `pub enum TestKind`.

## Used by

- [[cw-wiki]] — from `maps_cpp_tests_and_the_command_that_runs_them`, `maps_csharp_tests_and_the_command_that_runs_them`, `maps_go_tests_and_the_command_that_runs_them`, `maps_tests_to_the_files_they_reach`
- [[Testmap Module]] — from `ImpactedTestFile`, `TestFile`, `command`
- [[Js Module]] — from `command`
- [[PythonTests]] — from `command`

## Gotchas

- The serialized form is lowercase. A change of variant names would change stored data.

## See also

- [[_index]]
- [[Testmap Module]]
