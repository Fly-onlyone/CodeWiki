---
tags: [core, codewiki, module]
---

# Embed Cuda Module

> The worker that runs a built-in embedding model on an NVIDIA card.

## What it does

The same code as `codewiki embed-worker`, built with the CUDA engine. It is a program of its own because a program that holds that engine does not start where NVIDIA's libraries are missing, and `codewiki` must start everywhere. The backend starts it, with the same arguments, only on a machine with an NVIDIA card:

```text
codewiki-embed-cuda embed-worker --model <id> --models-dir <folder>
```

## Source

- `crates/codewiki/src/bin/embed_cuda.rs` — module `codewiki::bin::embed_cuda`

## Items

- `fn value(arguments: &[String], name: &str) -> Option<String>` — the value that follows `name` among the arguments.
- `fn main() -> ExitCode` — private function, returns `ExitCode`.
- `fn run(model: &str, models_dir: &std::path::Path) -> ExitCode` — private function, returns `ExitCode`; called by `main`.

## Depends on

- [[cw-embed-engine]] — uses `worker`

## See also

- [[_index]]
- [[codewiki]]
