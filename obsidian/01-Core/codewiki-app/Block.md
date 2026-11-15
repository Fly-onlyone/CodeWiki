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

- [[Doc Module]] — uses `Choice`, `Line`, `Row`, `Stat`
- [[Action]] — uses `Action`
- [[Tone]] — uses `Tone`

## Used by

- [[Doc]] — from `Doc`, `flat`, `push`, `walk`
- [[Pages Module]] — from `file_list`, `updates_card`
- [[Doc View Module]] — from `draw`

## See also

- [[_index]]
- [[Doc Module]]
