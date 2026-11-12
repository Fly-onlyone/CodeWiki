---
tags: [core, cw-wiki, module]
---

# Generate Module

> The generation pipeline: scan, plan, freeze names, render, lint, write.

## Source

- `crates/cw-wiki/src/generate.rs` — module `cw_wiki::generate`

## How it works

Main items:

- [[WriterUsage]] — what a writer spent during one run.
- [[ProseWriter]] — replaces the structural prose of a note, typically with an LLM.
- `Report` — struct
- `WriterContext` — facts handed to a `ProseWriter`.
- `generate` — generate or update the vault of the repository at `root`.
- `illustration_path` — vault path of the illustration that belongs to the note at `note_path`.
- `TEMPLATE_VERSION` — bump when rendering changes in a way that should refresh every note.
- `list_notes` — Vault-relative paths of every Markdown note on disk.

## Depends on

- [[VaultMeta]] — uses `VaultMeta`, `load`, `registry`, `save`
- [[ProseWriter]] — uses `ProseWriter`, `illustrate`, `rewrite`, `usage`
- [[Manifest Module]] — uses `NoteEntry`, `SourceRef`, `body_hash`
- [[Registry]] — uses `entries`, `new`, `reserve`
- [[RenderCtx]] — uses `illustration`, `new`, `render`
- [[cw-illustrate]] — uses `EXTENSION`, `check`
- [[Lifecycle Module]] — uses `action`, `state`
- [[Lint Module]] — uses `link_resolution_rate`, `lint`

## Used by

- [[cw-wiki]] — from `run`, `run_settled`
- [[Api Module]] — from `run_update`, `update_repo`
- [[Service Module]] — from `update`, `update_in`
- [[LlmWriter]] — from `illustrate`, `rewrite`
- [[ProseWriter]] — from `illustrate`, `rewrite`
- [[codewiki]] — from `print_report`
- [[Writer Module]] — from `read_sources`
- [[Query Module]] — from `read_note`

## See also

- [[_index]]
- [[cw-wiki]]
