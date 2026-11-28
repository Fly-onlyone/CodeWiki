---
tags: [core, cw-wiki, module]
---

# Doctext Module

> From a doc comment to text that may stand in a note.

## What it does

Cleans doc comments from source code so they can be pasted into a wiki note. A comment may hold a heading, a doctest, a rustdoc link or a Javadoc tag. A note has stricter rules, and the linter refuses a whole run that breaks them: headings only from the template, no Markdown links, no tags, and no code span that looks like a file with a line number. Every function here returns text that keeps those rules, whatever the comment holds.

## Source

- `crates/cw-wiki/src/doctext.rs` — module `cw_wiki::doctext`

## How it works

There are three public ways to take text from a comment.

- `first_sentence` takes the first paragraph, cleans it with `clean_inline`, cuts at the first sentence end, and closes it with `ensure_period`. It is the one-line summary of an item.
- `doc_body` gives the whole comment for a note. With `after_first_sentence` it leaves the summary out, for a note whose summary already is that sentence.
- `item_text` gives a comment as the text of one list item. Paragraphs run on as one, nested lists follow as indented items, and code blocks are dropped.

`doc_body` and `item_text` both call `blocks`. It reads the comment line by line into parts: running text, a label, a list, a code block or a table. A heading such as `# Errors` becomes a label, since a note takes headings only from its template. `doc_tag` turns Javadoc and JSDoc tags such as `@param` and `@returns` into list items. `clean_line` then runs `inline_tags` and `clean_inline` over each line, and rewrites any code span that would read as a line reference.

`clean_inline` keeps code spans as written. It turns rustdoc links into plain text, and Markdown links keep only their text. It defuses wikilink brackets, drops a `#` that would read as a tag, and squeezes whitespace.

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

## Gotchas

- Tags with no value for a reader, such as `@author` and `@since`, are dropped, with the lines that follow them.
- A fence labelled `mermaid` becomes `text`, because the app would draw a diagram and diagrams belong in a flow note.
- An unlabelled fence takes the language of the source: `rust`, `go`, `kotlin`, `c` or `cpp`. In a Rust comment, rustdoc words such as `no_run` or `ignore` also mean `rust`.
- `sentence_break` skips `e.g.`, `i.e.` and `etc` and anything inside backticks. A sentence that ends in another abbreviation is cut early.
- A Python doctest block runs until the next blank line.
- A fence or doctest that the comment never closes is closed at the end, so it cannot swallow the rest of the note.
- A code span that reads as a line reference, such as an address with a port, loses its backticks. An unpaired backtick becomes an apostrophe.

## See also

- [[_index]]
- [[cw-wiki]]
