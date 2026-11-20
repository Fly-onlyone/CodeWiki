---
tags: [core, cw-models, module]
---

# Worker Module

> The child process that holds a model, seen from both sides.

## Source

- `crates/cw-models/src/worker.rs` — module `cw_models::worker`

## How it works

Main items:

- [[Supervisor]] — the parent's side: one child at a time, started when needed.
- `global` — the supervisor of this process.
- `ProgramLauncher` — starts the worker as a hidden program: `codewiki` for the processor, `cuda_program` for the graphics card.
- `Spec` — which child is wanted.
- `Launcher` — starts a child.
- `Hello` — the child's first line.
- `Status` — what a page shows about the child.
- `Kind` — whether a text is what a user asks or what is searched.

## Depends on

- [[Supervisor]] — uses `Supervisor`, `new`, `tick`
- [[Error]] — uses `external`, `invalid`
- [[Error Module]] — uses `Result`
- [[Logs Module]] — uses `flush`
- [[Game Module]] — uses `busy`
- [[Hardware Module]] — uses `nvidia_card`
- [[Device]] — uses `Device`

## Used by

- [[Supervisor]] — from `Supervisor`, `ask`, `device`, `embed`
- [[BuiltinEmbedder]] — from `BuiltinEmbedder`, `embed`, `embed_query`, `new`
- [[Models Module]] — from `describe`, `remove`, `wait_out_game`
- [[cw-embed-engine]] — from `worker`
- [[Model Module (codewiki)]] — from `recommend`
- [[Settings Module (cw-server)]] — from `describe`
- [[Engine]] — from `embed`

## See also

- [[_index]]
- [[cw-models]]
