<p align="center"><img src="crates/codewiki-app/resources/codewiki.png" width="128" alt="CodeWiki logo"></p>

# CodeWiki

A native Windows app and a Rust backend that write and maintain a wiki for a codebase, for both people and coding agents.

**[User guide](https://fly-3.gitbook.io/codewiki/)**: install, your first wiki, the app, coding agents, the command line and troubleshooting.

The wiki is an Obsidian vault of Markdown notes at `<repo>/obsidian/`. It lives in the repository, so it is versioned with the code and readable without CodeWiki.

> Status: version 0.5.0. See `ROADMAP.md` for what is verified and what is not.

## What it does

- **Writes the wiki.** Parses the repository, builds a dependency graph, and writes notes for packages, modules, key types, startup flows, external integrations, patterns and operations, with Mermaid diagrams. Your coding agent then explains each note and adds pictures to it. With auto explaining on (Settings → Repositories), Claude Code with the plugin explains waiting notes in the background when a session starts, 20 by default, or as many as you choose, up to all of them; "To explain (N)" in the toolbar lists the notes that still wait.
- **Keeps it current.** After new commits it rewrites only the notes whose source changed.
- **Protects your edits.** A note you edited is never overwritten. Proposed changes wait in a review queue until you approve or reject them. When the code a note describes is deleted, a note with written work goes only where git can bring it back, and notes you wrote yourself only after you approve. The Review page shows the queue beside the open proposal; you can select several (click, Ctrl, Shift or the checkboxes) and approve or reject them in one step.
- **Tells you what happens.** A small notice appears in the corner of the window, and a Windows notification takes its place while the window is hidden or minimized. It says when the app has connected to its backend again and when new proposals wait for review.
- **Reads in tabs.** Right-click a tab to close it, the others, those to its right or left, or all, to reveal its note in the sidebar, or to copy its path. Right-click a note in the sidebar to open it in a new tab, or Ctrl+click or middle-click it. Settings → Appearance → Tabs turns tabs off, or opens every note in a tab of its own.
- **Reconnects by itself.** If the backend stops answering, the app tries again on its own, and so does the Settings page when your embedding model server is not running yet. No restart needed.
- **Answers agents.** An MCP server gives Claude Code and other agents the wiki, symbol detail, search, risk and impact analysis.
- **Searches by meaning.** Full-text search, joined by vectors from a model server on your machine (LM Studio, Ollama, Llama for Windows, Lemonade or mistral.rs) that you run and choose the model of. Without one, CodeWiki matches word forms. No key needed, no text leaves the machine. The search box suggests notes, symbols and files as you type.
- **Analyses the code.** Hotspots, ownership, change coupling, blast radius, dead code, documentation drift, file health, test impact, coverage gaps, security patterns, refactoring suggestions, decision records and API contracts.

Symbols are extracted for Rust, Python, TypeScript, JavaScript and Java. Other files are indexed by path.

CodeWiki calls no language model, and writing the wiki needs no API key. It writes each note from the comments in your code and from which function calls which. What a note cannot get from the code alone, such as what a thing is for and how it works, your coding agent writes through the plugin, and CodeWiki checks what the agent sends before it is saved. [How a note is written](#how-a-note-is-written) says who writes what.

## Parts

| Part | What it is |
|---|---|
| `codewiki` | Command line tool and backend server (HTTP API, no web pages) |
| `codewiki-app` | Desktop app for Windows 11, native Rust on GPUI |
| `codewiki-mcp` | Small stdio program that connects coding agents to the backend |

The app and the MCP program start a local backend when none is running, or connect to one in Docker or on another machine.

## Install

Requires Windows 11. Download `CodeWiki-<version>-windows-x64-setup.exe` from the [latest release](https://github.com/Fly-onlyone/CodeWiki/releases/latest) and double-click it. It needs no administrator rights. The setup program is not signed, so Windows SmartScreen shows "Windows protected your PC": click "More info", then "Run anyway". The wizard shows Ready to Install, Preparing and Completing; "Start CodeWiki" is checked on the last page.

It copies the programs and the plugins to `%LOCALAPPDATA%\Programs\CodeWiki`, adds that folder to your user `PATH`, creates a Start Menu shortcut and adds an entry "CodeWiki" to Settings → Apps. If something fails, it says why. The app started from the last page has the `PATH` from before the install: open a new terminal before you start a coding agent. To remove CodeWiki, uninstall it in Settings → Apps. It asks whether to delete your data (`%LOCALAPPDATA%\CodeWiki`: the search index, the logs and the list of repositories) and settings (`%APPDATA%\CodeWiki`) as well. It keeps them unless you answer Yes, and the wiki notes in your repositories stay either way. A silent uninstall (`/VERYSILENT`) keeps them too, unless you add `/PURGEDATA`. A running CodeWiki is closed for you. If a file in the install folder is in use by another program, the uninstall stops and keeps the entry: close that program and uninstall again. A terminal whose current folder is the install folder only leaves the empty folder behind: delete it afterwards.

The other way is the zip. Download `CodeWiki-<version>-windows-x64.zip` from the same page; the `.sha256` file beside it holds its checksum. Unpack it and double-click `install.cmd`. It does the same, except for the Settings → Apps entry. `uninstall.cmd` in the install folder reverses it (use it only for an installation made from the zip); `uninstall.cmd -PurgeData` also deletes the index, the logs and the settings. Running the setup program over a zip installation works too.

To build both from source, with Rust 1.95 or newer and the MSVC toolchain:

```
pwsh scripts\package.ps1
```

It writes `dist\CodeWiki-<version>-windows-x64.zip`, about 30 MB. When Inno Setup 6 is installed, it also writes `dist\CodeWiki-<version>-windows-x64-setup.exe`; `-NoSetup` skips that.

The `.cmd` files run `install.ps1` and `uninstall.ps1`, which Windows refuses to run directly unless scripts have been allowed. From a terminal, `powershell -ExecutionPolicy Bypass -File install.ps1` does the same.

### Updates

An installed app updates itself, whether it came from the setup program or from the zip. Once a day, while it runs, it asks GitHub for the latest release; when there is a newer one, the toolbar, Home and the tray menu show it. One click downloads the zip, checks it against its `.sha256` file, replaces the programs and restarts the app on the same repository. The version in the Settings → Apps entry is updated too, when there is one. Settings → Updates shows the version, checks on demand ("Check now") and turns the daily check off. Ctrl+K → "Check for updates" does the same.

The backend is stopped first and a new one starts with the new app. A coding agent's MCP bridge that is running keeps working and reaches the new backend on its next call; restarting the agent's session gives it the new bridge too. If anything goes wrong, the app says what and leaves the installation as it was. Pre-releases are never offered, and a copy that was only unpacked, or built from source, does not update itself.

### The tray

Closing the app's window does not end the app. CodeWiki stays in the notification area (the tray) with its backend, so a coding agent's first call finds a backend that is already running.

- **Click the icon** to open the window again, as you left it.
- **Right-click the icon** for the menu: what CodeWiki is doing, Open CodeWiki, Update wiki now, Recent repositories, Settings, Check for updates and Quit.
- **The tooltip** names the open repository, says whether a wiki is being updated (by you, an agent or a git webhook) and whether an agent's MCP bridge called in the last five minutes.
- **Quit** ends the app and its backend. It is in the tray menu and in the palette (Ctrl+K → "Quit CodeWiki").

Windows 11 puts a new icon behind the `^` arrow of the taskbar; drag it onto the taskbar to keep it in view. Starting CodeWiki while it runs opens the window of the running app, on the folder you named if you named one.

Settings → Startup → "Start with Windows" starts CodeWiki in the tray when you sign in, with no window. It is off until you turn it on, and `uninstall.cmd` removes it.

## Use

In a repository:

```
codewiki init          # index the code and write the wiki
codewiki update        # after new commits
codewiki status
codewiki check         # findings for CI; --format sarif or junit
```

`codewiki --help` lists every command. Start `codewiki-app` with a repository folder as its first argument, or from inside one. It opens on Home: "Open folder…" and the projects opened before, the latest first; a project whose folder is not found stays listed, with a Remove button. Ctrl+K opens the command palette. The search box lists matching notes, symbols and files as you type: the arrow keys and Enter open one, and Enter with none chosen shows every hit. A note that an agent explained shows its pictures inside the sections they explain: boxes joined by arrows, and marked key points. "Agent view" in the toolbar shows the note as a coding agent reads it. A click on a diagram in a note opens it over the window, where the wheel zooms it and a drag moves it. Controls follow Material UI and move on springs; reports are pages of tiles, tables and cards. Behind the glass, beside the page, a sky of notes and links drifts in the theme's colors; light runs along the links, more often while a wiki is being written, and the nodes near the pointer brighten. It stands still when the window is not in front or animations are off, and Settings → Appearance → "Background graph" turns it off. Settings fills the window and has a category for each kind of setting: the theme and animations (by default the app follows the Windows animation setting), semantic search, the backend, repositories, updates, the keyboard shortcuts and diagnostics. It opens from the "Settings" row at the foot of the sidebar and of Home, with Ctrl+, , or from the palette.

Closing the app also stops the local backend it used; one named by `CODEWIKI_URL` is left running. The app, the backend and the MCP bridge write their logs to `%LOCALAPPDATA%\CodeWiki\logs` (`app.log`, `backend.log`, `mcp.log`). "Open logs folder", in the palette or in Settings, shows them in Explorer. Attach them to a bug report; they name the folders you open. For more in `app.log`, choose a level in Settings → Diagnostics; `CODEWIKI_LOG=debug` does the same for any of the programs and wins over that choice. Set `CODEWIKI_LOG_DIR` to write the logs elsewhere.

### How a note is written

Two writers write a note, and each has its own sections.

- **CodeWiki's own rules** always run. They read your code and never guess. They cost nothing and give the same note on every run. They copy what your comments say, and they list which function calls which. They write the facts: where the thing is, its items, what it depends on, who uses it.
- **Your coding agent** writes the explanation: what the thing is for, how it works, what can go wrong. In Claude Code with the [plugin](#claude-code-plugin), run `/codewiki:write-notes`. The agent reads the note's source files, writes its sections and sends them. It never sends a whole note, so it cannot change a fact. CodeWiki itself calls no language model.

So a wiki that no agent has explained is complete, and it is as good as the comments in your code. To get better notes without an agent, write comments. A `//!` comment at the top of a Rust module (a docstring in Python, a `/** */` comment in TypeScript or Java) says what the module is for. A `///` comment above a function says what it does. The wiki copies both whole.

A note about a module, a package of your own or a type has these sections:

| Section | What it holds | Who writes it |
|---|---|---|
| Summary, the line under the title | What the thing is for, in one sentence | Rules: the first sentence of its comment |
| What it does | The purpose, and what a user of the program sees of it | The agent. Until then, rules: the rest of the comment |
| Source | The file | Rules: the index |
| How it works | The path from a cause to its result, with conditions and error paths | The agent. Until then, rules: numbered steps of which function calls which |
| Items | Each function, type and constant, with its signature | Rules: the item's comment. For an item with no comment: what the code says of it, such as "private function, returns `String`; called by `enabled`" |
| Depends on, Used by | The notes on either side | Rules: the index |
| Gotchas | What a maintainer could get wrong | The agent, or you |

Other kinds of note divide the same way. In an integration note the agent writes What it does, Wire-up, Auth mode and Gotchas; in a flow or operations note, How it works and Gotchas, under the diagram or the commands that rules write; in a pattern note, Why and Don't.

Three things to know when you read one:

- **"Calls are listed in the order of the source."** The steps that rules write under How it works name calls in the order they stand in the file. The rules do not see an `if` or a loop, so "`set` calls `write`, then `remove`" can mean "one or the other". The agent's explanation says which; so does the code.
- **Not every call is found.** A call through a value whose type the index cannot tell, such as a trait object or a callback, is left out and never guessed.
- **A long list stops at 20 items** and says how many it leaves out. A private item with no comment that plays no part in the steps is left out first.

A note about a package from outside the repository (an integration) says what the package is, which names each module takes from it, and which version and features each build file asks for. What the package is stands in no file of your repository. CodeWiki reads it from files a build left on the machine (cargo's download cache, `node_modules`), never from the network, and keeps it in `obsidian/.codewiki/packages.json`, so that the note reads the same on a machine that has not built the code. For a Python or Java package it knows the version only.

**Pictures.** The agent may give a section a picture: a row of numbered boxes joined by arrows that shows what happens from a cause to its result, and marked key points or warnings. The app draws it inside the section it explains. A picture is a short text kept beside the note, in `<Note>.openui`; the note's Markdown stays as it is, and Obsidian shows the note without the picture. A note has at most four pictures, and a note no agent has explained has none: rules cannot tell what a thing is for, so they draw nothing.

**When the code changes.** `codewiki update` writes the facts of an explained note anew and keeps the agent's sections and pictures as they are. The explanation was written from the files as they were, so the note is marked: its explanation may be behind. The app says so in the note's details, and such notes come first when an agent asks what waits. `/codewiki:write-notes changed` writes the notes about the files that changed. After a code change of its own, Claude writes the notes about the files it changed, at most three, without being asked.

**Auto explaining.** It is on by default. When a session starts fresh (startup, resume or `/clear`), the briefing tells the agent to explain the waiting notes by itself: Claude Code starts the `codewiki:note-writer` agent in the background on Sonnet when you send your first message, and Codex uses its `write-notes` skill after your first request. It writes 20 notes per session by default, stale ones first, so it uses your allowance; "Notes per session" under the switch sets 5, 10, 20, 50, 100 or Unlimited. One note-writer writes at most 20, so Claude Code starts one for each 20 notes, all at once, and Codex lists 20 at a time until it has written them all. A session in a linked git worktree is not asked, and neither is a folder you keep from coding agents. The hook leaves the order out when the host reports plan mode; the order itself tells the agent to skip it in plan mode. Several sessions at once get different notes: a listed note is lent to one agent for 30 minutes. Turn it off in Settings → Repositories → "Auto explaining". The toolbar shows "To explain (N)" while notes wait; it opens the Waiting notes page, which lists them. The count follows updates the app's backend runs; one run on the command line or an edit in Obsidian shows after the next change or after you reopen the repository.

**What CodeWiki checks.** A submission is refused, with every fault named, when a section is not one of the agent's, holds a `#` or `##` heading or more than 300 words, links to a note that does not exist, or breaks the house style; when a picture uses a component that does not exist or is too large; or when the note or its code changed since the agent read it.

**What a coding agent gets.** The MCP tool `read_note` gives an agent a note without its pictures. While How it works is only the list of calls that rules write, it is left out too: the agent reads the calls in the code it is about to edit. Once an agent explained that section, it is always included. `read_note` with `full: true` gives the whole text. In the app, "Agent view" in a note's toolbar shows the same.

**Your edits.** A note you change by hand is yours from then on. Neither CodeWiki nor an agent writes over it. When its code changes, the new text waits as a proposal that you approve or reject; what an agent writes for it waits the same way, with its pictures. After a CodeWiki update that changes how notes are written, the facts of every note you did not edit are written again; explanations stay.

**When code is deleted or moved.** A note describes something in the code. When that is gone, the note describes nothing, and CodeWiki takes it out of the wiki without losing anything you or an agent wrote:

- A note nobody wrote in is deleted with its code.
- A note with an agent's explanation or your edits follows its code when that only moved: the note moves with it, keeps what was written, and waits for an agent to look again. Where it is not clear what moved where, nothing is carried over, and the sync says so.
- When the code is gone, such a note is deleted only after the deletion is committed, and only if git holds the note as it is now, so `git` can always bring it back. Until then it stays, marked "What it describes is gone from the repository", and counts as no note of the wiki.
- A note you wrote yourself is about the files it names in backticks, anywhere in its text. When none of them exists any more, CodeWiki never deletes it by itself: all such notes wait in one proposal on the Review page. Untick the notes you want to keep, then approve; the others are deleted. A note you keep, or all of them if you reject, stays as history and is not proposed again. A note that still names a file that exists, or names no file at all, is left alone. When a file a note names was only moved, the path in the note is updated.
- Links to a deleted note become plain words in the notes that stay, and a picture or other attachment only deleted notes showed goes with them.
- An agent's explanation keeps its note for as long as its code exists, even when other code becomes more important than it.

Without git, nothing that holds written work is ever deleted.

**Git worktrees.** A linked git worktree, such as the ones a coding agent makes for a task, gets its answers from CodeWiki like any folder. It is not listed as a project on Home: the row of its repository says how many worktrees it has. When a worktree's folder is removed, CodeWiki forgets it and deletes its index.

### Search

Search combines full-text matching with vectors. `CODEWIKI_EMBED` selects where vectors come from: `local` (default: it matches word forms and needs no model), `lmstudio`, `ollama`, `llamacpp`, `lemonade`, `mistralrs`, `openai` or `none`. CodeWiki runs no model of its own, does not install a server and does not download a model: you run the server, and CodeWiki asks it for vectors. These embed text for search only: nothing writes notes with them. `CODEWIKI_EMBED=builtin`, and a saved choice of the built-in model from an earlier version, are no longer valid; see the end of this section.

Every server except Ollama is asked through the OpenAI-style route `POST /v1/embeddings`. A server needs a model: `CODEWIKI_EMBED_MODEL` names it, and Settings → Semantic search lists the models the running server offers, with the embedding ones first. Without a model, search stays full-text alone and the page says so.

| Embedder | Setting for its address (default) | Setting for its key | How to serve an embedding model | Devices on Windows | Port |
|---|---|---|---|---|---|
| `lmstudio` | `CODEWIKI_LMSTUDIO_URL` (`http://127.0.0.1:1234`) | `CODEWIKI_LMSTUDIO_KEY`, optional | Download an embedding model in LM Studio and start the server on the Developer tab | Processor or graphics card, by the runtime chosen in LM Studio's settings | 1234 |
| `ollama` | `OLLAMA_HOST` (`http://127.0.0.1:11434`) | none | `ollama pull embeddinggemma`, or another embedding model | Processor, NVIDIA (CUDA), AMD (ROCm) and Vulkan | 11434 |
| `llamacpp` | `CODEWIKI_LLAMACPP_URL` (`http://127.0.0.1:9931`) | `CODEWIKI_LLAMACPP_KEY`, optional | Load an embedding model (GGUF) in Llama for Windows, or start `llama-server` with one. For long texts, start `llama-server` with `-ub 2048 -b 2048`, or add those flags to the custom serve arguments of Llama for Windows | Processor, CUDA, Vulkan, HIP or SYCL, by the build of `llama-server` | 9931 |
| `lemonade` | `CODEWIKI_LEMONADE_URL` (`http://127.0.0.1:13305`) | `CODEWIKI_LEMONADE_KEY`, optional | Pull an embedding model in Lemonade, such as `nomic-embed-text-v1-GGUF` | Processor, graphics card, and the AMD NPU through FastFlowLM | 13305 |
| `mistralrs` | `CODEWIKI_MISTRALRS_URL` (`http://127.0.0.1:1234`) | none: the server has no key | `mistralrs serve -m Qwen/Qwen3-Embedding-0.6B` | Processor with the prebuilt program; CUDA needs a build from source | 1234 |
| `openai` | `OPENAI_BASE_URL` (`https://api.openai.com/v1`) | `OPENAI_API_KEY`, required | A service with an OpenAI-compatible embeddings route | Not on your machine | none |

On a Windows without the Microsoft Visual C++ Redistributable (a clean install has none), `llama-server`, Lemonade's own `llama-server` and mistral.rs do not start: Windows reports a missing DLL (exit code `0xC0000135`), and Lemonade answers "llama-server failed to start". Install the x64 redistributable from Microsoft (https://aka.ms/vs/17/release/vc_redist.x64.exe) first. LM Studio's command-line install (`irm https://lmstudio.ai/install.ps1 | iex`) can fail with "DirectoryNotFoundException" when the path of your `TEMP` folder is long: run it from a PowerShell where `$env:TEMP` and `$env:TMP` name a short folder such as `C:\t`. On the processor mistral.rs loads `Qwen/Qwen3-Embedding-0.6B` as F32, about 2.3 GB, and refuses with "This model does not fit on the devices" when less is free: add `--isq 8` to load it 8-bit. Ollama takes about two seconds for a request whose length in tokens it has not seen since it loaded the model, so with Ollama a search by meaning can take about two seconds; the other servers answered in under 100 ms. All of this was found in a clean Windows 11 on 2026-11-21 and 2026-11-22; how the servers compare there is in `ROADMAP.md` ("Embedding servers compared").

LM Studio and mistral.rs both use port 1234, and Llama for Windows and `llama-server` both use 9931: run one of each pair at a time, or give one another address. An address pasted with `/v1` at the end, as the servers show it, works like the bare address. A server that wants a key and gets none, or a wrong one, answers with an error that names the setting to fill in. The first request to a server may wait up to five minutes while it loads the model; later requests wait two minutes.

A text is cut at 8,000 characters. A batch that a server refuses as too long is sent once more with each text cut to 2,000 characters. Refused again, its texts are sent one at a time, and a text refused alone is sent once more cut to 400 characters; a text refused even then ends the job with the server's error. mistral.rs is asked to cut a long text itself.

Some models expect a short text in front of a query or a document: `embeddinggemma`, `qwen3-embedding`, `nomic-embed-text`, `mxbai-embed`, the `bge` v1.5 models and the `e5` models. CodeWiki adds it for them, whichever server serves the model, and adds `+<name>` to the name its vectors are stored under, so vectors made with and without it are never compared.

After a change of embedder or model, search is full-text alone until the documents are embedded again. The backend does that in the background after every sync, on every platform, and the Settings page shows the progress with a Cancel button. A server that is down, or a model it does not know, ends the job with that error, shown as a warning; nothing retries by itself, and the next sync ("Sync now"), Test or `POST /v1/embed/reembed` tries once more. `GET /v1/embed/job` reads the job, `POST /v1/embed/reembed` starts one and `POST /v1/embed/cancel` stops it; a backend that listens beyond its own machine refuses all three.

**From the built-in model.** Earlier versions could download a model and run it themselves. That is gone, with its worker, its NVIDIA folder `cuda`, the command `codewiki model` and `CODEWIKI_EMBED_DEVICE`. When the backend starts it moves a saved choice of the built-in model to `local`, forgets the model and the device, and deletes the downloaded models in the data folder `models`. An update removes an old `cuda` folder. Choose a server in Settings → Semantic search to search by meaning again. `CODEWIKI_EMBED=builtin` in the environment cannot be changed for you: search is full-text alone and the page says the built-in embedder was removed.

### Saved settings

The variables above need not be set before every start: the same settings can be chosen in the app, under Settings → Semantic search, and take effect at once. Its Test button sends the chosen embedder one small request and shows its answer or its error. A variable that is set wins over the saved value, and the page then shows the setting locked, with the variable's name.

The backend keeps them in `settings.json` in its data folder, so the command line and an agent's MCP bridge use them too. An API key is stored encrypted for your Windows user; the page shows only its last four characters, and nothing sends it back. Over HTTP, `GET /v1/settings` shows each setting and where its value comes from, `PUT /v1/settings` changes the saved ones, and `POST /v1/settings/test/embed` runs the test. A backend that listens beyond its own machine, as the one in Docker does, takes its settings from its variables only; the app shows them and cannot change them.

Earlier versions could call a language model themselves, through `CODEWIKI_LLM`, `CODEWIKI_MODEL`, `ANTHROPIC_API_KEY` or CLIProxyAPI. Those settings have no effect any more. The backend removes them from `settings.json` when it starts, the saved keys with them, and its log names a variable that is still set.

Settings → Backend saves the address and token of a backend in Docker or on another machine, in place of `CODEWIKI_URL` and `CODEWIKI_TOKEN`. "Connect" checks that it answers and takes the token before anything is saved. The app and the MCP bridge on this machine then both use it. If it stops answering, the app says so and offers "Use local backend"; it never goes back to a local backend by itself.

### Error reports

CodeWiki sends nothing about you or your code unless you ask it to. Settings → Diagnostics → "Send error reports" is off until you turn it on. Then a crash or an error of the app, the backend or the agents' bridge goes to the developer through [Sentry](https://sentry.io): what failed, where in the code, the versions of CodeWiki and Windows, and the log lines just before it. Every file path is replaced by `<path>` before it leaves, and no report holds note text, a search query, a key or your computer's name. The log lines can name a repository, a note, or the address of a backend on another machine. Sentry keeps neither the network address a report came from nor the city and country of that address. Turning it off stops the reports at once; turning it on takes effect when each program next starts.

A developer can point a build at their own Sentry project with `SENTRY_DSN`; the switch then shows as set by that variable.

### Claude Code plugin

The plugin in `plugins/claude-code/` connects Claude Code to the wiki and the index, so that Claude reads what CodeWiki knows about the code before it explores the code by hand, and writes the explanations of the notes. It adds three things.

**An MCP server.** The plugin runs `codewiki-mcp`, which starts a local backend on first use, or uses the one at `CODEWIKI_URL`. It gives Claude 27 tools:

| Purpose | Tools |
|---|---|
| Orientation | `get_overview`, `get_architecture`, `get_workspace`, `list_repos` |
| Reading the code | `get_context`, `get_symbol`, `search_codebase`, `get_answer`, `read_note`, `get_dependency_path`, `get_why`, `list_decisions` |
| Before and after a change | `get_risk`, `get_change_risk`, `get_blast_radius`, `get_impacted_tests` |
| Code health | `get_hotspots`, `get_dead_code`, `get_health`, `get_coverage`, `get_security_findings`, `get_refactoring_plan`, `get_contracts` |
| Upkeep | `sync_wiki` |
| Writing notes | `list_waiting_notes`, `get_note_brief`, `submit_note` |

`sync_wiki` follows the same rule as `codewiki update`: a note a person edited is never overwritten, and the change waits in the review queue.

**The skill `write-notes`**, which is also the command `/codewiki:write-notes`. With it Claude explains notes (see [How a note is written](#how-a-note-is-written)):

```
/codewiki:write-notes                      the first 5 notes that wait
/codewiki:write-notes waiting 12           the first 12
/codewiki:write-notes 01-Core/cw-parse/    the waiting notes of one folder
/codewiki:write-notes "Scan Module"        one note
/codewiki:write-notes changed              the notes about the files you changed
```

For each note Claude reads its source files, writes the sections that are its own and sends them with `submit_note`. Writing notes uses your Claude usage allowance: a note is a few file reads and one or two tool calls, and a first pass over a large repository is several hundred notes. Write a folder at a time.

**A session-start briefing.** When a session starts or resumes, and after `/clear` or compaction, a hook runs `codewiki hook session-start`. Claude Code adds what it prints to Claude's context; it does not show in the chat. The briefing is four or five lines:

1. how many notes the wiki has, and its home note, `obsidian/_HOME.md`;
2. whether the wiki is up to date with `HEAD`. It is stale only when files outside `obsidian/` changed since the commit the wiki was written from, so committing the wiki itself does not make it stale. Without git, the line says the wiki cannot be compared;
3. how many proposals wait for review;
4. how many notes wait for an explanation, when any does. The line states a count. It tells Claude to write notes when you ask, or for files it changed, and, when auto explaining is on and the session is fresh, to start the note-writer;
5. that `get_overview` and `get_context` answer from the wiki, and that Claude should call them before it explores the code by hand.

In a repository with no wiki yet, the briefing is two lines: there is no wiki, and `sync_wiki` writes one.

The hook reads the wiki's manifest and proposals, and asks git for `HEAD`. It does not start the backend, does not update the wiki and writes nothing. It looks in the project folder and the folders above it, so a session started in a subfolder still finds the wiki. On any error it prints nothing and exits with status 0, so it cannot fail a session.

To see what Claude sees, run `codewiki hook session-start` in the repository. The [user guide](https://fly-3.gitbook.io/codewiki/agents/claude-code-plugin#the-session-start-briefing) explains each line.

`plugins/claude-code/README.md` describes each tool and shows a sample briefing.

To install the plugin:

1. Install CodeWiki as described in [Install](#install). Then open a new terminal, so that `codewiki` and `codewiki-mcp` are on `PATH`.
2. Add the `plugins` folder of the installation as a plugin marketplace:

   ```
   claude plugin marketplace add "%LOCALAPPDATA%\Programs\CodeWiki\plugins"
   ```

   In PowerShell the folder is `"$env:LOCALAPPDATA\Programs\CodeWiki\plugins"`. It holds the agent plugins and nothing else, and Claude Code reads it where it is: nothing is downloaded, and git is not needed.

   Without the Windows package (programs built from source, or a backend on another machine), add this repository instead, or the folder of a clone:

   ```
   claude plugin marketplace add Fly-onlyone/CodeWiki --sparse .claude-plugin plugins/claude-code
   ```

   With `--sparse`, Claude Code checks out only the marketplace file and the plugin's folder, not the whole repository. It needs git. A marketplace added from GitHub without `--sparse` holds the whole repository: run `claude plugin marketplace remove codewiki`, then add it again with this command.
3. Install the plugin:

   ```
   claude plugin install codewiki@codewiki
   ```

   It is installed for your user. With `--scope project` it is recorded in the repository's `.claude/settings.json` instead, which you can commit to share it.
4. In a repository that has no wiki yet, run `codewiki init`.
5. Start Claude Code in the repository, or run `/reload-plugins` in a session that is already open. `/mcp` lists the server as `plugin:codewiki:codewiki`.

To use a backend in Docker or on another machine, set `CODEWIKI_URL` and `CODEWIKI_TOKEN` in the environment Claude Code runs in (see [Docker](#docker)).

To update or remove the plugin:

```
claude plugin update codewiki@codewiki        # then restart Claude Code
claude plugin uninstall codewiki@codewiki
claude plugin marketplace remove codewiki
```

### Codex plugin

The plugin in `plugins/codex/` gives Codex the same MCP server and session-start briefing as the Claude Code plugin, and three skills.

- `write-notes` writes the explanations of waiting notes, as the Claude Code skill does. With auto explaining on, the briefing tells Codex to use it after your first request;
- `using-codewiki` says which tool to call at each step of a change, and when to write or refresh the wiki with `sync_wiki`;
- `wiki-note-style` holds the house style of CodeWiki's notes, for when Codex edits a note in `obsidian/` by hand.

To install it, after installing CodeWiki and opening a new terminal:

```
codex plugin marketplace add "%LOCALAPPDATA%\Programs\CodeWiki\plugins"
codex plugin add codewiki@codewiki
```

Without the Windows package, add the repository instead:

```
codex plugin marketplace add Fly-onlyone/CodeWiki --sparse .agents/plugins --sparse plugins/codex
```

With `--sparse`, Codex downloads only the marketplace file and the plugin's folder, not the whole repository. A marketplace added from GitHub without `--sparse` holds the whole repository with its history: run `codex plugin marketplace remove codewiki`, then add it again with this command.

Codex skips a hook that comes with a plugin until you trust it. Run `/hooks` in Codex once and trust the `SessionStart` hook that runs `codewiki hook session-start --agent codex`. After an update that changed the hook, trust it again.

`plugins/codex/README.md` covers updating, removing, and the same setup without the plugin. The plugin's files are checked against Codex's source and by a test; it has not yet been run in a Codex session.

### Keep coding agents out of a folder

You can keep coding agents from using CodeWiki in a folder, for a project whose code or wiki must not reach an agent. The list is the file `agents-off.json` in the data folder (`%LOCALAPPDATA%\CodeWiki`, or `CODEWIKI_DATA_DIR`). An entry covers its folder and everything below it, and it also keeps agents out of every folder that holds it: with `D:\work\clients\acme` listed, an agent working in `D:\work`, or naming `D:\` as a tool's `repo`, gets no CodeWiki, because indexing that folder would index acme's code. `D:\work\client` neither holds `D:\work\clients\acme` nor lies in it: folders are compared by whole names. Three ways manage the list:

- **The app.** Settings → Repositories → "Kept from coding agents" adds and removes folders. On Home, a project's row has "Keep agents out" and "Let agents in", and the palette has "Keep agents out of this project" and "Let agents into this project". A project kept by a folder above it says so ("Kept from agents by clients"), and so does a project that holds a kept folder ("Kept from agents (contains acme)"); such a row links to Settings instead of offering a button. A project listed itself while a folder above it is listed too shows the folder above, since removing its own entry would not let agents in.
- **The command line.** `codewiki agents off [<folder>]`, `codewiki agents on [<folder>]`, `codewiki agents list` and `codewiki agents status [<folder>]`. Without a folder, the current one. `status` says `kept from coding agents by <entry>` for a covered folder and `kept from coding agents: contains <entry>` for one that holds an entry. `on` that removes a folder's own entry while a folder above it is still listed says `still kept from coding agents by <entry>` and exits 1.
- **The file.** Edit `agents-off.json` by hand: `{"version": 1, "folders": ["D:\\work\\secret"]}`. Each entry must be an absolute folder; a list with an empty or relative entry cannot be read.

What an agent sees in a kept folder, or in one that holds a kept folder:

- The session-start briefing prints nothing.
- Every MCP tool refuses, with an answer that tells the agent that CodeWiki is turned off for this project. The skills tell the agent to stop using CodeWiki's tools and the `codewiki` command for the rest of the session.

What an agent sees elsewhere:

- A tool whose `repo` names a kept repository, or one that holds a kept folder, by path, id or name, refuses for that repository alone: "CodeWiki is turned off for that repository", with no path in it. The skills tell the agent not to use CodeWiki for that repository again. The bridge checks the repository a call reaches once the backend has named it, so neither a lookup that failed nor the backend's fallback to a repository's name lets a call through.
- `list_repos`, `get_workspace` and `get_contracts` with `workspace: true` leave out kept repositories and those that hold a kept folder. One limit: a route of a visible repository that only a kept repository calls leaves the matched list and does not show as unconsumed.

The app and the command line still work in a kept folder: the list keeps agents out, not you. A project that holds a kept folder is kept from agents as a whole, its root included; to let agents into the rest of it, move the kept code out of the project or take the entry off the list.

A list that cannot be read keeps every agent out, until you fix or delete the file; Settings offers to start a new list.

With a backend on another machine (see [Docker](#docker)), the bridge reads that backend's repository paths as paths on this machine. A repository whose path names a kept folder here, or holds one, is refused and hidden; on Windows a Linux path such as `/srv/code/app` reads as `\srv\code\app` on the current drive. Other repositories of that backend are not hidden: the list cannot name the other machine's folders. The folders the agent works in, and a `repo` given as a path, are checked as they are with a local backend. A `repo` given as a path is always read as a path on this machine, so an agent cannot register a new repository of a remote backend by its path there; a repository already registered is still reached by its id or name.

Another way, without CodeWiki's list, is Claude Code's own per-project switch: set `"enabledPlugins": {"codewiki@codewiki": false}` in `.claude/settings.local.json` of the project (key and file from Claude Code's [settings reference](https://code.claude.com/docs/en/settings-reference)). Claude Code then does not load the plugin there at all. It covers Claude Code only. The list covers every agent that calls CodeWiki through the MCP bridge; its silent briefing needs an agent host that runs the plugin's session-start hook.

### Docker

```
docker build -t codewiki .
docker run --rm -p 127.0.0.1:7411:7411 -e CODEWIKI_TOKEN=<secret> -v codewiki-data:/data -v C:\path\to\repos:/repos codewiki
```

Each released version is also pushed as `ghcr.io/fly-onlyone/codewiki:<version>`. Point the app or the MCP program at it with `CODEWIKI_URL=http://127.0.0.1:7411` and the same `CODEWIKI_TOKEN`. Set `CODEWIKI_WEBHOOK_SECRET` to let a git host trigger updates through `POST /v1/hooks/git`.

## Develop

```
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check advisories bans sources
```

`pwsh scripts\clean.ps1` frees the disk space builds leave behind. `AGENTS.md` holds the project rules.

A debug build keeps its own data, in `%LOCALAPPDATA%\CodeWiki-Dev` and `%APPDATA%\CodeWiki-Dev`, and its window says "CodeWiki Dev". It never uses or stops the backend of an installed CodeWiki, so the two can run side by side. It starts with no projects and the default embedder.

CI runs these four on every push and pull request. It also runs `codewiki check` on this repository and reports the findings to code scanning, and gitleaks scans new commits for secrets. Dependabot opens update pull requests weekly.

To release, set `version` in the root `Cargo.toml`, merge it, and push a matching tag:

```
git tag v0.3.1
git push origin v0.3.1
```

The tag publishes the Windows zip as a GitHub Release and pushes the Docker image. A tag that does not match the `Cargo.toml` version fails the release.

### Test a package in a clean Windows

`scripts\vm` installs the package into a Hyper-V guest, uses it by keyboard and mouse, uninstalls it and keeps screenshots and a recording. It needs Hyper-V with your account in the group "Hyper-V Administrators", a Windows 11 ISO, `oscdimg.exe` from the Windows ADK, `ffmpeg` for the recording, and about 20 GB of disk (40 GB with the graphics-card guest).

```
pwsh scripts\package.ps1
pwsh scripts\vm\provision.ps1 -Iso <windows.iso> -MemoryMb 2048    # once per image
pwsh scripts\vm\test-release.ps1
pwsh scripts\vm\provision-gpu.ps1          # once, optional: a guest that shares your graphics card
pwsh scripts\vm\test-release.ps1 -Gpu
```

Results go to `report\vm-test-<date>\`, which git ignores: a report stays on the machine that ran it. Read the screenshots: the script checks exit codes, files and processes, not what a page looks like.

## License

Apache-2.0. See `LICENSE` and `NOTICE`.
