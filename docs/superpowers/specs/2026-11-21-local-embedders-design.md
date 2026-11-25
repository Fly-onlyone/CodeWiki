# Design: local-server embedders replace the built-in model

Approved by the owner on 2026-11-21. The implementation plan (`superpowers:writing-plans`) is written once the `agent-writer` worktree has merged into `main`.

## Context

Semantic search can run a neural model by itself (`CODEWIKI_EMBED=builtin`). That model comes from a three-model catalog, is downloaded into `<data dir>\models\`, and runs in a hidden `codewiki embed-worker` child process built on candle. NVIDIA cards get a second program, `cuda\codewiki-embed-cuda.exe`, and its three NVIDIA DLLs, which add about 460 MB to the package. The feature brings two whole crates (`cw-models`, `cw-embed-engine`), a supervisor, a game detector, hardware readings, model routes, Settings and Home UI, a CLI subcommand, CUDA build scripts and three VM cases.

The owner wants it gone. Their reasons are **maintenance** (too much code to own) and **model choice** (users should pick any embedding model, not one of three). Semantic search is to use local model servers that the user installs and runs: **LM Studio**, **Llama for Windows / llama.cpp `llama-server`**, **Lemonade Server** and **mistral.rs**. Ollama and the OpenAI-compatible embedder stay.

## Goals

- Delete the built-in embedding model and everything that exists only for it.
- Add four embedders, each its own entry in Settings → Search: `lmstudio`, `llamacpp`, `lemonade`, `mistralrs`.
- Let the user pick the model from what the running server offers.
- Keep repositories embedding fully after a Sync, also with a slow local server.
- Move users who chose `builtin` to a working state, without asking them.

## Non-goals

- CodeWiki does not install, start, stop or update any model server. It does not download models.
- No speed benchmark. The owner chose to decide from research (2026-11-21).
- No LLM or chat use of these servers: embeddings only.
- No change to the `local` hash embedder or to `none`.

## Anti-goals (failure even if it works)

- Running a model inside CodeWiki again, under any name. That includes the `mistralrs` crate, which is built on candle.
- Comparing vectors made by two different models. This happens if a server swaps models under one stored name.
- Leaving a user who had `builtin` with search silently off, a stale catalog id in `embed.model`, or gigabytes of orphaned model files.

## Constraints

- **Hard rules in `AGENTS.md`:** no WebView; the app talks HTTP only; never log a token, key, query, prompt or note text. Secrets are encrypted with `cw_core::secret`. A backend that listens beyond its machine refuses settings changes. Tests never read the machine's settings (`ServerConfig::env`).
- **Sequencing** (owner, 2026-11-21): the unmerged `agent-writer` worktree (14 commits) rewrites `cw-core/src/settings.rs`, `cw-server/src/settings.rs`, `settings_pages.rs`, `settings_view.rs` and the docs. It also removes the "AI model" settings page. The owner decided to wait for `agent-writer` to merge into `main`. The implementation plan is written against `main` after that merge, and the work runs in a worktree of its own. Reuse its `drop_retired` pattern (`RETIRED_SECTIONS`, `RETIRED_VARIABLES`) and its Search page layout. Before planning, check that the merge happened (`git log main` shows `feat(app): switch a note to the agent view`).
- **Verification:** third-party API shapes below come from official docs and source. Where a shape is marked *verify*, the builder checks it against the server's docs or source at build time.

## Approaches

| | Approach | Trade-off | Outcome |
|---|---|---|---|
| A | Four named providers on one shared OpenAI-style client. Each has its own address (and key where the server has one). Small per-server parts handle listing models and quirks. | More settings rows. In return, every quirk has a home and users see names they know. | **Picked by the owner** |
| B | One "Local server" provider with a kind select that pre-fills one shared address, key and model. | Fewer settings, but switching kind overwrites the address. It also does not match how Ollama works. | Rejected |
| C | No new providers: make the `openai` key optional and document each server's address. | Least code, but no model list and no per-server fixes (mistral.rs fails on long text). Works against model choice. | Rejected |
| — | Link the `mistralrs` crate in-process. | The same engine weight being removed. Fails the maintenance reason. | Rejected |

Runner set: research ranked LM Studio, Ollama, Lemonade, llama.cpp/Llama for Windows and mistral.rs. The owner chose to keep all four new ones, mistral.rs included. Dropped from research: Foundry Local (now an in-process SDK), TEI and Infinity (no native Windows build), KoboldCpp (AGPL), OpenVINO Model Server (needs a config file, no JIT loading), GPT4All (stale).

## Design

### 1. What is removed

- **Whole crates:** `crates/cw-models`, `crates/cw-embed-engine`.
- **Whole files:**
  - `crates/codewiki/src/model.rs`, `crates/codewiki/src/bin/embed_cuda.rs`, `crates/codewiki/tests/model.rs`;
  - `crates/cw-server/tests/models.rs`, which gets ported (see §6);
  - `scripts/cuda-worker.ps1`, `scripts/cuda-toolkit.ps1`, `scripts/cuda-common.ps1`;
  - `docs/superpowers/{plans,specs}/2026-11-20-builtin-dropdown-dev-cuda*`.
- **`cw-embed`:**
  - `BuiltinEmbedder`, `CUDA_VRAM`, `EMBED_DEVICE_ENV`, `DEVICES`, the `"builtin"` arm and its tests;
  - `slow()`, whose only user was builtin, together with the slow branch in `semantic::embed_within`;
  - the `cw-models` dependency.

  `from_config_in` and `from_settings_in` fold into `from_config` and `from_settings`, since nothing needs a data dir any more.
- **`codewiki` crate:** the `model` and `embed-worker` subcommands, the `cuda` feature, the `[[bin]] codewiki-embed-cuda`, the Windows target dependency on `cw-embed-engine`, and `default-run`.
- **`cw-server`:**
  - the Install, Use and Remove paths, `HardwareCache`, the game wait, the worker stop, and the `cw_models::ENGINE` gates;
  - the `Host` fields `hardware`, `game` and `pause_poll`;
  - the builtin validation, the builtin device option and `embed.device` in `settings.rs`.
- **`cw-core`:** `EMBED_DEVICE` leaves `ALL`.
- **App:**
  - the "Built-in model" section and model cards in `settings_pages.rs`;
  - the model state in `settings_view.rs`;
  - the Home "Search by meaning" offer (`live.rs` `ModelOffer`) and its wiring in `main.rs`;
  - `InstallModel`, `UseModel`, `RemoveModel` and `DismissModelHint` in `doc.rs`;
  - the game text in `status.rs`.
- **Root and packaging:**
  - the `[profile.dev]` opt-level overrides for candle, gemm, pulp, rayon, tokenizers and cw-embed-engine;
  - the cuda parts of `package.ps1`, `install.ps1` and `release.yml`;
  - the cuda checks and cases EM1 to EM3 in `vm/test-release.ps1`;
  - the two source folders in `.idea/CodeWiki.iml`;
  - the model licences in `NOTICE`.

  `uninstall.ps1` keeps removing a leftover `cuda\` folder and `<data dir>\models\`.

### 2. Providers

| Id | Label | Address setting (default) | Key setting | Embeddings request | Model list |
|---|---|---|---|---|---|
| `local` | Word forms: no model needed | — | — | in-process hash | — |
| `lmstudio` | LM Studio | `CODEWIKI_LMSTUDIO_URL` (`http://127.0.0.1:1234`) | `CODEWIKI_LMSTUDIO_KEY`, optional Bearer | `POST /v1/embeddings` | `GET /api/v0/models`: `type` `embeddings` (also accept `embedding`), `state` |
| `ollama` | Ollama | `OLLAMA_HOST` (unchanged) | — | `POST /api/embed` (unchanged) | `GET /api/tags`, then `POST /api/show` `capabilities` contains `embedding` (*verify*) |
| `llamacpp` | Llama / llama.cpp | `CODEWIKI_LLAMACPP_URL` (`http://127.0.0.1:9931`) | `CODEWIKI_LLAMACPP_KEY`, optional Bearer | `POST /v1/embeddings` | router `GET /models`, falling back to `/v1/models`; embedding flag unknown (*verify*) |
| `lemonade` | Lemonade | `CODEWIKI_LEMONADE_URL` (`http://127.0.0.1:13305`) | `CODEWIKI_LEMONADE_KEY`, optional Bearer (`LEMONADE_API_KEY` on the server side) | `POST /v1/embeddings` | `GET /v1/models`, `labels` contains `embeddings` (*verify*) |
| `mistralrs` | mistral.rs | `CODEWIKI_MISTRALRS_URL` (`http://127.0.0.1:1234`) | — (the server has no auth) | `POST /v1/embeddings` with `"truncate_sequence": true` | `GET /v1/models` |
| `openai` | OpenAI-compatible | `OPENAI_BASE_URL` (unchanged) | `OPENAI_API_KEY` (unchanged, still required) | unchanged | `GET <base>/models`, ids containing `embed` (builder may skip) |
| `none` | Off: full-text search only | — | — | — | — |

