# Local Embedders Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the built-in embedding model and let semantic search use LM Studio, llama.cpp (Llama for Windows, `llama-server`), Lemonade and mistral.rs, beside Ollama and the OpenAI-compatible embedder.

**Architecture:** One OpenAI-style HTTP client in `cw-embed` serves all four new servers. Each server has its own address, an optional key and a model list read from the server, and a small table adds the prefixes known model families expect. The background embedding job in `cw-server` drops everything that only the built-in model needed, keeps the rest, and serves every embedder. The app's Settings → Search shows the chosen server's rows and a model select.

**Tech Stack:** Rust 2024 workspace, `ureq` 3 (HTTP client), axum (backend), GPUI with `gpui-kit` (app), PowerShell (packaging), std `TcpListener` stand-in servers in tests.

**Spec:** `docs/superpowers/specs/2026-11-21-local-embedders-design.md`. Read it before any task: every "why" is there.

## Global Constraints

- **Worktree:** work in `.claude/worktrees/local-embedders` (create it with `superpowers:using-git-worktrees`), branched from `main` at or after `505ecbd`. It builds into its own `target\`; never set `CARGO_TARGET_DIR` to the main checkout's.
- **After each `.rs` edit:** run RustRover `get_file_problems` (`mcp__rustrover__execute_tool`), then `cargo fmt --all`. Do not use `reformat_file` on Rust files: it is not rustfmt.
- **Every task ends green:**
  ```
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
  ```
  Task 11 adds `cargo deny check advisories bans sources`.
- **Commits:** load `qingyi:commit` before each commit. Messages are `type(scope): imperative subject`, with a body that says what and why.
- **Hard rules from `AGENTS.md`:**
  - Never log a token, a key, a query, a prompt or note text. An embedder error never quotes a text it was given.
  - The app talks to the backend over HTTP only (`cw-client`).
  - Secrets are encrypted with `cw_core::secret`; no route returns one.
  - A backend whose `settings_writable` is false refuses changes.
  - Tests never read the machine's settings (`ServerConfig::env`, `Host`). Tests never call a real model server.
- **Defaults** (spec §2):

  | Setting | Value |
  |---|---|
  | `CODEWIKI_LMSTUDIO_URL` | `http://127.0.0.1:1234` |
  | `CODEWIKI_LLAMACPP_URL` | `http://127.0.0.1:9931` |
  | `CODEWIKI_LEMONADE_URL` | `http://127.0.0.1:13305` |
  | `CODEWIKI_MISTRALRS_URL` | `http://127.0.0.1:1234` |
  | Keys | `CODEWIKI_LMSTUDIO_KEY`, `CODEWIKI_LLAMACPP_KEY`, `CODEWIKI_LEMONADE_KEY`. mistral.rs has none. |

- **Provider ids, in page order:** `local`, `lmstudio`, `ollama`, `llamacpp`, `lemonade`, `mistralrs`, `openai`, `none`.
- **Vector names:** `<provider>:<model>`, plus `+<rule id>` when a prefix rule matches. `openai:` and `ollama:` keep their current form apart from that suffix.
- **Timeouts:** `REQUEST_TIMEOUT` 120 s per batch. `FIRST_LOAD_TIMEOUT` 300 s for an embedder instance's first call. `PROBE_TIMEOUT` 60 s. `LIST_TIMEOUT` 10 s for a model list.
- **No index schema change:** do not bump `SCHEMA_VERSION`.
- **Do not loosen `[profile.dev]`.** Removing the candle opt-level block (Task 6) is allowed: it was an addition made for the built-in engine.
- **UI copy:** plain sentences, as on the existing Settings pages.
- **Where API shapes come from:** official docs and source, cited in the spec's research notes. Each stand-in test file says so in its header. Items marked *verify* are checked against the server's docs or source while implementing; record what you found in a code comment with its URL.

## Review Focus

Each of these is pinned by a test in the task named.

1. **An address pasted with `/v1`.** LM Studio, Lemonade and mistral.rs show `http://localhost:1234/v1`. Such an address must work like the bare root, not call `/v1/v1/embeddings` (Task 1).
2. **A server that asks for a token, with none set.** A 401 or 403 must say that the server wants a token, and which setting holds it. It must not read as a bare "HTTP 401" (Task 1).
3. **A text longer than the model's context.** bge-small takes 512 tokens and CodeWiki clips at 8,000 characters. A batch the server refuses as too long is sent once more with texts cut to 2,000 characters. Neither the Sync nor the job fails (Task 1).
4. **One model's details fail in an Ollama list.** That model is listed with embedding unknown; the list is not lost (Task 3).
5. **The server is down when the backend starts behind.** Resume tries once, the job ends `failed` with the reason, and nothing loops. The next Sync or reembed tries again (Task 5).

