---
tags: [core, cw-core]
---

# Language

> Source language of a file.

## What it does

Names the source language of an indexed file. It decides which parser runs, how a file is described in notes, and which health checks apply. Rust, Python, TypeScript, JavaScript, Java, C#, Go, Kotlin, C and C++ get symbols and edges. Markdown, TOML, JSON and YAML are recorded as files only. Everything else is `Other`, and some `Other` files still get a language name for display (`file_level_name`).

## Source

- `crates/cw-core/src/model.rs` — definition of `Language`

## How it works

`from_path` looks only at the extension, lower-cased: for example `rs`, `py` and `pyi`, `ts` `tsx` `mts` `cts`, `js` `jsx` `mjs` `cjs`, `java`, `cs`, `go`, `kt`, `c` and `h`, the C++ extensions, `md`, `toml`, `json`, `yml` and `yaml`. No extension, or an unknown one, gives `Other`. A `.gradle.kts` file is a build script, so it is `Other`, as is a Maven `pom.xml`. A `.h` file is C here; the scan tells C++ headers apart because it knows the other files.

`as_str` and `parse` convert to and from the lowercase text stored in the index, where [[Index]] calls `parse`. `has_symbol_support` is true for the ten code languages; callers use it to decide whether symbol-level work makes sense. `is_c_family` groups C and C++, which share headers and names.

`file_level_name` gives a display name such as Swift, Ruby, SQL or Terraform for a source file with no parser. It returns `None` for any language with a parser, for data and prose, and for unknown extensions. Such a file is mapped by folder, language and size only. A language that gains a parser leaves this table by getting its own `Language` variant.

## Items

Declared as `pub enum Language`.

Members:

- `pub fn from_path(path: &str) -> Self` — detect the language from a repo-relative path.
- `pub fn as_str(self) -> &'static str` — public method, returns `&'static str`; called by `Language::file_level_name`, `Language::from_path`.
- `pub fn parse(s: &str) -> Self` — public method, returns `Self`; called by `Index::files`.
- `pub fn has_symbol_support(self) -> bool` — true when symbols and edges are extracted for this language.
- `pub fn is_c_family(self) -> bool` — C and C++ are one family: they share headers, modules and names, and a `.h` file may be either.
- `pub fn file_level_name(path: &str) -> Option<&'static str>` — display name of the language of a source file that CodeWiki has no parser for, read from the extension alone: `C#` for `Program.cs`. Such a file is mapped at file level: its folder, language and size are known, its symbols are not. `None` for a file of a language with symbol support (a parser maps it), for data and prose (Markdown, JSON, YAML, TOML, text, lock files, images) and for anything else. A language that gets a parser leaves this table by giving its extensions a `Language` variant that has symbol support. C and C++ have parsers and are not in the table; `.m` and `.mm` are Objective-C and stay file-level.

## Used by

- [[RenderCtx]] — from `composition`, `entry_label`, `new`, `render_crate`
- [[Resolver]] — from `by_bare_name`, `lookup_c`, `lookup_from`, `new`
- [[Explain Module]] — from `facts`, `is_rust`, `items`, `name_in`
- [[Testmap Module]] — from `covers`, `gather`, `impacted_tests`, `language`
- [[Health Module]] — from `dead_code`, `doc_drift`, `file_health`, `is_c_api`
- [[Doctext Module]] — from `blocks`, `doc_body`, `item_text`
- [[Manifests Module]] — from `describe`, `describe_in`, `other`
- [[Render Module]] — from `external_noun`, `language_name`, `unit_noun`

## Gotchas

- Detection ignores content and file names. A `Makefile` or a shebang script is `Other`.
- `parse` maps any unknown text to `Other` instead of failing. A language added to the enum but not to `parse` silently reads back as `Other`.
- Adding a variant means updating `from_path`, `as_str`, `parse` and `has_symbol_support` together.
- Do not list a language in `file_level_name` once it has a variant with symbol support: the function returns `None` for those anyway, and the table would mislead.

## See also

- [[_index]]
- [[Model Module]]
