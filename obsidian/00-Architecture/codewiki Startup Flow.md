---
tags: [architecture, flow]
---

# codewiki Startup Flow

> Call sequence from the `main` entry point of `codewiki` across 6 parts of the codebase.

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
    participant p4 as Agents Off Module
    participant p5 as Paths Module
    p0->>p1: init()
    p0->>p2: init_console()
    p0->>p3: vault_root()
    p0->>p4: load()
    p0->>p5: data_dir()
    p0->>p4: blocking()
    p0->>p3: session_start()
    p0->>p1: sending()
    p0->>p1: send_test_report()
    p0->>p4: unreadable()
    p0->>p4: file()
    p0->>p4: normalize()
    p0->>p4: add()
    p0->>p4: remove()
```

The sequence follows resolved calls outward from `main`, keeping only calls that cross from one documented part into another. Calls reached through trait objects or macros do not appear.

## Depends on

- [[codewiki]]
- [[cw-telemetry]]
- [[Logs Module]]
- [[Briefing Module]]
- [[Agents Off Module]]
- [[Paths Module]]

## See also

- [[_index]]
- [[Crate Dependency Flow]]