---

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `crates/cw-core/src/settings.rs` | modify | 7 new `Key`s; drop `EMBED_DEVICE`; `retire_builtin` |
| `crates/cw-embed/src/lib.rs` | modify | `Server`, shared `OpenAiEmbedder`, constructors, `from_config` for the new providers, retry and token messages; removal of `BuiltinEmbedder` |
| `crates/cw-embed/src/prefix.rs` | create | prefix rules and `rule()` |
| `crates/cw-embed/src/list.rs` | create | `Listed`, `list_models`, per-server parsers |
| `crates/cw-embed/tests/wire.rs` | modify | stand-ins for each server, `serve_many` |
| `crates/cw-server/src/settings.rs` | modify | new keys, clearing rule, `embed_models` |
| `crates/cw-server/src/embed_job.rs` | create (from `models.rs`) | the reembed job, for every embedder |
| `crates/cw-server/src/models.rs` | delete | |
| `crates/cw-server/src/{lib.rs,api.rs}` | modify | `Host`, routes |
| `crates/cw-server/tests/embed_job.rs` | create | the ported job tests |
| `crates/cw-server/tests/models.rs` | delete | |
| `crates/cw-models/`, `crates/cw-embed-engine/` | delete | |
| `crates/codewiki/{Cargo.toml,src/main.rs,src/serve.rs}` | modify | drop the model and worker commands; call `retire_builtin` |
| `crates/codewiki/{src/model.rs,src/bin/embed_cuda.rs,tests/model.rs}` | delete | |
| `crates/cw-wiki/src/{semantic.rs,service.rs}`, `tests/reembed.rs` | modify | drop `slow`, `configure_in` and `reembed` |
| `crates/cw-update/src/swap.rs` | modify | remove the old `cuda\` folder |
| `crates/codewiki-app/src/{settings.rs,settings_pages.rs,settings_view.rs,doc.rs,live.rs,status.rs,main.rs,prefs.rs}` | modify | the Search page, the job, removals |
| `scripts/{package,install,uninstall}.ps1`, `scripts/vm/test-release.ps1`, `.github/workflows/release.yml`, `.idea/CodeWiki.iml`, `NOTICE` | modify | no CUDA, no model cases |
| `scripts/cuda-{worker,toolkit,common}.ps1`, `docs/superpowers/{plans,specs}/2026-11-20-builtin-dropdown-dev-cuda*` | delete | |
| `README.md`, `AGENTS.md`, `ROADMAP.md`, `obsidian/` | modify | docs and the regenerated wiki |

---

### Task 1: Settings keys and the shared HTTP client for the four servers

**Files:**
- Modify: `crates/cw-core/src/settings.rs:55-110`, `crates/cw-embed/src/lib.rs`
- Test: `crates/cw-embed/tests/wire.rs`, unit tests in both files

**Interfaces:**
- Produces, in `cw_core::settings`:
  - `LMSTUDIO_URL`, `LMSTUDIO_KEY`, `LLAMACPP_URL`, `LLAMACPP_KEY`, `LEMONADE_URL`, `LEMONADE_KEY`, `MISTRALRS_URL`.
  - Sections `lmstudio`, `llamacpp`, `lemonade`, `mistralrs`; fields `url` and `api_key`. Keys have `secret: true`. Defaults as in Global Constraints.
  - The new keys go after `OLLAMA_HOST` in `ALL`, which becomes `[Key; 13]`.
- Produces, in `cw_embed`:
  - `pub enum Server { OpenAi, LmStudio, LlamaCpp, Lemonade, MistralRs }`, with:
    - `pub fn id(self) -> &'static str`: `openai`, `lmstudio`, `llamacpp`, `lemonade`, `mistralrs`;
    - `pub fn label(self) -> &'static str`: `the embeddings API`, `LM Studio`, `llama.cpp`, `Lemonade`, `mistral.rs`;
    - `pub fn url_env(self) -> &'static str`;
    - `pub fn key_env(self) -> Option<&'static str>`.
  - `pub fn server_root(base: &str) -> &str`: trims `/` and then one trailing `/v1`.
  - `OpenAiEmbedder::new(server: Server, base_url: &str, api_key: Option<&str>, model: &str) -> Self` and `OllamaEmbedder::new(base_url: &str, model: &str) -> Self`. Fields become private: each holds `warm: AtomicBool`.
  - `PROVIDERS: [&str; 9] = ["local", "builtin", "lmstudio", "ollama", "llamacpp", "lemonade", "mistralrs", "openai", "none"]`. `builtin` leaves in Task 6.

- [ ] **Step 1: Write the failing tests**

  In `wire.rs`, add `fn serve_many(replies: Vec<(u16, Value)>) -> (String, JoinHandle<Vec<Received>>)`. It serves the replies in order on one listener and returns every request received. Then:

  ```rust
  #[test]
  fn each_local_server_gets_its_path_and_a_token_only_when_one_is_set() {
      for (server, key) in [(Server::LmStudio, Some("t")), (Server::LlamaCpp, None),
                            (Server::Lemonade, Some("t")), (Server::MistralRs, None)] {
          let (url, srv) = serve_once(200, json!({ "data": [{ "index": 0, "embedding": [3.0, 4.0] }] }));
          let e = OpenAiEmbedder::new(server, &url, key, "m");
          assert_eq!(e.embed(&["a".into()]).unwrap(), [vec![0.6, 0.8]]);
          let got = srv.join().unwrap();
          assert_eq!(got.path, "/v1/embeddings");
          assert_eq!(got.authorization.is_some(), key.is_some());
          assert_eq!(got.body["model"], "m");
          assert_eq!(got.body.get("truncate_sequence").is_some(), server == Server::MistralRs);
          assert_eq!(e.name(), format!("{}:m", server.id()));
      }
  }
  #[test] fn an_address_ending_in_v1_is_the_server_root()                     // Review Focus 1
  // base "http://127.0.0.1:PORT/v1/" → path "/v1/embeddings"; also server_root("http://h:1/v1/") == "http://h:1"
  #[test] fn a_refused_token_says_which_setting_holds_it()                    // Review Focus 2
  // serve 401 {"error":{"message":"Unauthorized"}} → error text contains "CODEWIKI_LMSTUDIO_KEY" and "token"
  #[test] fn a_batch_refused_as_too_long_is_sent_again_with_shorter_texts()   // Review Focus 3
  // serve_many([(400, {"error":{"message":"input is too large to process. increase the physical batch size"}}),
  //             (200, two vectors)]) with one 9,000-char text and one short text; embed succeeds;
  //   second request's first input has len <= 2_000 and the short text is unchanged.
  //   A 400 whose message is not about length is not retried (one request only, error returned).
  ```

  In the unit tests of `cw-embed/src/lib.rs`, add:
  - `configuration_selects_the_local_servers`:
    - `CODEWIKI_EMBED=lmstudio` with a model gives the name `lmstudio:m`;
    - the default address is `http://127.0.0.1:1234`;
    - without `CODEWIKI_EMBED_MODEL`, the error contains `choose a model`.
  - `the_first_call_may_wait_for_a_model_to_load`: `e.timeout()` equals `FIRST_LOAD_TIMEOUT` before a call and `REQUEST_TIMEOUT` once `warm` is set.

  In `cw-core`, add `the_servers_have_their_own_address_and_key`: the paths are `lmstudio.url` and `lmstudio.api_key`, `LMSTUDIO_KEY.secret` is true, and so on for each key.

- [ ] **Step 2: Run them to verify they fail**

  Run `cargo test -p cw-embed`, then `cargo test -p cw-core settings::`.
  Expected: they fail to compile, because `Server` and the `new` constructors do not exist yet.

