# Built-in model in the Embedder dropdown, and the graphics card in dev builds — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** List "Built-in model" in the Embedder dropdown at all times, saved as a pending choice until a model is installed, and let a development build run built-in models on the NVIDIA card.

**Architecture:** The backend stops hiding `builtin` and accepts it without a model (a "pending" state whose embedder fails to build, so search falls back to full-text). The app turns the dropdown's `builtin` entry into Use when a model is installed and shows a "choose a model below" note while pending. A new script builds the CUDA worker into `<target>\<profile>\cuda\` for dev runs; `package.ps1` reuses it.

**Tech Stack:** Rust (axum backend `cw-server`, GPUI app `codewiki-app`), PowerShell 7 scripts, candle with the `cuda` feature, CUDA 13.4.3 toolkit.

**Spec:** `docs/superpowers/specs/2026-11-20-builtin-dropdown-dev-cuda-design.md` (written in Task 0 from the Design section below).

---

## Context

The owner, on the Semantic search settings page of a dev build (`target\debug\codewiki-app.exe`), asked:
1. The Embedder dropdown must list "Built-in model". Today `GET /v1/settings` lists `builtin` only once a model is installed (`crates/cw-server/src/settings.rs:74-95`).
2. Why "GTE ModernBERT Base, on the processor" is recommended on a machine with an RTX 4070 (12 GB).

Findings:
- **Cause of 2:** `recommend(hardware, cuda)` (`crates/cw-models/src/recommend.rs:47`) picks the card tier only when `cw_models::worker::cuda_usable()` is true. That needs `cuda\codewiki-embed-cuda.exe` beside the running program (`crates/cw-models/src/worker.rs:478`). Only `scripts\package.ps1` builds it, so a dev build has none. An installed package on this machine recommends Qwen3 Embedding 0.6B on the card.
- **Latent bug:** picking `builtin` in the dropdown fails even with a model installed. `settings::change` clears `embed.model` when the provider crosses to `builtin`, then refuses for want of a model (`settings.rs:280-302`). Only the cards' Install/Use (`/v1/models/{id}/install|use`) set provider and model together.
- A `builtin` embedder that cannot build is already handled: `semantic::built` logs and turns semantic search off (`crates/cw-wiki/src/semantic.rs:40`); `settings` reports it as `embed.problem`; `models::resume` does nothing (`crates/cw-server/src/models.rs:601`).

## Design (owner's decisions)

- **Dropdown, owner's pick: "saved pending provider".** `builtin` is listed wherever the engine exists (`cw_models::ENGINE`, Windows). Choosing it with no model installed saves `embed.provider = builtin` with no model. "In use" then says to choose a model below, and search is full-text alone until Install or Use.
- **GPU in dev, owner's pick: "shared script".** `scripts\cuda-worker.ps1` builds `codewiki-embed-cuda` and fills `<target>\<debug|release>\cuda\` with the worker and NVIDIA's three DLLs. `package.ps1` calls it with `-Release`.
- **Approaches considered.** Dropdown: (a) an in-app picking state that saves nothing until Install/Use. The owner first leaned this way, then picked (b), the saved pending provider. GPU: (a) the shared script, chosen; (b) a variable pointing a dev build at an installed package's worker. (b) was rejected: a dev-only knob in product code, and the worker would lag behind the source.
- **My calls (owner may correct):**
  - With a model installed, the dropdown's "Built-in model" starts Use with the recommended model if installed, else the first installed one in catalog order. Saving a pending state there would leave search off for nothing.
  - No scrolling to the cards. The note sits in the Embedder card, directly above the Built-in model card.
  - Home's "Search by meaning" offer also shows while `builtin` is pending.
  - The dev worker is built in the dev profile (candle already has opt-level 3 there), with `CUDA_COMPUTE_CAP=75` like the package. DLLs are hard links from the toolkit when on the same volume, else copies.
  - The page text about the missing worker stays as is (the owner did not pick "Both").
