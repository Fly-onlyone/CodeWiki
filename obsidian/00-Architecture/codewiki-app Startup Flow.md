---
tags: [architecture, flow]
---

# codewiki-app Startup Flow

> Call sequence from the `main` entry point of `codewiki-app` across 14 parts of the codebase.

## Source

- `crates/codewiki-app/src/main.rs` — entry point

## How it works

```mermaid
sequenceDiagram
    autonumber
    participant p0 as codewiki-app
    participant p1 as Backdrop Module
    participant p2 as Fonts Module
    participant p3 as Logfile Module
    participant p4 as Motion Module
    participant p5 as Themes Module
    participant p6 as Backend
    participant p7 as Update Module
    participant p8 as cw-client
    participant p9 as Doc View Module
    participant p10 as Startup Module
    participant p11 as Sink
    participant p12 as Glass Module
    participant p13 as Logs Module
    p0->>p1: new()
    p0->>p2: register()
    p0->>p3: init()
    p0->>p4: init()
    p0->>p5: init()
    p0->>p6: shutdown()
    p0->>p7: installing()
    p0->>p8: uses_remote()
    p0->>p9: new()
    p0->>p10: recent()
    p0->>p11: value()
    p1->>p9: read_by()
    p1->>p12: debug_backdrop()
    p3->>p13: path()
```

The sequence follows resolved calls outward from `main`, keeping only calls that cross from one documented part into another. Calls reached through trait objects or macros do not appear.

## Depends on

- [[codewiki-app]]
- [[Backdrop Module]]
- [[Fonts Module]]
- [[Logfile Module]]
- [[Motion Module]]
- [[Themes Module]]
- [[Backend]]
- [[Update Module]]
- [[cw-client]]
- [[Doc View Module]]
- [[Startup Module]]
- [[Sink]]
- [[Glass Module]]
- [[Logs Module]]

## See also

- [[_index]]
- [[Crate Dependency Flow]]
