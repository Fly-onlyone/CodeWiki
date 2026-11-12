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
