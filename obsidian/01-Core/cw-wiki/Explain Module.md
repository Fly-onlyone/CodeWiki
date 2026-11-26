---
tags: [core, cw-wiki, module]
---

# Explain Module

> What a note says beyond its subject's own comment: the walk-through of what happens when the code is used, and the list of its items.

## What it does

Rules write each note from facts. Two parts of a note need more than a doc comment. One is a short numbered walk-through of what happens when the code is used. The other is the list of items with one bullet each. This module builds both from the call graph and the symbol table, with no guessing. It cannot know conditions, loops or reasons. A coding agent adds those later.

## Source

- `crates/cw-wiki/src/explain.rs` — module `cw_wiki::explain`

## How it works

`walk` picks the entry points of one note: its functions and methods that a program starts at, or that other notes call. Test code and low-confidence calls are ignored. Entry points are sorted: program starts first, then those called from the most notes, then source order. At most 5 become steps; the rest are named as "more entry points". If nothing calls in, the note's own non-private functions that call something become the steps and the text says so.

Each step lists up to 6 calls in source order. Up to 2 callees that belong to the same note are described too, each only once in the whole walk. `Walk::text` turns this into numbered sentences with links to other notes, and adds a line that conditions and loops are not shown.

`items` lists the children of a note's subject, one bullet each. Public items come first, then non-private, then private ones that `walk` names. Source order breaks ties; at most 20 are shown and the rest are counted. A bullet starts with the signature (or the name if it is over 100 characters or contains a backtick). With a doc comment, the bullet holds `first_sentence`/`item_text` of it. Without one, `facts` states: visibility, kind, return type and up to 4 users.

## Items

- `const MAX_STEPS: usize` — steps of one walk-through.
- `const MAX_CALLS: usize` — calls named in one step.
- `const MAX_CALLERS: usize` — callers named for one entry point.
- `const MAX_HELPERS: usize` — functions of the note itself that a step goes on to describe.
- `const MAX_MORE: usize` — entry points named after the steps.
- `const MAX_ITEMS: usize` — items listed in a note.
- `const FEW_ITEMS: usize` — items listed when the note has almost none that are not private.
- `const MAX_USERS: usize` — names given as the users of an item that has no comment.
- `const MAX_SIGNATURE: usize` — A longer signature is replaced by the item's name.
- `type Callers<'a> = Vec<(&'a Symbol, &'a str)>` — the callers of a function from other notes, each with the id of its note.
- `pub(crate) struct Step<'a>` — one step of a walk-through: a function of the note, who calls it from outside, and what it calls.
- `pub(crate) struct Walk<'a>` — what happens when the code of one note is used.
- `pub(crate) fn short_name(symbol: &Symbol) -> String` — how a reader names `symbol`: a method with its type.
- `fn name_in(symbol: &Symbol, subject: &str) -> String` — `short_name` in the note about `subject`: a method of the subject itself goes by its own name there.
- `fn in_order(items: &[(String, Option<String>)], more: usize) -> String` — `items` as `a, then b, then c`, with `and N more` for the rest. Each item is a name and, when it lives in another note, the link to that note. Neighbours in one note share its link: `a, then b (both in [[N]])`.
- `pub(crate) fn walk<'a>(ctx: &RenderCtx<'a>, note_id: &str) -> Walk<'a>` — walk through the note with `note_id`: its functions that other notes call, each with what it calls in turn.
- `fn return_type(signature: &str, language: Language) -> Option<String>` — the type a function returns, as its signature writes it, or `None` when the signature names none or cannot be read with confidence.
- `fn label(symbol: &Symbol) -> String` — the code span an item's bullet starts with: its signature, or its name when the signature is too long to read in a list.
- `fn facts(ctx: &RenderCtx, symbol: &Symbol) -> String` — what the code alone says of an item that has no comment: how visible it is, what kind of thing, what it returns, and who uses it.
- `pub(crate) fn items(ctx: &RenderCtx, parent: &str, walk: &Walk) -> String` — the items directly inside `parent`, one bullet each, in source order: every item that is not private, and the private ones `walk` names. An item with a comment gives the whole comment; one without gives what the code says of it. Past `MAX_ITEMS` the rest is counted in a last line.

And 1 more private item with no comment.

## Depends on

- [[Repo]] — uses `calls_from`, `children_of`, `is_test`, `subtree_rank`
- [[Doctext Module]] — uses `first_sentence`, `item_text`
- [[Language]] — uses `Language`, `from_path`
- [[RenderCtx]] — uses `RenderCtx`, `link`
- [[Lint Module]] — uses `is_line_ref`
- [[Render Module]] — uses `lower_first`
- [[Symbol]] — uses `Symbol`

## Used by

- [[RenderCtx]] — from `render_crate`, `render_module`, `render_type`

## Gotchas

- No resolved incoming call does not mean unused. A call through a value of unknown type is not followed.
- `return_type` returns nothing for unit results, types over 60 characters, and languages other than Rust, Python, TypeScript and JavaScript.
- A signature that looks like a file and line (checked by `is_line_ref`) is replaced by the name, because the linter would refuse it.
- Constants cut their signature at the equals sign so the value is not printed.

## See also

- [[_index]]
- [[cw-wiki]]