- **Settings:** each new address and key is a `cw_core::settings::Key` in its own section (`lmstudio.url`, `lmstudio.api_key` and so on), listed in `ALL`. Each key is encrypted like `openai.api_key`. The env var wins over the file, as for every setting.
- **The model is required** for the four new providers. No server-side `"default"` model name: one stored name must always mean one model (anti-goal). Without a model, `embed.problem` says "choose a model" and search is full-text only. The Settings page shows the list to choose from.
- **Vector name:** `<provider>:<model>`, plus `+<rule>` when a prefix rule applies (§5). A changed name re-embeds, as today.
- **Ports:** LM Studio and mistral.rs both default to 1234, and Llama for Windows and `llama-server` share 9931. The page's help line for each says so.

### 3. Shared client and quirks (`cw-embed`)

- One OpenAI-style embedder, configured by provider: base URL, optional bearer token, extra body fields, provider name for errors and the vector name. `OpenAiEmbedder` becomes that client or a thin case of it. Batching (64), clipping (8,000 chars), `post_json`, `checked` and `parse_openai` are reused, and so is normalising every vector.
- **Per-server quirks, in one place:**
  - mistral.rs: add `truncate_sequence: true`, or a long text is an error.
  - llama.cpp (*verify against `llama-server` source or docs*): with the default physical batch (`-ub 512`), may an embedding input exceed it? If it fails, `llamacpp` clips its texts to fit, and the help line names `-ub`/`-b` or Llama for Windows' custom serve arguments. Builder decides which, from what the server does.
  - LM Studio: with JIT loading, the first call loads the model. The probe timeout (60 s) covers it.
