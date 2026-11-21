---
tags: [core, codewiki-app]
---

# Glyph

> An icon of the app: the path its SVG is served under.

## Source

- `crates/codewiki-app/src/icons.rs` — definition of `Glyph`

## Items

Declared as `pub struct Glyph`.

Members:

- `fn path(self) -> SharedString` — public method, returns `SharedString`.

## Used by

- [[Icons Module]] — from `for_command`, `for_theme`, `hue`, `tint`
- [[Doc Module]] — from `Choice`, `Row`, `new`
- [[Button Module]] — from `IconButton`, `new`
- [[List Module]] — from `ListFolder`, `new`
- [[Menu Module]] — from `MenuItem`, `icon`
- [[Select Module]] — from `Select`, `icon`
- [[Text Field Module]] — from `TextField`, `icon`
- [[Button]] — from `Button`, `icon`

## See also

- [[_index]]
- [[Icons Module]]
