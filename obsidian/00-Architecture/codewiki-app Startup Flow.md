---
tags: [architecture, flow]
---

# codewiki-app Startup Flow

> Call sequence from the `main` entry point of `codewiki-app` across 8 parts of the codebase.

## Source

- `crates/codewiki-app/src/main.rs` — entry point

## How it works

```mermaid
sequenceDiagram
    autonumber
    participant p0 as codewiki-app
    participant p1 as Themes Module
    participant p2 as Sink
    participant p3 as Backend
    participant p4 as Markdown Module
    participant p5 as cw-client
    participant p6 as Paths Module
    participant p7 as Palette Module
    p0->>p1: init()
    p0->>p2: value()
    p0->>p3: connect()
    p0->>p3: repo_id()
    p0->>p4: search_page()
    p0->>p5: encode()
    p3->>p5: codewiki_binary()
    p3->>p5: is_healthy()
    p3->>p5: read_backend_file()
    p3->>p6: data_dir()
    p0->>p7: new()
    p0->>p7: actions()
    p0->>p7: filter()
    p0->>p1: list()
```

The sequence follows resolved calls outward from `main`, keeping only calls that cross from one documented part into another. Calls reached through trait objects or macros do not appear.

## Depends on

- [[codewiki-app]]
- [[Themes Module]]
- [[Sink]]
- [[Backend]]
- [[Markdown Module]]
- [[cw-client]]
- [[Paths Module]]
- [[Palette Module]]

## See also

- [[_index]]
- [[Crate Dependency Flow]]
