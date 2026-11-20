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
    participant p1 as Logs Module
    participant p2 as cw-embed-engine
    participant p3 as Model Module (codewiki)
    participant p4 as Serve Module
    participant p5 as Error
    participant p6 as Paths Module
    participant p7 as Resolved
    participant p8 as cw-index
    participant p9 as Index
    p0->>p1: init()
    p0->>p1: init_console()
    p0->>p2: worker()
    p0->>p3: run()
    p0->>p4: run()
    p0->>p5: invalid()
    p0->>p5: io()
    p0->>p5: not_found()
    p0->>p6: data_dir()
    p0->>p7: load()
    p0->>p8: search()
    p0->>p9: files()
    p0->>p9: find_symbols()
    p0->>p9: symbols()
```

The sequence follows resolved calls outward from `main`, keeping only calls that cross from one documented part into another. Calls reached through trait objects or macros do not appear.

## Depends on

- [[codewiki]]
- [[Logs Module]]
- [[cw-embed-engine]]
- [[Model Module (codewiki)]]
- [[Serve Module]]
- [[Error]]
- [[Paths Module]]
- [[Resolved]]
- [[cw-index]]
- [[Index]]

## See also

- [[_index]]
- [[Crate Dependency Flow]]
