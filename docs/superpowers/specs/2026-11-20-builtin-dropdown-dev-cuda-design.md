# Built-in model in the Embedder dropdown, and the graphics card in dev builds — Design

Date: 2026-11-20. Plan: `docs/superpowers/plans/2026-11-20-builtin-dropdown-dev-cuda.md`.

## Context

The owner, on the Semantic search settings page of a dev build (`target\debug\codewiki-app.exe`), asked:
1. The Embedder dropdown must list "Built-in model". Today `GET /v1/settings` lists `builtin` only once a model is installed (`crates/cw-server/src/settings.rs:74-95`).
2. Why "GTE ModernBERT Base, on the processor" is recommended on a machine with an RTX 4070 (12 GB).

Findings:
- **Cause of 2:** `recommend(hardware, cuda)` (`crates/cw-models/src/recommend.rs:47`) picks the card tier only when `cw_models::worker::cuda_usable()` is true. That needs `cuda\codewiki-embed-cuda.exe` beside the running program (`crates/cw-models/src/worker.rs:478`). Only `scripts\package.ps1` builds it, so a dev build has none. An installed package on this machine recommends Qwen3 Embedding 0.6B on the card.
- **Latent bug:** picking `builtin` in the dropdown fails even with a model installed. `settings::change` clears `embed.model` when the provider crosses to `builtin`, then refuses for want of a model (`settings.rs:280-302`). Only the cards' Install/Use (`/v1/models/{id}/install|use`) set provider and model together.
- A `builtin` embedder that cannot build is already handled: `semantic::built` logs and turns semantic search off (`crates/cw-wiki/src/semantic.rs:40`); `settings` reports it as `embed.problem`; `models::resume` does nothing (`crates/cw-server/src/models.rs:601`).

## Goals

- "Built-in model" is always in the Embedder dropdown where the engine runs.
- A development build can run a built-in model on the NVIDIA card, so it recommends and uses what an installed package would.

## Non-goals

- Changing the recommendation rule.
- Building the dev worker on every `cargo build`.
- RustRover run configurations.

## Design (owner's decisions)

- **Dropdown, owner's pick: "saved pending provider".** `builtin` is listed wherever the engine exists (`cw_models::ENGINE`, Windows). Choosing it with no model installed saves `embed.provider = builtin` with no model. "In use" then says to choose a model below, and search is full-text alone until Install or Use.
- **GPU in dev, owner's pick: "shared script".** `scripts\cuda-worker.ps1` builds `codewiki-embed-cuda` and fills `<target>\<debug|release>\cuda\` with the worker and NVIDIA's three DLLs. `package.ps1` calls it with `-Release`.

## Approaches considered

- Dropdown:
  - (a) An in-app picking state that saves nothing until Install or Use. The owner first leaned this way.
  - (b) A saved pending provider. Chosen.
- GPU in dev:
  - (a) The shared script. Chosen.
  - (b) A variable pointing a dev build at an installed package's worker. Rejected: a dev-only knob in product code, and the worker would lag behind the source.

## Calls left to the builder (made, owner may correct)

- With a model installed, the dropdown's "Built-in model" starts Use with the recommended model if installed, else the first installed one in catalog order. Saving a pending state there would leave search off for nothing.
- No scrolling to the cards. The note sits in the Embedder card, directly above the Built-in model card.
- Home's "Search by meaning" offer also shows while `builtin` is pending.
- The dev worker is built in the dev profile (candle already has opt-level 3 there), with `CUDA_COMPUTE_CAP=75` like the package. DLLs are hard links from the toolkit when on the same volume, else copies.
- The page text about the missing worker stays as is: the owner did not ask for both.
