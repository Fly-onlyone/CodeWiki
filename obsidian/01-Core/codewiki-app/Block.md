---
tags: [core, codewiki-app]
---

# Block

> A part of a page.

## Source

- `crates/codewiki-app/src/doc.rs` — definition of `Block`

## How it works

Declared as `pub enum Block`.

## Depends on

- [[Doc Module]] — uses `Choice`, `Line`, `Press`, `Row`
- [[Settings Module]] — uses `Note`
- [[Action]] — uses `Action`
- [[Tone]] — uses `Tone`
- [[FieldId]] — uses `FieldId`

## Used by

- [[Settings Pages Module]] — from `ai_model`, `appearance`, `backend`, `built_in`
- [[Doc]] — from `Doc`, `flat`, `push`, `walk`
- [[Doc View Module]] — from `draw`
- [[Pages Module]] — from `file_list`

## See also

- [[_index]]
- [[Doc Module]]
