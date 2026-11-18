---
tags: [moc, core, cw-embed]
---

# Core / cw-embed — Map of Content

> Notes for `cw-embed`: 5 concepts.

## Notes

- [[cw-embed]] — text embeddings for CodeWiki's semantic search: a built-in local embedder and HTTP providers.
- [[Embedder]] — `Embedder` trait defined in `cw_embed` with 3 methods.
- [[LocalEmbedder]] — the built-in embedder: signed feature hashing, no model and no network.
- [[OllamaEmbedder]] — A model served by Ollama on this machine or another.
- [[OpenAiEmbedder]] — any endpoint that speaks the OpenAI embeddings API.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
