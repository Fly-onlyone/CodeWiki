---
tags: [core, codewiki-files]
---

# CodeWiki (codewiki-files)

> PowerShell and Batch source of the repository with 22 source files, mapped at file level.

## What it does

This unit holds the PowerShell and Batch files of the repository: the packaging and install scripts under `scripts`, and the test rig under `scripts/vm` that installs a packaged release into a clean Windows guest on Hyper-V (Microsoft's built-in virtual machine host) and keeps the evidence. The code is not Rust, so CodeWiki maps it at file level only. A developer or the release process runs these scripts. The app does not use them. See [[Scripts Folder]] for the folders.

## Source

- `scripts/vm/test-release.ps1` — PowerShell, 6973 lines
- `scripts/vm/Vm.ps1` — PowerShell, 1891 lines
- `scripts/vm/guest/agent.ps1` — PowerShell, 300 lines
- `scripts/release-notes.ps1` — PowerShell, 221 lines
- `scripts/vm/guest/probe.ps1` — PowerShell, 203 lines

## How it works

Two paths matter.

### Release notes

`release-notes.ps1` writes the Markdown of a GitHub Release. It reads the conventional commit subjects since the previous release tag with `git log`. Commits of type feat go under New, fix and perf under Fixes, and every other commit goes in a folded list. It adds the highlights from the version's file under `docs/releases`, install steps and a checksum command. A final release counts from the previous final release, a pre-release from any earlier tag. Without `-Tag` it previews HEAD under the version in `Cargo.toml`.

### The VM test

`test-release.ps1` is the main program of the rig. It restores the guest from a checkpoint, copies the zip and setup program in, then runs numbered cases (such as A1, C1, IS1 to IS8) that install, use and uninstall the release. It saves screenshots, a recording and `results.json` under a report folder. `Vm.ps1` is dot-sourced by it and drives the guest: PowerShell Direct for files and commands, a scheduled task for programs on the desktop, and the Hyper-V WMI provider for screenshots, keyboard and mouse. It also writes a timing log and has `Wait-Until` for condition polls.

Inside the guest, `guest/agent.ps1` is a long-running helper that runs the scripts the host sends, so each call does not start a new PowerShell. `guest/probe.ps1` prints one JSON document of what the machine has and what CodeWiki left on it (install files, PATH, shortcut, processes, backend health, registry values, crash events). The test calls it between cases.

## Items

Holds 22 source files with 11078 lines in 3 folders: 20 in PowerShell and 2 in Batch.

Folders:

- [[Scripts Folder]] — 22 source files, 11078 lines

## Gotchas

- The guest scripts run under Windows PowerShell 5.1. The host scripts need PowerShell 7 and the Hyper-V module.
- The agent takes a request by renaming its json file, and the host may cancel the same way, so only one side wins. A script that never started is run again the old way. A script that started is never run twice.
- Only a script that ends by timeout has its child processes killed. One that ends by itself leaves its children alone, because an uninstaller returns at once while a copy of it goes on deleting.
- The host asks the guest for state with `probe.ps1 -Quick` while the app runs. The full probe is slow.
- The rig decides only what a program can decide. Whether a page looks right is for whoever reads the screenshots.
- `release-notes.ps1` warns, and does not stop, when the highlights file for the version is missing.
- `test-release.ps1` with `-Plugin` copies the host's Claude login token into the guest and takes it back before the run ends.

## See also

- [[_index]]