- [ ] **Step 3: Implement**

  - **The keys:** add them to `cw-core`.
  - **The client:** `OpenAiEmbedder` builds its URL in one place.
    - For `Server::OpenAi`: `{base}/embeddings`, as today.
    - For the other servers: `{server_root(base)}/v1/embeddings`.
  - **Request body:** mistral.rs adds `"truncate_sequence": true`. The Bearer header is sent only when a key is set.
  - **A 401 or 403:** gives `"{label} refused the request (HTTP {n}): it asks for an API token; set {key_env} or save one in Settings → Semantic search"`. For a server without a key setting, the message keeps the generic form.
  - **Retry once:** `post_json` reports the status and detail. If a batch gets a 4xx or 5xx whose detail, lowercased, contains `context`, `too long`, `too large`, `exceed` or `maximum`, the batch is sent once more with each text cut to `SHORT_CHARS = 2_000` (on a char boundary).
  - **Timeouts:** `fn timeout(&self) -> Duration` returns `FIRST_LOAD_TIMEOUT` until the first success, which sets `warm`. `probe` keeps `PROBE_TIMEOUT`.
  - **`from_config`:** adds the arms `lmstudio`, `llamacpp`, `lemonade` and `mistralrs`. Each reads `server.url_env()` with the default from Global Constraints, and `server.key_env()`. It requires `CODEWIKI_EMBED_MODEL`; without one, the error is `"choose a model for {label}: Settings → Semantic search lists the ones it serves, or set CODEWIKI_EMBED_MODEL"`.
  - **Callers:** update `OllamaEmbedder` and `OpenAiEmbedder` construction there, and in `wire.rs`, to the constructors.

- [ ] **Step 4: Run the tests to verify they pass**

  Run `cargo test -p cw-embed` and `cargo test -p cw-core`. Expected: PASS. Then run the workspace commands.

- [ ] **Step 5: Commit** as `feat(embed): add LM Studio, llama.cpp, Lemonade and mistral.rs embedders`.

---

### Task 2: Prompt prefixes for known model families

**Files:**
- Create: `crates/cw-embed/src/prefix.rs`
- Modify: `crates/cw-embed/src/lib.rs`, to apply the prefixes in `OllamaEmbedder` and `OpenAiEmbedder`
- Test: unit tests in `prefix.rs`, and `wire.rs`

**Interfaces:**
- Produces:
  - `pub struct Rule { pub id: &'static str, pub parts: &'static [&'static str], pub query: &'static str, pub document: &'static str }`
  - `pub const RULES: &[Rule]`
  - `pub fn rule(model: &str) -> Option<&'static Rule>`
  - Both embedders override `embed_query`.

- [ ] **Step 1: Write the failing tests**

  ```rust
  #[test]
  fn each_rule_matches_the_names_its_servers_give() {
      let id = |m: &str| rule(m).map(|r| r.id);
      for m in ["embeddinggemma", "embeddinggemma:300m", "google/embeddinggemma-300m",
                "text-embedding-embeddinggemma-300m"] { assert_eq!(id(m), Some("gemma"), "{m}"); }
      for m in ["Qwen/Qwen3-Embedding-0.6B", "qwen3-embedding:0.6b",
                "text-embedding-qwen3-embedding-0.6b"] { assert_eq!(id(m), Some("qwen3"), "{m}"); }
      for m in ["nomic-embed-text", "nomic-embed-text-v1-GGUF", "text-embedding-nomic-embed-text-v1.5"] {
          assert_eq!(id(m), Some("nomic"), "{m}"); }
      assert_eq!(id("mixedbread-ai/mxbai-embed-large-v1"), Some("mxbai"));
      assert_eq!(id("BAAI/bge-small-en-v1.5"), Some("bge"));
      assert_eq!(id("intfloat/multilingual-e5-large"), Some("e5"));
      for m in ["text-embedding-3-small", "bge-m3", "granite-embedding", "gte-modernbert-base"] {
          assert_eq!(id(m), None, "{m}"); }
  }
  #[test] fn the_qwen3_query_keeps_the_instruction_of_the_old_catalog()
  // rule("qwen3-embedding").unwrap().query ==
  //   "Instruct: Given a web search query, retrieve relevant passages that answer the query\nQuery:"
  ```

  In `wire.rs`, add `a_known_model_gets_its_prefixes_and_a_name_that_says_so`:
  - With Ollama and `embeddinggemma`, `embed` sends `"title: none | text: a"` and `embed_query("q")` sends `"task: search result | query: q"`.
  - The name is `ollama:embeddinggemma+gemma`.
  - With `text-embedding-3-small` on OpenAI, the name is unchanged and the input is unchanged.

- [ ] **Step 2: Run them to verify they fail**

  Run `cargo test -p cw-embed prefix`. Expected: it fails to compile, because the `prefix` module does not exist.

- [ ] **Step 3: Implement the table and `rule`**

  The table, in this order. The first match wins.

  | id | parts | query | document |
  |---|---|---|---|
  | `gemma` | `["embeddinggemma"]` | `task: search result \| query: ` | `title: none \| text: ` |
  | `qwen3` | `["qwen3-embedding"]` | the catalog string above | `` |
  | `nomic` | `["nomic-embed-text"]` | `search_query: ` | `search_document: ` |
  | `mxbai` | `["mxbai-embed"]` | `Represent this sentence for searching relevant passages: ` | `` |
  | `bge` | `["bge-", "-v1.5"]` | same as `mxbai` | `` |
  | `e5` | `["e5-"]` | `query: ` | `passage: ` |

  - **Matching:** the model name is lowercased. Every part must occur. The first part must also start at the beginning or after a character that is not alphanumeric.
  - **Applying a rule:** the embedders look up the rule once, in `new`. `embed` puts the document prefix in front of each text before clipping. `embed_query` puts the query prefix in front. `name()` appends `+{id}`.
  - Check each prefix against its model card while implementing, and cite the card's URL in a comment above the table.

- [ ] **Step 4: Run the tests to verify they pass**

  Run `cargo test -p cw-embed`. Expected: PASS. Then run the workspace commands.

- [ ] **Step 5: Commit** as `feat(embed): add the prefixes known embedding models expect`.

---

### Task 3: Read the models a server offers

**Files:**
- Create: `crates/cw-embed/src/list.rs`
- Test: unit tests in `list.rs` (the parsers), and `wire.rs` (over HTTP)

**Interfaces:**
- Produces:
  - `pub struct Listed { pub id: String, pub embedding: Option<bool>, pub loaded: Option<bool> }`
  - `pub fn list_models(lookup: impl Fn(&str) -> Option<String>) -> Result<Option<Vec<Listed>>>`
  - `pub fn list_models_in(resolved: &cw_core::settings::Resolved) -> Result<Option<Vec<Listed>>>`
  - `None` means the provider lists nothing (`local`, `none`). The result is ordered: known embedding models first, then unknown ones, each group by id. Models known not to embed are left out.

