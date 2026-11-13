# CodeWiki roadmap

Status as of 2026-11-12. "Verified" means covered by automated tests or by a live run recorded here or in `spikes/VERDICTS.md`. "Seen" means checked by eye on a window-only screenshot. 

## Current state

- 16 crates. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (372 tests) and `cargo deny check` all pass.
- CodeWiki documents its own repository: 162 notes, 100% of wikilinks resolve, a second sync writes nothing.
- The Docker image builds (271 MB) and was exercised over HTTP and through the MCP bridge.

## Phase 1 — wiki core

| Item | Status | Notes |
|---|---|---|
| M0 Scaffold, license policy, CI workflow | Verified | Published at https://github.com/Fly-onlyone/CodeWiki. CI passes on GitHub: format, lint and all tests on Windows; licenses, bans, sources and advisories. 7 "unmaintained" advisories for crates the UI kit depends on are ignored by ID in `deny.toml`; any other advisory fails the build |
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
| M9 Theme picker | Seen | 53 themes in 27 files; the choice is remembered. The default is CodeWiki Violet, one of eight CodeWiki themes (four hues, each dark and light) whose text colors are held to WCAG AA by tests. A test fails on a color key gpui-kit does not read. Every theme tints the chrome toward its primary color, sets section labels in its link color and draws sidebar and toolbar icons in its own palette; the CodeWiki themes' backgrounds, borders and muted text carry their hue, and their icon colors are held to 3:1. Seen in CodeWiki Violet dark and light, CodeWiki Ember and Gruvbox Dark |
| M9 Tabs, Ctrl+K palette, settings, Open in Obsidian | Verified in part | Driven by input posted to the window: palette opened by its button, filtered by typing, moved with arrow keys, run with Enter, closed with Escape; new tab by its button. The tab row is gpui-kit's tab bar: opening, switching and closing a tab were driven by posted clicks. Ctrl chords (Ctrl+K, Ctrl+T, Ctrl+W, Ctrl+Tab) could not be sent that way; they were pressed on a VM's keyboard on 2026-11-13 and work |
| M9 Report pages | Seen in part | Decisions, health, dead code, drift, tests, coverage, security, refactoring plan, workspace, API contracts. Page builders are unit-tested; the health and contracts pages were seen in the app |
| M10 Note editor | Verified | Source editor beside a live preview, wikilink completion, unsaved-edit guard. Driven by posted input: typing updates the preview, a suggestion click writes the full `[[Name]]`, Save writes the file and the note comes back marked as edited by hand. Ctrl+S was pressed on a VM's keyboard on 2026-11-13 and saves. See Deviations |
| M11 Glass over Mica, custom title bar | Seen in part | Custom title bar with system caption buttons seen. Every surface is translucent over the backdrop, at an opacity worked out per theme so that text keeps its contrast (unit-tested). The layering was seen over a forced backdrop color (`CODEWIKI_BACKDROP`). The Mica material itself cannot be confirmed from a window capture; snap layouts were not tried by hand |
| M11 Fonts, reading typography, icons | Seen | Inter and JetBrains Mono are bundled. A note has a bounded line length, a heading scale and styled code blocks and tables. Material Icons in the sidebar, toolbar, tabs, palette and pages; a test loads every one |
| M11 Material UI components | Seen in part | Every control is drawn to MUI's measurements with TradingAgent's look: buttons, text field, tabs, list sidebar, palette, alerts, chips, cards, tooltips. A small button measures 30.7 px high (MUI 30.75) and the text field 40 px. Hover, ripple, the sliding tab bar and folder collapse were seen at reduced speed (`CODEWIKI_MOTION_SCALE`). Tab order and Enter or Space on the new controls were not checked: the checks cannot post keys |
| M11 Pages built from components | Seen in part | The twenty generated pages are stat tiles, tables, cards and pressable rows, not Markdown; each has a test that reads its content and its actions. Settings, themes, health, tests and a file page were seen in Violet dark and light, and a click on a theme card applied the theme. The proposal and sync pages were not seen. Illustrations use the same components; their tab strip was not seen, as no note here has one |
| M11 Motion and drifting backdrop | Seen in part | A setting follows Windows, reduces or forces animations: clicked, saved and seen again after a restart. The backdrop drifts in a focused window (seen with `CODEWIKI_BACKDROP_DRIFT`) at about 4% of one core in a release build, and the app is not redrawn by it. Motion runs on springs with TradingAgent's Framer Motion presets. A page entering was caught at one tenth speed: its blocks arriving in turn and its headline numbers counting up. In a release build the app uses 0% of one core at rest, 7 to 12% during a ripple and 9 to 19% during a page entrance of about a second. GPUI cannot scale an element, so Framer's hover and press scale are left out. The palette's fade-out was not caught on a capture |
| M11 Installer | Verified in a clean VM, with open defects | `scripts/package.ps1` builds a zip with `install.ps1` and `uninstall.ps1`. Tested on 2026-11-13 in a clean guest (Windows 11 Pro 25H2 from a tiny11 image, no VC++ runtime, no git) by `scripts/vm/test-release.ps1`, first on VirtualBox and then on Hyper-V: install, PATH, shortcut, CLI, wiki generation, MCP bridge, app start, keyboard, mouse clicks on the toolbar, a Windows restart and uninstall. The package from `main` did not start there (`VCRUNTIME140.dll` missing) and its backend opened a terminal window; both are fixed, and the fixed package passed 27 of 32 cases on Hyper-V, both without a graphics card in the guest and with a share of the host's (where the idle app costs 6 percent of the processors and loads the NVIDIA driver). Open defects are under Known issues. The reports, with screenshots and recordings, are kept on the development machine outside git (`report/` is ignored). See Deviations |

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
| P6 Agent hooks | Verified | `codewiki hook session-start` briefing, wired to Claude Code's SessionStart hook in the plugin; `claude plugin validate` passes. In a real Claude Code run with the plugin loaded, the briefing arrived in the model's context | |
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

