---
tags: [core, cw-wiki]
---

# Run

> One pass that brings the stored vectors of some kinds in line with their documents, a batch at a time.

## Source

- `crates/cw-wiki/src/semantic.rs` — definition of `Run`

## How it works

Declared as `pub struct Run`.

Public API:

- `pub fn plan( index: &mut Index, embedder: &dyn Embedder, kinds: &[&str], documents: &[Document], ) -> Result<Self>` — compare the stored vectors with `documents`, drop the vectors of removed documents, and note which documents must be embedded.
- `pub fn store(&mut self, index: &mut Index, vectors: Vec<Vec<f32>>) -> Result<()>` — store the vectors of the texts `Run::next` gave, in their order.
- `pub fn is_done(&self) -> bool` — whether every document has its vector.
- `pub fn finish(&self, index: &Index) -> Result<()>` — record that every vector of the kinds comes from this run's embedder.
- `pub fn next(&self) -> Vec<String>` — the texts to embed next; empty when the run is done.
- `pub fn progress(&self) -> Progress`

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
