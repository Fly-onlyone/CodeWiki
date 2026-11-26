---
tags: [core, cw-wiki]
---

# Run

> One pass that brings the stored vectors of some kinds in line with their documents, a batch at a time.

## What it does

A neural model takes minutes over a repository. A run stores every batch, so one that is stopped keeps what it did and the next run goes on from there: a document whose vector is stored is not embedded again. The caller drives it, which lets a server embed without holding a lock and take the lock for `Run::store` alone:

```text
let mut run = Run::plan(index, embedder, kinds, documents)?;
while !run.is_done() {
    let vectors = embedder.embed(&run.next())?;
    run.store(index, vectors)?;
}
run.finish(index)?;
```

## Source

- `crates/cw-wiki/src/semantic.rs` — definition of `Run`

## How it works

`plan` asks the index for the stored vector hashes of the kinds. For each document it hashes the embedder name plus the document text. If the stored hash differs, the document is added to the pending list. Vectors of documents that no longer exist are pruned. For each kind, `WAITING` is set to 1 when any of its documents is pending. When a kind has vectors from another embedder and work is pending, the key that names the embedder is set to a marker meaning mixed. This way a run cut short is never taken as current for either embedder.

`next` returns the texts of the next batch, sized by the embedder. `store` checks that the number of vectors equals the batch size, writes them as rows, and moves the cursor. `finish` does nothing until every document is done. Then it records the embedder name and clears `WAITING` for each kind.

## Items

Declared as `pub struct Run`.

Members:

- `plan` — compare the stored vectors with `documents`, drop the vectors of removed documents, and note which documents must be embedded.
- `pub fn progress(&self) -> Progress` — public method, returns `Progress`.
- `pub fn is_done(&self) -> bool` — whether every document has its vector.
- `pub fn next(&self) -> Vec<String>` — the texts to embed next; empty when the run is done.
- `pub fn store(&mut self, index: &mut Index, vectors: Vec<Vec<f32>>) -> Result<()>` — store the vectors of the texts `Run::next` gave, in their order.
- `pub fn finish(&self, index: &Index) -> Result<()>` — record that every vector of the kinds comes from this run's embedder. Does nothing while documents still wait.

## Depends on

- [[Semantic Module]] — uses `MADE_BY`, `MIXED`, `Pending`, `Progress`
- [[cw-index]] — uses `prune_vectors`, `upsert_vectors`, `vector_hashes`
- [[Index]] — uses `Index`, `meta`, `set_meta`
- [[Error Module]] — uses `Result`
- [[Hash Module]] — uses `sha256_hex`
- [[Vectors Module]] — uses `VectorRow`
- [[Error]] — uses `invalid`
- [[Embedder]] — uses `Embedder`

## Used by

- [[Semantic Module]] — from `embed_documents_with`, `set_inline_budget`
- [[Embed Job Module]] — from `embed_repo`

## Gotchas

- `store` must get the vectors of exactly the texts `next` returned, in order. A wrong count is an invalid error and nothing is stored.
- The hash includes the embedder name, so a new embedder re-embeds everything, while the same one embeds only new or changed documents.
- Skipping `finish` leaves the kind marked as waiting. This is the intended sign for a stopped run, and `behind` then reports it.
- `plan` writes to the index (prune and meta), before any embedding happens.

## See also

- [[_index]]
- [[Semantic Module]]