| Provider | Request | Embedding flag | Loaded |
|---|---|---|---|
| `lmstudio` | `GET {root}/api/v0/models` | `type` is `embeddings` or `embedding` | `state` is `loaded` |
| `ollama` | `GET /api/tags` (`models[].name`), then `POST /api/show {"model": name}` for each | `capabilities` contains `embedding` | unknown |
| `llamacpp` | `GET {root}/v1/models` (`data[].id`) | unknown | unknown |
| `lemonade` | `GET {root}/v1/models` | `labels` contains `embeddings`; no `labels` field gives unknown | unknown |
| `mistralrs` | `GET {root}/v1/models` | unknown | unknown |
| `openai` | `GET {base}/models` | the id contains `embed` | unknown |

*Verify*:
- the shape of Ollama's `/api/show`;
- whether llama.cpp's router lists models that are not loaded on `/v1/models`; if it does not, use `GET {root}/models`;
- Lemonade's `labels`.

Every request sends the Bearer key when one is set, and uses `LIST_TIMEOUT`.

- [ ] **Step 1: Write the failing tests**

  The parser tests use one fixture per server, copied from the docs:
  - `lm_studio_marks_its_embedding_models`: the fixture has a `vlm`, an `llm` and an `embeddings` model. Only the embedding model is kept, with `embedding: Some(true)` and `loaded: Some(false)`.
  - `an_openai_list_keeps_the_embedding_ids`.
  - `lemonade_reads_the_labels`.
  - `the_order_puts_known_embedding_models_first`.

  In `wire.rs`:
  - `ollama_lists_each_model_with_its_capabilities`: `serve_many` returns tags for two models, then `show` for each. Expected ids are in order, each with `embedding: Some(true)`.
  - `an_ollama_model_whose_details_fail_is_still_listed` (Review Focus 4): the second `show` answers 500. That model is still listed, with `embedding: None`.
  - `a_server_that_does_not_answer_is_an_error_naming_it`: the address is a closed port. The error contains `LM Studio`.

- [ ] **Step 2: Run them to verify they fail**

  Run `cargo test -p cw-embed list`. Expected: it fails to compile.

- [ ] **Step 3: Implement**

  Write the pure parsers in `list.rs` and one `list_models` that dispatches on `CODEWIKI_EMBED`. Reuse `server_root`, and reuse the address and key lookups of Task 1.

- [ ] **Step 4: Run the tests to verify they pass**

  Run `cargo test -p cw-embed`. Expected: PASS. Then run the workspace commands.

- [ ] **Step 5: Commit** as `feat(embed): read the models a server offers`.

---

### Task 4: The backend's settings for the new servers and the model-list route

**Files:**
- Modify:
  - `crates/cw-server/src/settings.rs`: `EMBEDDER_KEYS` at :18, `checked` at :150, the clearing rule at :264-273, and a new `embed_models`;
  - `crates/cw-server/src/api.rs`: routes at :213-221, and a new handler beside `test_embedder` at :286.
- Test: `crates/cw-server/tests/api.rs`

**Interfaces:**
- Consumes: `cw_embed::list_models_in` (Task 3), and the keys from Task 1.
- Produces:
  - `pub(crate) fn embed_models(state: &AppState) -> Value`, with the shape `{"provider": id, "reachable": bool, "models": null | [{"id", "embedding", "loaded"}], "problem": null | text}`;
  - the route `GET /v1/settings/embed/models`. It needs the token and runs through `blocking(...)`, like `test_embedder`. It is answered off loopback too, as `/settings/test/embed` is.

- [ ] **Step 1: Write the failing tests** in `tests/api.rs`:

  - `the_local_servers_are_offered_with_their_addresses`:
    - `GET /v1/settings` lists the provider options in page order, with `builtin` still present on Windows until Task 6;
    - `lmstudio.url.default` is `http://127.0.0.1:1234`;
    - `lmstudio.api_key.state` is `unset`;
    - `llamacpp.url.default` is `http://127.0.0.1:9931`;
    - `lemonade.url.default` is `http://127.0.0.1:13305`;
    - `mistralrs.url.default` is `http://127.0.0.1:1234`.
  - `an_address_must_be_http`: `PUT {"lmstudio":{"url":"ftp://x"}}` gives 400. `"http://h:1/"` is saved as `http://h:1`.
  - `a_change_of_embedder_clears_the_model_unless_it_names_one`:
    - `ollama` with model `m`, then `lmstudio`, gives `embed.model.value` null;
    - `{"embed":{"provider":"lmstudio","model":"x"}}` keeps `x`;
    - a model set by the environment (`model_from_env`) is left alone.
  - `a_local_server_without_a_model_says_to_choose_one`: `embed.problem` contains `choose a model`.
  - `the_model_list_comes_from_the_chosen_server`: point `lmstudio.url` at a `stand_in` that serves the LM Studio fixture. `GET /v1/settings/embed/models` gives `reachable` true and one embedding model.
  - `the_model_list_says_when_the_server_does_not_answer`: point it at a closed port. `reachable` is false, `models` is null and `problem` names LM Studio.
  - `local_lists_no_models`: `models` is null and `reachable` is true.

- [ ] **Step 2: Run them to verify they fail**

  Run `cargo test -p cw-server --test api`. Expected: the new tests fail.

- [ ] **Step 3: Implement**

  - Add the new keys to `EMBEDDER_KEYS`, so that a change to any of them reconfigures the embedder.
  - `checked`: the four `*_URL` keys follow the rule `OLLAMA_HOST` has.
  - Clearing rule: when the provider changes, `!model_named`, and the model's source is not `Env`, clear `EMBED_MODEL`. This replaces the builtin-crossing test.
  - Write `embed_models` and its route.

- [ ] **Step 4: Run the tests to verify they pass**

  Run `cargo test -p cw-server`. Expected: PASS, the existing builtin tests included. Then run the workspace commands.

- [ ] **Step 5: Commit** as `feat(server): settings and model list for the local embedding servers`.

---

### Task 5: One background embedding job for every embedder

