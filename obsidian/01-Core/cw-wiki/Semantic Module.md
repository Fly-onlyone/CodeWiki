---
tags: [core, cw-wiki, module]
---

# Semantic Module

> Semantic search: embeds search documents and blends vector matches with full-text matches.

## What it does

Adds meaning-based search on top of full-text search. An embedder turns a document into a vector of numbers. A query is turned into a vector too, and the closest document vectors are found. This finds documents that use other forms of the typed words, or, with a neural model, other words for the same idea. The module keeps the vectors in the index up to date, and merges vector hits with full-text hits when a user searches. It is used by the server, the CLI and the index jobs.

## Source

- `crates/cw-wiki/src/semantic.rs` — module `cw_wiki::semantic`

## How it works

A process-wide [[Slot]] holds the embedder. Programs call `configure` at start, and the backend calls it again when settings change, so no restart is needed. `embedder` returns the current one, or `None` when semantic search is off.

Updating vectors is done by [[Run]]. `embed_documents_with` plans a run, embeds batch by batch, stores each batch, and asks `on_batch` whether to go on. `refresh` does this with the process embedder inside a time budget set by `set_inline_budget`. The run stops after the batch in which the budget ran out. The rest waits, and `behind` reports that.

Two meta keys per document kind record the state. `MADE_BY` names the embedder that made the stored vectors. `WAITING` is set to 1 when documents still need a vector.

`search_with` runs full-text search and, if there is an embedder, `nearest`. `nearest` only looks at kinds whose vectors the current embedder made. Both lists are merged with reciprocal rank fusion: each hit scores 1 divided by 60 plus its rank, and scores add up when a hit is in both lists. So the two score scales never need to match. Vector hits below 0.18 similarity are dropped. A hit with no snippet gets the opening of its document.

## Items

- `const MADE_BY: &str` — start of the index's meta key that names the embedder which made the vectors of one kind of document.
- `const WAITING: &str` — start of the meta key that says whether documents of one kind wait for their vector: "1" from the plan of a run until the run is done.
- `fn built(embedder: Result<Option<Box<dyn Embedder>>>) -> Option<Arc<dyn Embedder>>` — an embedder, or the fact that semantic search is off. A bad setting is reported once and turns semantic search off rather than failing every index update.
- [[Slot]] — the place that holds the embedder in use.
- `pub fn embedder() -> Option<Arc<dyn Embedder>>` — the embedder this process uses; `None` when semantic search is off.
- `pub fn configure(resolved: &Resolved)` — use the embedder the settings in effect name. The programs call this at their start, and the backend again when the settings change, so no restart is needed.
- `fn text_of(document: &Document) -> String` — private function, returns `String`; called by `Run::plan`.
- `fn made_by(index: &Index, kind: &str, name: &str) -> bool` — whether the vectors of `kind` can be compared with a query that the embedder `name` made. Vectors of an embedder chosen earlier stay in the index until the next update; compared with another embedder's query they would give scores that mean nothing. An index written before the embedder was recorded counts as current.
- `pub fn behind(index: &Index, name: &str, kinds: &[&str]) -> Result<bool>` — whether documents of `kinds` still wait for a vector of the embedder `name`: another embedder made the stored ones, or a run was cut short. The backend embeds such an index in the background.
- `pub struct Progress` — how far a run is.
- [[Run]] — one pass that brings the stored vectors of some kinds in line with their documents, a batch at a time.
- `embed_documents_with` — bring the stored vectors of `kinds` in line with `documents`: embed new and changed documents, drop vectors of removed ones. After each batch `on_batch` hears how far the run is and may stop it; what is stored stays, and the next run embeds the rest. Returns where the run ended.
- `embed_documents` — `embed_documents_with` to the end. Returns how many documents were embedded.
- `static INLINE_BUDGET: AtomicU64` — milliseconds `refresh` embeds for before it leaves the rest to a later run; `u64::MAX` is no limit.
- `pub fn set_inline_budget(budget: Option<Duration>)` — let `refresh` embed for about `budget` and leave the rest waiting (`behind` then says so); `None` takes the limit away. The backend sets this: an update holds its lock and its client's patience while it embeds, and a first embedding with a neural model takes minutes. The embedder that needs no model is through thousands of documents within any budget, so for it nothing changes. A program that sets nothing embeds everything where it stands. It is time and not a count of documents, because a document takes one model a hundred times as long as another.
- `pub fn refresh(index: &mut Index, kinds: &[&str], documents: &[Document])` — `embed_documents_with` with the process embedder, within the inline budget. A failing embedder (for example an unreachable Ollama) is reported and leaves full-text search working.
- `embed_within` — `embed_documents_with` that stops after the batch during which `budget` ran out; `None` is no limit. It embeds at least one batch.
- `nearest` — the stored vectors closest to `text`, among the kinds of document whose vectors `embedder` made. Nothing is sent to the embedder when it made none of them.
- `search_with` — search with full-text and vector matches merged by reciprocal rank fusion. Without an embedder this is plain full-text search.
- `pub fn search(index: &Index, text: &str, kind: Option<&str>, limit: usize) -> Result<Vec<Hit>>` — `search_with` using the process embedder.

And 6 more items.

## Depends on

- [[Run]] — uses `finish`, `is_done`, `plan`, `progress`
- [[cw-index]] — uses `document_opening`, `nearest`, `search`, `vector_kinds`
- [[Slot]] — uses `Slot`, `configure`, `new`
- [[Index]] — uses `Index`, `meta`
- [[Error Module]] — uses `Result`
- [[Search Module]] — uses `Hit`
- [[Resolved]] — uses `Resolved`
- [[Embedder]] — uses `Embedder`

## Used by

- [[cw-server]] — from `default`, `no_model_server`, `serve`, `waits`
- [[Run]] — from `Run`, `finish`, `plan`, `progress`
- [[Embed Job Module]] — from `after_update`, `embed_repo`, `resume`
- [[Service Module]] — from `index_notes`, `reindex`
- [[Slot]] — from `configure`, `get`
- [[codewiki]] — from `run`
- [[cw-wiki]] — from `no_model_server`
- [[Serve Module]] — from `run`

## Gotchas

- A bad embedder setting turns semantic search off with one warning. It does not fail index updates. A failing embedder in `refresh` is logged and full-text search keeps working.
- Vectors from an earlier embedder are never compared with a query from another. During a switch, the key holds the marker for mixed vectors, so neither embedder counts as current until the run is done. A run cut short stays "behind".
- An index written before the embedder name was recorded counts as current.
- The inline budget is time, not a count. At least one batch is always embedded, even with a zero budget.
- A program that never sets a budget embeds everything where it stands.

## See also

- [[_index]]
- [[cw-wiki]]
