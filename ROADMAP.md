# CodeWiki roadmap

Status as of 2026-11-12. "Verified" means covered by automated tests or by a live run recorded here or in `spikes/VERDICTS.md`. "Seen" means checked by eye on a window-only screenshot. 

## Current state

- 16 crates. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (285 tests) and `cargo deny check licenses bans sources` all pass.
- CodeWiki documents its own repository: 162 notes, 100% of wikilinks resolve, a second sync writes nothing.
- The Docker image builds (271 MB) and was exercised over HTTP and through the MCP bridge.

## Phase 1 — wiki core

| Item | Status | Notes |
|---|---|---|
| M0 Scaffold, license policy, CI workflow | Done | The CI workflow file has never run on a remote |
| M1 Rust parser and name resolver | Verified | |
| M2 Graph metrics and layout | Verified | PageRank, betweenness, SCC, Louvain, ForceAtlas2 |
| M3 Vault model, registry, linter, manifest | Verified | |
| M4 Keyless wiki generation in the owner's style | Verified | |
| M5 LLM providers and prose writer | Partly verified | Claude Code CLI provider run live for notes, illustrations and `ask`. The Anthropic, OpenAI-compatible and Ollama providers are tested over real HTTP against a stand-in server (request path, headers, body, response and error handling), not against the real services: this machine has no API key and no Ollama |
| M6 Incremental updates, proposals, adoption | Verified | |
| M7 HTTP server with token auth | Verified | |
| M7 Docker image | Verified | Final image (271 MB) built and run: 401 without token, repository registered by container path, sync, semantic search, workspace, signed webhook (202, and 401 on a bad signature), runs as a non-root user. The MCP bridge was driven against an earlier build of the image |
| M7 Semantic search | Verified with the built-in embedder | Vectors stored in the index, merged with full-text results by rank. The Ollama and OpenAI-compatible embedders are tested over real HTTP against a stand-in server, not against the real services. See Deviations |
| M8 MCP bridge and Claude Code plugin | Verified | 24 tools. All were driven over real stdio except `get_coverage` with a report present (it answered "no coverage report", as it should here). A live Claude Code run called tools through the plugin's `.mcp.json` |
| M8 Codex integration | Skipped | Not tested by the owner's choice (no Codex subscription at present). Config is documented in `plugins/codex/README.md` |
| M9 Viewer, search, sync, review queue | Seen | |
| M9 Native Mermaid diagrams | Verified | 62 of 65 real diagrams render |
| M9 Native illustrations (OpenUI Lang) | Verified | Parser matches the reference on 55 of 55 programs |
| M9 Graph view | Verified | 60 fps with level of detail; numbers in `spikes/VERDICTS.md` |
| M9 Theme picker | Seen | 26 theme files; the choice is remembered |
| M9 Tabs, Ctrl+K palette, settings, Open in Obsidian | Verified in part | Driven by input posted to the window: palette opened by its button, filtered by typing, moved with arrow keys, run with Enter, closed with Escape; new tab by its button. Ctrl chords (Ctrl+K, Ctrl+T, Ctrl+W, Ctrl+Tab) could not be sent that way and are untried |
| M9 Report pages | Seen in part | Decisions, health, dead code, drift, tests, coverage, security, refactoring plan, workspace, API contracts. Page builders are unit-tested; the health and contracts pages were seen in the app |
| M10 Note editor | Verified | Source editor beside a live preview, wikilink completion, unsaved-edit guard. Driven by posted input: typing updates the preview, a suggestion click writes the full `[[Name]]`, Save writes the file and the note comes back marked as edited by hand. Ctrl+S is untried. See Deviations |
| M11 Mica and custom title bar | Seen | Custom title bar with system caption buttons seen. Mica is requested from Windows but cannot be confirmed from a window capture; snap layouts were not tried by hand |
| M11 Installer | Verified | `scripts/package.ps1` builds a zip with `install.ps1` and `uninstall.ps1`; install, run and uninstall were tested into a scratch folder. See Deviations |

## Later phases