**Files:**
- Create: `crates/cw-server/src/embed_job.rs`, from `models.rs`, keeping `Jobs`, `Job`, `Phase`, `embed_repo`, `embed_all`, `after_update` and `resume`.
- Delete: `crates/cw-server/src/models.rs`, `crates/cw-server/tests/models.rs`.
- Modify:
  - `crates/cw-server/src/lib.rs`: `Host` at :59-81, `Inner` at :92-125, and `serve` at :152-169;
  - `crates/cw-server/src/api.rs`: :22, :216-221, :245-248, :293-354, :452;
  - `crates/codewiki/src/serve.rs:81`.
- Create: `crates/cw-server/tests/embed_job.rs`, which is not `cfg(windows)`.

**Interfaces:**
- Produces:
  - `pub struct Host { pub embedder: fn() -> Option<Arc<dyn cw_embed::Embedder>> }`. `Default` uses `cw_wiki::semantic::embedder`.
  - `enum Phase { Embedding, Done, Failed, Cancelled }`.
  - `struct Job { phase, model: Option<String>, repo, repo_index, repo_count, docs_done, docs_total, skipped, note, error }`. Here `model` is the embedder's name.
  - `pub(crate) fn start(state: &AppState) -> Result<Value>`, `after_update(state, root)` and `resume(state)`.
  - Routes, each refused by `embed_closed` with 403 `"this backend's embedding is managed on its own machine"` when `!settings_writable`:
    - `GET /v1/embed/job` answers `{"job": snapshot}`;
    - `POST /v1/embed/reembed` answers 202 `{"job"}`;
    - `POST /v1/embed/cancel` answers 200 `{"job"}`.
  - `/v1/status` keeps `model_job` as `{"phase","model","repo"}`.
- Removed:
  - `Kind`, `HardwareCache` and `Inner.hardware`;
  - `switch`, `wait_out_game`, `remove`, `describe`, `unavailable` and `store`;
  - the routes under `/v1/models*`;
  - the `Host` fields `hardware`, `game` and `pause_poll`.

  `resume` no longer checks the `builtin:` prefix, and `after_update` no longer checks `ENGINE`.

- [ ] **Step 1: Write the failing tests** in `tests/embed_job.rs`

  Reuse `StandIn` from the old file: it embeds with `LocalEmbedder`, sleeps 60 ms, and has batch 1. Add `Failing`, whose `embed` returns the error `"the server did not answer"`.

  - `a_backend_that_listens_beyond_this_machine_answers_no_job_route`: the three routes answer 403.
  - `an_update_leaves_the_rest_to_a_job_that_embeds_every_repository`: register and sync. Poll `GET /v1/embed/job` until `phase == "done"`, then check `docs_done == docs_total` and that `/v1/status` `model_job` is null.
  - `one_job_runs_at_a_time_and_cancel_stops_it`: the second reembed is refused while one runs; cancel gives `cancelled`.
  - `a_job_whose_embedder_fails_says_why_and_can_be_started_again`: with `Failing`, the phase is `failed` and `error` contains `did not answer`. A new `POST /v1/embed/reembed` is accepted.
  - `a_backend_that_starts_behind_tries_once` (Review Focus 5): run `cw_server::serve` with `Failing` on a data dir whose index is behind, as `shutdown_route_ends_serve` does. After start the job is `failed`. Two seconds later it is still the same failed job; no new job has started.

- [ ] **Step 2: Run them to verify they fail**

  Run `cargo test -p cw-server --test embed_job`. Expected: it fails to compile, because the routes and `Host` are the old ones.

- [ ] **Step 3: Implement**

  - Move the job and cut it to the interfaces above.
  - Rename `models_closed` to `embed_closed`.
  - `serve` keeps `set_inline_budget(Some(INLINE_BUDGET))` and `resume`.
  - Update `serve.rs` and `tests/api.rs` for the new `Host`.

- [ ] **Step 4: Run the tests to verify they pass**

  Run `cargo test -p cw-server`. Expected: PASS. Then run the workspace commands. The app still compiles: it reads `/v1/models` as untyped JSON, and Task 8 rewires it.

- [ ] **Step 5: Commit** as `refactor(server)!: run the embedding job for every embedder`. The body notes that the `/v1/models*` routes are gone.

---

### Task 6: Remove the built-in model

**Files:**
- Delete:
  - `crates/cw-models/`, `crates/cw-embed-engine/`;
  - `crates/codewiki/src/model.rs`, `crates/codewiki/src/bin/embed_cuda.rs`, `crates/codewiki/tests/model.rs`.
- Modify:
  - root `Cargo.toml:36-65`: drop the opt-level blocks for candle, gemm, pulp, rayon, tokenizers and cw-embed-engine;
  - `crates/codewiki/Cargo.toml`: drop `default-run` and its comment, `cuda`, the `[[bin]]`, `cw-models` and `cw-embed-engine`;
  - `crates/codewiki/src/main.rs`: :3, :136-152, :247-249, :793-798, :825-834, :855-858;
  - `crates/cw-embed/{src/lib.rs,Cargo.toml}`: `BuiltinEmbedder`, `CUDA_VRAM`, `DEVICES`, `EMBED_DEVICE_ENV` and `slow()`. `from_config_in` and `from_settings_in` fold into `from_config` and `from_settings`. Drop the `cw-models` dependency, and `tempfile` if nothing else uses it. `PROVIDERS` becomes `[&str; 8]`;
  - `crates/cw-wiki/src/semantic.rs`: drop the slow branch at :394. `Slot::configure_in` and `configure_in` go, and `configure(resolved)` remains;
  - `crates/cw-wiki/src/service.rs:51`: `reembed` and its tests in `tests/reembed.rs`. Keep the tests of `open_existing_index_in`;
  - `crates/cw-core/src/settings.rs`: `EMBED_DEVICE` leaves `ALL`, which becomes `[Key; 12]`;
  - `crates/cw-server/src/settings.rs`: `EMBED_DEVICE` and the device options at :95-100. The `offered` filter goes and options come straight from `PROVIDERS`. Drop the builtin validation at :279-282, `data_dir` passed to `configure` and `from_settings`, and the `cw-models` dependency in `Cargo.toml`;
  - `crates/cw-server/tests/api.rs`: :299 (options), and delete :492, :512 and :638;
  - `crates/cw-telemetry/src/lib.rs:1-2`: no "embedding worker".

