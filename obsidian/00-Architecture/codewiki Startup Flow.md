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
    participant p1 as cw-telemetry
    participant p2 as Logs Module
    participant p3 as Briefing Module
    participant p4 as Paths Module
    participant p5 as Serve Module
    participant p6 as Error
    participant p7 as Resolved
    participant p8 as Semantic Module
    participant p9 as Service Module
    p0->>p1: init()
    p0->>p2: init_console()
    p0->>p3: vault_root()
    p0->>p3: session_start()
    p0->>p1: sending()
    p0->>p4: data_dir()
    p0->>p1: send_test_report()
    p1->>p2: init_with()
    p0->>p5: run()
    p0->>p6: io()
    p0->>p7: load()
    p0->>p8: configure()
    p0->>p9: update()
    p0->>p9: open_index()
```

The sequence follows resolved calls outward from `main`, keeping only calls that cross from one documented part into another. Calls reached through trait objects or macros do not appear.

## Depends on

- [[codewiki]]
- [[cw-telemetry]]
- [[Logs Module]]
- [[Briefing Module]]
- [[Paths Module]]
- [[Serve Module]]
- [[Error]]
- [[Resolved]]
- [[Semantic Module]]
- [[Service Module]]

## See also

- [[_index]]
- [[Crate Dependency Flow]]
