---
tags: [architecture, flow]
---

# codewiki Startup Flow

> Call sequence from the `main` entry point of `codewiki` across 11 parts of the codebase.

## Source

- `crates/codewiki/src/main.rs` — entry point

## How it works

```mermaid
sequenceDiagram
    autonumber
    participant p0 as codewiki
    participant p1 as Logs Module
    participant p2 as Serve Module
    participant p3 as Error
    participant p4 as Paths Module
    participant p5 as Resolved
    participant p6 as cw-index
    participant p7 as Index
    participant p8 as cw-llm
    participant p9 as Workspace Module (cw-server)
    participant p10 as VaultMeta
    p0->>p1: init()
    p0->>p1: init_console()
    p0->>p2: run()
    p0->>p3: invalid()
    p0->>p3: io()
    p0->>p3: not_found()
    p0->>p4: data_dir()
    p0->>p5: load()
    p0->>p6: search()
    p0->>p7: find_symbols()
    p0->>p8: writer_from()
    p0->>p9: workspace()
    p0->>p9: workspace_contracts()
    p0->>p10: load()
```

The sequence follows resolved calls outward from `main`, keeping only calls that cross from one documented part into another. Calls reached through trait objects or macros do not appear.

## Depends on

- [[codewiki]]
- [[Logs Module]]
- [[Serve Module]]
- [[Error]]
- [[Paths Module]]
- [[Resolved]]
- [[cw-index]]
- [[Index]]
- [[cw-llm]]
- [[Workspace Module (cw-server)]]
- [[VaultMeta]]

## See also

- [[_index]]
- [[Crate Dependency Flow]]