- [ ] **Step 1: Write the failing check**

  This test goes in `crates/codewiki/tests/` as `no_engine.rs`:

  ```rust
  #[test]
  fn the_workspace_carries_no_model_engine() {
      let lock = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock")).unwrap();
      for name in ["candle-core", "tokenizers", "nvml-wrapper", "cw-models", "cw-embed-engine"] {
          assert!(!lock.contains(&format!("name = \"{name}\"")), "{name} is still in Cargo.lock");
      }
  }
  ```

  Also update the provider-options assertion in `tests/api.rs`: the options equal the eight ids in page order, on every platform.

- [ ] **Step 2: Run it to verify it fails**

  Run `cargo test -p codewiki --test no_engine`. Expected: FAIL, `candle-core is still in Cargo.lock`.

- [ ] **Step 3: Delete and edit as listed in Files**

  Then run `cargo check --workspace` without `--locked` once, so that `Cargo.lock` drops the crates.

- [ ] **Step 4: Run the checks to verify they pass**

  Run `cargo test --workspace`, then the workspace commands. Expected: PASS. `rg -n "cw_models|cw_embed_engine|BuiltinEmbedder|EMBED_DEVICE" crates` prints only `RETIRED_VARIABLES`, after Task 7.

- [ ] **Step 5: Commit** as `feat!: remove the built-in embedding model`.

---

### Task 7: Move users of the built-in model to a working state

**Files:**
- Modify:
  - `crates/cw-core/src/settings.rs`: `RETIRED_VARIABLES` at :118, and a new `retire_builtin`;
  - `crates/codewiki/src/serve.rs:86-99`;
  - `crates/cw-embed/src/lib.rs`, the `from_config` arm.
- Test: unit tests in `cw-core`, and `crates/cw-server/tests/api.rs`.

**Interfaces:**
- Produces:
  - `pub struct Retired { pub provider: bool, pub device: bool, pub freed: Option<u64> }`
  - `pub fn retire_builtin(data_dir: &Path) -> Result<Retired>`. If the saved `embed.provider` is `builtin`, it becomes `local` and `embed.model` is cleared. `embed.device` leaves the file. `<data_dir>\models\` is deleted, and its size in bytes is reported. A file that holds none of these is not written.
- `RETIRED_VARIABLES` becomes `[&str; 6]`, with `"CODEWIKI_EMBED_DEVICE"`.
- The `serve.rs` log for a retired variable reads `"{variable} is set, but CodeWiki no longer reads it; the variable has no effect"`.
- `from_config` with `builtin` gives the error `"the built-in embedder was removed; choose local, lmstudio, ollama, llamacpp, lemonade, mistralrs, openai or none"`.

- [ ] **Step 1: Write the failing tests**

  - `a_built_in_choice_becomes_word_forms_once`:
    - Write `{"version":1,"embed":{"provider":"builtin","model":"gte-modernbert-base","device":"cuda"}}` and `models\x\w.bin` (10 bytes).
    - `retire_builtin` gives `provider` true, `device` true and `freed` `Some(10)`.
    - The file then holds `embed.provider == "local"`, no `embed.model` and no `embed.device`, and `models\` is gone.
    - A second call returns all false and `None`, and leaves the file's bytes unchanged.
  - `other_choices_are_left_alone`: a file with `ollama` and `m` is unchanged.
  - In `tests/api.rs`, `a_built_in_embedder_named_by_the_environment_says_it_was_removed`: `env` returns `builtin` for `CODEWIKI_EMBED`. `embed.problem` contains `was removed`.

- [ ] **Step 2: Run them to verify they fail**

  Run `cargo test -p cw-core retire` and `cargo test -p cw-server --test api removed`. Expected: FAIL.

- [ ] **Step 3: Implement**

  - Write `retire_builtin`, editing the document directly for `embed.device`.
  - Call it in `serve.rs` right after `drop_retired`. Log `"moved the removed built-in embedder to local"` and `"deleted the built-in models, {n} MB"`, and never log a path.

- [ ] **Step 4: Run the tests to verify they pass**

  Run the workspace commands. Expected: PASS.

- [ ] **Step 5: Commit** as `feat(core): move a built-in embedder choice to word forms`.

---

### Task 8: Remove the old CUDA folder after an update

**Files:**
- Modify: `crates/cw-update/src/swap.rs:203`, `recover_and_clean`.
- Test: the unit tests in `swap.rs`.

**Interfaces:**
- `recover_and_clean(install)` also deletes `install\cuda` when `install\cuda\codewiki-embed-cuda.exe` exists. It adds `"cuda"` to the returned names. A failure, such as a locked file, is left for the next start.

- [ ] **Step 1: Write the failing test** `the_old_cuda_worker_folder_is_removed`

  Make an install dir with `cuda\codewiki-embed-cuda.exe` and `cuda\cublas64_13.dll`. After `recover_and_clean`, `cuda\` is gone and the result contains `"cuda"`.

  Also check that a `cuda\` folder without that exe is left alone.

- [ ] **Step 2: Run it to verify it fails**

  Run `cargo test -p cw-update cuda`. Expected: FAIL.

- [ ] **Step 3: Implement it** in `recover_and_clean`.

- [ ] **Step 4: Run the tests to verify they pass**

  Run `cargo test -p cw-update`. Expected: PASS.

- [ ] **Step 5: Commit** as `feat(update): remove the old graphics-card worker folder`.

---

### Task 9: The app without the built-in model, showing the job

**Files:**
- Modify:
  - `crates/codewiki-app/src/settings_pages.rs`:
    - `Model` fields :60-64 become `job: Option<&'a Value>`;
    - in `search()`, drop :440-462, :475-483 and :495;
    - delete :534-889, except `job`, which is rewritten, and `job_runs`;
    - the label at :387;
    - tests :1161-1188 and :1586-1930.
  - `crates/codewiki-app/src/settings_view.rs`:
    - :46-48, :81-89 and :133-137;
    - :210-228 and :325-399, which become `load_embed_job`, `take_embed_job` and `watch_embed_job`;
    - :524-534 and :731-752;
    - tests :1069-1126.
  - `crates/codewiki-app/src/doc.rs:84-92`: `InstallModel`, `UseModel`, `RemoveModel` and `DismissModelHint` go. `CancelModelJob` is renamed `CancelEmbedJob`.
  - `crates/codewiki-app/src/live.rs:464-569`: drop `ModelOffer`, `model_offer` and the "Search by meaning" card, and the tests :717-727, :860-949. Remove the `Span` import if it becomes unused.
  - `crates/codewiki-app/src/status.rs:68-80`: only `"Embedding {repo} for search"` or `"Embedding for search"` remain. Update the test at :229.
  - `crates/codewiki-app/src/main.rs:890-891,3470-3482`.
  - `crates/codewiki-app/src/prefs.rs`: add `pub fn remove(name: &str)`. `State::new` calls `prefs::remove("model_hint")`.