- **Non-goals:** changing the recommendation rule; a dev worker built on every `cargo build`; RustRover run configurations.

## Global Constraints

- Work in a worktree made with the native `EnterWorktree` tool, branch `feat/builtin-dropdown-dev-cuda`. The worktree builds in its own target dir; never point `CARGO_TARGET_DIR` at `D:\CodeWiki\target`.
- CUDA toolkit: `D:\packages\cuda\13.4.3` (pass `-CudaToolkit`); CUDA 13.4 or lower.
- Format with `cargo fmt --all`, never RustRover's reformat. `cargo clippy --workspace --all-targets -- -D warnings` stays clean.
- Run tests per crate (`cargo test -p <crate>`); a workspace run exhausted memory before.
- Load `qingyi:commit` before every commit; subject `type(scope): imperative`.
- Keep disk use low: delete check data dirs afterwards; run `pwsh scripts\clean.ps1` at the end.
- User-facing copy is plain English in the existing voice (short sentences, no jargon).

## Review Focus

1. **`CODEWIKI_EMBED=builtin` set in the environment with no model.** Expect the same pending note and no crash; the dropdown is locked as today. Test: Task 2, `pending_note_shows_for_builtin_without_model` with source `env`.
2. **Pending, then the user picks Ollama.** Expect Ollama saved, model stays null, no "needs a model" left over. Test: Task 1, extend the switch-away assertions.
3. **Pending, with `embed.device = cuda` saved, on a machine without the worker.** Expect the provider accepted; the device is checked when a model is set. Test: Task 1, `pending_builtin_does_not_check_the_device`.
4. **Dev worker in use while the script runs again.** The exe and DLLs are locked. Expect a clear "stop CodeWiki first" error, not a half-replaced folder. Covered by the script's own check in Task 3, verified by hand.
5. **A test program never finds the dev worker** (`target\debug\deps\cuda` is absent). The device-options assertion `["auto","cpu"]` in `crates/cw-server/tests/api.rs:518-528` must still pass after the script has run. Verified in Task 3, step 6.

---

### Task 0: Worktree, spec and plan in the repo

**Files:**
- Create: `docs/superpowers/specs/2026-11-20-builtin-dropdown-dev-cuda-design.md` (Context + Design sections above)
- Create: `docs/superpowers/plans/2026-11-20-builtin-dropdown-dev-cuda.md` (this plan)

- [ ] **Step 1:** Load the `EnterWorktree` tool via ToolSearch and create the worktree on branch `feat/builtin-dropdown-dev-cuda`. If the native tool fails, use `git worktree add .claude/worktrees/feat+builtin-dropdown-dev-cuda -b feat/builtin-dropdown-dev-cuda` (`.claude/worktrees/` is already git-ignored).
- [ ] **Step 2:** Run the baseline: `cargo build -p codewiki -p codewiki-app`, `cargo test -p cw-server`, `cargo test -p codewiki-app`. Expect all to pass. Report any failures to the owner before going on.
- [ ] **Step 3:** Write both docs. Commit `docs(search): design built-in model in the embedder dropdown and the dev CUDA worker`.

### Task 1: Backend offers and accepts a pending built-in embedder

**Files:**
- Modify: `crates/cw-server/src/settings.rs:74-95` (options) and `:296-302` (refusal)
- Test: `crates/cw-server/tests/api.rs:331-335`, `:513-608`

**Interfaces:**
- Produces: `GET /v1/settings` → `embed.provider.options` holds `"builtin"` whenever `cw_models::ENGINE`. `PUT /v1/settings {"embed":{"provider":"builtin"}}` → 200; `embed.model.value` null, `embed.name` null, `embed.problem` contains `"needs a model"`. `GET /v1/models` → `"embedder": "builtin"`.

