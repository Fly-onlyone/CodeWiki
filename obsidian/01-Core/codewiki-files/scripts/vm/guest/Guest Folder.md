---
tags: [core, codewiki-files, folder]
---

# Guest Folder

> PowerShell source folder `scripts/vm/guest` of `CodeWiki` with 10 source files, mapped at file level.

## What it does

These scripts run inside the Hyper-V test guest, not on the host. The host test in [[Vm Folder]] copies them in and calls them. They fall into four groups: the helper that runs guest scripts fast (`agent.ps1`), the inspectors (`probe.ps1`, `sampler.ps1`), the stand-in servers (`update-server.ps1` for GitHub's release API, `embed-server.ps1` for an embedding server, `serve-logged.ps1` to keep a real model server's output), and the setup tools (`harden.ps1` for Windows protection, `update-windows.ps1` for Windows Update). `mcp-brief.ps1` and `mcp-write.ps1` act as a coding agent that talks to the `codewiki-mcp` bridge.

## Source

- `scripts/vm/guest/agent.ps1` — PowerShell, 300 lines
- `scripts/vm/guest/probe.ps1` — PowerShell, 203 lines
- `scripts/vm/guest/harden.ps1` — PowerShell, 146 lines
- `scripts/vm/guest/mcp-brief.ps1` — PowerShell, 106 lines
- `scripts/vm/guest/embed-server.ps1` — PowerShell, 104 lines
- `scripts/vm/guest/mcp-write.ps1` — PowerShell, 102 lines
- `scripts/vm/guest/update-server.ps1` — PowerShell, 65 lines
- `scripts/vm/guest/sampler.ps1` — PowerShell, 56 lines
- `scripts/vm/guest/update-windows.ps1` — PowerShell, 48 lines
- `scripts/vm/guest/serve-logged.ps1` — PowerShell, 19 lines

## How it works

### Run and inspect

`agent.ps1` polls a folder for requests from the host, runs each script in a child `powershell.exe` or in a runspace of its own, and writes a `.done` file. `probe.ps1` prints one JSON document with the OS, runtime DLLs, PATH, shortcut, CodeWiki processes, backend health, registry values and crash events. `sampler.ps1` writes a CSV line every few seconds with the model server's memory, CPU, free memory and GPU use, tagged with the phase named in `phase.txt`.

### Stand-ins

`update-server.ps1` listens on a local port and answers the latest-release call, the package download and `/stop`, so the app's self-update can be tested before a release exists. `embed-server.ps1` answers `/v1/models` and `/v1/embeddings`. Its vectors come from hashing each word, so texts that share words lie close together, which is all search needs. Both log every request.

### Setup tools

`harden.ps1` turns Defender, attack surface reduction rules, SmartScreen, the firewall, UAC and LSA protection up, then prints what is in force and which step failed. With `-Audit` it only moves named rules from block to audit. `update-windows.ps1` installs the monthly Windows update through the Update Agent and logs to a file whose last line starts with DONE or FAILED.

### Agent stand-ins

`mcp-brief.ps1` and `mcp-write.ps1` speak JSON-RPC over stdio to the bridge: `list_waiting_notes`, `get_note_brief`, `submit_note`. `mcp-write.ps1` also sends an old base and a wrong heading to check they are refused, then checks claims between two agents. Both print `key: value` lines for the host.

## Items

Folder `scripts/vm/guest`, inside [[Vm Folder]] of [[CodeWiki (codewiki-files)]].

Holds 10 source files with 1149 lines in 1 folder: 10 in PowerShell.

Files:

- `agent.ps1` — PowerShell, 300 lines
- `embed-server.ps1` — PowerShell, 104 lines
- `harden.ps1` — PowerShell, 146 lines
- `mcp-brief.ps1` — PowerShell, 106 lines
- `mcp-write.ps1` — PowerShell, 102 lines
- `probe.ps1` — PowerShell, 203 lines
- `sampler.ps1` — PowerShell, 56 lines
- `serve-logged.ps1` — PowerShell, 19 lines
- `update-server.ps1` — PowerShell, 65 lines
- `update-windows.ps1` — PowerShell, 48 lines

## Gotchas

- Most guest scripts run under Windows PowerShell 5.1, so they avoid PowerShell 7 syntax.
- `harden.ps1` only turns protection up. It adds no exclusion, because what Windows then stops is the finding.
- `update-windows.ps1` runs as a scheduled task as SYSTEM, because the Windows Update Agent refuses to download or install from a remote session such as PowerShell Direct.
- `serve-logged.ps1` starts the server in the user's folder. From Task Scheduler the folder is `System32`, and one model server failed there with "Access is denied".
- `embed-server.ps1` casts the vectors to `double[]`. Without the cast, Windows PowerShell 5.1 wraps the array in an object in the JSON.
- The stand-in servers stop on a request to `/stop`, and the sampler stops when `stop.flag` appears or after `-MaxSeconds`.
- `mcp-write.ps1` asks for only a few notes when it tests claims. A claim lasts 30 minutes, and an earlier version that took all of them left a later case with nothing free.
- `agent.ps1` ends itself after an in-process script times out, since its runspace may still run. The host starts a new helper.

## See also

- [[_index]]
- [[CodeWiki (codewiki-files)]]
- [[Vm Folder]]
