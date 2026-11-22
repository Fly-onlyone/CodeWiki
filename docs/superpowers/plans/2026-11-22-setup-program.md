# Setup Program Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Each CodeWiki release carries `CodeWiki-<version>-windows-x64-setup.exe` (an Inno Setup 6 setup program that wraps `install.ps1` / `uninstall.ps1`) beside the zip, with release notes that point to it.

**Architecture:** An Inno Setup script packs the same staged folder the zip is made from as `dontcopy` files. In `PrepareToInstall` it extracts them to `{tmp}` and runs `install.ps1 -Target {app}`; Inno itself only writes its uninstaller and the per-user Apps entry. The uninstaller runs the installed `uninstall.ps1` in `CurUninstallStepChanged(usUninstall)` and aborts on failure. `cw-update` writes the new version into the Apps entry after a self-update. `package.ps1` and `release.yml` build and publish it; the clean-VM rig gets setup cases.

**Tech Stack:** Inno Setup 6.7 (ISCC, Pascal Script), PowerShell 5.1/7, Rust (`cw-update`, `windows-registry` 0.100), GitHub Actions (`windows-latest` = Windows Server 2025, Inno Setup 6.7.1 preinstalled), `gh` CLI.

**Spec:** `docs/superpowers/specs/2026-11-22-setup-program-design.md` (approved 2026-11-22). Verified third-party facts (Inno Setup 6.7, the runner image, `gh release create`), each with its source URL: `docs/superpowers/plans/2026-11-22-setup-program-facts.md`. Items marked NOT VERIFIED there must be checked against the cited docs before relying on them.

## Global Constraints

