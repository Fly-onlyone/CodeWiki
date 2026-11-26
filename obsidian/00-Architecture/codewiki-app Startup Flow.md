---
tags: [architecture, flow]
---

# codewiki-app Startup Flow

> Call sequence from the `main` entry point of `codewiki-app` across 13 parts of the codebase.

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
    participant p9 as Tab Prefs Module
    participant p10 as Fonts Module
    participant p11 as Themes Module
    participant p12 as Zoom Module
    p0->>p1: read()
    p0->>p2: level_named()
    p0->>p3: init()
    p0->>p4: acquire()
    p0->>p5: launch()
    p0->>p4: hold()
    p0->>p6: on_console_stop()
    p0->>p7: init()
    p0->>p8: init()
    p0->>p9: init()
    p0->>p10: register()
    p0->>p11: init()
    p0->>p12: init()
    p0->>p12: step_by()
```

`main` runs in two parts: setup before GPUI, then setup inside the GPUI run loop.

Before GPUI, it reads the log level from the settings through the [[Prefs Module]]. `CODEWIKI_LOG` can name another level for one run. Every record at Info and above goes to the log file, because a release build has no console. It then starts [[cw-telemetry]], which sends error reports only if the user turned them on.

Next it asks the [[Instance Module]] for the single-instance slot, passing the launch request from the [[Startup Module]]. If another copy of the app already runs, the request is handed over to it and `main` returns with no window. A primary copy holds the slot. A separate copy runs on its own. The [[Win32 Module]] then makes a console stop (Ctrl+C) quit the app the way the tray does.

Inside the run loop, `gpui_kit::init` comes first, then the [[Motion Module]], the [[Backdrop Module]] and the [[Tab Prefs Module]], then the [[Fonts Module]] registers its fonts, then the [[Themes Module]] and finally the [[Zoom Module]]. Key bindings and the zoom actions are bound to the app. The window opens with the Mica backdrop and the `Shell` view, and the window-close, notification and quit hooks are set. On quit, the backend is asked to stop.

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
- [[Tab Prefs Module]]
- [[Fonts Module]]
- [[Themes Module]]
- [[Zoom Module]]

## Gotchas

- Instance handover must run before anything of GPUI. A handed-over launch ends with no window.
- Fonts register before the themes, which name the font families. The Zoom Module comes after the themes, because it takes the theme's sizes as its 100% sizes.
- Closing the window only hides it when a tray icon exists. Quit from the tray ends the app. Without a tray icon, close ends the app, unless unsaved edits keep the window open.
- The zoom key actions are bound to the app, not to a view. A focused button in the generated page would not reach the view's handlers otherwise.
- On quit the app does not wait for the backend. It asks the backend to stop, and the backend finishes its current work and exits itself.
- The diagram keeps only calls that cross between documented parts. Calls inside `main.rs` do not show.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
