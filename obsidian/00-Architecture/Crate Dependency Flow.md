---
tags: [architecture, flow]
---

# Crate Dependency Flow

> How the 20 workspace crates depend on each other, from entry points down to shared foundations.

## Source

- `crates/codewiki-app/Cargo.toml` — declares dependencies
- `crates/codewiki-mcp/Cargo.toml` — declares dependencies
- `crates/codewiki/Cargo.toml` — declares dependencies
- `crates/cw-client/Cargo.toml` — declares dependencies
- `crates/cw-core/Cargo.toml` — declares dependencies
- `crates/cw-embed-engine/Cargo.toml` — declares dependencies
- `crates/cw-embed/Cargo.toml` — declares dependencies
- `crates/cw-graph/Cargo.toml` — declares dependencies
- `crates/cw-illustrate/Cargo.toml` — declares dependencies
- `crates/cw-index/Cargo.toml` — declares dependencies
- `crates/cw-llm/Cargo.toml` — declares dependencies
- `crates/cw-mermaid/Cargo.toml` — declares dependencies
- `crates/cw-models/Cargo.toml` — declares dependencies
- `crates/cw-parse/Cargo.toml` — declares dependencies
- `crates/cw-server/Cargo.toml` — declares dependencies
- `crates/cw-telemetry/Cargo.toml` — declares dependencies
- `crates/cw-update/Cargo.toml` — declares dependencies
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
    n5[cw-embed-engine]
    n6[cw-embed]
    n7[cw-graph]
    n8[cw-illustrate]
    n9[cw-index]
    n10[cw-llm]
    n11[cw-mermaid]
    n12[cw-models]
    n13[cw-parse]
    n14[cw-server]
    n15[cw-telemetry]
    n16[cw-update]
    n17[cw-vault]
    n18[cw-wiki]
    n19[openui-lang]
    n0 --> n3
    n0 --> n4
    n0 --> n8
    n0 --> n11
    n0 --> n15
    n0 --> n16
    n0 --> n19
    n1 --> n3
    n1 --> n4
    n1 --> n15
    n2 --> n4
    n2 --> n9
    n2 --> n10
    n2 --> n12
    n2 --> n14
    n2 --> n15
    n2 --> n17
    n2 --> n18
    n3 --> n4
    n5 --> n4
    n5 --> n12
    n5 --> n15
    n6 --> n4
    n6 --> n12
    n7 --> n4
    n8 --> n19
    n9 --> n4
    n9 --> n13
    n10 --> n4
    n10 --> n8
    n10 --> n9
    n10 --> n17
    n10 --> n18
    n12 --> n4
    n13 --> n4
    n14 --> n4
    n14 --> n6
    n14 --> n9
    n14 --> n10
    n14 --> n12
    n14 --> n15
    n14 --> n17
    n14 --> n18
    n15 --> n4
    n17 --> n4
    n18 --> n4
    n18 --> n6
    n18 --> n7
    n18 --> n8
    n18 --> n9
    n18 --> n13
    n18 --> n17
```

An arrow points from a crate to a crate it depends on. Foundations with no workspace dependencies: `cw-core`, `cw-mermaid`, `cw-update`, and `openui-lang`. Nothing in the workspace depends on `codewiki-app`, `codewiki-mcp`, `codewiki`, and `cw-embed-engine`.

## Depends on

- [[codewiki-app]] — used by 0 workspace crates
- [[codewiki-mcp]] — used by 0 workspace crates
- [[codewiki]] — used by 0 workspace crates
- [[cw-client]] — used by 2 workspace crates
- [[cw-core]] — used by 15 workspace crates
- [[cw-embed-engine]] — used by 0 workspace crates
- [[cw-embed]] — used by 2 workspace crates
- [[cw-graph]] — used by 1 workspace crates
- [[cw-illustrate]] — used by 3 workspace crates
- [[cw-index]] — used by 4 workspace crates
- [[cw-llm]] — used by 2 workspace crates
- [[cw-mermaid]] — used by 1 workspace crates
- [[cw-models]] — used by 4 workspace crates
- [[cw-parse]] — used by 2 workspace crates
- [[cw-server]] — used by 1 workspace crates
- [[cw-telemetry]] — used by 5 workspace crates
- [[cw-update]] — used by 1 workspace crates
- [[cw-vault]] — used by 4 workspace crates
- [[cw-wiki]] — used by 3 workspace crates
- [[openui-lang]] — used by 2 workspace crates

## See also

- [[_index]]