- [ ] **Step 1: Update the failing tests.**
  - `api.rs:332-335`: expected options become `if cfg!(windows) { ["local","builtin","ollama","openai","none"] } else { ["local","ollama","openai","none"] }`.
  - Rename `the_built_in_embedder_is_offered_and_taken_only_with_an_installed_model` to `the_built_in_embedder_is_offered_and_waits_for_a_model`.
  - Remove the `{provider: builtin}` → "needs a model" refusal case. In its place, PUT it and assert status 200, `["embed"]["provider"]["value"] == "builtin"`, `["embed"]["model"]["value"] == Null`, `["embed"]["name"] == Null`, and that `["embed"]["problem"]` contains `"needs a model"`.
  - Assert the options include `"builtin"` before `install_model`.
  - Keep the unknown-model and not-installed refusals.
  - Lines 594-602: switching to builtin after Ollama's `nomic-embed-text` is now accepted, with `model` null and the problem saying "needs a model".
  - Then PUT `{provider: "ollama"}` and assert `name == "ollama:embeddinggemma"`, `problem == Null` (Review Focus 2).
  - Add `pending_builtin_does_not_check_the_device`, cfg(windows): save `device: "cpu"` first, then `{provider:"builtin", device:"cuda"}` with no model. Expect 200, because nothing runs yet. Then PUT a model that is installed and expect the "no worker for the graphics card" refusal.
- [ ] **Step 2:** Run `cargo test -p cw-server --test api built_in` and `cargo test -p cw-server --test api settings`. Expect FAIL on the new assertions.
- [ ] **Step 3: Implement.**
  - Options: offer `builtin` when `cw_models::ENGINE`, not when a model is installed. Drop `has_model`. Update the comment: the built-in embedder is listed wherever the engine runs and may wait for a model.
  - In `change`: run `cw_embed::from_settings_in(&after, …)?` only when `after` resolves `EMBED_MODEL` to a value. With no model, accept, since `builtin` is pending. Update the comment.
- [ ] **Step 4:** Run `cargo test -p cw-server`. Expect PASS.
- [ ] **Step 5:** Commit `feat(server): offer the built-in embedder before a model is installed`.

### Task 2: App — dropdown entry, pending note, Home offer

**Files:**
- Modify: `crates/codewiki-app/src/settings_pages.rs` (`search` at :472, `select` at :306, `in_use` at :365)
- Modify: `crates/codewiki-app/src/live.rs:414` (`model_offer`)
- Test: `settings_pages.rs` tests module (`semantic_search_offers_the_embedders_a_test_and_a_sync` :1493, fixture `models()` :1101); `live.rs` tests near :831-862

**Interfaces:**
- Consumes: Task 1's `/v1/settings` shape; `/v1/models` fields `models[].installed`, `recommended.model`, `embedder`.
- Produces: `fn builtin_choice(models: Option<&Value>) -> Action` in `settings_pages.rs`. It returns `Action::UseModel(id)`, where `id` is the recommended model when installed, else the first installed one. With nothing installed, or without models, it returns `Action::SetSetting { section: "embed", field: "provider", value: Some("builtin") }`.

- [ ] **Step 1: Write failing tests** in `settings_pages.rs`:
  - `the_built_in_entry_waits_for_a_model_when_none_is_installed`. Set fixture options to include `"builtin"`. Assert `actions.contains(&set("embed","builtin"))` and that the words contain `"Built-in model"` among the options.
  - `the_built_in_entry_uses_an_installed_model`. Mark `models[0]` (granite) installed and assert the action list holds `Action::UseModel("granite-small-english-r2")` and not `set("embed","builtin")`. Then also mark `models[1]` (recommended GTE) installed and assert `UseModel("gte-modernbert-base")`.
  - `pending_note_shows_for_builtin_without_model`. Set provider value `"builtin"`, model null, name null, and problem `"the built-in embedder needs a model; set CODEWIKI_EMBED_MODEL"`. Assert the words contain `"Choose a model below. Install downloads it and makes it the embedder; until then search is full-text alone."` and do not contain `"needs a model"`. Repeat with `provider.source = "env"` (Review Focus 1): the same note shows and the dropdown is locked.
  - In `live.rs`: a test that `model_offer` returns `Some` when `embedder == "builtin"` and nothing is installed.
