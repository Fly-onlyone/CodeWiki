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

1. `embed_repo` in [[Models Module]] and `embed_documents_with` in [[Semantic Module]], and 1 more, call `plan`. `plan` calls `text_of` in [[Semantic Module]], then `sha256_hex` in [[Hash Module]].

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

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

- [[Semantic Module]] — from `embed_documents_with`, `embed_within`, `set_inline_budget`
- [[Models Module]] — from `embed_repo`

## See also

- [[_index]]
- [[Semantic Module]]