**Interfaces:**
- Consumes: `GET /v1/embed/job` and `POST /v1/embed/cancel` (Task 5).
- Produces:
  - `settings_view::State.embed_job: Option<Value>`.
  - `pub fn job_runs(job: &Value) -> bool`, which is true when `phase == "embedding"`.
  - `fn job(job: &Value) -> Vec<Block>`:
    - `embedding` gives `Progress { label: "Embedding {repo}", share, detail: "{docs_done} of {docs_total}", moving: true }` and Cancel (`CancelEmbedJob`);
    - `failed` gives an Error alert with `error`;
    - `cancelled` gives an Info alert;
    - `done` gives `note` and a `skipped` warning.

- [ ] **Step 1: Write the failing tests** in `settings_pages.rs`

  - `a_running_job_shows_how_far_it_is_and_can_be_cancelled`: the fixture `job` is `{"phase":"embedding","repo":"r","docs_done":5,"docs_total":20}`. The words contain `Embedding r 25%: 5 of 20`, and the actions contain `CancelEmbedJob`.
  - `a_failed_job_says_why`.
  - `the_search_page_offers_no_built_in_model`: the words contain neither `Built-in model` nor `Install`.

  In `status.rs`, check that no text mentions a game.

- [ ] **Step 2: Run them to verify they fail**

  Run `cargo test -p codewiki-app settings_pages::`. Expected: FAIL to compile.

- [ ] **Step 3: Implement the removals and the job wiring**

  The view loads the job when Search opens and after an embed change. It polls it every second while `job_runs`. When a job ends, it reads the settings again. A remote backend is skipped, as before.

- [ ] **Step 4: Run the tests to verify they pass**

  Run `cargo test -p codewiki-app`, then the workspace commands. Expected: PASS.

- [ ] **Step 5: Commit** as `feat(app)!: drop the built-in model from Settings and Home`.

---

### Task 10: Settings → Search for each server, with its model list

**Files:**
- Modify:
  - `crates/codewiki-app/src/settings.rs:82-129`;
  - `crates/codewiki-app/src/settings_pages.rs`: `embedder_name` at :384, `search()`, and the "Keys and addresses" card at :497-525;
  - `crates/codewiki-app/src/settings_view.rs`: the state, the loaders, `changes_the_embedder` at :204, and `run_settings_action`;
  - `crates/codewiki-app/src/doc.rs`.

**Interfaces:**
- Consumes: `GET /v1/settings/embed/models` (Task 4).
- Produces:
  - `FieldId` adds `LmStudioUrl`, `LmStudioKey`, `LlamaCppUrl`, `LlamaCppKey`, `LemonadeUrl`, `LemonadeKey` and `MistralRsUrl`, and `FieldId::ALL` becomes `[Self; 13]`.
    - `place()` maps them to `(server, "url")` or `(server, "api_key")`.
    - `secret()` is true for the three keys.
    - `placeholder()` gives the default address, or `"Only if the server asks for one"`.
  - `Action::RefreshEmbedModels`.
  - `settings_view::State.embed_models: Option<Value>`.
  - `fn server_rows(model: &Model, settings: &Value, provider: &str) -> Vec<Block>`.
  - `fn serve_help(provider: &str) -> Option<&'static str>`.
  - `Model.embed_models: Option<&'a Value>`.
- Labels:

  | Provider | Label |
  |---|---|
  | `lmstudio` | LM Studio |
  | `llamacpp` | Llama / llama.cpp |
  | `lemonade` | Lemonade |
  | `mistralrs` | mistral.rs |

- Rows, under the Embedder select:

  | Provider | Rows |
  |---|---|
  | `lmstudio` | address, key |
  | `ollama` | `OllamaHost` |
  | `llamacpp` | address, key |
  | `lemonade` | address, key |
  | `mistralrs` | address |
  | `openai` | `OpenAiKey`, `OpenAiBaseUrl` |
  | `local`, `none` | none |

  Then comes the Model row, for every provider except `local` and `none`:
  - When `embed_models.models` is a non-empty array, it is a `Block::Select` labelled "Model", whose options are the ids. Each option's action is `SetSetting { section: "embed", field: "model", value: Some(id) }`. The selected option is the current `embed.model.value`, and `locked` is `lock(.. embed.model)`.
  - Otherwise it is `field(.., FieldId::EmbedModel, "Model", ..)`. If `reachable` is false, a Warning alert follows with `problem`.
  - In both cases a `Buttons([Press::new("Refresh", Action::RefreshEmbedModels)])` follows.

  Then the `serve_help` text and the existing "A key is stored encrypted…" line, when a key row shows. The "Keys and addresses" card goes.
- `serve_help` drafts. Check each command against the server's docs:
  - `lmstudio`: "In LM Studio, download an embedding model and start the server on the Developer tab. LM Studio and mistral.rs both use port 1234."
  - `ollama`: "Run `ollama pull embeddinggemma`, or name another embedding model."
  - `llamacpp`: "Load an embedding model (GGUF) in Llama for Windows, or start llama-server with one. Both use port 9931."
  - `lemonade`: "Pull an embedding model in Lemonade, such as nomic-embed-text-v1-GGUF."
  - `mistralrs`: "Run `mistralrs serve -m Qwen/Qwen3-Embedding-0.6B`. mistral.rs and LM Studio both use port 1234."

- [ ] **Step 1: Write the failing tests**

  - `each_server_shows_only_its_own_rows`: for each provider, `doc.fields()` equals its rows from the table, plus `EmbedModel` when no list is loaded.
  - `a_listed_server_offers_its_models_to_choose`: `embed_models` is `{"reachable":true,"models":[{"id":"a","embedding":true},{"id":"b","embedding":null}]}`. The words contain `Model: a (a / b)`, and the actions contain `SetSetting{embed, model, Some("b")}`.
  - `a_server_that_does_not_answer_keeps_the_typed_model_and_says_why`: the words contain the `problem` text, and `fields()` contains `EmbedModel`.
  - Update `semantic_search_offers_the_embedders_a_test_and_a_sync` at :1544 to the eight providers.
  - In `settings.rs`, the secrets test at :203 becomes `[OpenAiKey, LmStudioKey, LlamaCppKey, LemonadeKey, RemoteToken]`.
  - In `settings_view.rs`, `a_change_to_a_server_address_reads_the_models_again`: `changes_the_embedder` is true for a body with `lmstudio`, `llamacpp`, `lemonade`, `mistralrs`, `ollama`, `openai` or `embed`.

