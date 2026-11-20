---
tags: [moc, core, cw-embed]
---

# Core / cw-embed — Map of Content

> Notes for `cw-embed`: 6 concepts.

## Notes

- [[BuiltinEmbedder]] — A neural model of CodeWiki's catalog, installed under the data directory and run by a hidden child process that CodeWiki starts on the first request and stops when it is idle.
- [[cw-embed]] — text embeddings for CodeWiki's semantic search: a built-in local embedder and HTTP providers.
- [[Embedder]] — `Embedder` trait defined in `cw_embed` with 6 methods.
- [[LocalEmbedder]] — the built-in embedder: signed feature hashing, no model and no network.
- [[OllamaEmbedder]] — A model served by Ollama on this machine or another.
- [[OpenAiEmbedder]] — any endpoint that speaks the OpenAI embeddings API.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
