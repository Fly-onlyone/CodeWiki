---
tags: [core, cw-wiki, module]
---

# Explain Module

> What a note says beyond its subject's own comment: the walk-through of what happens when the code is used, and the list of its items.

## What it does

Both are built from facts alone. The walk-through follows resolved calls, in the order they stand in the source, and so says nothing it cannot know: no condition, no loop, no reason. A writer that reads the bodies adds those.

## Source

- `crates/cw-wiki/src/explain.rs` — module `cw_wiki::explain`

## How it works

1. `RenderCtx::illustration` and `RenderCtx::render_crate` in [[RenderCtx]], and 2 more, call `walk`.
2. `RenderCtx::render_crate` and `RenderCtx::render_module` in [[RenderCtx]], and 1 more, call `items`. `items` calls `first_sentence` in [[Doctext Module]], then `lower_first` in [[Render Module]], then `item_text` in [[Doctext Module]], then `Language::from_path` in [[Language]], then `label`, then `facts`. `label` calls `is_line_ref` in [[Lint Module]]. `facts` calls `return_type`, then `Language::from_path` in [[Language]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

And 2 more items.

## Depends on

- [[Repo]] — uses `calls_from`, `children_of`, `is_test`, `subtree_rank`
- [[Doctext Module]] — uses `first_sentence`, `item_text`
- [[Language]] — uses `Language`, `from_path`
- [[RenderCtx]] — uses `RenderCtx`, `link`
- [[Lint Module]] — uses `is_line_ref`
- [[Render Module]] — uses `lower_first`
- [[Symbol]] — uses `Symbol`

## Used by

- [[RenderCtx]] — from `illustration`, `render_crate`, `render_module`, `render_type`

## See also

- [[_index]]
- [[cw-wiki]]
