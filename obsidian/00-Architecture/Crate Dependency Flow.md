---
tags: [architecture, flow]
---

# Crate Dependency Flow

> How the 16 workspace crates depend on each other, from entry points down to shared foundations.

## Source

- `crates/codewiki-app/Cargo.toml` — declares dependencies
- `crates/codewiki-mcp/Cargo.toml` — declares dependencies
- `crates/codewiki/Cargo.toml` — declares dependencies
- `crates/cw-client/Cargo.toml` — declares dependencies
- `crates/cw-core/Cargo.toml` — declares dependencies
- `crates/cw-embed/Cargo.toml` — declares dependencies
- `crates/cw-graph/Cargo.toml` — declares dependencies
- `crates/cw-illustrate/Cargo.toml` — declares dependencies
- `crates/cw-index/Cargo.toml` — declares dependencies
- `crates/cw-llm/Cargo.toml` — declares dependencies
- `crates/cw-mermaid/Cargo.toml` — declares dependencies
- `crates/cw-parse/Cargo.toml` — declares dependencies
- `crates/cw-server/Cargo.toml` — declares dependencies
- `crates/cw-vault/Cargo.toml` — declares dependencies
- `crates/cw-wiki/Cargo.toml` — declares dependencies
- `crates/openui-lang/Cargo.toml` — declares dependencies

## How it works

```mermaid
flowchart LR
    n0[codewiki-app]
    n1[codewiki-mcp]
    n2[codewiki]
    n3[cw-client]
    n4[cw-core]
    n5[cw-embed]
    n6[cw-graph]
    n7[cw-illustrate]
    n8[cw-index]
    n9[cw-llm]
    n10[cw-mermaid]
    n11[cw-parse]
    n12[cw-server]
    n13[cw-vault]
    n14[cw-wiki]
    n15[openui-lang]
    n0 --> n3
    n0 --> n7
    n0 --> n10
    n0 --> n15
    n1 --> n3
    n1 --> n4
    n2 --> n4
    n2 --> n8
    n2 --> n9
    n2 --> n12
    n2 --> n13
    n2 --> n14
    n3 --> n4
    n5 --> n4
    n6 --> n4
    n7 --> n15
    n8 --> n4
    n8 --> n11
    n9 --> n4
    n9 --> n7
    n9 --> n8
    n9 --> n13
    n9 --> n14
    n11 --> n4
    n12 --> n4
    n12 --> n8
    n12 --> n9
    n12 --> n13
    n12 --> n14
    n13 --> n4
    n14 --> n4
    n14 --> n5
    n14 --> n6
    n14 --> n7
    n14 --> n8
    n14 --> n11
    n14 --> n13
```

An arrow points from a crate to a crate it depends on. Foundations with no workspace dependencies: `cw-core`, `cw-mermaid`, and `openui-lang`. Nothing in the workspace depends on `codewiki-app`, `codewiki-mcp`, and `codewiki`.

## Depends on

- [[codewiki-app]] — used by 0 workspace crates
- [[codewiki-mcp]] — used by 0 workspace crates
- [[codewiki]] — used by 0 workspace crates
- [[cw-client]] — used by 2 workspace crates
- [[cw-core]] — used by 11 workspace crates
- [[cw-embed]] — used by 1 workspace crates
- [[cw-graph]] — used by 1 workspace crates
- [[cw-illustrate]] — used by 3 workspace crates
- [[cw-index]] — used by 4 workspace crates
- [[cw-llm]] — used by 2 workspace crates
- [[cw-mermaid]] — used by 1 workspace crates
- [[cw-parse]] — used by 2 workspace crates
- [[cw-server]] — used by 1 workspace crates
- [[cw-vault]] — used by 4 workspace crates
- [[cw-wiki]] — used by 3 workspace crates
- [[openui-lang]] — used by 2 workspace crates

## See also

- [[_index]]
