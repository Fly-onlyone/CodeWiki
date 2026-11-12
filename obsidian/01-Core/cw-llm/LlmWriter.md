---
tags: [core, cw-llm]
---

# LlmWriter

> `LlmWriter` struct defined in `cw_llm::writer` with 6 methods.

## Source

- `crates/cw-llm/src/writer.rs` — definition of `LlmWriter`

## How it works

Declared as `pub struct LlmWriter`.

Public API:

- `pub fn new(provider: Box<dyn LlmProvider>) -> Self`
- `fn name(&self) -> String`
- `fn rewrite(&self, note: &Planned, draft: &str, context: &WriterContext) -> Option<String>`
- `fn illustrate( &self, note: &Planned, text: &str, structural: Option<&str>, _context: &WriterContext, ) -> Option<String>`
- `fn usage(&self) -> WriterUsage`

## Depends on

- [[Writer Module]] — uses `PROMPT_VERSION`, `SYSTEM`, `extract_note`, `kind_hint`
- [[Illustrate Module]] — uses `draw`
- [[Generate Module]] — uses `WriterContext`
- [[LlmProvider]] — uses `LlmProvider`
- [[ProseWriter]] — uses `ProseWriter`
- [[WriterUsage]] — uses `WriterUsage`
- [[Planned]] — uses `Planned`

## Used by

- [[cw-llm]] — from `writer_from_env`

## See also

- [[_index]]
- [[Writer Module]]
