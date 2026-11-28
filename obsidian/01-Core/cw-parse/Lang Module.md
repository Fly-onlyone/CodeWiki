---
tags: [core, cw-parse, module]
---

# Lang Module

> Plumbing shared by the Python, TypeScript, Java, Kotlin, C# and Go extractors: the fact sink, text helpers, the file-to-module mapping and import classification.

## What it does

This module holds what all the language extractors share: [[Python Module]], [[Typescript Module]], [[Java Module]], [[Kotlin Module]], [[Csharp Module]] and [[Go Module]]. That is the fact collector [[Sink]], small text helpers, the rules that map a file path to a module name, and the step that sorts imports into repository code, outside packages and a language's own library. Names use `::` between segments in every language, so one resolver serves all of them. Python's `pkg.util.shout` becomes `pkg::util::shout`.

## Source

- `crates/cw-parse/src/lang.rs` — module `cw_parse::lang`

## How it works

The helpers are small. `named` and `field_all` list tree-sitter children. `dotted` and `split_qualified` cut names into segments. `collapse` squeezes a declaration into one line, drops a trailing `{`, `:`, `;` or `=`, and cuts it to 300 characters. `block_doc` reads a doc comment that starts with `/**`. `dedent` strips shared indent. `marker_name` reads a decorator name after `@`.

Path rules: `python_module` starts the name at the outermost folder that has an `__init__.py`. `ts_module` is the path without the extension. `java_default_package` and `csharp_default_namespace` use the directory. `resolve_ts_import` follows a relative TypeScript import by trying the path, then each extension, then an `index` file.

`link_imports` runs once after all files are parsed. It builds the set of repository modules per language, then rewrites each file's imports:

- Python: repository modules first, then modules next to the script, then the standard library list (dropped), else an external package.
- Java and Kotlin: one shared package set. `java`, `javax`, `jdk`, `sun` (and `kotlin` for Kotlin) are dropped; the rest is external.
- C#: `global using` lines are spread over every namespace. Platform namespaces (`System`, a few `Microsoft.*`) are dropped. Other outside namespaces are matched to a NuGet id from project files (`nuget_ids`), else named by their root.
- Go: an import is a whole package. It is a repository package, dropped as standard library or generated code, or an outside module found from `go.mod` requires or a guess from the host (`GoBuild`).

An external import keeps one segment naming the package, and what it takes goes in the `external` field.

## Items

- `pub(crate) fn named(node: Node) -> Vec<Node>` — named children of `node`, collected so the tree can be walked while the extractor is mutated.
- `pub(crate) fn field_all<'t>(node: Node<'t>, name: &str) -> Vec<Node<'t>>` — every child stored under the field `name`.
- `pub(crate) fn split_qualified(qualified: &str) -> Vec<String>` — internal function, returns `Vec<String>`; called by `Extractor::export`, `Extractor::import_from`, `Extractor::qualify`, `Extractor::qualify_in` and 3 more.
- `pub(crate) fn dotted(text: &str) -> Vec<String>` — segments of a dotted name such as `pkg.util.shout`.
- `pub(crate) fn collapse(raw: &str) -> String` — declaration text with whitespace collapsed and the body opener removed.
- `pub(crate) fn dedent(text: &str) -> String` — remove the indentation shared by the lines after the first.
- `pub(crate) fn block_doc(comment: &str) -> Option<String>` — text of a `/** ... */` comment with the markers and leading stars removed. Returns `None` for any other comment.
- `pub(crate) fn sanitize(segment: &str) -> String` — make a path component usable as a name segment.
- `pub(crate) fn starts_uppercase(name: &str) -> bool` — internal function, returns `bool`; called by `Extractor::call_by_name`, `Extractor::invocation`, `Extractor::member_call`, `Extractor::walk`.
- `pub(crate) fn is_generic_param(name: &str) -> bool` — single uppercase letters are almost always generic parameters.
- `pub(crate) fn marker_name(text: &str) -> String` — name of a decorator or annotation: the text after `@` up to its arguments.
- [[Sink]] — collects the facts of one file.
- `pub fn python_module(files: &HashSet<&str>, path: &str) -> (String, bool)` — module a Python file defines, and whether the file is a package (`__init__.py`). The name starts at the outermost directory that has an `__init__.py`, so it matches what `import` statements write. A file outside any package is named by its path from the repository root, minus a leading `src` or `lib`.
- `pub fn ts_module(path: &str) -> String` — module a TypeScript file defines: its path without the extension.
- `pub fn resolve_ts_import(files: &HashSet<&str>, from: &str, spec: &str) -> Option<String>` — file a relative TypeScript import specifier names, as written in `from`. Tries the specifier itself, then each source extension, then an `index` file in a directory of that name. Non-relative specifiers return `None`.
- `pub(crate) fn ts_package(spec: &str) -> Option<String>` — package a non-relative TypeScript import specifier names, or `None` when it is a Node.js builtin or a path alias whose target is not known.
- `pub(crate) fn java_default_package(path: &str) -> String` — module used for a Java file with no `package` declaration: its directory.
- `pub(crate) fn csharp_default_namespace(path: &str) -> String` — module used for a C# file with no `namespace` declaration (the global namespace): its directory.
- `pub(crate) fn nuget_ids(xml: &str) -> Vec<String>` — the NuGet package ids a project or props file names, in a `PackageReference` or a `PackageVersion`.
- `pub(crate) fn link_imports(parsed: &mut [(String, ParsedFile)], nuget: &[String], go: &GoBuild)` — decide, for every Python, Java, Kotlin, C# and Go import, whether it names repository code, an outside package, or the language's own library. Imports of the last kind are dropped. Also records which Python package holds which module.

And 17 more items.

## Depends on

- [[Language]] — uses `Language`, `from_path`
- [[Jsbuild Module]] — uses `resolve_source`
- [[Rust Module]] — uses `ModDecl`
- [[GoModules]] — uses `GoModules`
- [[PythonRoots]] — uses `module`
- [[ParsedFile]] — uses `ParsedFile`
- [[UseImport]] — uses `UseImport`

## Used by

- [[Kotlin Module]] — from `annotation_name`, `before_lambda`, `call`, `call_by_name`
- [[Typescript Module]] — from `bindings`, `children`, `class`, `class_body`
- [[C Module]] — from `bases`, `block_test`, `comment_text`, `declaration_in`
- [[Csharp Module]] — from `base_list`, `declarators`, `doc`, `field`
- [[Go Module]] — from `assertion_target`, `collect_locals`, `function`, `imports`
- [[Java Module]] — from `children`, `doc`, `enum_constant`, `field`
- [[Python Module]] — from `bindings`, `block`, `class`, `collect_roots`
- [[Sink]] — from `inherits`, `type_ref`

## Gotchas

- The Python standard library list and the Node built-in list are fixed arrays. A module missing from them is treated as an outside package.
- A Java external package keeps at most two segments, so `org.junit.Test` belongs to `org.junit`.
- A TypeScript spec starting with `@/`, `~`, `#` or a path is treated as an alias and gives no package, because its target is unknown.
- A Python script outside any package also finds sibling modules, since its own folder is on the import path.
- `link_imports` also adds a module declaration for Python packages, so the resolver can find a module under its parent.
- Only `System` and four named `Microsoft.*` namespaces count as the .NET platform. Any other `Microsoft.*` namespace is a package. `Systematic` is not `System`.
- For Go, a path under a module of the repository that has no such package names nothing and is dropped.
- A C# file with no namespace is named by its directory, so two folders give two modules.

## See also

- [[_index]]
- [[cw-parse]]
