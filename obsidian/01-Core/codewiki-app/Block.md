---
tags: [core, codewiki-app]
---

# Block

> A part of a page.

## Source

- `crates/codewiki-app/src/doc.rs` — definition of `Block`

## Items

Declared as `pub enum Block`.

## Depends on

- [[Doc Module]] — uses `Choice`, `Line`, `Press`, `Row`
- [[Settings Module]] — uses `Note`
- [[Action]] — uses `Action`
- [[Tone]] — uses `Tone`
- [[FieldId]] — uses `FieldId`

## Used by

- [[Settings Pages Module]] — from `agents_card`, `appearance`, `backend`, `card`
- [[Doc]] — from `Doc`, `flat`, `push`, `walk`
- [[Doc View Module]] — from `draw`
- [[Pages Module]] — from `file_list`

## See also

- [[_index]]
- [[Doc Module]]
