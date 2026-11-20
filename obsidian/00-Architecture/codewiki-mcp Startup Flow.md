---
tags: [architecture, flow]
---

# codewiki-mcp Startup Flow

> Call sequence from the `main` entry point of `codewiki-mcp` across 4 parts of the codebase.

## Source

- `crates/codewiki-mcp/src/main.rs` — entry point

## How it works

```mermaid
sequenceDiagram
    autonumber
    participant p0 as codewiki-mcp
    participant p1 as Logs Module
    participant p2 as Paths Module
    participant p3 as Resolved
    p0->>p1: init()
    p1->>p2: data_dir()
    p1->>p3: load()
```

The sequence follows resolved calls outward from `main`, keeping only calls that cross from one documented part into another. Calls reached through trait objects or macros do not appear.

## Depends on

- [[codewiki-mcp]]
- [[Logs Module]]
- [[Paths Module]]
- [[Resolved]]

## See also

- [[_index]]
- [[Crate Dependency Flow]]
