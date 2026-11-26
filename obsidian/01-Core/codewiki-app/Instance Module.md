---
tags: [core, codewiki-app, module]
---

# Instance Module

> One running app for each installed program.

## What it does

The app keeps running in the tray when its window closes. So a second launch, for example a click on a shortcut or a folder passed on the command line, must not start a second app. This module makes sure only one app runs for each installed program file. A later launch passes its request (a folder and a view) to the running app and ends. The running app shows its window.

"The same app" means the same program file. An installed copy and a development build are two apps. Set `CODEWIKI_INSTANCE=new` to start a separate app whatever runs, for tests.

## Source

- `crates/codewiki-app/src/instance.rs` — module `codewiki_app::instance`

## How it works

`main` calls `acquire` before any GPUI code. `acquire` builds a `Request` from the launch arguments and a key from the program path, then calls `acquire_in`. That function opens `instance-<key>.lock` in the settings folder and tries to lock it.

If the lock is free, this process is the running app. `listen` binds a TCP port on 127.0.0.1, creates a random token, writes port, token and process id to `instance-<key>.txt` (written to a draft file and renamed, so nobody reads half of it), and starts a thread. For each connection the thread reads one line, and checks the token. It passes the request to `deliver`, and answers `ok` or `no`. `main` keeps the result with `hold`.

If the lock is taken, `hand_over` reads the info file, connects, and sends one JSON line. It also lets the running app take the foreground window. On `ok`, `acquire` returns `HandedOver` and `main` exits.

When neither works, `acquire_in` sleeps and retries until a timeout. Then it returns `Separate`. `release` removes the info file and wakes the listener thread so it frees the port. It runs on quit and before an update starts the new version.

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

## Gotchas

- The app never refuses to start. Any failure (no settings folder, lock error, no listener) gives `Separate`: a second app, not an error.
- The lock file is held for the life of the process. Windows drops it when the process ends, even in a crash. The info file is separate because another process cannot read a locked file.
- The token only stops a program that found the port by chance. It does not protect against anything that can read the user's settings folder.
- `PROTOCOL` is the version of the line. An app of another version answers `no`, and the launch then runs by itself.
- The key is built from the program path with FNV-1a, ignoring case, and must match in every process. Do not switch to a hasher with a random seed.
- A launch that arrives while the running app is starting or ending may wait a few seconds before it gives up and runs alone.

## See also

- [[_index]]
- [[codewiki-app]]
