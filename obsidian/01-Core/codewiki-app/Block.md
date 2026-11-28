---
tags: [core, codewiki-app]
---

# Block

> A part of a page.

## What it does

`Block` is one part of a generated page. A page function turns backend data into a `Doc`: a title, an optional lead, and a list of blocks. `Block` says what each part is, not how it looks. The [[Doc View Module]] draws it. Keeping the two apart lets a test read a page without a window.

There are blocks for text (heading, paragraph, list, code), data (stats, table, rows), grouping (card, which holds more blocks), and notices (alert, empty). There are also blocks for settings: switch, select, typed field, secret, choices, setting and buttons. `Progress` shows long work, `Logo` the app's logo, and `Search` marks the place of Home's search field.

## Source

- `crates/codewiki-app/src/doc.rs` — definition of `Block`

## How it works

A page function builds a `Doc` and calls `push` for each block. A `Card` holds a list of blocks in `body`, so blocks nest through cards. Anything a user can press carries an [[Action]]. The view sends that action back to the app, which decides what to do.

Setting blocks carry no value of their own in two cases. A `Field` has an id, a [[FieldId]], and its text lives in the input the app keeps for that id. A `Secret` has an id and a `SecretState`, and never holds the secret. A setting can say why it cannot be changed, in `locked`. A `Select`, `Field` or `Secret` may also carry a settings `Note`.

`Search` has the same shape: it holds nothing. The app owns the input and the list of suggestions under it, and the view draws them in the place the block marks.

In tests, `Doc::flat` opens cards out into one list of blocks. `words` and `actions` use it to read a page's text and its actions.

## Items

Declared as `pub enum Block`.

## Depends on

- [[Doc Module]] — uses `Choice`, `Line`, `Press`, `Row`
- [[Settings Module]] — uses `Note`
- [[Action]] — uses `Action`
- [[Tone]] — uses `Tone`
- [[FieldId]] — uses `FieldId`

## Used by

- [[Settings Pages Module]] — from `agents_card`, `appearance`, `auto_explain_card`, `backend`
- [[Doc]] — from `Doc`, `flat`, `push`, `walk`
- [[Doc View Module]] — from `draw`
- [[Pages Module]] — from `file_list`

## Gotchas

- Adding a variant breaks every `match` on `Block`: the draw code in the view and the test helpers `words` and `actions`. Update them together.
- Cards nest. Code that walks a page must go into `Card` bodies, as `flat` does.
- A `Field` or `Secret` block does not hold its text. Read it from the input kept for its id.
- `Progress` without a `share` only says that work goes on. A bar that is not `moving` stands still, for work that waits.
- `Search` shows nothing when a page is drawn without the app's input, for instance in a test.

## See also

- [[_index]]
- [[Doc Module]]
