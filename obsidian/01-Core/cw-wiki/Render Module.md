---
tags: [core, cw-wiki, module]
---

# Render Module

> Rendering of planned notes into Markdown in the vault's house style.

## What it does

Turns each planned note into Markdown, with no model involved. Every sentence comes from structural facts (symbols, edges, manifests) and from doc comments in the code. The result is the factual skeleton of a note: source, items, dependencies. Agents later add the explanation sections on top. The note layout produced here is the contract the vault linter enforces. The module holds the small text helpers and the `Doc` builder; the main work is in [[RenderCtx]].

## Source

- `crates/cw-wiki/src/render.rs` — module `cw_wiki::render`

## How it works

`RenderCtx::render` makes a `Doc` from the note's front matter, title and one-line summary. It then picks a renderer by the note's subject: crate, module, type, crate flow, startup flow, integration, pattern, operations, or the file-level notes for source no parser maps (a unit or a folder). Each renderer adds named sections to the `Doc`. A section with an empty body is dropped. `Doc::finish` joins them through the vault's own note layout, so agent-written sections and rule-written ones share one format. Map notes (`_index`) and the home note are built by hand and skip `Doc`. The home note also links the top folders of notes their owner wrote, so none is cut off from the map.

The helpers are small. `join_and` writes lists with an Oxford comma. `count` pluralises. `zone_label` turns `01-Core` into `Core`. `language_name`, `unit_noun` and `external_noun` choose words such as crate or package. `build_role` names a build file. `lower_first` lowercases the first letter of a summary so it fits after a dash in a bullet. `cpp_override`, `csharp_override` and `kotlin_override` write the stub a type needs to take on a member of an interface, for pattern notes.

## Items

- `type NoteDeps = HashMap<String, BTreeMap<String, BTreeSet<String>>>` — Note-level dependency edges: note id -> other note id -> symbol names.
- `pub struct HandZone` — A top folder of the vault that holds notes their owner wrote by hand.
- [[RenderCtx]] — struct
- `struct Doc` — accumulates a note's sections; empty ones are dropped.
- `fn front(tags: &[String]) -> String` — private function, returns `String`; called by `Doc::new`, `RenderCtx::render_home`, `RenderCtx::render_index`.
- `fn zone_label(segment: &str) -> &str` — `01-Core` -> `Core`; other folder names are kept.
- `fn top_children<'a>(repo: &Repo<'a>, parent: &str, limit: usize) -> Vec<&'a Symbol>` — private function, returns `Vec<&'a Symbol>`; called by `RenderCtx::summary`.
- `pub(crate) fn join_and(items: &[String]) -> String` — internal function, returns `String`; called by `FileMap::home_summary`, `RenderCtx::render_crate`, `RenderCtx::render_crate_flow`, `RenderCtx::render_operations` and 5 more.
- `pub(crate) fn count(n: usize, noun: &str) -> String` — `1 module`, `3 modules`.
- `fn language_name(language: Language) -> &'static str` — private function, returns `&'static str`; called by `RenderCtx::composition`, `RenderCtx::unit_language`.
- `fn unit_noun(language: Language) -> &'static str` — what one unit of a language is called.
- `fn external_noun(files: &BTreeSet<&str>) -> &'static str` — `crate` when only Rust files import the dependency, else `package`.
- `fn implementation_files(symbol: &Symbol) -> Vec<&str>` — the files, besides the header a symbol stands in, that define what the header declares (C and C++).
- `fn cpp_override(signature: &str) -> String` — A member of a C++ interface as the override a class that takes it on writes: `virtual` goes, `override` comes, and a pure virtual's `= 0` goes.
- `fn csharp_override(signature: &str) -> String` — A C# member as a stub in a type that extends or implements it: an interface member has no modifier and becomes public, an abstract or virtual one is overridden.
- `fn kotlin_type_parameters(signature: &str, name: &str) -> Vec<String>` — the names of the type parameters a Kotlin declaration lists after its name: `T` and `R` for `interface Mapper<in T, out R : Any> : Base<T>`.
- `fn kotlin_override(signature: &str) -> String` — A Kotlin member as a stub in a type that extends or implements it: what let it be overridden is replaced by `override`.
- `fn build_role(path: &str) -> &'static str` — what a build file outside Cargo is for.
- `pub(crate) fn lower_first(text: &str) -> String` — lowercase the first letter unless the text starts with code or an acronym.
- `fn starts_with_language_unit(text: &str) -> bool` — true for the sentence of a unit that starts with the name of its language and that name is an ordinary word: `Go module `app` with 3 packages.` keeps its capital, a sentence that starts with "Go to" does not. (`C`, `C++` and `C#` are never lowered, whatever follows.).

And 6 more items.

## Depends on

- [[Repo]] — uses `Repo`, `children_of`, `subtree_rank`
- [[Language]] — uses `Language`, `from_path`
- [[C Module]] — uses `IMPL_ATTR`
- [[Note Module]] — uses `join`
- [[Plan Module]] — uses `Planned`
- [[Symbol]] — uses `Symbol`
- [[Section]] — uses `Section`

## Used by

- [[RenderCtx]] — from `RenderCtx`, `composition`, `dep_bullets`, `link`
- [[Filemap Module]] — from `file_lines`, `folder_bullet`, `folder_summary`, `holds`
- [[Explain Module]] — from `items`
- [[Generate Module]] — from `generate_with`
- [[Tally]] — from `names_up_to`

## Gotchas

- `lower_first` keeps the capital for code, acronyms, CamelCase words, one-letter words, the names in `PROPER_NOUNS`, and a sentence that starts with the name of a language unit (`Go module ...`). A new product name must be added to that list, or it will be lowered mid-list.
- `zone_label` only strips a two-digit prefix. `cw-core` stays as it is.
- `Doc::section` keeps empty sections until `finish`, which drops them. Do not rely on a heading being there.
- Hand-written notes listed in the home note are capped at 8 names per folder, and the rest are counted.

## See also

- [[_index]]
- [[cw-wiki]]
