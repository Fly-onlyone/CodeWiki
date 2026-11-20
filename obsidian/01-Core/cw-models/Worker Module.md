---
tags: [core, cw-models, module]
---

# Worker Module

> The child process that holds a model, seen from both sides.

## What it does

A model is run by a hidden child of the program, never by the program itself: native model code can end its process (out of memory, a fault in a graphics driver), and a backend runs for weeks. The two speak JSON lines over the child's standard input and output:

```text
child:  {"protocol":1,"device":"cpu","dimensions":768}        once, when the model is loaded;
                                                               or {"error":"..."} and it ends
parent: {"id":1,"kind":"document","texts":["...","..."]}
child:  {"id":1,"vectors":[[...],[...]]}                      or {"id":1,"error":"..."}
```

The child ends when its input closes, so none outlives its parent.

`serve` is the child's loop; the engine crate gives it the function that embeds. `Supervisor` is the parent's side: it starts the child on the first request, gives up on one that hangs or dies, and stops one that is idle.

## Source

- `crates/cw-models/src/worker.rs` — module `cw_models::worker`

## How it works

1. `recommend` in [[Model Module (codewiki)]] and `BuiltinEmbedder::new` in [[BuiltinEmbedder]] call `cuda_program`.
2. `describe` in [[Models Module]] and `describe` in [[Settings Module (cw-server)]] call `cuda_usable`. `cuda_usable` calls `cuda_program`, then `nvidia_card` in [[Hardware Module]].
3. `describe` and `remove` in [[Models Module]], and 3 more, call `global`. `global` calls `Supervisor::new` in [[Supervisor]].
4. `worker` in [[cw-embed-engine]] calls `serve`.
5. `worker` in [[cw-embed-engine]] calls `refuse`.

1 more entry point: `wait_line`.

Calls are listed in the order of the source; a condition or a loop around a call is not shown.

## Items

- `pub const PROTOCOL: u32` — version of the lines above. A child that speaks another one is refused, which can only happen while an update replaces the programs.
- `const LOAD_TIMEOUT: Duration` — how long a child may take to load its model and say hello.
- `const REPLY_TIMEOUT: Duration` — how long a child may take over one request. A batch is a handful of texts; a child that takes longer hangs.
- `pub const IDLE: Duration` — how long a child stays without a request.
- `const CUDA_FAILURES: u32` — failures in a row after which the graphics card is given up for the processor.
- `pub enum Kind` — whether a text is what a user asks or what is searched.
- `pub struct Hello` — the child's first line.
- `pub type Embedded = std::result::Result<Vec<Vec<f32>>, String>` — what the child's embedding gives: one vector per text, or why not.
- `serve` — the child's loop: say hello, then answer requests until the input closes. `embed` must put no text of its input into an error: the parent logs it.
- `pub fn refuse(mut output: impl Write, reason: &str) -> std::io::Result<()>` — the child's only line when it cannot load its model; it ends after it. `reason` must hold nothing secret: the parent logs it.
- `pub struct Spec` — which child is wanted.
- `pub struct Channel` — the two ends of a running child, and the way to end it.
- `pub trait Launcher: Send + Sync` — starts a child. The real one starts a program; a test starts a thread.
- `pub struct Status` — what a page shows about the child.
- [[Supervisor]] — the parent's side: one child at a time, started when needed.
- `wait_line` — private function, returns `Result<String>`; called by `Supervisor::ask`, `Supervisor::start`.
- `pub fn cuda_program() -> Option<PathBuf>` — the program that runs models on the graphics card: the same worker, built with CUDA, in the folder `cuda` beside this program with the NVIDIA libraries it needs. `None` when this installation has none.
- `pub fn cuda_usable() -> bool` — whether a model can run on a graphics card here: this installation has the worker for it, and the machine has an NVIDIA card. Every installed package has the worker; most machines have no such card.
- `pub struct ProgramLauncher` — starts the worker as a hidden program: `codewiki` for the processor, `cuda_program` for the graphics card.
- `pub fn global() -> &'static Supervisor` — the supervisor of this process. Every built-in embedder asks it, so the process holds one child however many embedders were made.

And 6 more items.

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
