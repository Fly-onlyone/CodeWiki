---
tags: [architecture, flow]
---

# codewiki-app Startup Flow

> Call sequence from the `main` entry point of `codewiki-app` across 12 parts of the codebase.

## Source

- `crates/codewiki-app/src/main.rs` — entry point

## How it works

```mermaid
sequenceDiagram
    autonumber
    participant p0 as codewiki-app
    participant p1 as Prefs Module
    participant p2 as Logs Module
    participant p3 as cw-telemetry
    participant p4 as Instance Module
    participant p5 as Startup Module
    participant p6 as Win32 Module
    participant p7 as Motion Module
    participant p8 as Backdrop Module
    participant p9 as Fonts Module
    participant p10 as Themes Module
    participant p11 as Zoom Module
    p0->>p1: read()
    p0->>p2: level_named()
    p0->>p3: init()
    p0->>p4: acquire()
    p0->>p5: launch()
    p0->>p4: hold()
    p0->>p6: on_console_stop()
    p0->>p7: init()
    p0->>p8: init()
    p0->>p9: register()
    p0->>p10: init()
    p0->>p11: init()
    p0->>p11: step_by()
    p0->>p11: reset()
```

The sequence follows resolved calls outward from `main`, keeping only calls that cross from one documented part into another. Calls reached through trait objects or macros do not appear.

## Depends on

- [[codewiki-app]]
- [[Prefs Module]]
- [[Logs Module]]
- [[cw-telemetry]]
- [[Instance Module]]
- [[Startup Module]]
- [[Win32 Module]]
- [[Motion Module]]
- [[Backdrop Module]]
- [[Fonts Module]]
- [[Themes Module]]
- [[Zoom Module]]

## See also

- [[_index]]
- [[Crate Dependency Flow]]