- File name: `CodeWiki-<version>-windows-x64-setup.exe`; its checksum file `CodeWiki-<version>-windows-x64-setup.exe.sha256`, same format as the zip's (`<lowercase sha256>  <file name>` + LF, ASCII, no BOM).
- Install folder: `%LOCALAPPDATA%\Programs\CodeWiki` (`DefaultDirName={userpf}\CodeWiki`), per user, `PrivilegesRequired=lowest`, no administrator rights.
- `AppId={{7DEA7959-917F-46EC-80A8-625E2A9B24F5}` in the script; the Apps entry key is `Software\Microsoft\Windows\CurrentVersion\Uninstall\{7DEA7959-917F-46EC-80A8-625E2A9B24F5}_is1` under `HKEY_CURRENT_USER`; its version value is `DisplayVersion`.
- `AppName=CodeWiki`, `AppPublisher=CodeWiki`, `AppVersion=<full version, e.g. 0.3.1-rc.1>`, `VersionInfoVersion=<numeric part, e.g. 0.3.1>`.
- Wizard: no welcome, folder, Start Menu group or license page; the "Ready to Install" page; finish page with a checked "Start CodeWiki" that runs `{app}\codewiki-app.exe` with working folder `{app}` (`postinstall nowait skipifsilent`). English only. Icon: `crates\codewiki-app\resources\codewiki.ico`.
- The installed layout stays what self-update needs: `uninstall.ps1` present, `install.ps1` and `install.cmd` absent (`cw-update/src/lib.rs:60`). Inno adds `unins000.exe` / `unins000.dat`, which self-update leaves alone.
- Uninstall keeps data and settings and does not ask; a failed `uninstall.ps1` stops the uninstaller, keeps the Apps entry and exits non-zero.
- Release notes start with: ``**Download `CodeWiki-<version>-windows-x64-setup.exe`** and double-click it. The zip is for self-update and for installing by hand.`` then GitHub's generated notes (`gh release create --notes "<line>" --generate-notes`).
- Unsigned. Never install, uninstall or run the setup program on the development machine: only the clean VM guest. Building it (ISCC) on this machine is fine.
- Rust: `cargo fmt --all`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo deny check advisories bans sources` pass before each commit. Only `cargo fmt` formats Rust. Guest scripts run in Windows PowerShell 5.1 (AGENTS.md lessons).

## Review Focus

1. A running CodeWiki app, tray instance or backend during install over an existing installation: the setup program must not fail on locked files (install.ps1 stops them; Inno's own Restart Manager prompt must not appear — `CloseApplications=no`). Pinned by IS5 (the app runs during the setup-over-install case).
2. Install fails (install.ps1 throws, e.g. a locked file): the user sees the script's message and the setup exits non-zero with nothing half-registered. Pinned by IS6.
3. Uninstall while a program file is held: uninstaller stops, Apps entry, `PATH` and shortcut stay, a second uninstall succeeds. Pinned by IS3.
4. A setup-installed copy that updates itself: the Apps entry must show the new version, not the one the setup program wrote. Pinned by Task 2's unit tests and the U2 extension in Task 4.
5. Windows protection turned up (`-Hardened`): an unsigned setup program may be blocked; the run must say so instead of hanging. Pinned by IS1 recording the exit code and S2's protection report.

---

### Task 1: The setup script and its build

**Files:**
- Create: `scripts\setup\codewiki.iss`
- Modify: `scripts\package.ps1` (after the zip, line ~73-75)
- Modify: `.gitignore` only if Inno writes anything outside `dist\`

**Interfaces:**
- Produces: `scripts\package.ps1` writes `dist\CodeWiki-<version>-windows-x64-setup.exe` beside the zip when ISCC is found, from the same stage folder `dist\CodeWiki-<version>-windows-x64\`; prints `packaged <setup path> (<size> MB)`; when ISCC is not found, prints `no Inno Setup compiler (ISCC.exe) found; the setup program was not built` and still exits 0. New switch `-NoSetup` skips it.
- Produces: ISCC defines the script reads: `AppVersion` (full), `FileVersion` (numeric), `Stage` (absolute stage folder), `Icon` (absolute .ico path). Output via `/O<dist> /F<base name>`.

- [ ] **Step 1: Install the compiler on this machine.** `winget install --id JRSoftware.InnoSetup -e -s winget --silent` (Inno Setup 6, the line the CI runner has). Record the ISCC.exe path it gives.
- [ ] **Step 2: Write `scripts\setup\codewiki.iss`.** [Setup] per Global Constraints plus `ArchitecturesAllowed=x64compatible`, `ArchitecturesInstallIn64BitMode=x64compatible`, `CloseApplications=no`, `WizardStyle=modern`, `Compression=lzma2`, `SolidCompression=yes`, `UninstallDisplayIcon={app}\codewiki-app.exe`, `SetupIconFile={#Icon}`, `#ifndef` defaults that make a bare compile fail clearly (e.g. `#error` when `Stage` is not defined). [Files]: `Source: "{#Stage}\*"; DestDir: "{tmp}\pkg"; Flags: dontcopy recursesubdirs` (list them first: solid compression). [Run]: the postinstall entry. [UninstallDelete]: `Type: filesandordirs; Name: "{app}"`. [Code]:
  - `PrepareToInstall`: extract the package files to `{tmp}\pkg` (`ExtractTemporaryFiles` — check the pattern form in the 6.7 help), then `Exec('powershell.exe', '-NoProfile -ExecutionPolicy Bypass -File "<tmp>\pkg\install.ps1" -Target "<app>" *> "<tmp>\install.log"', ...)` hidden, waiting; on `Exec` failure or a non-zero exit, return `'CodeWiki could not be installed:' + #13#10 + <install.log contents>` (non-empty return stops Setup, exit code 7). Use `-Command` if redirection needs it; the log must hold the script's own error text.
  - `CurUninstallStepChanged(usUninstall)`: run `{app}\uninstall.ps1` the same way (log in `{tmp}`); on failure: unless `UninstallSilent`, `MsgBox` the log; then `Abort` (Abort is honoured at `usUninstall` — facts file items 6 and 7).
- [ ] **Step 3: Compile by hand once:** `& <ISCC> /Q "/DAppVersion=0.3.0" "/DFileVersion=0.3.0" "/DStage=<abs dist\CodeWiki-0.3.0-windows-x64>" "/DIcon=<abs .ico>" /O<abs dist> /FCodeWiki-0.3.0-windows-x64-setup scripts\setup\codewiki.iss` (build the stage first with `pwsh scripts\package.ps1` if missing). Expected: exit 0, the file exists. Also compile with no defines: expected exit 2 with your `#error` text.
- [ ] **Step 4: Add the setup step to `package.ps1`.** `[switch]$NoSetup`; find ISCC: `Get-Command ISCC.exe`, then `${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe`, then `$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe`; numeric version = `$version -replace '-.*$',''`; run ISCC with the four defines; throw when ISCC exits non-zero; print the line from Interfaces.
- [ ] **Step 5: Verify:** `pwsh scripts\package.ps1 -NoBuild` (after one normal build) → both `dist\…zip` and `dist\…setup.exe` exist; `pwsh scripts\package.ps1 -NoBuild -NoSetup` → only the zip is rebuilt and the "not built" line is not printed. Parse-check `package.ps1` under Windows PowerShell 5.1 (0 errors). Do NOT run the setup program here.
- [ ] **Step 6: Commit** — `feat(release): build a setup program beside the zip`

---

### Task 2: Self-update writes its version into the Apps entry

**Files:**
- Create: `crates\cw-update\src\apps_entry.rs`
- Modify: `crates\cw-update\src\lib.rs` (module, re-export, call after `swap` at ~:107-109), `crates\cw-update\Cargo.toml` (`[target.'cfg(windows)'.dependencies] windows-registry = "0.100"` — already in the tree via codewiki-app)

**Interfaces:**
- Produces: `pub const APPS_ENTRY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{7DEA7959-917F-46EC-80A8-625E2A9B24F5}_is1";`
- Produces: `pub fn record_version(key: &str, version: &str) -> Result<bool, String>` — under `HKEY_CURRENT_USER`, opens `key` (never creates it); when it exists, sets `DisplayVersion` to `version` and returns `Ok(true)`; when it does not exist, writes nothing and returns `Ok(false)`. On non-Windows: `Ok(false)`.
- `install()` calls `record_version(APPS_ENTRY, &release.version.to_string())` after a successful swap; a failure is logged (`log::warn!`) and does not fail the update.

- [ ] **Step 1: Write the failing tests** in `apps_entry.rs` (`#[cfg(all(test, windows))]`), using a per-process test key like `autostart.rs:120` (`Software\CodeWiki-test\apps-entry-{pid}`) and removing it (and the empty `Software\CodeWiki-test`) afterwards:
  - `a_missing_entry_is_left_alone`: `record_version(&key, "0.3.1") == Ok(false)` and the key still does not exist.
  - `an_existing_entry_gets_the_new_version`: create the key with `DisplayVersion = "0.3.0"`; `record_version(&key, "0.3.1") == Ok(true)`; reading `DisplayVersion` gives `"0.3.1"`; another value set beforehand (`DisplayName = "CodeWiki"`) is unchanged.
  - `the_key_is_the_setup_programs`: `APPS_ENTRY` ends with `{7DEA7959-917F-46EC-80A8-625E2A9B24F5}_is1` and starts with `Software\Microsoft\Windows\CurrentVersion\Uninstall\`.
- [ ] **Step 2: Run** `cargo test -p cw-update apps_entry --locked` → fail (no module).
- [ ] **Step 3: Implement** with `windows_registry::CURRENT_USER.open(key)` (not `create`) and `set_string("DisplayVersion", version)`, the API `autostart.rs:61-77` already uses; wire the call in `install()`.
- [ ] **Step 4: Run** the tests → pass; the four checks.
- [ ] **Step 5: Commit** — `feat(update): show the new version in Settings > Apps after an update`

---

### Task 3: The release workflow publishes the setup program

**Files:**
- Modify: `.github\workflows\release.yml` (`on:` 13-22, "Read the version" 46-60, "Write the checksum" 71-79, artifact 81-88, "Publish the release" 90-102)

**Interfaces:**
- Consumes: Task 1's `package.ps1` output names.
- Produces: outputs `setup=dist/CodeWiki-<version>-windows-x64-setup.exe` beside `zip`; four files in the artifact and on the release.

- [ ] **Step 1: Triggers.** Keep `push: tags: ["v*"]` and `workflow_dispatch`; add `scripts/setup/**` to the `pull_request.paths`; add `push: branches: [main]` with the same `paths` list (one YAML `push:` key holding both `tags` and `branches`+`paths` — check GitHub's rule that `paths` filters do not apply to tag pushes, and keep tag pushes always running).
- [ ] **Step 2: Build step** fails when the setup program is missing: after `package.ps1`, `if (-not (Test-Path $env:SETUP)) { throw "the setup program was not built" }` (the runner has Inno Setup 6.7.1; print the ISCC path `package.ps1` found).
- [ ] **Step 3: Checksums** for both files in the existing format (the zip's code, applied to each).
- [ ] **Step 4: Artifact** holds the four files.
- [ ] **Step 5: Publish** passes the four files and `--notes "<the Global Constraints line with the version>"` with `--generate-notes`, keeping `--title`, `--verify-tag` and the pre-release flag.
- [ ] **Step 6: Verify locally:** YAML parses (`python -c "import yaml,sys;yaml.safe_load(open(sys.argv[1]))" .github/workflows/release.yml` or PowerShell `ConvertFrom-Yaml` if present — say which); every `run:` block parses under pwsh (`[System.Management.Automation.Language.Parser]::ParseInput`). The real run happens in Task 7.
- [ ] **Step 7: Commit** — `ci(release): publish the setup program and point the notes at it`

---

### Task 4: Clean-VM cases for the setup program

**Files:**
- Modify: `scripts\vm\test-release.ps1` (param block 11-63; resolve like `$Zip` at 73-80; copy like line 276; new cases after K2 at ~3502 and before U2 at ~3523; U2 at 3523-3685 and `Remove-UpdateTestInstall` 3516-3520)
- Modify: `AGENTS.md` "Testing a package in a clean Windows" (~184-210): the IS cases and `-Setup`

**Interfaces:**
- Produces: parameter `[string]$Setup` (default: newest `dist\CodeWiki-*-windows-x64-setup.exe` beside the chosen zip, else none); IS cases run only when a setup program is given; a release run always gives it.

Case ids (S1–S3 are taken): each case cleans up after itself so the next starts from no installation.

- [ ] **IS1 — silent install:** run `setup.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /LOG="<guest log>"`; pass when exit 0, `%LOCALAPPDATA%\Programs\CodeWiki` holds the three programs, `plugins\`, `uninstall.ps1`, `unins000.exe` and no `install.ps1`; the user `PATH` holds the folder exactly once; `CodeWiki.lnk` exists; the Apps entry key exists with `DisplayVersion` = the zip's version, `DisplayName` = `CodeWiki`, `Publisher` = `CodeWiki`; `codewiki --version` answers the version; the app starts from the shortcut (one window). Record the exit code and the setup log in the result.
- [ ] **IS2 — uninstall from the Apps entry:** run the entry's `QuietUninstallString` (or `UninstallString` + `/VERYSILENT /SUPPRESSMSGBOXES`) with the app running; pass when exit 0, the folder, the `PATH` entry, the shortcut, the `Run` value (turn "Start with Windows" on first, as AS1 does or by writing the value) and the Apps entry are gone, and `%LOCALAPPDATA%\CodeWiki` and `%APPDATA%\CodeWiki` remain.
- [ ] **IS3 — uninstall refused while a program is held:** install silently; in the guest hold `codewiki-app.exe` open with `FileShare.None` (a `[IO.File]::Open` in a background PowerShell kept alive); uninstall silently; pass when the exit code is non-zero, the Apps entry, `PATH` entry, shortcut and the three programs remain; release the file, uninstall again → exit 0 and everything gone.
- [ ] **IS4 — the wizard by mouse:** run `setup.exe` with no switches; screenshot the "Ready to Install" page; click "Install" with the rig's mouse helper (find how existing cases click a control); screenshot the progress page if it is still showing; wait for the finish page; screenshot it (the "Start CodeWiki" box checked); click "Finish"; pass when the app's window appears and the install folder is as in IS1. Uninstall silently afterwards.
- [ ] **IS5 — over an install.cmd installation, with the app running:** install the zip with `install.cmd`; write a marker into `%APPDATA%\CodeWiki\settings.json`'s folder (a file `marker.txt`) and set the `Run` value; start the app; run the setup program silently; pass when exit 0, the `PATH` holds the folder exactly once, the marker and the `Run` value remain, the Apps entry exists, `codewiki --version` answers the version, and no Inno "close applications" prompt blocked it (silent exit within 120 s). Uninstall silently afterwards.
- [ ] **IS6 — a failing install says why:** make `install.ps1` fail by holding `%LOCALAPPDATA%\Programs\CodeWiki\codewiki.exe` open with `FileShare.None` after an install.cmd installation (copy over a locked file fails); run the setup program silently with `/LOG`; pass when the exit code is 7 and the setup log holds `CodeWiki could not be installed`, and no Apps entry was written. Release the file and clean up.
- [ ] **U2 extension:** before the update, create the Apps entry key with `DisplayVersion` = the old version (as a setup-installed old copy would have it); after the update, pass only if `DisplayVersion` = the new version too; `Remove-UpdateTestInstall` deletes the key.
- [ ] **Verify:** parse `test-release.ps1` and each new guest here-string under Windows PowerShell 5.1 (0 errors); `-Only IS1,IS2` is accepted by the filter (read `Test-Case` at ~132-134); no VM run in this task.
- [ ] **Commit** — `test(vm): install and uninstall with the setup program`

---

### Task 5: Run the setup cases in the guest

**Files:**
- Create: `report\vm-test-<date>-setup\report.md` (`report\` is git-ignored; summary first, screenshots and the recording, as earlier reports)
- Modify: whatever the run proves wrong in Tasks 1, 2 or 4 (each fix its own commit, re-run the cases it touches into `report\vm-test-<date>-setup-2\`)

- [ ] **Step 1:** `pwsh scripts\package.ps1` (with `CODEWIKI_SENTRY_DSN` unset: a test package) → zip and setup program in `dist\`.
- [ ] **Step 2:** Check the host has room for the guest (AGENTS.md "The guest run on the development machine"); then run detached (the session's memory reaper kills long background shells):
  `pwsh scripts\vm\test-release.ps1 -VmName "Tiny 11" -User "Software Tester" -Password "" -Snapshot "Clean GPU" -Gpu -OldZip D:\CodeWiki\dist\old\CodeWiki-0.2.0-windows-x64.zip -Only C1,C5,K1,K2,IS1,IS2,IS3,IS4,IS5,IS6,U2 -Report report\vm-test-<date>-setup` (add the cases the IS cases need to start from, if any).
- [ ] **Step 3:** Read every screenshot; write `report.md` (summary first: pass/fail per case, the setup program's size, the wizard screenshots); `Get-VM "Tiny 11"` is `Off` afterwards.
- [ ] **Step 4:** Commit any fixes (`fix(release): …`, `test(vm): …`), each followed by a re-run of its cases.

---

### Task 6: Docs

**Files:**
- Modify: `README.md` "## Install" (36-49): lead with the setup program (download `CodeWiki-<version>-windows-x64-setup.exe` from the latest release, double-click; SmartScreen's "Windows protected your PC" → "More info" → "Run anyway" because it is unsigned; what it installs; uninstall from Settings → Apps, data kept; `uninstall.cmd -PurgeData` to delete data), then the zip as the other way; building from source says `package.ps1` also builds the setup program when Inno Setup 6 is installed. Update paragraph (52-54): a setup-program installation updates itself too.
- Modify: `AGENTS.md` `## Commands` (~159-162: package.ps1 builds the setup program with ISCC, `-NoSetup`), `## CI` (~170: a tag publishes the zip, the setup program and their checksums), `## Releasing` (~226: `gh release view` shows four files), line ~55 (`cw-update` also writes `DisplayVersion` into the Apps entry).
- Modify: `ROADMAP.md`: a row for the setup program with what Task 5 verified and what waits for the release run.

- [ ] **Step 1:** Write the docs from what Tasks 1–5 actually built (read the code; quote file names and switches exactly).
- [ ] **Step 2:** `cargo test --workspace --locked` still passes (docs-only change; the plugin and wiki-check tests read some docs).
- [ ] **Step 3: Commit** — `docs: describe the setup program`

---

### Task 7: Prove the workflow on GitHub (controller, with the owner's OK)

- [ ] **Step 1:** Ask the owner before pushing: push the branch, then `gh workflow run release.yml --ref <branch>` (workflow_dispatch builds without publishing).
- [ ] **Step 2:** `gh run watch <id> --exit-status`; `gh run download <id>` into the scratchpad; pass when the artifact holds the zip, the setup program and both `.sha256` files, and each checksum matches its file.
- [ ] **Step 3:** Delete the downloaded artifact from the scratchpad. Merging and shipping 0.3.1 follow through the usual release run.