| Phase | Status | Done | Missing |
|---|---|---|---|
| P2 Git intelligence | Verified | Hotspots, ownership, bus factor, co-change, file risk, change risk, blast radius; ownership and change-coupling pages in the app | The two app pages are unit-tested but were not seen on a repository with history |
| P3 Decisions | Verified | ADR files and decision notes, staleness, `why`, decisions page in the app | |
| P4 Code health | Verified | Dead code, doc drift and coverage, 9 health detectors | |
| P5 Tests and impacted tests | Verified | Test map, impacted tests for a change, suggested commands | Rust only; reachability over the dependency graph, not execution |
| P5 Coverage | Verified | LCOV and Cobertura import, least-covered files ranked by risk | Line coverage only |
| P5 Security | Verified | 8 lexical rules, suppression comment, included in `check` | Rust only; no data flow |
| P5 Refactoring plans | Verified | Split file, extract type, break cycle, move function | |
| P5 More languages | Verified | Python, TypeScript/TSX, JavaScript and Java: symbols, imports, calls, inheritance, and full wiki planning (package notes, module notes, class notes, startup flows, integrations, operations). A real Python repository produced 184 notes with every link resolving and an idempotent second sync | A Python folder without `__init__.py` gives one package per subfolder. npm is assumed for JavaScript. TypeScript path aliases are not resolved |
| P6 CI outputs | Verified | `check` with text, SARIF and JUnit | |
| P6 Agent briefing | Verified | Briefing block for `CLAUDE.md` and `AGENTS.md` | |
| P6 Git webhook | Verified | `POST /v1/hooks/git` with GitHub signature or GitLab token; off unless `CODEWIKI_WEBHOOK_SECRET` is set | Matches a repository by folder name only; no branch filter; no debounce |
| P6 Agent hooks | Verified | `codewiki hook session-start` briefing, wired to Claude Code's SessionStart hook in the plugin; `claude plugin validate` passes | Not observed inside a live interactive Claude Code session |
| P6 Workspace view | Verified | `GET /v1/workspace`, CLI, MCP tool, app page: per-repository status and shared dependencies | Tried with one repository registered |
| P6 API contracts | Verified | Provided and consumed HTTP endpoints per repository and matched across the workspace. On this repository: 44 routes provided, 48 calls matched, 3 calls to outside services, 1 route nothing calls (the webhook) | Rust only and lexical. A path parameter matches any one segment, so unused routes are under-reported |

## Deviations from the original plan

- **Embeddings.** The default embedder is built in: it hashes words, word pairs and character trigrams into a 384-dimension vector. It needs no model file and works in Docker, and it finds other forms of the words typed, but it does not know that two unrelated words mean the same thing. A neural model is available through Ollama or an OpenAI-compatible endpoint (`CODEWIKI_EMBED`). `fastembed` and `sqlite-vec` are not used; vectors are compared in Rust.
- **Editor.** The plan asked for an in-place live-preview editor. What exists is a source editor with a live preview beside it.
- **Installer.** A zip with PowerShell install and uninstall scripts, not an MSI. No installer toolchain (WiX, Inno Setup) is on this machine and none was installed.
- Community detection is Louvain, not Leiden.
- Git access uses the `git` command line, not `gix`.
- Notes are rendered by Rust code in `crates/cw-wiki/src/render.rs`. There is no template pack and no `restyle` command.
- `deny.toml` allows MPL-2.0, used unmodified by the Mermaid renderer's CSS parser.
- Line charts in illustrations are drawn as bars; pie charts as proportional shares.
- Inheritance in Python, TypeScript and Java is stored as an `implements` edge; there is no separate `extends` kind.

## Known issues

- **Hotspots are empty for this repository** because it has no commits.
- Search with the built-in embedder ranks by shared words and word stems; a query such as "resolving names" can put a loosely related symbol first.
- Dead-code findings in Python, TypeScript, JavaScript and Java are reported as "possible", never "likely", because dynamic dispatch hides references from a static index.
- The editor has no Markdown syntax colouring. The UI kit's Markdown highlighter needs a different `tree-sitter` version than the parsers use, and Cargo cannot select both.

## Disk use

Debug builds once filled 44 GB. The dev profile now builds dependencies without debug information and without the incremental cache; a full build with tests takes about 7 GB. `scripts/clean.ps1` removes debug artifacts and spike build folders.

## Checks left to the owner

These cannot be done from an automated session on this machine.

- Run the Anthropic API, OpenAI-compatible and Ollama providers and embedders against the real services. Needs an API key or an Ollama install.
- Try the Ctrl shortcuts (Ctrl+K, Ctrl+T, Ctrl+W, Ctrl+Tab, Ctrl+S) and Windows snap layouts by hand. They need real keyboard focus.
- Confirm the Mica backdrop by eye.
- Open the session-start briefing in an interactive Claude Code session with the plugin installed.
- Codex round trip, when a Codex subscription is available.
