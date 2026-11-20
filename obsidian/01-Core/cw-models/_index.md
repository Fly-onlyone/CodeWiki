---
tags: [moc, core, cw-models]
---

# Core / cw-models — Map of Content

> Notes for `cw-models`: 13 concepts.

## Notes

- [[Catalog Module]] — the models CodeWiki offers, as a list inside the program.
- [[cw-models]] — Built-in embedding models of CodeWiki: the catalog, the machine's hardware, the recommendation and the model files on disk.
- [[Device]] — what runs the model.
- [[Entry]] — `Entry` struct defined in `cw_models::catalog` with 2 methods.
- [[Game Module]] — whether a game is running.
- [[Hardware]] — `Hardware` struct defined in `cw_models::hardware`.
- [[Hardware Module]] — what the machine has to run a model with.
- [[Nvidia]] — the NVIDIA card a model can run on.
- [[Recommend Module]] — which model fits a machine.
- [[Store]] — `Store` struct defined in `cw_models::store` with 9 methods.
- [[Store Module (cw-models)]] — the models on disk.
- [[Supervisor]] — the parent's side: one child at a time, started when needed.
- [[Worker Module]] — the child process that holds a model, seen from both sides.

## Folders

- [[01-Core/cw-models/game/_index|game]] — 1 notes

## See also

- [[_HOME]]
- [[01-Core/_index|Core]]
