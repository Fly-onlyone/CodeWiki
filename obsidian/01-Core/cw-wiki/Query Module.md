---
tags: [core, cw-wiki, module]
---

# Query Module

> Read-side queries served to the app and to agents.

## What it does

Serves the read side of the wiki to the app and to coding agents. The vault holds only curated concept notes, so questions about a file or a symbol cannot be answered from notes. These functions answer them live from the index and the vault metadata: note lists, one note's text, repository status, symbol detail, file context, dependency paths, and a note-based answer to a question. None of them calls a model.

## Source

- `crates/cw-wiki/src/query.rs` — module `cw_wiki::query`

## How it works

`list_notes` loads the vault metadata and turns each entry into a `NoteInfo` through `info`, sorted by path. `info` sets the lifecycle state and where the explanation stands. A note kind that is not explainable, and an adopted note nobody explained, get `None`. Otherwise it is `Waiting` (rules wrote it, no agent has), `Explained`, or `Behind` (the entry lists files that changed after the agent wrote).

`read_note_as` finds the entry with `find_entry`. That tries the vault path, then the path with `.md` added, then the note name without case. The found path goes through `check_vault_path`. With `View::Whole` it returns the text and the note's pictures, read from the illustration file beside the note. With `View::Agent` it runs `agent_view` and returns no pictures. `agent_view` cuts the rule-made `How it works` section from Concept notes only. The cut is skipped when an agent wrote that section.

`symbol_detail` finds a symbol by qualified or bare name. It collects `depends_on` and `used_by` from dependency edges, including the edges of its methods. It drops relations to itself and its own members, then sorts and removes duplicates. It also lists the notes that cite the symbol's file. `context` returns file detail (imports, importers) when the target is an indexed path, else symbol detail. `dependency_path` builds a graph of dependency edges and returns the shortest chain. `answer` searches notes and symbols and joins the summaries of up to 3 note hits.

`status` counts files, symbols, edges, notes and pending proposals, and compares the vault commit with the head commit.

## Items

- `pub struct NoteInfo` — public struct; used by `NoteContent`, `info`, `list_notes`.
- `pub enum Explanation` — where a note's explanation stands. Rules write a note's facts; what the thing is for and how it works is a coding agent's to write.
- `pub struct NoteContent` — public struct; used by `read_note`, `read_note_as`.
- `pub fn agent_view(text: &str, kind: NoteKind, by_agent: &[String]) -> (String, Vec<String>)` — the text of a note for a coding agent: the note without the sections in `AGENT_OMITS`, and the headings that were left out. Only a note about a module, a crate or a type is cut. In a flow note `How it works` is the diagram, which is the note. `by_agent` are the headings an agent wrote. Such a section stays: it holds reasons, conditions and error paths, which the code does not say in one place.
- `pub struct Status` — public struct; used by `Overview`, `repo_status`, `status`.
- `pub struct Overview` — public struct; used by `overview`, `repo_overview`.
- `pub struct Related` — A related symbol with how sure the resolver is of the relation.
- `pub struct SymbolDetail` — public struct; used by `Context`, `symbol_detail`.
- `pub struct FileContext` — public struct; used by `Context`, `context`.
- `pub struct Answer` — public struct; used by `answer`, `ask`.
- `pub fn list_notes(root: &Path) -> Result<Vec<NoteInfo>>` — public function, returns `Result<Vec<NoteInfo>>`; called by `list_notes`.
- `pub fn read_note(root: &Path, path_or_name: &str) -> Result<NoteContent>` — read a whole note by vault path or by name.
- `pub fn read_note_as(root: &Path, path_or_name: &str, view: View) -> Result<NoteContent>` — read a note by vault path or by name, as much of it as `view` says.
- `pub fn write_note(root: &Path, path: &str, text: &str) -> Result<()>` — save a human edit to an existing note. The manifest is left alone, so the note's changed hash marks it as human-edited from now on.
- `pub fn status(root: &Path, index: &Index) -> Result<Status>` — public function, returns `Result<Status>`; called by `overview`, `repo_status`, `summarize`.
- `pub fn overview(root: &Path, index: &Index) -> Result<Overview>` — public function, returns `Result<Overview>`; called by `repo_overview`.
- `pub fn symbol_detail(root: &Path, index: &Index, name: &str) -> Result<SymbolDetail>` — detail for one symbol, by qualified or bare name.
- `pub fn context(root: &Path, index: &Index, target: &str) -> Result<Context>` — context for a file path or a symbol name: what an agent should know before touching it.
- `pub fn dependency_path(index: &Index, from: &str, to: &str) -> Result<Vec<String>>` — shortest dependency chain between two symbols, by qualified or bare name.
- `pub fn answer(root: &Path, index: &Index, question: &str) -> Result<Answer>` — answer a question from the vault without an LLM: the best-matching notes' summaries, with their sources. An LLM-backed answer can build on the same retrieval.

And 9 more items.

## Depends on

- [[Index]] — uses `Index`, `crates`, `edges`, `edges_from`
- [[VaultMeta]] — uses `VaultMeta`, `by_path`, `load`, `notes_for_source`
- [[Error]] — uses `invalid`, `io`, `not_found`
- [[Model Module]] — uses `Edge`, `file_node`
- [[Lifecycle Module]] — uses `NoteState`, `state`
- [[Git Module]] — uses `head_commit`, `vault_is_behind`
- [[NoteKind]] — uses `NoteKind`, `explainable`
- [[cw-core]] — uses `VAULT_DIR`

## Used by

- [[Api Module]] — from `ask`, `context`, `dependency_path`, `list_notes`
- [[cw-wiki]] — from `a_link_by_folder_path_opens_its_note`, `a_note_that_is_behind_comes_first_and_is_cleared_by_writing_again`, `a_submission_is_woven_into_the_note_and_recorded`
- [[Explanations Module]] — from `brief`, `row`, `submit`
- [[Workspace Module (cw-server)]] — from `summarize`

## Gotchas

- `write_note` leaves the manifest alone on purpose. The changed hash is what marks the note as human-edited, so CodeWiki stops overwriting it. The note must already exist.
- `check_vault_path` rejects empty paths, `\`, `:`, a leading `/`, empty, `.`, `..` and hidden segments, and anything not ending in `.md`. A bad path gives an invalid error, an unknown note a not-found error.
- `agent_view` ignores `## ` lines inside code fences. Only Concept notes are cut. Flow and other kinds come back whole.
- `dependency_path` returns an empty list, not an error, when a symbol is not in the graph or no path exists. An unknown name is a not-found error.
- A note with no summary adds nothing to `answer`. When no note has one, `answer` returns a fixed hint text.

## See also

- [[_index]]
- [[cw-wiki]]
