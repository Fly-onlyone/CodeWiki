---
tags: [core, cw-wiki, module]
---

# Doctext Module

> From a doc comment to text that may stand in a note.

## What it does

A comment is written for the place it stands in: it may hold a heading (`# Errors`), a doctest, a rustdoc link, a Javadoc tag. A note has rules of its own, which the linter enforces by refusing the whole run: `## ` headings only from the note's template, no Markdown links, no tag in the text, no code span that reads as a line number. Every function here returns text that keeps those rules, whatever the comment holds.

## Source

- `crates/cw-wiki/src/doctext.rs` — module `cw_wiki::doctext`

## How it works

1. `items` in [[Explain Module]] and `RenderCtx::render_crate` in [[RenderCtx]], and 1 more, call `first_sentence`. `first_sentence` calls `first_paragraph`, then `sentence_break`, then `ensure_period`. `first_paragraph` calls `clean_inline`.
2. `RenderCtx::render_integration` and `RenderCtx::summary` in [[RenderCtx]] call `ensure_period`.
3. `RenderCtx::render_integration` and `RenderCtx::summary` in [[RenderCtx]] call `clean_inline`.
4. `RenderCtx::render_crate` and `RenderCtx::render_module` in [[RenderCtx]], and 2 more, call `doc_body`. `doc_body` calls `blocks`, then `clean_line`, then `sentence_break`. `blocks` calls `doc_tag`, then `list_item`. `clean_line` calls `clean_inline`, then `inline_tags`, then `is_line_ref` in [[Lint Module]].
5. `items` in [[Explain Module]] calls `item_text`. `item_text` calls `blocks`, then `ensure_period`, then `clean_line`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub(crate) fn ensure_period(text: &str) -> String` — close `text` with a period unless it already ends a sentence.
- `pub fn clean_inline(text: &str) -> String` — make doc-comment text safe to place in a note: rustdoc links become code, Markdown links keep only their text, and wikilink brackets are defused.
- `fn first_paragraph(doc: &str) -> String` — first paragraph of a doc comment, as one cleaned line.
- `fn sentence_break(paragraph: &str) -> Option<usize>` — index of the end of the first sentence, if the paragraph has more than one.
- `pub fn first_sentence(doc: &str) -> String` — first sentence of a doc comment, ending with a period.
- `enum Block` — one part of a doc comment.
- `const RUSTDOC_FENCE_WORDS: [&str; 6]` — words rustdoc puts after a code fence that say how to test the block, not what language it is in.
- `fn list_item(line: &str) -> Option<(String, &str)>` — the marker and the text of a list item, when `line` starts one.
- `fn doc_tag(line: &str) -> Option<String>` — A Javadoc or JSDoc tag line as a list item, or `None` for a tag that says nothing about what the code does (`@author`, `@since`).
- `fn inline_tags(line: &str) -> String` — ''X'' and ''X'' become code spans, and ''' tags become backticks.
- `fn blocks(doc: &str, language: Language) -> Vec<Block>` — private function, returns `Vec<Block>`; called by `doc_body`, `item_text`.
- `fn clean_line(text: &str) -> String` — A line of comment text, safe in a note: links and tags cleaned, and no code span that the linter would read as a file with a line number.
- `pub fn doc_body(doc: &str, language: Language, after_first_sentence: bool) -> String` — whole text of a doc comment for a note: its paragraphs, lists and code blocks, in order. With `after_first_sentence`, the first sentence is left out, for a note whose summary already is that sentence.
- `pub fn item_text(doc: &str, language: Language) -> String` — A doc comment as the text of one list item: its paragraphs run on as one, its own lists follow as nested items, and its code blocks are left out. Lines after the first are indented by two spaces, under the item.

## Depends on

- [[Lint Module]] — uses `is_line_ref`
- [[Language]] — uses `Language`

## Used by

- [[RenderCtx]] — from `render_crate`, `render_integration`, `render_module`, `render_pattern`
- [[Explain Module]] — from `items`

## See also

- [[_index]]
- [[cw-wiki]]
