---
tags: [core, cw-llm]
---

# cw-llm

> LLM providers and the prose writer for CodeWiki.

## What it does

Generation works without any of this (keyless mode). When a provider is configured, the writer replaces structural prose with explained prose.

## Source

- `crates/cw-llm/Cargo.toml` — package manifest
- `crates/cw-llm/src/lib.rs` — crate root

## How it works

Builds a library.

1. `run` in [[codewiki]] and `run_update` in [[Api Module]] call `writer_from`. `writer_from` calls `from_settings` in [[Provider Module]].

## Items

Modules:

- [[Answer Module]] — answers a question about the codebase with an LLM, grounded in the vault.
- [[Cliproxy Module]] — CLIProxyAPI: a server the user runs that holds many Claude and Codex subscription logins and serves them behind one OpenAI-compatible API, taking the next account for each request and resting one that reached its limit (github.com/router-for-me/CLIProxyAPI).
- [[Illustrate Module]] — asks an LLM to draw a note as an OpenUI Lang illustration.
- [[Provider Module]] — LLM providers: hosted APIs and locally installed agent CLIs.
- [[Writer Module]] — turns a structural draft into finished prose with an LLM.

Defined at the crate root:

- `pub fn writer_from(resolved: &cw_core::settings::Resolved) -> cw_core::Result<Option<LlmWriter>>` — the writer selected by the settings in effect, or `None` for keyless mode.

## Depends on

- [[Cliproxy Module]] — uses `models`, `reachable`
- [[LlmWriter]] — uses `LlmWriter`, `new`
- [[Error Module]] — uses `Result`
- [[Logs Module]] — uses `flush`
- [[Provider Module]] — uses `from_settings`
- [[Resolved]] — uses `Resolved`
- [[CliProxy]] — uses `CliProxy`
- [[Anthropic]] — uses `Anthropic`

## Used by

- [[codewiki]] — from `run`
- [[Api Module]] — from `run_update`

## See also

- [[_index]]
- [[Crate Dependency Flow]]
