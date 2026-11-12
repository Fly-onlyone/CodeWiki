---
tags: [architecture, flow]
---

# codewiki Startup Flow

> Call sequence from the `main` entry point of `codewiki` across 10 parts of the codebase.

## Source

- `crates/codewiki/src/main.rs` — entry point

## How it works

```mermaid
sequenceDiagram
    autonumber
    participant p0 as codewiki
    participant p1 as Serve Module
    participant p2 as Error
    participant p3 as Paths Module
    participant p4 as cw-index
    participant p5 as Index
    participant p6 as cw-llm
    participant p7 as Workspace Module (cw-server)
    participant p8 as VaultMeta
    participant p9 as Proposal Module
    p0->>p1: run()
    p0->>p2: invalid()
    p0->>p2: io()
    p0->>p2: not_found()
    p0->>p3: data_dir()
    p0->>p4: search()
    p0->>p5: find_symbols()
    p0->>p6: writer_from_env()
    p0->>p7: workspace()
    p0->>p7: workspace_contracts()
    p0->>p8: load()
    p0->>p8: save()
    p0->>p9: approve()
    p0->>p9: list()
```

The sequence follows resolved calls outward from `main`, keeping only calls that cross from one documented part into another. Calls reached through trait objects or macros do not appear.

## Depends on

- [[codewiki]]
- [[Serve Module]]
- [[Error]]
- [[Paths Module]]
- [[cw-index]]
- [[Index]]
- [[cw-llm]]
- [[Workspace Module (cw-server)]]
- [[VaultMeta]]
- [[Proposal Module]]

## See also

- [[_index]]
- [[Crate Dependency Flow]]
