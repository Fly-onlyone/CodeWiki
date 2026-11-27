---
tags: [core, cw-wiki, module]
---

# Units Module

> Units: the top-level parts of a repository, each with its own folder under `01-Core`.

## What it does

Finds the top-level parts of a repository, called units. Each unit gets its own folder under `01-Core` in the vault. A Cargo package is a unit. For Python, JavaScript, TypeScript and Java the scanner has no manifest it understands, so this module derives units from the layout and the build files. Every unit has the same shape ([[Unit]]), so the planner and the renderer treat Rust and other languages alike.

## Source

- `crates/cw-wiki/src/units.rs` — module `cw_wiki::units`

## How it works

`discover` builds a [[Units]] value in three steps.

1. Each crate from the scan becomes a Rust unit. Its entry is `main` when the crate has a binary.
2. Every non-Rust source file is put into a group by language family. Python files group by their top-level package, and a lone top-level module goes into a shared group. JavaScript and TypeScript files group by the nearest folder up the tree that holds a `package.json`, or by their top folder if there is none. Java files group by the nearest folder with `pom.xml` or a Gradle file, or by the first part of their package name.
3. Each group becomes a unit if it holds at least one real symbol outside test files. The unit's root module is the common prefix of its module names. Its manifests are found next to the group. A `package.json` supplies the name, description, scripts and entries: `main`, `bin` and the file named in the `start` script. `resolve_js_entry` maps a built path such as `dist/x.js` back to a source file under `src`.

`is_test_file` recognises non-Rust tests by folder names (`tests`, `__tests__`, `spec`, `e2e`) and by file patterns such as `test_*.py`, `*.test.ts` and `*Test.java`. `slug` makes a name safe as a folder and a tag.

## Items

- `const CSHARP_TEST_ATTRIBUTES: [&str; 11]` — attributes that make a C# method a test, under any test framework.
- `pub struct Manifest` — A build or dependency file that belongs to a unit.
- `pub struct Entry` — A symbol where execution starts.
- [[Unit]] — one top-level part of the repository.
- [[Units]] — every unit of a repository and the lookup from code to unit.
- `enum Family` — language family: one unit never mixes two of these.
- `pub fn is_test_file(path: &str) -> bool` — true for a test file of a language other than Rust.
- `fn is_c_test_name(file: &str, stem: &str) -> bool` — A C or C++ file named like a test: `list_test.c`, `test_list.cpp`, `list_tests.cc`, `ListTest.cpp`, `ListTests.cpp`. A file in a `test` or `tests` folder is caught by its folder.
- `pub(crate) fn links_a_test_framework(links: &[String]) -> bool` — true when a target links a test framework: `GTest::gtest_main`, `Catch2::Catch2WithMain`, `doctest::doctest`.
- `fn in_jvm_test_set(dirs: &[&str]) -> bool` — true for a folder of a Gradle source set of tests: `src/test`, and the ones that name their target, such as Kotlin Multiplatform's `src/commonTest`, `src/jvmTest` and Android's `src/androidTest`.
- `pub(crate) fn is_jvm_test_file(path: &str) -> bool` — true for a Java or Kotlin file of test code, the source sets of tests included. `is_test_file` leaves the sets of a Java file alone, since a Java repository's notes must not change; the test map looks further.
- `pub(crate) fn is_kotlin_entry(symbol: &Symbol) -> bool` — A Kotlin program's `main`: a function at the top of a file, or an `@JvmStatic` one of an `object` or a companion, taking no parameter or the arguments (`Array<String>`, or `vararg String`).
- `pub(crate) fn is_csharp_test_attribute(attribute: &str) -> bool` — true when `attribute` (as written: `Fact`, `Xunit.Fact`, `FactAttribute`) marks a C# test.
- `pub(crate) fn xml_element<'a>(xml: &'a str, tag: &str) -> Option<&'a str>` — text of the first `<tag>` element of `xml`, trimmed.
- `pub(crate) fn xml_elements<'a>(xml: &'a str, tag: &str) -> Vec<(&'a str, &'a str)>` — every element named `tag` in `xml` with its attribute text and, for one that is not self-closing, its inner text.
- `pub(crate) fn xml_attribute(attributes: &str, name: &str) -> Option<String>` — the value of attribute `name` in the attribute text of an element.
- `pub(crate) fn resolve_relative(dir: &str, target: &str) -> Option<String>` — the repo-relative path that the relative `target` names from `dir`, with its `..` and `.` segments resolved and either slash accepted; `None` when it climbs out of the repository.
- `pub(crate) fn slug(name: &str) -> String` — lowercase name safe as a folder and as a tag.
- `pub(crate) fn references_a_test_framework(project: &str) -> bool` — true when a project file references a test framework or says it is a test project.
- `discover` — find the units of a repository. `repo_name` names a unit that is the repository itself; `read` returns the text of a repo-relative file.

And 22 more items.

## Depends on

- [[Language]] — uses `Language`, `from_path`, `is_c_family`
- [[JsRepo]] — uses `manager_of`, `package_at`, `read`
- [[PythonRoots]] — uses `in_package`, `project_of`, `read`
- [[C Module]] — uses `ENTRY_NAMES`, `TEST_ATTR`
- [[Cfamily Module]] — uses `c_module_map`, `is_header`
- [[Gomod Module]] — uses `parse_go_mod`, `parse_go_work`
- [[Cmake]] — uses `Cmake`, `target_of`
- [[Cmake Module]] — uses `read_cmake`

## Used by

- [[Manifests Module]] — from `asked_in`, `central_version`, `js_lock`, `js_locked`
- [[Testmap Module]] — from `add_go_package_dependents`, `command`, `gather`, `is_csharp_test`
- [[Repo]] — from `at`, `is_test`, `new`
- [[Health Module]] — from `dead_code`, `is_test_symbol`
- [[RenderCtx]] — from `new`, `unit_commands`
- [[Filemap Module]] — from `build`
- [[Machine]] — from `nuget_description`
- [[Unit]] — from `Unit`

## Gotchas

- A unit never mixes language families. A repository with Python and TypeScript gets separate units.
- Configuration files or test suites alone do not make a unit.
- Unit names and folder slugs are kept unique separately, because a unit's key may equal its folder name.
- Manifests are read through a callback. When it returns nothing (as with `Repo::new`), the unit is still found but its manifest text is empty.

## See also

- [[_index]]
- [[cw-wiki]]
