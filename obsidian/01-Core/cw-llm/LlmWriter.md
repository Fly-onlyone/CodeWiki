---
tags: [core, cw-llm]
---

# LlmWriter

> `LlmWriter` struct defined in `cw_llm::writer` with 6 methods.

## Source

- `crates/cw-llm/src/writer.rs` — definition of `LlmWriter`

## How it works

What the functions of this type call:

1. `rewrite` calls `kind_hint`, then `guidance`, then `read_sources` (all in [[Writer Module]]), then `record`, then `extract_note` in [[Writer Module]].
2. `illustrate` calls `draw` in [[Illustrate Module]], then `record`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

Declared as `pub struct LlmWriter`.

Members:

- `pub fn new(provider: Box<dyn LlmProvider>) -> Self` — public method, returns `Self`; called by `writer_from`.
- `fn record(&self, update: impl FnOnce(&mut WriterUsage))` — private method; called by `LlmWriter::illustrate`, `LlmWriter::rewrite`.
- `fn name(&self) -> String` — public method, returns `String`.
- `fn rewrite(&self, note: &Planned, draft: &str, context: &WriterContext) -> Rewrite` — public method, returns `Rewrite`.
- `illustrate` — public method, returns `Option<String>`.
- `fn usage(&self) -> WriterUsage` — public method, returns `WriterUsage`.

## Depends on

- [[Writer Module]] — uses `PROMPT_VERSION`, `SYSTEM`, `extract_note`, `guidance`
- [[Generate Module]] — uses `Rewrite`, `Stopped`, `WriterContext`, `WriterUsage`
- [[Illustrate Module]] — uses `draw`
- [[LlmProvider]] — uses `LlmProvider`
- [[ProseWriter]] — uses `ProseWriter`
- [[Planned]] — uses `Planned`

## Used by

- [[cw-llm]] — from `writer_from`

## See also

- [[_index]]
- [[Writer Module]]
