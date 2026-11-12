---
tags: [core, cw-llm]
---

# cw-llm

> LLM providers and the prose writer for CodeWiki.

## Source

- `crates/cw-llm/Cargo.toml` — package manifest
- `crates/cw-llm/src/lib.rs` — crate root

## How it works

Builds a library.

Modules:

- [[Answer Module]] — answers a question about the codebase with an LLM, grounded in the vault.
- [[Illustrate Module]] — asks an LLM to draw a note as an OpenUI Lang illustration.
- [[Provider Module]] — LLM providers: hosted APIs and locally installed agent CLIs.
- [[Writer Module]] — turns a structural draft into finished prose with an LLM.

Defined at the crate root:

- `writer_from_env` — the writer selected by the environment, or `None` for keyless mode.

## Depends on

- [[LlmWriter]] — uses `LlmWriter`, `new`
- [[Error Module]] — uses `Result`
- [[Provider Module]] — uses `from_env`

## Used by

- [[codewiki]] — from `run`
- [[Api Module]] — from `run_update`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
