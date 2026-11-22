# A setup program on the release page

Date: 2026-11-22. Status: approved by the owner (2026-11-22). Target: release 0.3.1.

## Intent

The owner looked at the v0.3.0 release and saw "just source only, missing installer". The release does hold `CodeWiki-0.3.0-windows-x64.zip` and its `.sha256`, but GitHub lists its own "Source code (zip)" and "Source code (tar.gz)" beside them on every release, and a person who lands there does not see an installer. Asked what a setup program should fix, the owner chose one thing: **the release looks like source only.** The owner chose earlier to build a real setup program for 0.3.1, and now chose **Inno Setup** to build it.

So the point is a download that is plainly the installer: one file, named so nobody mistakes it, that installs CodeWiki on a double-click.

## What exists

- The zip already installs, in a plain way: unpack it and double-click `install.cmd`, which runs `install.ps1`. That copies `codewiki.exe`, `codewiki-mcp.exe`, `codewiki-app.exe`, `LICENSE`, `NOTICE`, `uninstall.ps1`, `uninstall.cmd` and `plugins\` to `%LOCALAPPDATA%\Programs\CodeWiki` (no administrator rights), adds that folder to the user `PATH` (agents start `codewiki-mcp` from it, and README tells them to add `%LOCALAPPDATA%\Programs\CodeWiki\plugins` as a plugin marketplace) and creates the Start Menu shortcut `CodeWiki.lnk` with the install folder as its working folder. `uninstall.cmd` removes the programs, the `PATH` entry, the shortcut, "Start with Windows" (the `CodeWiki` value under `HKCU\...\Run`) and the app's notification identity; `-PurgeData` also deletes the data and the settings. It cannot delete itself or the folder.
- Self-update (`cw-update`) runs only from an installation: a folder that holds `uninstall.ps1` and not `install.ps1` (`cw-update/src/lib.rs:60`). It downloads the release's zip, checks it against the `.sha256`, and swaps in the files the zip holds (except `install.ps1`/`install.cmd`); files it does not know are left alone.
- `scripts\package.ps1` builds the zip; `.github/workflows/release.yml` (on `windows-latest`) builds it on a tag, writes the `.sha256`, and runs `gh release create <tag> <zip> <zip>.sha256 --title "CodeWiki <version>" --generate-notes --verify-tag`. The VM rig (`scripts\vm\test-release.ps1`) installs the zip in a clean Windows, uses the app and uninstalls it.
- Nothing is code-signed.

## Goals

- Each release carries `CodeWiki-<version>-windows-x64-setup.exe` and its `.sha256`, beside the zip and its `.sha256`.
- The release notes start with one line that names the setup program as the download, above the generated notes.
- Double-clicking the setup program installs CodeWiki for the current user, with no administrator rights, to `%LOCALAPPDATA%\Programs\CodeWiki`, exactly as `install.cmd` does today (programs, `plugins\`, `PATH`, Start Menu shortcut), so the agents' instructions in README stay true.
- The installation it makes updates itself as today.
- It installs over an existing installation (one made by `install.cmd` or by an older setup program) without losing settings, data or "Start with Windows".
- README's Install section leads with the setup program.

## Non-goals

- Code signing. The setup program is unsigned for now; SmartScreen shows "Windows protected your PC" / unknown publisher, as it does for the unzipped programs today. Signing is a later decision.
- MSIX or an `.msi` (rejected below).
- Replacing the zip: the zip stays, because self-update downloads it and people who prefer unpacking keep `install.cmd`.
- Changing what self-update downloads.
- A choice of install folder, components or languages in the wizard.
- Publishing to winget, the Microsoft Store or any package manager.

## Anti-goals

- An installation that no longer updates itself, or that the updater breaks (for example by removing the uninstaller).
- Two installations side by side, or a second `PATH` entry, after installing over an existing one.
- A setup program that needs administrator rights, or installs anywhere but `%LOCALAPPDATA%\Programs\CodeWiki`.
- Uninstalling that leaves the `PATH` entry, the shortcut or "Start with Windows" behind, or that deletes the user's data or settings without being asked.
- Install logic that drifts between the setup program and `install.ps1`.

## Approaches

1. **Inno Setup — chosen by the owner.** A free, widely used builder of Windows setup programs. It makes one `setup.exe` with the usual short wizard, installs per user without administrator rights, and registers an entry in Settings → Apps with an uninstaller. Costs: the release build and `scripts\package.ps1` need its compiler (`ISCC.exe`), a small Windows tool, on CI and on the development machine.
2. **Our own `setup.exe` in Rust**, carrying the zip and doing what `install.ps1` does. No new tool, but we write and maintain an installer, it has no real wizard, and the Apps entry is ours to write. Not chosen.
3. **An `.msi` built with WiX.** Windows Installer tracks every file it installed; self-update replaces those files, and Windows Installer can treat that as damage and "repair" the old files back. Heavier tooling too. Not chosen.

MSIX was set aside before the choice: it installs only when signed by a certificate Windows trusts, and it puts programs in a read-only folder, so `PATH`, the plugin folder path and self-update by file swap would all change.

## Design

### The setup program wraps the existing scripts — *my call*

The setup program does not copy CodeWiki's files itself. It carries the same package the zip holds, unpacks it to its temporary folder, and runs `install.ps1 -Target <install folder>` from there (hidden, `powershell -NoProfile -ExecutionPolicy Bypass`). Its uninstaller runs the installed `uninstall.ps1` the same way, then deletes what is left in the install folder. Reasons: there is one install logic, already tested in the clean VM; `install.ps1` stops a running app and backend before copying (they lock their files); the installed layout stays exactly what self-update expects (`uninstall.ps1` present, `install.ps1` absent); and the setup program can never drift from `install.cmd`.

What the setup program adds to that:

- **Its own uninstaller in the install folder** (Inno's `unins000.exe` and `unins000.dat`). Self-update does not touch files it does not know, so they survive updates. `uninstall.ps1` must not delete them before Inno's uninstaller has finished (it deletes named files only, so it does not).
- **An entry in Settings → Apps** for the current user: name "CodeWiki", the version, the publisher, the app's icon, and the uninstaller. A fixed `AppId`, so a new setup program finds and replaces the entry of an older one.
- **The wizard:** as short as Inno allows for a per-user install with a fixed folder — no folder page, no Start Menu group page, no license page (the license is permissive and ships as `LICENSE`); a "Ready to install" page, the progress page, and a finish page with "Start CodeWiki" (checked) that starts `codewiki-app.exe` the way the Start Menu shortcut does (working folder: the install folder). English only. The app's icon on the setup program.
- **When the install script fails**, the setup program says so with the script's output and leaves the existing installation as it was as far as `install.ps1` does today.
- **Silent install and uninstall** work with Inno's usual switches, so the VM rig and people who script installs can use them.

### Uninstalling

From Settings → Apps (or the uninstaller): it runs `uninstall.ps1` (programs, `PATH` entry, shortcut, "Start with Windows", notification identity), then removes the leftovers in the install folder (`uninstall.ps1`, `uninstall.cmd`, the `.old` files and `.update` folder an update may leave, the uninstaller itself) and the folder. Data and settings are kept, as `uninstall.cmd` does without `-PurgeData`; the wizard does not ask. `uninstall.cmd -PurgeData` stays the way to delete them. *My call: matches today's default; a question in the uninstaller can come later.*

When `uninstall.ps1` fails (a program file still locked, for example), the uninstaller stops there: it shows the script's output, deletes nothing more, keeps the Apps entry and ends with an error, so the user can close what holds the file and uninstall again. `uninstall.ps1` already leaves `PATH` and the shortcut pointing at a working installation when it stops. *My call: deleting the folder and the entry after a failed script would leave the `PATH` entry or the shortcut behind, which is an anti-goal.*

### Self-update and the Apps entry

After the app updates itself from the zip, the Apps entry still shows the version the setup program wrote. Self-update therefore also writes the new version into that entry when the entry exists (the current user's uninstall key with the fixed `AppId`). *My call: a stale version in Settings → Apps would look broken.* An installation made by `install.cmd` has no entry and gets none from an update.

### Installing over an existing installation

Same folder, so `install.ps1` replaces the programs (stopping the running app and backend first, and saying so), keeps the single `PATH` entry, rewrites the shortcut, and leaves data, settings and "Start with Windows" alone. Over an `install.cmd` installation the Apps entry appears for the first time. Running an older setup program over a newer installation installs the older version; Inno's defaults are enough there.

### Building and publishing

- A setup script in the repository (for example `scripts\setup\codewiki.iss`) holds the wizard, the `AppId`, the publisher, the paths and the run/uninstall steps; the version comes from the build.
- `scripts\package.ps1` builds the setup program after the zip, from the same files, when `ISCC.exe` is found, and says clearly when it is not (the zip is still built). `dist\` then holds `CodeWiki-<version>-windows-x64.zip` and `CodeWiki-<version>-windows-x64-setup.exe`.
- `release.yml` makes sure the Inno Setup compiler is present on the runner (check whether `windows-latest` already has it; install it otherwise), builds both, writes a `.sha256` for each, and publishes all four files. The release notes start with a line such as "**Download `CodeWiki-<version>-windows-x64-setup.exe`** and double-click it. The zip is for self-update and for installing by hand." followed by the generated notes.
- The `.sha256` files keep today's format (LF line end, file name after two spaces), so the updater's check and `sha256sum -c` keep working.
- A pre-release (a tag such as `v0.3.1-rc.1`, published with `--prerelease` as today) gets the setup program, its `.sha256` and the notes line too. The Apps entry shows the full version (`0.3.1-rc.1`); the setup program's numeric file version uses the numeric part (`0.3.1.0`). *My call: a pre-release is tested the way a release is.*

### Testing

- The release workflow also runs, building the zip and the setup program without publishing, when a push to `main` or a pull request changes the setup script, `scripts\package.ps1`, `scripts\install.ps1`, `scripts\uninstall.ps1` or `release.yml` itself, so a broken setup script fails before a tag; the built files are kept as a CI artifact for that run. *My call: the setup program needs the real release programs to pack, and a full release build on every push would cost too much.*
- `scripts\vm\test-release.ps1` takes the setup program as a new parameter (beside `-Zip`); its cases run when it is given, and a release run always gives it. The existing `install.cmd` cases stay as they are. *My call.*
- Clean-VM cases in `scripts\vm\test-release.ps1`, run with the next release run:
  - install with the setup program silently in the guest: the programs, `plugins\`, the `PATH` entry, the shortcut and the Apps entry exist; `codewiki --version` answers; the app starts;
  - install with the wizard once by mouse, with screenshots of each page (Ready, progress, finish) and "Start CodeWiki" starting the app;
  - install the setup program over an `install.cmd` installation: one `PATH` entry, settings kept, an Apps entry now;
  - update a setup-program installation from the previous release's zip through the app (as U2 does): the Apps entry shows the new version afterwards;
  - uninstall from the Apps entry's uninstaller silently: nothing is left (folder, `PATH` entry, shortcut, Run value, Apps entry), data and settings remain.
- Unit tests for the updater's Apps-entry write (an entry exists → updated; no entry → nothing written), with a test registry key as `autostart.rs` does.

### Docs

README: the Install section leads with the setup program (download it from the latest release, double-click, what SmartScreen shows and how to continue — "More info" → "Run anyway"), then the zip as the other way; the plugin instructions keep pointing at `%LOCALAPPDATA%\Programs\CodeWiki\plugins`. AGENTS.md: the release now publishes four files, and the build needs Inno Setup. ROADMAP: a row for the setup program with what is verified.

## Left to the builder

- The Inno Setup version, the exact script, and how the compiler gets onto the runner and this machine (verify against Inno Setup's documentation and the runner image's software list; nothing from memory).
- The `AppId` value and the publisher string.
- How the setup program shows the install script's output on failure.
- Whether the setup step in `package.ps1` takes a switch to skip it.
- The VM case ids and how they fit the existing order.

## Calls made in the design, for the owner to check

- The setup program runs `install.ps1` / `uninstall.ps1` rather than copying files itself.
- The wizard has no folder, group or license page; it ends with "Start CodeWiki".
- Uninstalling keeps data and settings and does not ask.
- Self-update writes the new version into the Apps entry.
- A failed `uninstall.ps1` stops the uninstaller and keeps the Apps entry, so the user can retry.
- Pre-releases get the setup program too, with the full version in the Apps entry.
- The release workflow builds the setup program (without publishing) when the setup or install scripts change, not on every push.
- The VM rig takes the setup program as a new parameter; release runs always pass it.
- The file is named `CodeWiki-<version>-windows-x64-setup.exe`; the release notes start with a line pointing to it.
- Shipping 0.3.1 itself (version bump, VM run, tag) goes through the usual release run after this is built; it also brings out the agents-off feature now on `main`.
