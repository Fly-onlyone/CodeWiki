---
tags: [core, cw-wiki, module]
---

# Query Module

> Read-side queries served to the app and to agents.

## What it does

The vault holds curated concept notes only; file and symbol detail is answered live from the index by the functions here.

## Source

- `crates/cw-wiki/src/query.rs` — module `cw_wiki::query`

## How it works

1. `repo_status` in [[Api Module]] and `summarize` in [[Workspace Module (cw-server)]] call `status`. `status` calls `VaultMeta::load` in [[VaultMeta]], then `head_commit` in [[Git Module]], then `list` in [[Proposal Module]], then `vault_is_behind` in [[Git Module]].
2. `brief` and `queue` in [[Explanations Module]] call `title_of`.
3. `list_notes` in [[Api Module]] calls `list_notes`. `list_notes` calls `VaultMeta::load` in [[VaultMeta]], then `info`. `info` calls `title_of`, then `state` in [[Lifecycle Module]].
4. `brief` and `submit` in [[Explanations Module]] call `find_entry`. `find_entry` calls `title_of`, then `Error::not_found` in [[Error]], then `check_vault_path`. `check_vault_path` calls `Error::invalid` in [[Error]].
5. `read_note` in [[Api Module]] calls `read_note_as`. `read_note_as` calls `VaultMeta::load` in [[VaultMeta]], then `find_entry`, then `Error::io` in [[Error]], then `agent_view`, then `info`, then `illustration_path` in [[Generate Module]], and 1 more.

6 more entry points: `write_note`, `overview`, `symbol_detail`, `context`, `dependency_path`, `answer`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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
- `pub enum Context` — public enum; used by `context`.
- `pub struct Answer` — public struct; used by `answer`, `ask`.
- `pub fn list_notes(root: &Path) -> Result<Vec<NoteInfo>>` — public function, returns `Result<Vec<NoteInfo>>`; called by `list_notes`.
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
- [[Explanations Module]] — from `brief`, `queue`, `submit`
- [[cw-wiki]] — from `a_note_that_is_behind_comes_first_and_is_cleared_by_writing_again`, `a_submission_is_woven_into_the_note_and_recorded`
- [[Workspace Module (cw-server)]] — from `summarize`

## See also

- [[_index]]
- [[cw-wiki]]