- [ ] **Step 2: Run them to verify they fail**

  Run `cargo test -p codewiki-app settings`. Expected: FAIL.

- [ ] **Step 3: Implement**

  - `load_embed_models` calls `backend.get("/v1/settings/embed/models")`. It runs when Search opens, after a change `changes_the_embedder` accepts, and on `RefreshEmbedModels`.
  - The new `FieldId`s get their inputs from `State::new` without extra code.
  - `cw-client` needs no new method. It has no typed settings calls, so the spec's "matching call" is `Backend::get`.

- [ ] **Step 4: Run the tests to verify they pass**

  Run `cargo test -p codewiki-app`, then the workspace commands. Expected: PASS.

- [ ] **Step 5: Commit** as `feat(app): choose a local embedding server and its model in Settings`.

---

### Task 11: Packaging, scripts, CI and the VM test without CUDA

**Files:**
- Delete: `scripts/cuda-worker.ps1`, `scripts/cuda-toolkit.ps1`, `scripts/cuda-common.ps1`, `docs/superpowers/plans/2026-11-20-builtin-dropdown-dev-cuda.md`, `docs/superpowers/specs/2026-11-20-builtin-dropdown-dev-cuda-design.md`.
- Modify:
  - `scripts/package.ps1`: the header :4-14, `-NoCuda` and `-CudaToolkit` :24-29, the dot-source :34, the cuda build :48-58, the import check :74-76 (keep the `codewiki.exe` import check, minus `Get-CudaImports`), the copy :78-87, and the symbols :94;
  - `scripts/install.ps1`: :26, :42-46;
  - `scripts/uninstall.ps1`: :7, :12, :39. It keeps removing `cuda` and `<data dir>\models` as leftovers, and the text says so;
  - `scripts/vm/test-release.ps1`: :212-240, :349-350, and cases EM1-EM3 at :1497-1609;
  - `.github/workflows/release.yml`: :19-21, :41-44;
  - `.idea/CodeWiki.iml`: drop the `cw-embed-engine/src` and `cw-models/src` folders;
  - `NOTICE`: :43-55.

- [ ] **Step 1: Run the checks that prove them**

  Run:
  ```
  pwsh scripts\package.ps1
  ```
  Expected: `dist\CodeWiki-<version>-windows-x64.zip` is built. `tar -tf` on it lists no `cuda/`.

  Then run:
  ```
  pwsh -NoProfile -Command "foreach ($f in 'scripts\package.ps1','scripts\install.ps1','scripts\uninstall.ps1','scripts\vm\test-release.ps1') { $null = [System.Management.Automation.Language.Parser]::ParseFile($f, [ref]$null, [ref]$e); if ($e) { $f; $e } }"
  ```
  Expected: it prints nothing.

  Finally run `rg -n "cuda|EM1|EM2|EM3|embed-worker" scripts .github NOTICE`. Expected: only the `uninstall.ps1` leftovers.

- [ ] **Step 2: Run `cargo deny check advisories bans sources`**

  Expected: `advisories ok, bans ok, sources ok`.

- [ ] **Step 3: Commit** as `chore: package without the graphics-card worker`. Run `pwsh scripts\clean.ps1` afterwards.

---

### Task 12: Docs, the wiki and a look at the app

**Files:**
- Modify:
  - `README.md`: :19, :46, :162-187, :199. Semantic search becomes a table of the servers: address, how to serve an embedding model, devices on Windows, and port;
  - `AGENTS.md`:
    - the hard rules on candle opt-level and the CUDA program go;
    - the layout table drops `cw-models` and `cw-embed-engine`;
    - "Where state lives" and "Logs" drop `models\`, `model_hint.txt`, `embed-worker.log` and the `embed-worker` tag;
    - the sentence on `unsafe` modules drops `game.rs`;
    - the settings list gets the seven variables;
    - "Built-in embedding models" becomes "Embedding servers": providers, quirks, the prefix table, the job and its routes, the migration;
    - drop `default-run` from CI;
    - drop EM cases from the VM notes;
  - `ROADMAP.md`: crate count (17), test count from `cargo test --workspace`, the M7 line.
- Memory, outside the repo:
  - update `C:\Users\PC\.claude\projects\D--CodeWiki\memory\vm-test-scope.md`: the EM cases are gone;
  - delete `cuda-toolkit-location.md` and its `MEMORY.md` line.

- [ ] **Step 1: Regenerate the wiki**

  Run `cargo run -p codewiki -- update` at the worktree root.
  Expected: it reports that links resolved. `obsidian/01-Core/` holds no `cw-models` or `cw-embed-engine` folder.

- [ ] **Step 2: Check the app**

  Follow `AGENTS.md` "Checking the app without taking focus":
  - Give the app its own `CODEWIKI_DATA_DIR` and `APPDATA`.
  - Before each launch, write `settings.json` there with `{"version":1,"embed":{"provider":"<p>"}}` for `lmstudio`, `llamacpp`, `lemonade`, `mistralrs` and `ollama`.
  - Start `codewiki-app <repo> settings:search` through `ProcessStartInfo` with `CODEWIKI_INSTANCE=new` and `CODEWIKI_BACKDROP=#202020`.
  - Capture with `PrintWindow` into `report\local-embedders-<date>\`, then quit through `palette:Quit CodeWiki`.

  Expected: each picture shows that server's address row, its key row where it has one, the Model field, the warning that the server does not answer (none runs here), and its help line.

  Write `report\local-embedders-<date>\report.md` with the summary first, as in the memory `test-report-format`.

- [ ] **Step 3: Run the final checks**

  Run the three workspace commands, `cargo deny check advisories bans sources`, and `cargo test -p codewiki --test no_engine`. Expected: all pass.

- [ ] **Step 4: Commit**, in two commits:
  - `docs: document the local embedding servers`, holding README, AGENTS and ROADMAP;
  - `docs(wiki): regenerate the wiki`, holding `obsidian/`.
