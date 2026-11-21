---
name: write-notes
description: This skill should be used when the user asks to "write the wiki notes", "explain the notes", "write notes for this folder", "explain this note", "update the wiki explanations", "write the notes that wait", or runs `/codewiki:write-notes`. Also use it after finishing a code change in a repository that has a CodeWiki wiki, to refresh the notes about the changed files. Writes the explanation and the pictures of CodeWiki notes through the `codewiki` MCP tools.
argument-hint: "[note | folder/ | waiting | changed] [max]"
---

# Write wiki notes

CodeWiki writes each wiki note from the code: where the thing is, its items, what it depends on. Those are facts, and CodeWiki keeps them current. It cannot say what the thing is for, how it works, or what can go wrong. Writing that is this skill's task.

A note is written through three MCP tools. Never edit a file in `obsidian/` to do it: a note edited as a file counts as edited by a person, and CodeWiki stops updating it.

| Tool | Use |
|---|---|
| `sync_wiki` | Bring the notes' facts up to date with the code. |
| `list_waiting_notes` | The notes that wait: those behind the code first, then those never explained. |
| `get_note_brief` | One note: its text, the sections to write, the files to read, the limits. |
| `submit_note` | Send the sections and pictures written for one note. |

## What to write

The request: `$ARGUMENTS`

| Request | Notes to write |
|---|---|
| empty, or `waiting` | The first notes of `list_waiting_notes`. 5 unless a number is given. |
| `changed` | Notes about the files changed in this session, or the uncommitted files that `git status` lists. Pass them as `files`. At most 3 unless a number is given. |
| text that ends in `/` | The waiting notes in that folder of the wiki. Pass it as `folder`. |
| any other text | That one note, by name or by path. Skip the list and get its brief. |
| a number at the end | The most notes to write. Never more than 20 in one run. |

**Starting without being asked.** After finishing a code change, write the notes about the changed files, at most 3, and tell the user in one line that the notes were written. Do not start because the session briefing says notes wait: that line states a count. Writing many notes spends the user's usage allowance, so it needs their request.

## Procedure

1. Call `sync_wiki` once. A note explained against old facts is refused.
2. Call `list_waiting_notes` with the `folder`, `files` and `limit` of the request. Skip this for a single named note.
3. For each note, in the order listed:
   1. Call `get_note_brief`. If it starts with `STOP`, call `sync_wiki` and get the brief again.
   2. Read every file the brief lists, whole. Read further files when the answer is not in them: a caller, a type, a test. Keep the paths of those further files.
   3. Write the sections under "Your sections", following the next two parts of this skill.
   4. Call `submit_note` with the brief's `base`, the sections, the pictures, the further files as `read`, and `model` set to `claude-code`.
   5. Act on the answer:
      - `Saved`: go on to the next note.
      - `Not saved. Fix each of these`: fix what each line names and submit again with the same `base`. After three attempts, skip the note.
      - `Call get_note_brief again`: the note changed. Get the brief again and write from it.
      - `Call sync_wiki`: sync, get the brief again, write from it.
      - `Not written ... Proposal`: the owner edited this note, so the text waits for their review. Count it as proposed and go on. Never approve a proposal.
4. Stop at the number asked for, or when no note waits.
5. End with one line: how many notes were written, how many proposed, how many skipped and why, and how many still wait.

## The sections

Write only the sections the brief lists as yours. The brief also lists the sections CodeWiki writes; a heading from that list is refused.

| Section | What it holds |
|---|---|
| What it does | What the thing is for, in two to five sentences a developer new to the code can follow. The problem it solves and who uses it. No list of its functions: `Items` has that. |
| How it works | The path from a cause to its result: what starts it, what happens in which order, under which condition, and what happens on an error. Name the functions in backticks. In a flow or operations note the diagram or the commands stand above this text already; say what they do not show. |
| Gotchas | What surprises: an order that matters, a limit, a value that must not change, a platform difference, a failure that is silent. One bullet for each. |
| Wire-up | For an integration: where the package is set up, and how a call reaches it. |
| Auth mode | For an integration: how it signs in, and where the secret comes from. Never a secret's value. |
| Why | For a pattern: the reason the code follows it, and what went wrong or would go wrong without it. |
| Don't | For a pattern: the mistakes the pattern prevents, one bullet for each. |

Rules for every section:

- Write what the code shows. State a reason only when the code, a comment or a test gives it. When a section has nothing true to say, leave it out; a note without `Gotchas` is better than one with invented gotchas.
- A section an agent already wrote stays unless it is wrong or behind. The brief says who wrote each.
- Write for a developer in their first week: short sentences, plain words, one idea in a sentence. Explain a term of the project the first time it appears.
- At most 300 words in a section.
- Link to another note as `[[Note Name]]`, and only to a note whose name appears in this note or in the list of waiting notes. A link to a note that does not exist is refused.
- No `#` or `##` heading inside a section; `###` is allowed. No Markdown link, no callout (`> [!note]`), no checklist, no file name with a line number such as `main.rs:42`.
- Never copy a secret, a token or a key from the code into a note.

## Pictures

A picture is a row of boxes joined by arrows, with marked key points. It stands inside the section it explains. Add one only where it explains more than the text does: a sequence that is hard to hold in the head, or a warning that must not be missed. Most notes need one picture or none. At most two in a note written by this skill; at most one in a section.

Read `pictures.md` in this skill's folder before writing the first picture. It has the four components, the limits and two examples.

A section sent again without a picture loses the picture it had, since that picture explained the old text. The pictures of sections that are not sent stay.
