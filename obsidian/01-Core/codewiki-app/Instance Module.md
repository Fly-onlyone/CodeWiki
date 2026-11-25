---
tags: [core, codewiki-app, module]
---

# Instance Module

> One running app for each installed program.

## What it does

The app stays in the tray when its window closes, so starting it again must not start a second one: the launch hands what it was asked to open to the app that is running, which shows its window, and ends.

"The same app" is the same program file. An installed copy and a build from the sources are two apps, each with its window; otherwise a build under test would hand over to the installed version. `CODEWIKI_INSTANCE=new` starts a separate app whatever is running, for tests and for checks that must not bring another window to the front.

Two files in the settings folder, both named after the program file:

- `instance-<key>.lock` is held locked by the running app for as long as it lives. Windows drops the lock when the process ends, however it ends.
- `instance-<key>.txt` says where that app listens: `port=`, `token=` and `pid=`, one per line. It is a second file because a locked file cannot be read by another process.

A launch that finds the lock taken sends one line of JSON to `127.0.0.1:<port>` and waits for `ok`. The token keeps a program that merely found the port from driving the app; it guards against nothing that can read the user's settings folder.

The app never refuses to start. A launch that can neither take the lock nor reach the running app (it is just starting, or just ending) tries again for a few seconds and then runs by itself.

## Source

- `crates/codewiki-app/src/instance.rs` — module `codewiki_app::instance`

## How it works

1. `main` in [[codewiki-app]] calls `acquire`. `acquire` calls `dir` in [[Prefs Module]], then `Request::new`, then `acquire_in`, then `key`. `Request::new` calls `named_repo` in [[Startup Module]]. `acquire_in` calls `listen`, then `hand_over`.
2. `main` in [[codewiki-app]] calls `hold`.
3. `AppView::install_update` and `main` in [[codewiki-app]] call `release`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `const PROTOCOL: u64` — version of the line a launch sends. A running app of another version answers `no` to a line it does not know, and the launch runs by itself.
- `const CONNECT_TIMEOUT: Duration` — how long connecting to the running app may take; it is on this machine.
- `const READ_TIMEOUT: Duration` — how long the running app waits for the line of a launch that connected.
- `const LINE_LIMIT: u64` — the longest line the running app reads: a path, a view and a token.
- `pub struct Request` — what a second launch asks the running app to do.
- `pub enum Acquired` — how this launch stands to the app that may be running already.
- `pub struct Instance` — the lock of the running app and the listener launches reach it through.
- `static HELD: Mutex<Option<Instance>>` — the instance this process holds, kept so that `release` finds it.
- `pub fn acquire(launch: &Launch, deliver: impl Fn(Request) -> bool + Send + 'static) -> Acquired` — become the running app, or hand `launch` to the one that is. `deliver` receives, on another thread, each request a later launch hands over; it returns whether the app took it.
- `pub fn hold(instance: Instance)` — keep `instance` for `release`.
- `pub fn release()` — give up being the running app, if this process is it.
- `acquire_in` — `acquire` with everything it reads from the system passed in.
- `listen` — start listening for later launches and say where in the info file.
- `fn read_request(stream: &TcpStream, token: &str) -> Option<Request>` — the request a launch that connected sends, when it knows the token.
- `fn hand_over(info: &Path, request: &Request) -> bool` — send `request` to the running app the info file names. True when it answered that it took it.
- `struct Target` — where a running app listens, from its info file.
- `fn key(exe: &Path) -> String` — the name the two files carry for the program at `exe`: the same for the same file however its path is spelled in upper and lower case, as Windows reads paths. FNV-1a, because the name must be the same in every process.
- `fn new_token() -> String` — 128 bits nobody can guess, from the randomly seeded hashers the standard library keeps for its maps.
- `fn encode(request: &Request, token: &str) -> String` — the line a launch sends.
- `fn decode(line: &str, token: &str) -> Option<Request>` — the request in `line`, when it is of this version and carries `token`.

And 7 more items.

## Depends on

- [[Startup Module]] — uses `Launch`, `named_repo`
- [[Diagrams Module]] — uses `store`
- [[Icons Module]] — uses `load`
- [[Prefs Module]] — uses `dir`
- [[Review Module]] — uses `name`
- [[Win32 Module]] — uses `allow_foreground`

## Used by

- [[codewiki-app]] — from `AppView`, `Event`, `install_update`, `main`

## See also

- [[_index]]
- [[codewiki-app]]