- `from_config` builds each provider from its settings. An unknown provider keeps today's error. `builtin` gets its own message (§7).

### 4. Model list route (`cw-server`)

- `GET /v1/settings/embed/models` lists the models of the provider in effect, asked at its address in effect. Response: `{ provider, reachable, models: [{ id, embedding: true|false|null, loaded: true|false|null }], problem }`.
- Known non-embedding models are left out. Unknown ones are listed after the embedding ones.
- It needs the token, like the other settings routes. It is answered the same way as `/v1/settings/test/embed` for a backend that listens beyond its machine.
- `cw-client` gets a matching call. The parsing for each server lives in `cw-embed`, beside its provider, and is tested with stand-in servers.

### 5. Prompt prefixes (my call)

No server adds the text a model expects in front of a query or document, and the built-in engine did. Without it, search quality drops for those models.

- **The table:** `cw-embed` gets a small table keyed by a case-insensitive substring of the model name. Each rule holds an id, a query prefix and a document prefix. It starts with:
  - `embeddinggemma`: query `task: search result | query: `, document `title: none | text: `
  - `qwen3-embedding`: query `Instruct: <the catalog's instruction>\nQuery:`, document none. Copy the instruction from `crates/cw-models/catalog.json` before deleting it.
  - `nomic-embed-text`: `search_query: ` / `search_document: `
  - `e5`: `query: ` / `passage: `
  - `bge-*-v1.5` and `mxbai-embed`: query `Represent this sentence for searching relevant passages: `, document none
