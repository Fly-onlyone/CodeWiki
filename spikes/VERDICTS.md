# Spike verdicts

Each spike answered one question before the stack was committed to. Code in this folder is throwaway and sits outside the workspace.

## S1 — GPUI on Windows: pass

Question: does `gpui-kit =0.9.1` build and run on this machine, and can it render a note?

- Release toolchain: Rust 1.99.1, MSVC, no extra setup. First build 3 min 34 s.
- A window opens; idle memory about 75 MB for the hello-world, about 90–115 MB for the real app with a note open.
- `TextView::markdown` renders vault notes (headings, lists, inline code, blockquotes, emoji). Links are intercepted with `on_link_click`, which is how wikilinks navigate.
- Theme switching works through `ThemeRegistry` and `Theme::apply_config`; 21 theme files load from JSON.
- Not measured: the 5,000-line scroll rate at 60 fps. The viewer uses the virtualized `scrollable(true)` mode; a frame-rate measurement is still owed.

## S2 — Mermaid rendering: pass

Question: can a Rust renderer draw the Mermaid found in real vaults?

- Renderer: `merman` 0.9.3 with `default-features = false` and features `svg`, `png`, `diagram-flowchart`, `diagram-sequence`, `diagram-state`.
- Corpus: every Mermaid block in five vaults (this repository, TradingAgent, ZZZ bot, Image Sorter, MusicMod).
- Result: 62 of 65 blocks render (95.4%), mean 1.1 ms, slowest 3.2 ms.
- The 3 failures are flowchart labels written as `[/ws/agent]`. `[/` opens a parallelogram in Mermaid, so the source itself is invalid; the notes need `["/ws/agent"]`.
- `sequenceDiagram` with `autonumber` renders with visible numbers.
- Licenses: no EPL (ELK layout is excluded by the feature list). `cssparser` and its helpers are MPL-2.0, a file-level copyleft that does not extend to this project's code; it is allowed in `deny.toml` with that note.
- Diagrams follow the app theme through Mermaid's `init` directive (light or dark palette).

## S3 — OpenUI Lang parser and native renderer: pass

Question: can a Rust parser match the reference parser, and can the result be drawn natively?

