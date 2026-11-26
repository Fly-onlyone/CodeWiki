---
tags: [core, cw-parse, module]
---

# Python Module

> Python symbol and fact extraction.

## What it does

This module reads one Python file and turns it into a [[ParsedFile]]: the module, its classes, functions and methods, and which names each one calls, uses, imports or extends. It is the Python front end of the scanner and works on the tree-sitter syntax tree, using the node names in the grammar's `node-types.json`. No query files are used.

## Source

- `crates/cw-parse/src/python.rs` — module `cw_parse::python`

## How it works

`parse_python` gets the file path, the module name (such as `pkg::util`), whether the file is a package (`__init__.py`) and the text. It first collects the "roots": names bound by imports and top-level definitions. Only a dotted call that starts at a root is treated as a path. It then records the module symbol with its docstring and finds the names bound in the file with `bindings`.

It then walks the tree. A `def` becomes a function, or a method when it sits inside a class. A `class` becomes a class, and its plain base names are recorded as inheritance. Decorators become attributes. For a method, the first parameter (usually `self`) is remembered, so `self.name()` is a call on the enclosing class.

`call` sorts calls into three cases. A bare name is a plain call. A dotted call whose start is a root is a path. Anything else is a method call on a receiver of unknown type. Imports are stored as written, with relative ones made absolute. [[Scan Module]] and [[Lang Module]] sort out later which of them name repository code.

## Items

- `parse_python` — parse one Python source file. `module` is the qualified module the file defines, for example `pkg::util`; `is_package` is true for `__init__.py`, which changes what a relative import is relative to. Imports are recorded as written (relative ones made absolute); the scanner decides afterwards which of them name repository modules.
- `pub const MAIN_GUARD_ATTR: &str` — attribute recorded on the module symbol of a file that runs as a script: it has a top-level `if __name__ == "__main__":` block.
- `fn has_main_guard(root: Node, src: &[u8]) -> bool` — true when the file has a top-level `if __name__ == "__main__":`.
- `fn text<'a>(node: Node, src: &'a [u8]) -> &'a str` — private function, returns `&'a str`; called by `has_main_guard`, `pattern_names`.
- `fn pattern_names(node: Node, src: &[u8], out: &mut HashSet<String>)` — names bound by an assignment target, loop variable or `as` clause.
- `fn bindings(node: Node, src: &[u8], out: &mut HashSet<String>)` — names bound anywhere in a scope, not descending into nested scopes.

And 4 more private items with no comment.

## Depends on

- [[Lang Module]] — uses `collapse`, `dedent`, `dotted`, `field_all`
- [[Sink]] — uses `Sink`, `between`, `inherits`, `module_symbol`
- [[Error]] — uses `external`, `invalid`
- [[Error Module]] — uses `Result`
- [[Visibility]] — uses `Visibility`
- [[ParsedFile]] — uses `ParsedFile`

## Used by

- [[Scan Module]] — from `scan_repo`
- [[Units Module]] — from `discover`

## Gotchas

- A name bound locally (assignment, loop variable, `as`, parameter) hides a module-level symbol of the same name. Uses of it are not recorded as references.
- A top-level `if __name__ == "__main__":` adds the attribute `__main__` to the module symbol. Other code uses it to tell scripts from libraries.
- A name starting with one underscore is private, as is anything nested inside a function. Names like `__init__` are public.
- Only plain names count as base classes. `metaclass=Meta` and `Generic[T]` give no inheritance.
- `import a.b.c` records two imports: the name `a` it binds, and the path `a.b.c` it loads.
- A name rebound at module level, like `handler = wrap(handler)`, stays a function.

## See also

- [[_index]]
- [[cw-parse]]
