---
tags: [core, cw-wiki]
---

# ProseWriter

> Replaces the structural prose of a note, typically with an LLM.

## What it does

Anything but `Rewrite::Text` keeps the structural draft.

## Source

- `crates/cw-wiki/src/generate.rs` — definition of `ProseWriter`

## Items

Declared as `pub trait ProseWriter`.

Required and provided methods:

- `fn name(&self) -> String` — identifier recorded in the manifest, for example a model name.
- `fn rewrite(&self, note: &Planned, draft: &str, context: &WriterContext) -> Rewrite` — public method, returns `Rewrite`.
- `illustrate` — an illustration for the finished note, as OpenUI Lang source, or `None` to keep the structural one. `structural` is the illustration built from facts alone, when the note has one. The result is checked against the component library before it is saved.
- `fn usage(&self) -> WriterUsage` — calls and tokens spent so far; reported after each run.

Implemented by [[LlmWriter]], `Draws`, and `Scripted`.

## Depends on

- [[Generate Module]] — uses `Rewrite`, `WriterContext`, `WriterUsage`
- [[Planned]] — uses `Planned`

## Used by

- [[Service Module]] — from `run_update`, `update`, `update_in`
- [[cw-wiki]] — from `Draws`, `Scripted`
- [[Generate Module]] — from `generate`, `generate_with`
- [[codewiki]] — from `run`
- [[Api Module]] — from `run_update`
- [[LlmWriter]] — from `LlmWriter`

## See also

- [[_index]]
- [[Generate Module]]
- [[Implementing ProseWriter Pattern]]
