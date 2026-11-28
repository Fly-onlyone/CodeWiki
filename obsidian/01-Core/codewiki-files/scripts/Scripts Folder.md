---
tags: [core, codewiki-files, folder]
---

# Scripts Folder

> PowerShell and Batch source folder `scripts` of `CodeWiki` with 22 source files, mapped at file level.

## What it does

The `scripts` folder holds the tools that build, ship and clear away CodeWiki on Windows. `package.ps1` builds the release programs and packs them into a zip and a setup program. `install.ps1` and `uninstall.ps1` put CodeWiki on a user's machine and remove it, with `install.cmd` and `uninstall.cmd` as double-click launchers. `release-notes.ps1` writes the text of a GitHub Release. `clean.ps1` frees the disk space that builds leave behind. The test rig lives in the subfolder [[Vm Folder]].

## Source

- `scripts/release-notes.ps1` — PowerShell, 221 lines
- `scripts/clean.ps1` — PowerShell, 181 lines
- `scripts/uninstall.ps1` — PowerShell, 114 lines
- `scripts/package.ps1` — PowerShell, 105 lines
- `scripts/install.ps1` — PowerShell, 64 lines
- `scripts/install.cmd` — Batch, 15 lines
- `scripts/uninstall.cmd` — Batch, 15 lines

## How it works

### Package

`package.ps1` reads the version from the workspace `Cargo.toml`, runs `cargo build --release --locked` for the three programs (`codewiki`, `codewiki-mcp`, `codewiki-app`), and copies them with the licence files, the plugins and the four install scripts into `dist`. It zips that folder. It then looks for the Inno Setup compiler `ISCC.exe` and, if found, builds the setup program from the same folder. Debug files go to a separate `symbols` folder for Sentry, never into the zip.

### Install and uninstall

`install.ps1` stops any running copy that sits in the target folder, copies the programs, adds the folder to the user PATH and writes a Start Menu shortcut. It needs no administrator rights. `uninstall.ps1` stops the processes, deletes the programs first, then the PATH entry, the shortcut, the start-with-Windows value and the notification identity key. Only with `-PurgeData` does it delete the index, logs and settings folders. The `.cmd` files run the `.ps1` files with the script policy bypassed for that one process, and keep the window open after a double-click.

### Clean

`clean.ps1` builds the two programs the run configurations use, reads the unit hashes cargo reports, and deletes every debug artifact whose hash is not among them. `-DryRun` lists, `-All` runs `cargo clean` and removes `dist`, `-Docker` also prunes Docker's cache.

## Items

Folder `scripts`, in [[CodeWiki (codewiki-files)]].

Holds 22 source files with 11078 lines in 3 folders: 20 in PowerShell and 2 in Batch.

Folders below:

- [[Vm Folder]] — 15 source files, 10363 lines

Files:

- `clean.ps1` — PowerShell, 181 lines
- `install.cmd` — Batch, 15 lines
- `install.ps1` — PowerShell, 64 lines
- `package.ps1` — PowerShell, 105 lines
- `release-notes.ps1` — PowerShell, 221 lines
- `uninstall.cmd` — Batch, 15 lines
- `uninstall.ps1` — PowerShell, 114 lines

## Gotchas

- `uninstall.ps1` and `uninstall.cmd` cannot delete themselves, so the folder stays. The script says so.
- Uninstall is safe to run twice: each step removes only what is there, so a second run finishes an interrupted one. Wikis inside repositories are never touched.
- Debug builds in `CodeWiki-Dev` folders are never touched by the uninstaller.
- Without `ISCC.exe`, `package.ps1` still builds the zip and only prints a line. A release job needs the setup program, so it fails later.
- `package.ps1` sets the release profile to keep line tables and not strip, so the PDB files exist. A plain `cargo build --release` writes none.
- If either build in `clean.ps1` fails, nothing is deleted. `-All` with `-DryRun` is refused.
- `clean.ps1` also deletes Proton Drive "Name clash" copies in the build folders, because rustc picks them over the real file and linking breaks.
- The install shortcut sets the working folder to the install folder, not the profile, so the app never indexes the profile by accident.
- `release-notes.ps1` differs for versions below 0.4.0 in how it describes data removal on uninstall.

## See also

- [[_index]]
- [[CodeWiki (codewiki-files)]]
