---
description: Which languages CodeWiki reads in depth, and how it maps the rest.
icon: code
---

# Languages

Every repository gets a wiki. How much the wiki knows about the code depends on the language.

## Read in depth

CodeWiki reads the **symbols** of ten languages: Rust, Python, TypeScript, JavaScript, Java, C#, Go, Kotlin, C and C++. For these it knows the functions, types and constants, what calls what, which files depend on which, where the program starts, and which tests exist.

That feeds the notes, the [graph](../the-app/graph.md), the reports and the [MCP tools](../agents/mcp-tools.md).

| Language               | What CodeWiki understands                                                                                                                                                                                                        |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust                   | Crates, modules and symbols.                                                                                                                                                                                                     |
| Python                 | A module is named by where its import starts. A folder of `.py` files counts whether or not it has an `__init__.py`, below a root such as the folder of `pyproject.toml`, `setup.py` or `setup.cfg`, or a `src` folder under it. |
| TypeScript, JavaScript | Imports resolve the way the project's own tooling resolves them: `paths` and `baseUrl` of `tsconfig.json` or `jsconfig.json`, and workspaces of `pnpm`, `yarn`, `bun` and `npm`. Bundler aliases are not read.                   |
| Java and Kotlin        | They share packages. A Gradle or Maven module with both languages is one unit.                                                                                                                                                   |
| C#                     | A project, found from its `.csproj`, is a unit. Namespaces are modules. Partial types are one type. Avalonia `.axaml` files stay mapped at file level.                                                                           |
| Go                     | A module, found from its `go.mod`, is a unit. Every package below it is a module of the unit.                                                                                                                                    |
| C and C++              | They are read together. A header and the source of its name are one module. A CMake target is a unit; without CMake, a unit is a folder. Other build systems are not read.                                                       |

## Mapped at file level

Source in any other language is mapped at file level. The language is named from the file extension. Examples are Swift, Ruby, PHP, shell, SQL, HTML, CSS and XAML.

For such source, CodeWiki writes a note for each project folder and for each folder that holds three or more of its files. A note lists the files, their language and their size in lines. It has no symbols, no imports and no call links.

Your coding agent can explain these notes from the files themselves. See [Agent-written notes](../agents/writing-notes.md).

The Wiki map then says, for example, that the repository is a Swift codebase mapped at file level, and that CodeWiki has no parser for Swift yet. Files that are not source code are indexed by path.

## What works for which language

| Feature                                      | Languages                                                                      |
| -------------------------------------------- | ------------------------------------------------------------------------------ |
| Notes, graph, dependencies, search           | The ten languages above. Other languages: file-level notes and search by path. |
| Impacted tests, with the command to run them | The ten languages above.                                                       |
| Health, dead code                            | The languages with symbols.                                                    |
| Security findings and API contracts          | Rust only.                                                                     |

## Impacted tests

After a change, the MCP tool `get_impacted_tests`, `codewiki impacted-tests` and the Tests page name the tests to run, nearest first, with the command for each language:

| Language               | Command                                                                                                                                      |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust                   | `cargo test`.                                                                                                                                |
| C#                     | `dotnet test`. It names the test classes when the project runs under VSTest.                                                                 |
| Go                     | `go test`, with the tests named, or the whole package when more than eight tests are impacted.                                               |
| Java and Kotlin        | A Gradle command for the module, or a Maven command that names the test classes. The whole module when more than eight classes are impacted. |
| C and C++              | `ctest`, or the test executable with its framework's filter.                                                                                 |
| Python                 | `pytest` with node ids, or `unittest`. Behind `uv run`, `poetry run`, `pdm run` or `hatch run` when the project's files say so.              |
| TypeScript, JavaScript | The package's own runner (`vitest`, `jest`, `mocha`, `playwright`, `ava`, the Node test runner or the Bun one) through its package manager.  |

The tests are told from the sources. A C# test is marked `[Fact]`, `[Theory]`, `[Test]` or `[TestMethod]`. A Go test is a `TestXxx`, `BenchmarkXxx`, `FuzzXxx` or `ExampleXxx` function in a `_test.go` file. C and C++ tests are GoogleTest, Catch2, doctest or Boost.Test.
