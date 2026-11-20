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
2. `answer` in [[Answer Module]] and `ask` in [[Api Module]] call `answer`. `answer` calls `Note::parse` in [[Note]].
3. `list_notes` in [[Api Module]] calls `list_notes`. `list_notes` calls `VaultMeta::load` in [[VaultMeta]], then `info`. `info` calls `title_of`, then `state` in [[Lifecycle Module]].
4. `read_note` in [[Api Module]] calls `read_note_as`. `read_note_as` calls `VaultMeta::load` in [[VaultMeta]], then `title_of`, then `Error::not_found` in [[Error]], then `check_vault_path`, then `Error::io` in [[Error]], then `agent_view`, and 2 more. `check_vault_path` calls `Error::invalid` in [[Error]].
5. `write_note` in [[Api Module]] calls `write_note`. `write_note` calls `check_vault_path`, then `VaultMeta::load` in [[VaultMeta]], then `Error::not_found`, then `Error::io` (both in [[Error]]).

4 more entry points: `overview`, `symbol_detail`, `context`, `dependency_path`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub struct NoteInfo` — public struct; used by `NoteContent`, `info`, `list_notes`.
- `pub struct NoteContent` — public struct; used by `read_note`, `read_note_as`.
- `pub enum View` — how much of a note a reader gets.
- `pub fn agent_view(text: &str, kind: NoteKind) -> (String, Vec<String>)` — the text of a note for a coding agent: the note without the sections in `AGENT_OMITS`, and the headings that were left out. Only a note about a module, a crate or a type is cut. In a flow note `How it works` is the diagram, which is the note.
- `pub struct Status` — public struct; used by `Overview`, `repo_status`, `status`.
- `pub struct Overview` — public struct; used by `overview`, `repo_overview`.
- `pub struct Related` — A related symbol with how sure the resolver is of the relation.
- `pub struct SymbolDetail` — public struct; used by `Context`, `symbol_detail`.
- `pub struct FileContext` — public struct; used by `Context`, `context`.
- `pub enum Context` — public enum; used by `context`.
- `pub struct Answer` — public struct; used by `answer`, `ask`, `context`.
- `pub fn list_notes(root: &Path) -> Result<Vec<NoteInfo>>` — public function, returns `Result<Vec<NoteInfo>>`; called by `list_notes`.
- `pub fn read_note_as(root: &Path, path_or_name: &str, view: View) -> Result<NoteContent>` — read a note by vault path or by name, as much of it as `view` says.
- `pub fn write_note(root: &Path, path: &str, text: &str) -> Result<()>` — save a human edit to an existing note. The manifest is left alone, so the note's changed hash marks it as human-edited from now on.
- `pub fn status(root: &Path, index: &Index) -> Result<Status>` — public function, returns `Result<Status>`; called by `overview`, `repo_status`, `summarize`.
- `pub fn overview(root: &Path, index: &Index) -> Result<Overview>` — public function, returns `Result<Overview>`; called by `repo_overview`.
- `pub fn symbol_detail(root: &Path, index: &Index, name: &str) -> Result<SymbolDetail>` — detail for one symbol, by qualified or bare name.
- `pub fn context(root: &Path, index: &Index, target: &str) -> Result<Context>` — context for a file path or a symbol name: what an agent should know before touching it.
- `pub fn dependency_path(index: &Index, from: &str, to: &str) -> Result<Vec<String>>` — shortest dependency chain between two symbols, by qualified or bare name.
- `pub fn answer(root: &Path, index: &Index, question: &str) -> Result<Answer>` — answer a question from the vault without an LLM: the best-matching notes' summaries, with their sources. An LLM-backed answer can build on the same retrieval.

And 7 more items.

## Depends on

- [[Index]] — uses `Index`, `crates`, `edges`, `edges_from`
- [[Error]] — uses `invalid`, `io`, `not_found`
- [[VaultMeta]] — uses `by_path`, `load`, `notes_for_source`
- [[Model Module]] — uses `Edge`, `file_node`
- [[Lifecycle Module]] — uses `NoteState`, `state`
- [[Manifest Module]] — uses `NoteEntry`, `Origin`
- [[Git Module]] — uses `head_commit`, `vault_is_behind`
- [[cw-core]] — uses `VAULT_DIR`

## Used by

- [[Api Module]] — from `ask`, `context`, `dependency_path`, `list_notes`
- [[Answer Module]] — from `answer`, `context`
- [[Workspace Module (cw-server)]] — from `summarize`

## See also

- [[_index]]
- [[cw-wiki]]
