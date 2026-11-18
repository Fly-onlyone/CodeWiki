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

- `writer_from` — the writer selected by the settings in effect, or `None` for keyless mode.

## Depends on

- [[LlmWriter]] — uses `LlmWriter`, `new`
- [[Error Module]] — uses `Result`
- [[Logs Module]] — uses `flush`
- [[Provider Module]] — uses `from_settings`
- [[Resolved]] — uses `Resolved`
- [[Anthropic]] — uses `Anthropic`
- [[Ollama]] — uses `Ollama`
- [[OpenAiCompatible]] — uses `OpenAiCompatible`

## Used by

- [[cw-server]] — from `call_as`, `hook`
- [[codewiki]] — from `run`
- [[cw-embed]] — from `post_json`
- [[Provider Module]] — from `post_json`
- [[Api Module]] — from `run_update`
- [[Download Module]] — from `get`
- [[Release Module]] — from `check`
- [[Proposal Module]] — from `unified_diff`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
