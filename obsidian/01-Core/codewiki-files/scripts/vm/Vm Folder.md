---
tags: [core, codewiki-files, folder]
---

# Vm Folder

> PowerShell source folder `scripts/vm` of `CodeWiki` with 15 source files, mapped at file level.

## What it does

This folder is the host side of the Hyper-V test rig. It tests the packaged Windows release in a clean virtual machine (the guest), the way a user would: install, use, update, uninstall. `provision.ps1` builds the guest once. `provision-gpu.ps1` makes a second checkpoint that has the host's graphics driver inside. `test-release.ps1` runs the cases. `Vm.ps1` is the library of helpers they share. `record.ps1` films the guest's screen. The scripts that run inside the guest are in [[Guest Folder]].

## Source

- `scripts/vm/test-release.ps1` — PowerShell, 6973 lines
- `scripts/vm/Vm.ps1` — PowerShell, 1891 lines
- `scripts/vm/provision.ps1` — PowerShell, 182 lines
- `scripts/vm/provision-gpu.ps1` — PowerShell, 95 lines
- `scripts/vm/record.ps1` — PowerShell, 73 lines

## How it works

### Provision

`provision.ps1` copies a Windows 11 ISO, adds an answer file (`autounattend.xml`) so Setup asks nothing, and builds a new ISO with `oscdimg`. It creates a generation 2 VM with a fixed 1600x900 screen, no network during Setup, and waits until the guest writes a done marker. It then reconnects the network, proves one cold start reaches the desktop, and takes the checkpoint `clean`.

`provision-gpu.ps1` restores `clean`, updates Windows if its build has a known display fault, copies the host's graphics driver into the guest (about 2 GB), and takes the checkpoint `clean-gpu`. Only then does it attach the card and check that the guest starts the device and shows a desktop.

### A test run

`test-release.ps1` dot-sources `Vm.ps1`, restores a checkpoint, starts the guest, copies in the zip and setup program, and runs the cases in order. Each case waits on conditions, not fixed sleeps. It writes screenshots, `results.json`, a timing log and a recording into one report folder. `-Only` and `-Skip` choose cases. `-NoGpuSubset` runs the list of cases that suit a guest without a graphics card.

### Recording

`record.ps1` runs as its own process. Hyper-V has no recorder, so it takes screenshots about four times a second and pipes the pixels to `ffmpeg`. It stops when a stop file appears or after `-MaxMinutes`.

## Items

Folder `scripts/vm`, inside [[Scripts Folder]] of [[CodeWiki (codewiki-files)]].

Holds 15 source files with 10363 lines in 2 folders: 15 in PowerShell.

Folders below:

- [[Guest Folder]] — 10 source files, 1149 lines

Files:

- `Vm.ps1` — PowerShell, 1891 lines
- `provision-gpu.ps1` — PowerShell, 95 lines
- `provision.ps1` — PowerShell, 182 lines
- `record.ps1` — PowerShell, 73 lines
- `test-release.ps1` — PowerShell, 6973 lines

## Gotchas

- Hyper-V does not checkpoint a VM that has a graphics card attached, so the card is never in a checkpoint. A test restores `clean-gpu` and calls `Enable-GuestGpu` before starting the guest.
- Windows 11 builds 26100 and 26200 with a revision from 4484 to 7170 lose the desktop screen once a card is attached. `provision-gpu.ps1` updates such a guest, unless `-SkipWindowsUpdate` is given.
- The guest gets all its memory at start. `provision.ps1` checks the host's free memory first and stops, not Hyper-V later. A restored checkpoint brings back its own memory figure, so `-MemoryMb` is set after each restore.
- The guest password is kept to letters, digits, dot, dash and underscore, because it is typed into command lines in the answer file.
- Provisioning needs `oscdimg.exe` from the Windows ADK, and about 6 GB of temporary disk space.
- A running recording repeats the last picture while the VM is off, so one file covers a restart.
- `-Plugin` passes the host's Claude token to the guest. `Copy-ClaudeLogin` in `Vm.ps1` says what is handed over, and the token is taken back before the run ends.
- Evidence of an earlier run in the same report folder is deleted at the start, so keep one folder per run.

## See also

- [[_index]]
- [[CodeWiki (codewiki-files)]]
- [[Scripts Folder]]
