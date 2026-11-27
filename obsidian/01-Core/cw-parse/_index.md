---
tags: [moc, core, cw-parse]
---

# Core / cw-parse — Map of Content

> Notes for `cw-parse`: 32 concepts.

## Notes

- [[C Module]] — C and C++ symbol and fact extraction, one walker for both languages.
- [[Cdeps Module]] — the files a C or C++ project names its libraries in, other than `CMakeLists.txt` (see `cmake.rs`): `vcpkg.json`, `conanfile.txt`, `conanfile.py` and `conan.lock`.
- [[Cfamily Module]] — what the scan decides about C and C++ before and after it parses a file: which language a `.h` is, which header and source form one module, what an `#include` names, and how the parts of a declaration become one symbol.
- [[Cmake]] — `Cmake` struct defined in `cw_parse::cmake` with 3 methods.
- [[Cmake Module]] — A reader of `CMakeLists.txt`, for what a repository's units need: the targets and the files each lists, the include directories, the links between targets, the libraries the project asks for and the tests it registers with CTest.
- [[CPlan]] — what the scan knows about the C and C++ files of one repository.
- [[CrateInfo]] — one Cargo package found in the repository.
- [[Csharp Module]] — C# symbol and fact extraction.
- [[cw-parse]] — source parsing and symbol extraction for CodeWiki.
- [[Go Module]] — go symbol and fact extraction.
- [[Gomod Module]] — what Go's build files say: `go.mod`, `go.work` and `go.sum`, read as text.
- [[GoModules]] — the Go modules of a repository: the folder of each `go.mod` and the module path it declares.
- [[Gradle Module]] — Gradle's version catalog, `gradle/libs.versions.toml`, read as TOML.
- [[Java Module]] — Java symbol and fact extraction.
- [[Jsbuild Module]] — what the build files of a JavaScript or TypeScript repository say, read as data and never run: `tsconfig.json` and `jsconfig.json` (`baseUrl`, `paths`, `extends`), `package.json` (name, entry points, workspaces, the package manager), `pnpm-workspace.yaml`, and the lock files of npm, pnpm, Yarn and Bun.
- [[JsPackage]] — A package of the repository: a `package.json` with a name.
- [[JsRepo]] — everything a repository's JavaScript build files say, read once.
- [[Kotlin Module]] — Kotlin symbol and fact extraction.
- [[Lang Module]] — plumbing shared by the Python, TypeScript, Java, Kotlin, C# and Go extractors: the fact sink, text helpers, the file-to-module mapping and import classification.
- [[Manager]] — `Manager` enum defined in `cw_parse::jsbuild` with 1 methods.
- [[ParsedFile]] — `ParsedFile` struct defined in `cw_parse::rust`.
- [[Pyroots Module]] — where Python's import names start: the folders an absolute import is taken from, read from the build files (`pyproject.toml`, `setup.cfg`, `setup.py`) and never run.
- [[Python Module]] — Python symbol and fact extraction.
- [[PythonRoots]] — the Python projects of a repository and their import roots.
- [[Resolve Module]] — name resolution without a compiler.
- [[Resolver]] — `Resolver` struct defined in `cw_parse::resolve` with 17 methods.
- [[Rust Module]] — Rust symbol and fact extraction.
- [[Scan Module]] — walk a repository and produce every fact the index needs.
- [[Sink]] — collects the facts of one file.
- [[Typescript Module]] — TypeScript and TSX symbol and fact extraction.
- [[UseImport]] — one name brought into scope by a `use` declaration.
- [[Workspace Module]] — Cargo package discovery and the file-to-module mapping.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
