---
tags: [moc, core, cw-embed]
---

# Core / cw-embed — Map of Content

> Notes for `cw-embed`: 8 concepts.

## Notes

- [[cw-embed]] — text embeddings for CodeWiki's semantic search: a built-in local embedder and HTTP providers.
- [[Embedder]] — `Embedder` trait defined in `cw_embed` with 5 methods.
- [[List Module (cw-embed)]] — the models a server offers, for the page that lets someone pick one.
- [[LocalEmbedder]] — the built-in embedder: signed feature hashing, no model and no network.
- [[OllamaEmbedder]] — A model served by Ollama on this machine or another.
- [[OpenAiEmbedder]] — any endpoint that speaks the OpenAI embeddings API.
- [[Prefix Module]] — the text some embedding models expect before a query and before a document.
- [[Server]] — A server that speaks the OpenAI embeddings API.

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
