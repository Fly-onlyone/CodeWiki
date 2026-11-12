---
tags: [core, cw-wiki]
---

# ProseWriter

> Replaces the structural prose of a note, typically with an LLM.

## Source

- `crates/cw-wiki/src/generate.rs` — definition of `ProseWriter`

## How it works

Declared as `pub trait ProseWriter`.

Returning `None` keeps the structural draft.

Required and provided methods:

- `fn illustrate( &self, _note: &Planned, _text: &str, _structural: Option<&str>, _context: &WriterContext, ) -> Option<String>` — an illustration for the finished note, as OpenUI Lang source, or `None` to keep the structural one.
- `fn usage(&self) -> WriterUsage` — calls and tokens spent so far; reported after each run.
- `fn name(&self) -> String` — identifier recorded in the manifest, for example a model name.
- `fn rewrite(&self, note: &Planned, draft: &str, context: &WriterContext) -> Option<String>`

Implemented by [[LlmWriter]].

## Depends on

- [[Generate Module]] — uses `WriterContext`
- [[WriterUsage]] — uses `WriterUsage`
- [[Planned]] — uses `Planned`

## Used by

- [[Service Module]] — from `update`, `update_in`
- [[codewiki]] — from `run`
- [[Api Module]] — from `run_update`
- [[Generate Module]] — from `generate`
- [[LlmWriter]] — from `LlmWriter`

## See also

- [[_index]]
- [[Generate Module]]