- Search with the built-in embedder ranks by shared words and word stems; a query such as "resolving names" can put a loosely related symbol first.
- Dead-code findings in Python, TypeScript, JavaScript and Java are reported as "possible", never "likely", because dynamic dispatch hides references from a static index.
- The editor has no Markdown syntax colouring. The UI kit's Markdown highlighter needs a different `tree-sitter` version than the parsers use, and Cargo cannot select both.

Found by the VM tests of 2026-11-13, on VirtualBox and again on Hyper-V, not yet fixed:

- Without a graphics processor (Direct3D on the software renderer) an idle window uses every core: both processors of a Hyper-V guest were 97 to 99 percent busy, against 12 percent before the app started and 13 percent with motion reduced, and the guest answered no command meanwhile. The load was the same for a window without the keyboard focus. The drifting backdrop redraws 24 times a second.
- With motion reduced, a generated page shows only a "CodeWiki" heading until the first input of any kind, a pointer move included. `mui/motion.rs` holds an entrance at full transparency while it waits.
- `install.ps1` started plainly from a terminal is refused by Windows's default execution policy. README now says to use right-click, "Run with PowerShell", or `-ExecutionPolicy Bypass`; the zip still has no launcher that does it.
- The Start menu shortcut starts the app in the user's profile folder, which it then takes as the repository.
- The app waits 15 seconds for a backend it started and does not try again.
- `uninstall.ps1 -PurgeData` leaves `%APPDATA%\CodeWiki`.
- No program writes a log file.
- `uninstall.ps1` deletes `codewiki-app.exe` right after `Stop-Process`, without waiting for the process to end. In one of three runs that failed with "Access to the path is denied" and left the installation half removed; a second run finished it.
- To check: in the Hyper-V guest the graph page showed no edge lines and reported a first frame of 4.4 to 5.5 s, with and without a graphics card. The screenshots are 16-bit, so the lines need a look on a real screen; the frame time does not.

## Disk use

Debug builds once filled 44 GB. The dev profile now builds dependencies without debug information and without the incremental cache; a full build with tests takes about 7 GB. `scripts/clean.ps1` removes debug artifacts and spike build folders.

## Checks left to the owner

These cannot be done from an automated session on this machine.

- Run the Anthropic API, OpenAI-compatible and Ollama providers and embedders against the real services. Needs an API key or an Ollama install.
- Try Windows snap layouts by hand. The Ctrl shortcuts (Ctrl+K, Ctrl+T, Ctrl+W, Ctrl+Tab, Ctrl+S) were pressed on the VM's keyboard on 2026-11-13 and all work.
- Look at the glass over the real Mica backdrop, in dark and in light system mode, and at a light theme on a dark system.
- Press Tab through the toolbar and a page, and press Enter and Space on a button, a tab and a sidebar row.
- Open a note whose illustration has tabs, and switch between them.
- Close the palette with Escape, to see it fade out.
- Codex round trip, when a Codex subscription is available.
