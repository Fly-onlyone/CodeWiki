---
tags: [architecture, flow]
---

# codewiki Startup Flow

> Call sequence from the `main` entry point of `codewiki` across 8 parts of the codebase.

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
    participant p5 as Agents Off Module
    participant p6 as Auto Explain Module
    participant p7 as Git Module
    p0->>p1: init()
    p0->>p2: init_console()
    p0->>p3: vault_root()
    p0->>p4: data_dir()
    p0->>p5: load()
    p0->>p5: blocking()
    p0->>p6: load()
    p0->>p6: load_count()
    p0->>p3: explain_for()
    p0->>p7: is_linked_worktree()
    p0->>p3: session_start()
    p0->>p1: sending()
    p0->>p1: send_test_report()
    p0->>p5: unreadable()
```

`main` first parses the command line with clap. Then it picks one of four paths, in this order.

1. **The session-start hook.** `codewiki hook session-start` (with `--agent claude-code` or `codex`) calls `session_start_hook` inside `catch_unwind`, with the panic hook silenced. It resolves the project folder (from `CLAUDE_PROJECT_DIR` when `--repo` is the default), finds the vault root with `vault_root`, and reads the list of folders kept from agents (`agents_off::load` and `blocking`). Both the session folder and the vault root are checked. If either is blocked, or the list cannot be read, it prints nothing. Otherwise it reads the auto explain switch and count, asks `explain_for` whether this session should be ordered to explain notes, and prints the briefing from `session_start`. Any session that is not already plain facts is forced back to facts when its folder, or its vault root, is a linked git worktree (`is_linked_worktree`). The exit status is always success.
2. **`sentry-test`.** Calls `cw_telemetry::init`, checks `sending()`, and sends one report with `send_test_report`, or panics on request.
3. **`agents`.** Edits or lists the folders kept from coding agents, using `file`, `unreadable` and the other `agents_off` functions.
4. **Everything else, through `run`.** Logging starts first: `serve` calls `cw_telemetry::init` (the only command that sends error reports), every other command calls `init_console` at warning level. `run` hands `serve` to its own module at once. For the rest it resolves the repository root, loads the settings through `data_dir` and configures the embedder from them, then dispatches to the matching `cw_wiki` function.

The diagram lists only calls that cross into another documented part. The long list of analysis commands such as `health` or `risk` is not in it, because those calls go to `cw-wiki` and `cw-server`, which have their own notes.

## Depends on

- [[codewiki]]
- [[cw-telemetry]]
- [[Logs Module]]
- [[Briefing Module]]
- [[Paths Module]]
- [[Agents Off Module]]
- [[Auto Explain Module]]
- [[Git Module]]

## Gotchas

- The hook must never disturb an agent session. A panic is caught and hidden, an argument error exits with success when the command line holds `hook`, and any failure prints nothing.
- `hook_input` reads stdin on a helper thread, at most 64 KB, and waits at most one second. A host that keeps stdin open cannot hold up the session start. With auto explaining off, stdin is not read at all.
- A folder closed to coding agents gets no briefing. The same holds for a vault root that is closed, and when the list file is unreadable: when in doubt, the hook says nothing.
- A linked git worktree never gets the order to explain notes. The worktree check runs on the session folder and, when it differs, on the vault root, because a worktree inside the main checkout finds the main checkout's vault.
- Only `serve` starts error reporting. Other commands log warnings and errors to the console and send nothing.
- `serve`, `sentry-test`, `agents` and `hook` are handled before the repository path is resolved, so they work outside a repository. The later `unreachable!` arms depend on that order.
- `agents on` and `agents off` read the list first. A list that cannot be read is reported and never overwritten.

## See also

- [[_index]]
- [[Crate Dependency Flow]]