- **Where it applies:** every HTTP embedder, Ollama and OpenAI-compatible included. `embed_query` gets the query prefix and `embed` the document prefix.
- **Existing vectors:** a matching rule adds `+<rule id>` to the vector name. Existing Ollama vectors of, for example, `embeddinggemma` re-embed once.
- The builder checks each prefix against its model card.
- **Matching** (my call): a rule is a list of substrings, and it matches when all of them appear in the lowercased model name. For example, `bge` is `["bge-", "-v1.5"]`. The first matching rule in table order wins. The table lists the most specific rules first, so `nomic-embed-text` comes before `e5`. `e5` is `["e5-"]` with a boundary check, so `nomic-embed-text-v1.5` does not match it. A test pins one model name per rule, plus names that match no rule, so a reorder that changes a rule id fails the test.

### 6. Background embedding job (my call)

The job finishes embedding after a Sync, for any embedder, on every platform.

- **What stays:** `cw-server/src/models.rs` becomes `embed_job.rs`. It keeps the reembed job: `embed_all`, `embed_repo`, `after_update`, cancel, one job at a time, embedding without a lock with `write_lock` per batch, starting over when an update overtakes it, and resume at start for any embedder that is `behind`.
- **What goes:** the `ENGINE` gate (Linux and Docker now finish embedding too), Install, Use, Remove, the hardware readings and the game pause.
- **Kept as is:** the 1 s inline budget (`set_inline_budget`) and `semantic::Run` and `behind`.
- **Routes:** `POST /v1/embed/cancel` and `POST /v1/embed/reembed` replace `/v1/models/cancel` and `/v1/models/reembed`. `/v1/status` keeps reporting the job under its current field name `model_job`, so older and newer clients still read it.
- **`Host`:** keeps only `embedder`, for tests.
- **Tests:** the job tests of `tests/models.rs` are ported without builtin: reembed after an update, cancel, one at a time, an embedder set by the environment, and resume.
- **Server down or wrong model** (my call): a failed batch ends the job. The job records its error, which `model_job` reports, the Settings page shows as a warning and the log keeps. The repository stays `behind`. No retry loop and no backoff: the next Sync, Test or reembed tries again. Resume at backend start tries once.
  - **Amendment, 2026-11-25.** The rule still holds for the background embedding job. What changed is the Settings page: while Settings → Semantic search is open, the window is on screen and the backend is local, and the server does not answer the request for its model list (`GET /v1/settings/embed/models`), the app asks again after 3 s, then 6, 12, 24 and 30 s at the most, and shows the toast "Connected to <server>" when it answers. This is the app's own retry of a read, not a retry of embedding work: the job and the routes of `embed_job.rs` are unchanged.
- **First load** (my call): the first batch of a job, and the first call after an embedder change, wait up to 300 s, so a large model can load on first request. Every later batch keeps `REQUEST_TIMEOUT` (120 s).
- **Route rules** (my call): `/v1/embed/cancel` and `/v1/embed/reembed` refuse a backend that listens beyond its machine, as `/v1/models/*` do today (`models_closed`). The old paths get no aliases. An older client that meets a newer backend gets a 404 on Cancel, which is accepted: the app and the bridge update together with the backend.

### 7. Settings changes and migration

