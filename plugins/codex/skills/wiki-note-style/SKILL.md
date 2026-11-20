---
name: wiki-note-style
description: Use before you create or edit a Markdown note in a CodeWiki wiki (the `obsidian/` folder of a repository that has `obsidian/_HOME.md`). Gives the house style CodeWiki's own notes follow: frontmatter, title, summary, section order per kind of note, links and length.
---

# Wiki note style

CodeWiki checks every note it writes against a house style and throws away text that breaks it. Notes edited by hand are checked more loosely, but they should read like the rest of the wiki. Follow these rules whenever you write in `obsidian/`.

## Before you edit

- Prefer `sync_wiki` when a note is only out of date: CodeWiki rewrites it from the code. Edit by hand to add what the code cannot show, such as a reason, a constraint or a gotcha.
- A note you edit counts as edited by a person from then on. CodeWiki never overwrites it again; when the code behind it changes, the new text waits as a proposal for review.
- Never edit anything under `obsidian/.codewiki/`.
- To record a decision, run `codewiki decisions --new "<title>"` instead of creating the file yourself.

## Shape of a note

1. **Frontmatter** holds only `tags`, and the list is not empty. Example: `tags: [core, cw-wiki, module]`.
2. **Title**: one `# Title` heading. The file name is the title plus `.md`.
3. **Summary**: one sentence in a `>` blockquote directly under the title. It says what the thing is for, not how it works.
4. **Sections**: `## ` headings from the note's template, in the template's order. Leave a section out rather than leave it empty. Keep the headings the note already has.
5. **`## See also`** is the last section. Its first link goes up the map: `[[_index]]` in an ordinary note; `[[_HOME]]` in a zone map such as `01-Core/_index.md`.

## Section templates

Every heading is optional, but the order is fixed. Recognise a note's kind by the headings it already has.

| Kind of note | Sections, in order |
|---|---|
| Architecture flow, module or concept, operations | Source, How it works, Depends on, Used by, Gotchas, See also |
| Pattern | When to apply, The pattern, Why, Don't, See also |
| Integration | Used for, Configuration, Wire-up, Auth mode, Gotchas, See also |
| Glossary entry | Definition, See also |
| Decision record | Status, Context, Decision, Consequences, See also |

The home note `_HOME.md` and the folder maps `_index.md` have no template.

## Writing

- Dense technical prose in English, 50 to 200 words in the whole note (maps excepted). Explain why, not only what.
- Put every identifier, file name and command in backticks.
- Refer to code by its repo-relative path in backticks, such as `crates/cw-wiki/src/briefing.rs`. Never add a line number.
- Use `**bold**` only for a constraint the reader must not break.
- Keep the bullets under `## Source` as they are; CodeWiki writes them from the index.
- Add `## Gotchas` only when the code shows something a maintainer could get wrong: an ordering constraint, a hidden side effect, an invariant that is not obvious, a failure mode.
- Put Mermaid diagrams only in architecture flow notes.

## Links

- Link other notes only as `[[Exact Note Name]]`, and only to notes that exist. Check the name with `search_codebase` or `read_note` first; a link that does not resolve is an error.
- No Markdown links, no embeds (`![[...]]`), no callouts (`> [!note]`), no checklists (`- [ ]`), and no tags in the text: tags belong in the frontmatter.
