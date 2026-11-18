---
tags: [core, codewiki-app, module]
---

# Instance Module

> One running app for each installed program.

## Source

- `crates/codewiki-app/src/instance.rs` — module `codewiki_app::instance`

## How it works

Main items:

- `Instance` — the lock of the running app and the listener launches reach it through.
- `Request` — what a second launch asks the running app to do.
- `Acquired` — how this launch stands to the app that may be running already.
- `release` — give up being the running app, if this process is it.
- `acquire` — become the running app, or hand `launch` to the one that is.
- `hold` — keep `instance` for `release`.
- `HELD` — the instance this process holds, kept so that `release` finds it.
- `PROTOCOL` — version of the line a launch sends.

## Depends on

- [[Startup Module]] — uses `Launch`, `named_repo`
- [[Diagrams Module]] — uses `store`
- [[Icons Module]] — uses `load`
- [[Prefs Module]] — uses `dir`
- [[Win32 Module]] — uses `allow_foreground`

## Used by

- [[codewiki-app]] — from `AppView`, `Event`, `install_update`, `main`

## See also

- [[_index]]
- [[codewiki-app]]