- [ ] **Step 2:** Run `cargo test -p codewiki-app settings_pages::` and `cargo test -p codewiki-app live::`. Expect FAIL.
- [ ] **Step 3: Implement.**
  - `select` takes the action per option from its caller. Simplest: after `provider(...)` in `search`, rewrite the `Choice` whose label is `embedder_name("builtin")` to `builtin_choice(model.models)`, and keep `selected`.
  - Pending, meaning provider value `"builtin"` and model value null: push `Block::Alert { tone: Tone::Info, text: <note above> }` instead of `in_use(...)`.
  - `model_offer`: accept `embedder` `"local"` or `"builtin"`.
- [ ] **Step 4:** Run `cargo test -p codewiki-app`. Expect PASS.
- [ ] **Step 5:** Commit `feat(app): list the built-in model in the embedder choice`.

### Task 3: Dev builds get the CUDA worker

**Files:**
- Create: `scripts/cuda-common.ps1`, which holds `$cudaLibraries = "curand64_10.dll","cublas64_13.dll","cublasLt64_13.dll"` and `function Get-CudaImports([string]$Program)`, both moved from `package.ps1:28-43`
- Create: `scripts/cuda-worker.ps1`
- Modify: `scripts/package.ps1:28-76, 94-116`

**Interfaces:**
- Produces: `pwsh scripts\cuda-worker.ps1 [-Release] [-CudaToolkit <dir>] [-NoBuild]`. Its last output line is the full path of the `cuda` folder it filled.

- [ ] **Step 1: Write `cuda-worker.ps1`.** Use the header-comment style of `cuda-toolkit.ps1`.
  - Toolkit: `-CudaToolkit`, else `$env:CODEWIKI_CUDA_TOOLKIT`, else the last line of `cuda-toolkit.ps1`. Throw when `bin\nvcc.exe` is missing.
  - Unless `-NoBuild`, set up the build environment and build. Move `package.ps1:54-74` as is: VS dev shell, `CL=/Zc:preprocessor`, `CUDA_PATH`, `PATH`, and `CUDA_COMPUTE_CAP` defaulting to 75. Then run `cargo build [--release] --locked -p codewiki --features cuda --bin codewiki-embed-cuda`.
  - Target dir comes from `cargo metadata --format-version 1 --no-deps` (`target_directory`), so a worktree uses its own. Profile folder: `release` or `debug`.
  - Before writing, check whether `<profile>\cuda\codewiki-embed-cuda.exe` is in use (it can be neither opened for write nor renamed). If so, throw "stop CodeWiki (its backend runs the worker) and run again" (Review Focus 4).
  - Fill `<profile>\cuda\`:
    - Copy the worker exe.
    - For each DLL in `$cudaLibraries`, remove the old one and create a hard link to `<toolkit>\bin\x64\<dll>` when on the same drive, else copy it.
    - Check that the worker imports only `$cudaLibraries` and `nvcuda.dll`, moved from `package.ps1:109-113`.
  - Print the folder path.
- [ ] **Step 2: Rework `package.ps1`.**
  - Dot-source `cuda-common.ps1`.
  - Replace the build block with `$cudaDir = & (Join-Path $PSScriptRoot "cuda-worker.ps1") -Release -CudaToolkit $CudaToolkit -NoBuild:$NoBuild | Select-Object -Last 1`.
  - Copy `$cudaDir\*` into `$stage\cuda` with `Copy-Item`, which writes real files from the links.
  - Keep the EULA copy and the stray-import check on `codewiki.exe`.
- [ ] **Step 3:** Parse-check all three scripts with `[System.Management.Automation.Language.Parser]::ParseFile(...)`. Expect no errors.
- [ ] **Step 4:** Run `pwsh scripts\cuda-worker.ps1 -CudaToolkit D:\packages\cuda\13.4.3` in the worktree. Expect the last line to be `<worktree target>\debug\cuda`. The folder holds `codewiki-embed-cuda.exe` and the three DLLs, and `fsutil hardlink list` on one DLL shows the toolkit path.
- [ ] **Step 5:** Run `cargo build -p codewiki`, then `target\debug\codewiki.exe model recommend --json` (`crates/codewiki/src/model.rs:105`). Expect `"model": "qwen3-embedding-0.6b", "device": "cuda"`.
- [ ] **Step 6:** Run `cargo test -p cw-server --test api built_in`. It still passes, and device options are `["auto","cpu"]` (Review Focus 5).
- [ ] **Step 7:** Commit `build(cuda): build the CUDA worker for development runs`.

### Task 4: Docs

**Files:**
- Modify: `AGENTS.md:88` (rule: `builtin` listed wherever the engine runs; accepted without a model as pending; refused with an unknown or uninstalled model)
- Modify: `AGENTS.md:95` (a dev build gets the worker from `scripts\cuda-worker.ps1`)
- Modify: `AGENTS.md:139` area (add `pwsh scripts\cuda-worker.ps1 -CudaToolkit D:\packages\cuda\13.4.3   # the card worker for a dev build`)
- Modify: `ROADMAP.md` (built: the dropdown entry and the dev worker; verified: what Task 5 shows)