- Parser: `crates/openui-lang` (lexer, statement splitter with auto-close, Pratt expression parser, static evaluator with the specification's error codes). No dependencies.
- Differential check: `spikes/s3-openui/` runs the reference TypeScript parser (`@openuidev/lang-core`, MIT) under bun on a 55-program corpus and compares its AST with the Rust AST. Result: 55 of 55 match. The corpus covers the specification's examples, operator precedence, ternaries across lines, member and index access, objects, strings and escapes, comments, negative numbers, partial input and malformed lines.
  ```
  cargo run -p openui-lang --example dump_ast -- spikes/s3-openui/corpus.txt rust.json
  bun spikes/s3-openui/reference.ts <openui-checkout> spikes/s3-openui/corpus.txt ref.json
  python spikes/s3-openui/compare.py ref.json rust.json spikes/s3-openui/corpus.txt
  ```
- Known difference, by design: the reference marks only custom `@Name()` calls with `fn`; the Rust AST records the `@` prefix on every function call. The comparison ignores that flag.
- The published specification has no formal grammar, no precedence table and no standard component library, so behavior follows the reference implementation and the component set is CodeWiki's own (`crates/cw-illustrate`, 18 components).
- Renderer: `crates/codewiki-app/src/illustration.rs` draws every library component with GPUI elements. Checked by eye on a showcase: card, header, text, stat tiles, badges with tones, divider, steps, tabs, table, bar chart, pie shares, stack and grid all render.
- Limits: `LineChart` is drawn with bars; `PieChart` is drawn as proportional shares, not a circle; tabs show all panels stacked because illustrations are static. Queries, mutations and actions are rejected with a warning.

## S4 — Graph canvas: pass, with level of detail

Question: can GPUI draw and pan a 50,000-node graph at 60 fps?

Layout for the graph is computed in the backend (`cw-graph`, ForceAtlas2 with Barnes-Hut) and served by `GET /v1/repos/{id}/graph`. Measured on 2026-11-12 with a release build and `CODEWIKI_GRAPH_STRESS`, redrawing every frame in a 1500x950 window.

| Case | Paint | Frame |
|---|---|---|
| 50,000 nodes, 100,000 edges, one quad per node | 249 ms | — |
| 50,000 nodes, overview (density cells, one path per color) | 2.0 ms | 6.4 ms |
| 4,900 nodes in detail (octagons, 5,000 edges) | 23.2 ms | 32.9 ms |
| 1,950 nodes in detail (octagons, 2,500 edges) | 7.5 ms | 14.7 ms |

- Verdict: 60 fps holds only with level of detail. Above 2,000 nodes in view the canvas draws density cells and at most 1,200 sampled edges; at or below that it draws each node and at most 2,500 edges.
- One `paint_quad` call costs 5 to 9 microseconds, so shapes are batched into one filled path per palette color.
- The backend layout time for 50,000 nodes was not measured; the stress graph is synthetic and laid out in the app.

## S5 — Built-in embedding engine: gate A (processor) pass; gate B (CUDA) open

Question: can `candle`, compiled into a CodeWiki program, run the three embedding models of the catalog, on this machine and on a clean Windows?

Code: `spikes/s5-embed-engine`. `fetch.ps1` downloads the models at pinned revisions, `reference.py` computes reference vectors with sentence-transformers, `compare.py` compares, `vm.ps1` runs the program in the guest. Measured on 2026-11-19 on an i7-12700F (12 cores), 15.8 GB RAM, with `candle` 0.11.1 and `tokenizers` 0.23.2.

Gate A, processor:

- All three models load with candle's own `modernbert` and `qwen3` modules, and the vectors equal the reference: the cosine between matching vectors is 1.000000 for each model, and the largest difference of a similarity is 0.000001. `gte-modernbert-base` and `Qwen3-Embedding-0.6B` also print the exact scores of their model cards (`[0.4289, 0.7131, 0.3366]`; `[0.7646, 0.1414]`, `[0.1355, 0.6000]`). The granite card prints none.
- What loading needs:
  - The published weights hold half-precision numbers; `VarBuilder::from_buffered_safetensors` with `DType::F32` converts them as it reads. candle's memory-mapped loader is an `unsafe fn` and is not used.
  - candle asks for tensors named `model.<name>`; an embedding checkpoint names them `<name>`. `VarBuilder::rename_f` strips the prefix.
  - ModernBERT takes a padded batch and the state of the first token. candle's Qwen3 takes no padding mask and keeps a cache between calls, so each text runs alone on a clone of the model, and the state of the last token is taken.
  - Padding and truncation are not in the `tokenizer.json` files; the program sets them. Qwen3's file appends the end-of-text token by itself.
- Speed, release build, texts cut at 500 tokens:

  | Model | Weights | Loads in | Documents/s | Tokens/s |
  |---|---|---|---|---|
  | granite-embedding-small-english-r2 | 95 MB | 0.2 s | 3.2 | 1600 |
  | gte-modernbert-base | 298 MB | 0.4 to 0.7 s | 1.5 | 730 |
  | Qwen3-Embedding-0.6B | 1.19 GB | 1.4 to 2.4 s | 0.6 | 290 |

  With texts of 64 tokens, the length of a symbol's document, gte-modernbert-base embeds 15 documents a second and Qwen3 3.2.
- A dev build (unoptimized) is 40 times slower: gte-modernbert-base makes 24 tokens a second against 963. A dev backend cannot embed a real repository unless the candle crates are optimized in the dev profile.
- Build: 231 crates; a release build from nothing takes 1 min 50 s and its `target\` is 1.6 GB; a dev build takes 35 s more and 1.5 GB more. The release exe is 7.2 MB with the engine, `sysinfo`, `nvml-wrapper` and the Windows counter calls in it.
- No C or C++ is compiled for the engine: `cc` enters only through `ring`, which the workspace already has. `tokenizers` needs `default-features = false` and `fancy-regex`.
- `cargo deny --all-features check` with the repository's `deny.toml` passes on the spike's tree, the CUDA crates included: advisories, bans, licenses and sources ok.
- The exe, built with `+crt-static`, imports only Windows' own DLLs (`kernel32`, `ntdll`, `advapi32`, `shell32`, `pdh`, `psapi`, `powrprof`, `oleaut32`, `bcryptprimitives`): no `VCRUNTIME140.dll`, no NVIDIA DLL.
- Clean guest (Tiny 11, checkpoint `Clean`, 2048 MB, no graphics card; `vcruntime140.dll`, `msvcp140.dll` and `nvml.dll` are absent from `System32`): the program starts, embeds with the small model at 2.5 documents a second, and its vectors equal the reference (cosine 1.000000).
- The machine readings work, on the host and in the guest:
  - `sysinfo` 0.40.4: memory, physical cores, free disk space.
  - `nvml-wrapper` 0.13.0: "NVIDIA GeForce RTX 4070, 12.0 GB video memory" on the host; without a driver `Nvml::init` returns an error ("LoadLibraryExW failed") and the program goes on.
  - `SHQueryUserNotificationState` answers 5 on a desktop.
  - The counter `\GPU Engine(*engtype_3D)\Utilization Percentage` gives the 3D load of each process by its id (`pid_<id>_...`), also in the guest.
- Not measured: a model larger than the small one in the guest (2048 MB is too little for the others), and the two readings while a game runs.

Gate B, CUDA: not run yet. It needs the CUDA toolkit on this machine.
