---
tags: [moc, home]
---

# CodeWiki — Knowledge Vault

A graph-linked tour of the codebase. **Start with the architecture flows**, then drill into crates, modules and patterns.

> **CodeWiki** is a Rust codebase of 16 crates and 166 source files. This vault maps its architecture, modules and conventions.

## 🧭 Start here — Architecture flows

- [[Crate Dependency Flow]] — how the 16 workspace crates depend on each other, from entry points down to shared foundations.
- [[codewiki Startup Flow]] — call sequence from the `main` entry point of `codewiki` across 10 parts of the codebase.
- [[codewiki-app Startup Flow]] — call sequence from the `main` entry point of `codewiki-app` across 8 parts of the codebase.

→ [[00-Architecture/_index|All architecture concepts]]

## 🦀 Core

- [[01-Core/codewiki-app/_index|codewiki-app/]] — CodeWiki desktop app for Windows.
- [[01-Core/codewiki-mcp/_index|codewiki-mcp/]] — MCP stdio bridge between coding agents and the CodeWiki backend.
- [[01-Core/codewiki/_index|codewiki/]] — CodeWiki command line and backend server.
- [[01-Core/cw-client/_index|cw-client/]] — client for the CodeWiki backend: discovery, autostart and HTTP calls.
- [[01-Core/cw-core/_index|cw-core/]] — shared identifiers, hashing, configuration and error types for CodeWiki.
- [[01-Core/cw-embed/_index|cw-embed/]] — text embeddings for CodeWiki's semantic search: a built-in local embedder and HTTP providers.
- [[01-Core/cw-graph/_index|cw-graph/]] — graph construction, metrics, community detection and layout for CodeWiki.
- [[01-Core/cw-illustrate/_index|cw-illustrate/]] — CodeWiki's illustration component library and structural illustration builders.
- [[01-Core/cw-index/_index|cw-index/]] — SQLite index: files, symbols, edges, metrics and full-text search for CodeWiki.
- [[01-Core/cw-llm/_index|cw-llm/]] — LLM providers and the prose writer for CodeWiki.
- [[01-Core/cw-mermaid/_index|cw-mermaid/]] — Mermaid diagram rendering for CodeWiki.
- [[01-Core/cw-parse/_index|cw-parse/]] — source parsing and symbol extraction for CodeWiki.
- [[01-Core/cw-server/_index|cw-server/]] — HTTP API of the CodeWiki backend.
- [[01-Core/cw-vault/_index|cw-vault/]] — Obsidian vault model for CodeWiki: notes, manifest, name registry, linter, lifecycle and proposals.
- [[01-Core/cw-wiki/_index|cw-wiki/]] — concept selection and wiki generation pipeline for CodeWiki.
- [[01-Core/openui-lang/_index|openui-lang/]] — parser and static evaluator for OpenUI Lang.

## 🔌 External Integrations

→ [[03-Integrations/_index|All integrations]] — axum, gpui_kit, rusqlite, serde, serde_json, tree_sitter.

## 🧩 Patterns

→ [[04-Patterns/_index|All patterns]] — Implementing Embedder Pattern, Implementing LlmProvider Pattern.

## 🛠️ Operations

→ [[05-Operations/_index|All operations]] — Build and Test.

## 🎯 Reading paths

**New contributor:** [[Crate Dependency Flow]] → [[cw-core]] → [[Error]] → [[Build and Test]]