- [ ] **Step 1:** Edit both files.
- [ ] **Step 2:** Commit `docs: note the pending built-in embedder and the dev CUDA worker`.

### Task 5: Check in the real app, and report

- [ ] **Step 1:** Make a check data dir and an `APPDATA` dir under the scratchpad.
- [ ] **Step 2:** Launch `<worktree>\target\debug\codewiki-app.exe settings:search` with these set:
  - `CODEWIKI_INSTANCE=new`
  - `CODEWIKI_DATA_DIR` and `APPDATA` pointing at the check dirs
  - `CODEWIKI_BACKDROP=#1E1B2E`

  Start it via ProcessStartInfo with CreateNoWindow, not `-WindowStyle Hidden`. Capture with `PrintWindow` only.
- [ ] **Step 3:** Check that the Built-in model card says `Recommended: Qwen3 Embedding 0.6B, on the graphics card. NVIDIA GeForce RTX 4070 with 12.0 GB of video memory.`
- [ ] **Step 4:** Check that the dropdown lists "Built-in model". Choose it; "In use" shows the pending note, and the dropdown shows "Built-in model".
- [ ] **Step 5:** Install Granite Embedding Small English R2 (94 MB). The job ends with it in use. Press Test; then "Loaded now: Granite Embedding Small English R2, on the graphics card."
- [ ] **Step 6:** Switch to Word forms, then choose "Built-in model" again. Expect Use to start at once with Granite.
- [ ] **Step 7:** Quit via `palette:Quit CodeWiki` + Enter. Confirm no backend is left running, then delete the check dirs.
- [ ] **Step 8:** Write `report\builtin-dropdown-dev-cuda\report.md`, summary first, with the screenshots and a recording.
- [ ] **Step 9:** Run `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` and `pwsh scripts\clean.ps1`. Confirm `target\debug\cuda\` survives the prune (its files carry no unit hash).

## Execution

Recommended: **Native**. I implement every task in this session, then one fresh reviewer checks the whole branch. Reasons: there are five small tasks, Task 2 depends directly on Task 1's response shape, and a mistake is cheap to undo on a branch. Say "subagent-driven" to get a reviewer after every task instead.

## Verification summary

- `cargo test -p cw-server`, `cargo test -p codewiki-app`, `cargo test -p cw-models` pass; clippy clean; fmt clean.
- `codewiki model recommend --json` in the worktree's dev build names Qwen3 on `cuda` after `cuda-worker.ps1`.
- The app check (Task 5) shows the dropdown entry, the pending note, a model running on the graphics card, and Use started from the dropdown.
- The package path (`package.ps1` → `cuda-worker.ps1 -Release`) is parse-checked only. A full package run happens at the next release (ship-release), noted in ROADMAP as not yet verified.