- **`settings::change`:** a change of provider that names no model clears `embed.model`, unless the environment sets it. This now holds between every pair of providers, because model names do not carry across servers.
- **Once, at backend start, beside `drop_retired`:**
  - Saved `embed.provider == "builtin"` becomes `local`, and `embed.model` is cleared.
  - `embed.device` leaves the file.
  - `<data dir>\models\` (CodeWiki's own downloads, including `.partial`) is deleted. The log records how much was freed.
  - `CODEWIKI_EMBED_DEVICE` joins `RETIRED_VARIABLES`, so a set variable gets a log line.
- **`CODEWIKI_EMBED=builtin` in the environment:** it cannot be rewritten. `embed.problem` says the built-in embedder was removed and names the choices. Search is full-text only, as it is for any unknown embedder.
- **App:** deletes `model_hint.txt` at start.
- **After an in-app update:** the old `cuda\` folder stays behind (`cw-update` swaps only the new package's entries). `recover_and_clean` removes `<install>\cuda` once no program in it runs. Builder: if that is not safe there, leave it to `uninstall.ps1` and say so in the release notes.

### 8. App: Settings → Search

```
Embedder   [ LM Studio                 v ]   Finds notes and symbols by meaning
Address    [ http://127.0.0.1:1234        ]   (shown for the chosen provider)
Key        [ ••••  (optional)             ]   (LM Studio, Llama, Lemonade, OpenAI)
Model      [ text-embedding-qwen3-0.6b  v ]   listed from the server  [Refresh]
           One line: how to load an embedding model in this server; port note.
In use: lmstudio:text-embedding-qwen3-0.6b   /  problem as a warning
[ Embedding 340 of 2,100 ......... ] [Cancel]   (while a job runs)
[Test] [Sync now]
```

- **Rows:** the Embedder select lists the eight providers in this order: local, lmstudio, ollama, llamacpp, lemonade, mistralrs, openai, none. Only the chosen provider's address and key rows show. They follow wherever `agent-writer` left the Ollama and OpenAI rows when it removed the "AI model" page.
- **Model:** a select filled from §4 when the server answers, and the existing text field when it does not. It has a Refresh action.
- **Job progress:** shown with `Block::Progress` and Cancel (§6).
- **Help lines:** one per provider, telling how to serve an embedding model, for example `mistralrs serve -m Qwen/Qwen3-Embedding-0.6B`. The builder takes each from the server's docs.
- **Rules for the page:** it follows the existing patterns: a block in `search()` of `settings_pages.rs`, `FieldId`s made at the view's start, an `Action` per button, `mui` controls, theme colours, and a test that reads the `Doc`.

### 9. CLI, MCP, client

- **CLI:** `codewiki model` goes. Nothing replaces it: Settings and the env vars cover setup.
- **MCP:** no change.
- **`cw-client`:** gets the model-list call and the renamed job routes.

### 10. Docs

- **README:** the semantic search section becomes a table of the servers, with each one's address, how to serve an embedding model, its devices on Windows and its port. The built-in model text and "the embedding worker" go.
- **AGENTS.md:** the "Built-in embedding models" section becomes "Embedding servers", covering providers, quirks, the job and the migration. The hard-rule lines on candle opt-level and the CUDA program go. The layout table drops two crates. The settings list, the logs paragraph (`embed-worker.log`, tag `embed-worker`) and the unsafe-modules sentence (the `game.rs` module is gone) are updated.
- **ROADMAP:** crate and test counts, and the M7 line.
- **Memory:** update `vm-test-scope.md` (EM cases gone) and `cuda-toolkit-location.md` (obsolete).

## Left to the builder

- Every place marked *verify*: Ollama `/api/show` shape, llama.cpp router listing and the physical-batch limit, Lemonade `labels`, and each help line's command.
- Whether `openai` gets a model list.
- How `OpenAiEmbedder` is refactored into the shared client.
- The exact wording of problems and help lines, within the house style (`AGENTS.md`, plain sentences).
- How the job's progress text reads in the tray.

## Verification

- **Commands:** `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check advisories bans sources`. Also `get_file_problems` on each changed `.rs` file.
- **Stand-in servers** in `cw-embed/tests/wire.rs`, one per new provider. They check: the request path, the Bearer header only when a key is set, `truncate_sequence` for mistral.rs, the listing parse with embedding flags, a prefix applied and its name suffix, and the error text of a 4xx/5xx. The shapes come from the docs cited in the research notes; say so in the test file.
- **`cw-core`:** the new keys, and the migration (`builtin` becomes `local`, `embed.device` dropped, `models\` deleted, a second run changes nothing).
- **`cw-server` `tests/api.rs`:**
  - the provider options and the clearing rule between every pair;
  - the list route, reachable and unreachable;
  - `CODEWIKI_EMBED=builtin` gives the removed-provider problem;
  - the ported job tests (§6).
- **App:**
  - a `settings_pages` test per provider block: address and key rows, the model select against the text field, and job progress;
  - `status.rs` without game text;
  - `live.rs` Home without the offer.
- **Build check:** `cargo tree -p codewiki | rg -i "candle|tokenizers|nvml|sysinfo"` prints nothing. `pwsh scripts\package.ps1` builds a zip without `cuda\`.
- **App check** (AGENTS.md "Checking the app"): `codewiki-app <repo> settings:search` with its own `CODEWIKI_DATA_DIR`, and screenshots of each provider's block. Live servers are not part of the check: the owner chose no install on this machine. A live smoke test with a real server is optional, and only when the owner asks.

## Research notes (2026-11-21, sources)

- **LM Studio:** `/v1/embeddings` on `:1234`, optional Bearer token, `/api/v0/models` with `type`, JIT loading with a 60-minute TTL and auto-evict.
  - Sources: lmstudio.ai/docs/developer (openai-compat, rest/endpoints, core/authentication, core/ttl-and-auto-evict).
- **Llama for Windows** v0.14.1 (github.com/ggml-org/Llama-Windows): WinUI tray app that runs `llama serve` (router).
  - `DefaultServerPort = 9931`.
  - A key exists only when the server is bound beyond loopback. It is passed to the server as `LLAMA_API_KEY` and kept DPAPI-locked in the app's own settings.
- **llama-server** (llama.cpp tools/server README): `/v1/embeddings` needs pooling other than none, normalises L2, and takes `--embeddings`, `--pooling`, `-ub`, `-b`, `-np`, `--sleep-idle-seconds` and `--models-max`. Windows zips exist for CPU, CUDA 12.4/13.5, Vulkan, HIP, SYCL, OpenVINO and Adreno.
- **Lemonade** v2026.47.1 (github.com/lemonade-sdk/lemonade):
  - `docs/api/openai.md` marks `POST /v1/embeddings` "fully available". It is forwarded to a llama-server or FastFlowLM (AMD NPU) backend.
  - Port 13305; `LEMONADE_API_KEY` Bearer is optional. It ships as MSI and runs as a service.
- **mistral.rs** v0.9.5: `mistralrs serve -m <model>` serves `/v1/embeddings` on `:1234` with no auth (`mistralrs-server-core/src/embeddings.rs`).
  - `truncate_sequence` defaults to false; `dimensions` is rejected.
  - On Windows the prebuilt is CPU only; CUDA needs a source build. Embedding models are loaded from safetensors.
- **Ollama:** CUDA, ROCm v7 and Vulkan on Windows. `keep_alive` is 5 minutes and `OLLAMA_NUM_PARALLEL` is 1. `/api/show` returns `capabilities`.
- **Engines:** all but mistral.rs run GGUF through llama.cpp, so the same model, quant and device give similar speed. No published cross-runner embedding benchmark exists.
- **Repo map:** an Explore agent pass on 2026-11-21. Every file and line listed in §1 comes from it.
