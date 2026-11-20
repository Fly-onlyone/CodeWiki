<p align="center"><img src="crates/codewiki-app/resources/codewiki.png" width="128" alt="CodeWiki logo"></p>

# CodeWiki

A native Windows app and a Rust backend that write and maintain a wiki for a codebase, for both people and coding agents.

**[User guide](https://fly-3.gitbook.io/codewiki/)**: install, your first wiki, the app, coding agents, the command line and troubleshooting.

The wiki is an Obsidian vault of Markdown notes at `<repo>/obsidian/`. It lives in the repository, so it is versioned with the code and readable without CodeWiki.

> Status: version 0.2.0. See `ROADMAP.md` for what is verified and what is not.

## What it does

- **Writes the wiki.** Parses the repository, builds a dependency graph, and writes notes for packages, modules, key types, startup flows, external integrations, patterns and operations, with Mermaid diagrams and illustrations.
- **Keeps it current.** After new commits it rewrites only the notes whose source changed.
- **Protects your edits.** A note you edited is never overwritten. Proposed changes wait in a review queue until you approve or reject them.
- **Answers agents.** An MCP server gives Claude Code and other agents the wiki, symbol detail, search, risk and impact analysis.
- **Searches by meaning.** Full-text search, joined on Windows by an embedding model that CodeWiki downloads and runs on your machine, on the processor or an NVIDIA card. No key, no other install, no text leaves the machine.
- **Analyses the code.** Hotspots, ownership, change coupling, blast radius, dead code, documentation drift, file health, test impact, coverage gaps, security patterns, refactoring suggestions, decision records and API contracts.

Symbols are extracted for Rust, Python, TypeScript, JavaScript and Java. Other files are indexed by path.

No LLM is required. Without one, notes are built from the code's structure. With one, the prose, the answers and the illustrations are written by the model and checked against the house style before they are saved.

## Parts

| Part | What it is |
|---|---|
| `codewiki` | Command line tool and backend server (HTTP API, no web pages) |
| `codewiki-app` | Desktop app for Windows 11, native Rust on GPUI |
| `codewiki-mcp` | Small stdio program that connects coding agents to the backend |

The app and the MCP program start a local backend when none is running, or connect to one in Docker or on another machine.

## Install

Requires Windows 11. Download `CodeWiki-<version>-windows-x64.zip` from the [latest release](https://github.com/Fly-onlyone/CodeWiki/releases/latest); the `.sha256` file beside it holds its checksum. Unpack it and double-click `install.cmd`. It copies the programs to `%LOCALAPPDATA%\Programs\CodeWiki`, adds that folder to your `PATH` and creates a Start Menu shortcut. `uninstall.cmd` in that folder reverses it; `uninstall.cmd -PurgeData` also deletes the index, the logs and the settings.

To build the same zip from source, with Rust 1.95 or newer and the MSVC toolchain:

```
pwsh scripts\package.ps1
```

It writes `dist\CodeWiki-<version>-windows-x64.zip`. The zip is about 490 MB, of which some 460 MB is the folder `cuda`: the worker that runs an embedding model on an NVIDIA card, and three NVIDIA libraries it needs, passed on under NVIDIA's licence (`cuda\NVIDIA-CUDA-EULA.txt`). Building that worker also needs Visual Studio's C++ tools and NVIDIA's CUDA toolkit, which the script fetches by itself (0.9 GB, no installer). `pwsh scripts\package.ps1 -NoCuda` builds a package of about 30 MB without the folder; it runs the models on the processor.

The `.cmd` files run `install.ps1` and `uninstall.ps1`, which Windows refuses to run directly unless scripts have been allowed. From a terminal, `powershell -ExecutionPolicy Bypass -File install.ps1` does the same.

### Updates

An installed app updates itself. Once a day, while it runs, it asks GitHub for the latest release; when there is a newer one, the toolbar, Home and the tray menu show it. One click downloads the zip, checks it against its `.sha256` file, replaces the programs and restarts the app on the same repository. Settings → Updates shows the version, checks on demand ("Check now") and turns the daily check off. Ctrl+K → "Check for updates" does the same.

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

`codewiki --help` lists every command. Start `codewiki-app` with a repository folder as its first argument, or from inside one. It opens on Home: "Open folder…" and the projects opened before, the latest first; a project whose folder is not found stays listed, with a Remove button. Ctrl+K opens the command palette. Controls follow Material UI and move on springs; reports are pages of tiles, tables and cards. Settings fills the window and has a category for each kind of setting: the theme and animations (by default the app follows the Windows animation setting), the AI model, semantic search, the backend, repositories, updates, the keyboard shortcuts and diagnostics. It opens from the "Settings" row at the foot of the sidebar and of Home, with Ctrl+, , or from the palette.

Closing the app also stops the local backend it used; one named by `CODEWIKI_URL` is left running. The app, the backend and the MCP bridge write their logs to `%LOCALAPPDATA%\CodeWiki\logs` (`app.log`, `backend.log`, `mcp.log`). "Open logs folder", in the palette or in Settings, shows them in Explorer. Attach them to a bug report; they name the folders you open. For more in `app.log`, choose a level in Settings → Diagnostics; `CODEWIKI_LOG=debug` does the same for any of the programs and wins over that choice. Set `CODEWIKI_LOG_DIR` to write the logs elsewhere.

### With an LLM

| Variable | Values |
|---|---|
| `CODEWIKI_LLM` | `anthropic`, `openai`, `ollama`, `claude-cli`, `codex-cli`, `cliproxy`, `none` |
| `CODEWIKI_MODEL` | Model name for the chosen provider |
| `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `OPENAI_BASE_URL` | Credentials and endpoint for the API providers |
| `OLLAMA_HOST` | Address of Ollama; `http://127.0.0.1:11434` when not set |
| `CODEWIKI_CLIPROXY_URL` | Address of CLIProxyAPI; `http://127.0.0.1:8317` when not set |
| `CODEWIKI_CLIPROXY_KEY`, `CODEWIKI_CLIPROXY_MANAGEMENT_KEY` | Its client key and its management key |

A note whose LLM call fails keeps its structural draft, and the next update asks again. A reply that comes back unusable (it breaks the house style) is not asked for again until the note's code changes.

### CLIProxyAPI

[CLIProxyAPI](https://github.com/router-for-me/CLIProxyAPI) is a separate program, MIT-licensed, that holds several Claude and Codex subscription logins and serves them behind one OpenAI-compatible address. For each request it takes the next account that is ready, and it rests an account that reached its usage limit. With it, a long update goes on across your accounts instead of stopping at the first limit. CodeWiki does not install, start or stop it: you run it, and CodeWiki connects to it.

1. Download the release for your system from its [releases page](https://github.com/router-for-me/CLIProxyAPI/releases) and unpack it.
2. Copy `config.example.yaml` to `config.yaml` and set at least:

   ```yaml
   server:
     host: "127.0.0.1"      # this machine only
     port: 8317
   management:
     secret-key: "<a long random text>"   # lets CodeWiki list and add accounts
     disable-control-panel: true
   access:
     api-keys:
       - "<another long random text>"     # CodeWiki's client key
   ```

3. Start it: `cli-proxy-api -config config.yaml`.
4. In the app, choose **CLIProxyAPI (Claude and Codex subscriptions)** under Settings → AI model, and enter the address, the client key and the management key. The page then lists the proxy's accounts.
5. Press **Add Claude account** or **Add Codex account**: the vendor's sign-in page opens in the browser, and the account shows on the page once you are done. Repeat for each subscription. Pick the model from the list, then press Test.

Accounts are added from the app only through a proxy on the same machine, because the vendor sends the browser back to a port on the proxy's machine. For a proxy elsewhere, sign in there with `cli-proxy-api -claude-login` or `-codex-login`; the app still lists and removes its accounts.

When every account is at its limit, the proxy says so and the update stops: the notes already written stay, the rest wait, and the wiki still reads as behind its code. The app shows "Sync stopped", how many notes wait and, when the proxy says, when an account is free again. The next update writes the notes that wait. `codewiki update` and the MCP tool `sync_wiki` report the same.

### Search

Search combines full-text matching with vectors. `CODEWIKI_EMBED` selects where vectors come from: `local` (default: it matches word forms and needs no model), `builtin` (a model CodeWiki runs by itself, see below), `ollama`, `openai` or `none`. `CODEWIKI_EMBED_MODEL` names the model for `ollama` and `openai`, and for `builtin` the id of an installed model. After a change of embedder, search is full-text alone until the documents are embedded again: by the next `codewiki update`, or for a built-in model in the background.

#### A built-in model

On Windows, CodeWiki can download an embedding model and run it on your machine. Search then finds notes and code by what they mean, not only by the words in them: "is the user playing" finds a function named `busy` in a module about games. Nothing else is installed, no key is needed, and no text leaves the machine.

In the app, Home offers the model recommended for your machine until one is installed, and Settings → Semantic search lists them all with Install, Use and Remove. Install downloads the model, makes it the embedder, and embeds every repository's documents again in the background, with a bar that shows how far it is. From the command line:

```
codewiki model list          # the models, and which are installed
codewiki model recommend     # the one for this machine, and why
codewiki model install gte-modernbert-base
codewiki model remove gte-modernbert-base
```

| Model | Download | Languages | Recommended for |
|---|---|---|---|
| `granite-small-english-r2` | 94 MB | English | under 8 GB of memory, or fewer than 4 processor cores |
| `gte-modernbert-base` | 288 MB | English | 8 GB of memory and 4 cores or more |
| `qwen3-embedding-0.6b` | 1.1 GB | many | an NVIDIA card with 6 GB of video memory or more |

All three are under the Apache License 2.0 and come from their publishers' repositories on Hugging Face; every file is checked against a SHA-256 that ships with CodeWiki. A download that is cut short goes on where it stopped. The command line only downloads and deletes: to search with a model it installed, choose it in the app, or set `CODEWIKI_EMBED=builtin` and `CODEWIKI_EMBED_MODEL` to its id.

The model runs in a hidden process of its own, which starts with the first search or embedding and is put away after ten idle minutes, so that its memory comes back. On a processor the first embedding of a repository takes a while: about 15 short documents a second with `gte-modernbert-base` on a 12-core desktop, fewer for long notes. On an NVIDIA card it is some 25 times faster; the package brings what the card needs (the folder `cuda`, which is most of the download), and `CODEWIKI_EMBED_DEVICE` (`auto`, `cpu`, `cuda`) or the "Runs on" setting says which to use. While a game runs full-screen and keeps the graphics card busy, the model is put away at once and background embedding waits until the game ends.

A backend that listens beyond its own machine does not let another machine install or remove its models, and the Docker image has no engine to run one.

### Saved settings

The variables above need not be set before every start: the same settings can be chosen in the app, under Settings → AI model and Settings → Semantic search, and take effect at once. Each has a Test button, which sends the chosen provider one small request and shows its answer or its error. A variable that is set wins over the saved value, and the page then shows the setting locked, with the variable's name.

The backend keeps them in `settings.json` in its data folder, so the command line and an agent's MCP bridge use them too. An API key is stored encrypted for your Windows user; the page shows only its last four characters, and nothing sends it back. Over HTTP, `GET /v1/settings` shows each setting and where its value comes from, `PUT /v1/settings` changes the saved ones, and `POST /v1/settings/test/llm` and `/test/embed` run the tests. A backend that listens beyond its own machine, as the one in Docker does, takes its settings from its variables only; the app shows them and cannot change them.

Settings → Backend saves the address and token of a backend in Docker or on another machine, in place of `CODEWIKI_URL` and `CODEWIKI_TOKEN`. "Connect" checks that it answers and takes the token before anything is saved. The app and the MCP bridge on this machine then both use it. If it stops answering, the app says so and offers "Use local backend"; it never goes back to a local backend by itself.

### Claude Code plugin

The plugin in `plugins/claude-code/` connects Claude Code to the wiki and the index, so that Claude reads what CodeWiki knows about the code before it explores the code by hand. It adds two things.

**An MCP server.** The plugin runs `codewiki-mcp`, which starts a local backend on first use, or uses the one at `CODEWIKI_URL`. It gives Claude 24 tools:

| Purpose | Tools |
|---|---|
| Orientation | `get_overview`, `get_architecture`, `get_workspace`, `list_repos` |
| Reading the code | `get_context`, `get_symbol`, `search_codebase`, `get_answer`, `read_note`, `get_dependency_path`, `get_why`, `list_decisions` |
| Before and after a change | `get_risk`, `get_change_risk`, `get_blast_radius`, `get_impacted_tests` |
| Code health | `get_hotspots`, `get_dead_code`, `get_health`, `get_coverage`, `get_security_findings`, `get_refactoring_plan`, `get_contracts` |
| Upkeep | `sync_wiki` |

`sync_wiki` follows the same rule as `codewiki update`: a note a person edited is never overwritten, and the change waits in the review queue.

**A session-start briefing.** When a session starts or resumes, and after `/clear` or compaction, a hook runs `codewiki hook session-start`. Claude Code adds what it prints to Claude's context; it does not show in the chat. The briefing is four lines:

1. how many notes the wiki has, and its home note, `obsidian/_HOME.md`;
2. whether the wiki is up to date with `HEAD`. It is stale only when files outside `obsidian/` changed since the commit the wiki was written from, so committing the wiki itself does not make it stale. Without git, the line says the wiki cannot be compared;
3. how many proposals wait for review;
4. that `get_overview` and `get_context` answer from the wiki, and that Claude should call them before it explores the code by hand.

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

   Without the Windows package (programs built from source, or a backend on another machine), add this repository instead: `claude plugin marketplace add Fly-onlyone/CodeWiki`, or the folder of a clone. Claude Code then clones the whole repository, and that needs git.
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

The plugin in `plugins/codex/` gives Codex the same MCP server and session-start briefing as the Claude Code plugin, and two skills:

- `using-codewiki` says which tool to call at each step of a change, and when to write or refresh the wiki with `sync_wiki`;
- `wiki-note-style` holds the house style of CodeWiki's notes, for when Codex edits a note in `obsidian/` by hand.

To install it, after installing CodeWiki and opening a new terminal:

```
codex plugin marketplace add "%LOCALAPPDATA%\Programs\CodeWiki\plugins"
codex plugin add codewiki@codewiki
```

Without the Windows package, add the repository instead: `codex plugin marketplace add Fly-onlyone/CodeWiki`.

Codex skips a hook that comes with a plugin until you trust it. Run `/hooks` in Codex once and trust the `SessionStart` hook that runs `codewiki hook session-start`.

`plugins/codex/README.md` covers updating, removing, and the same setup without the plugin. The plugin's files are checked against Codex's source and by a test; it has not yet been run in a Codex session.

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
cargo deny check
```

`pwsh scripts\clean.ps1` frees the disk space builds leave behind. `AGENTS.md` holds the project rules.

CI runs these four on every push and pull request. It also runs `codewiki check` on this repository and reports the findings to code scanning, and gitleaks scans new commits for secrets. Dependabot opens update pull requests weekly.

To release, set `version` in the root `Cargo.toml`, merge it, and push a matching tag:

```
git tag v0.2.0
git push origin v0.2.0
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
