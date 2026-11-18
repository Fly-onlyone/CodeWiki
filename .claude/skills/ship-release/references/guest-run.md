# The guest run of a release

Details for step 6 and step 8.4 of the skill. `AGENTS.md` ("Testing a package in a clean Windows") explains the rig; `qingyi:clean-vm-test` gives the method. This file holds what is particular to this machine and to a release.

## The guest

- VM `Tiny 11`, account `Software Tester` with an empty password, checkpoints `Clean GPU` (off; the host's graphics driver is in it) and `Clean` (no graphics driver). These names are this machine's; the rig's own defaults are other names, so pass them every time.
- The host runs one guest at a time and usually has too little memory free. The rig trims working sets by itself before a start; `-MemoryMb 3072` lowers what the guest asks for. Close none of the user's programs.
- The checkpoints carry no git. Keep it so: the cases B1 and D2 need a Windows without it. The plugin cases install git themselves.
- Keeping the guest current: restore the checkpoint without the graphics card, `Update-GuestWindows -Pattern '^(?!.*(Preview|Windows 11, version)).*$'` in rounds with a restart between them until it says "nothing to install", `Update-MpSignature`, shut down, rename the old checkpoint to `<name> (before update <date>)`, `Checkpoint-VM` once (its list lags; never call it twice). Hyper-V takes no checkpoint while the card is attached. Delete an old checkpoint only when the user says so.

## The run

```
pwsh scripts\vm\test-release.ps1 -VmName "Tiny 11" -User "Software Tester" -Password "" `
    -Snapshot "Clean GPU" -Gpu -Hardened -Plugin -MemoryMb 3072 `
    -Zip dist\CodeWiki-<version>-windows-x64.zip -OldZip dist\old\CodeWiki-<previous>-windows-x64.zip `
    -Report report\vm-test-<date>-<version> *> report\vm-test-<date>-<version>.log
```

- Run it in the background and read the log file; the script writes none itself. A full pass takes about 45 minutes, the plugin cases included.
- Check the host's Claude token before the run: the plugin cases refuse one that ends within 20 minutes.
- A build of minutes ago is refused under `-Hardened` (case S3 records it and goes on); the same build two days later was let through. Neither is a failure of the run.
- To read 100 screenshots, make contact sheets of six with ffmpeg (`xstack`) and read those; open a single picture only where a sheet is not enough.
- Pass `-Zip`: the default is the newest zip in `dist\`, which is not always the one meant.
- `-OldZip` is the previous published package: `gh release download v<previous> --repo Fly-onlyone/CodeWiki --pattern "*.zip*" --dir dist\old`, then check it against its `.sha256`. Before the version is bumped, `scripts\package.ps1` writes a zip with the old version's name over the one in `dist\`.
- A rerun gets a new `-Report` folder. The script empties the screenshots of a folder it is given again.
- `-Only A1,C1` runs single cases after the usual setup; most cases build on earlier ones.
- The script stops the guest at its end, but not when it fails during setup. Afterwards: `Get-VM "Tiny 11"`; when it is not `Off`, `Stop-VM -Name "Tiny 11" -Force`.

## `-Hardened`

`scripts\vm\guest\harden.ps1` turns Windows' own protection up before anything is installed: today's definitions, cloud block level at zero tolerance, controlled folder access, network protection, every attack surface reduction rule, UAC on "always notify", SmartScreen on block. It excludes and allows nothing. S1 records what is in force, S2 lists what Defender stopped. A block that names a CodeWiki program is a finding, not something to exclude.

## `-Plugin`

Cases CC0 to CC8: Git for Windows, then Claude Code as its docs say, the plugin from the installation's `plugins` folder, one real session that calls `get_overview`, then the removal and the GitHub form of the marketplace.

It uses the user's Claude account, for two short requests on the cheapest model. Rules for the login:

- Only the access token of the host's saved login goes into the guest. Never the refresh token: a renewal in the guest can end the login on the host, and every session on it. Never the other services' tokens in the same file.
- Never print the token, write it to a log or a report, or pass it on a command line. `Copy-ClaudeLogin` sends it over PowerShell Direct.
- Take no checkpoint while it is in the guest. `Remove-ClaudeLogin` deletes it at the end; the next restore would too.
- `ENABLE_CLAUDEAI_MCP_SERVERS=false` keeps the account's own connectors out of the guest's sessions and out of the report.
- When the host's token ends within 20 minutes, or Claude does not answer with it, ask the user to log in by hand in the guest. Do not look for another way in.

## Against the published release (step 8.4)

```
gh release download v<version> --repo Fly-onlyone/CodeWiki --pattern "*.zip*" --dir <scratchpad folder>
pwsh scripts\vm\test-release.ps1 -VmName "Tiny 11" -User "Software Tester" -Password "" `
    -Snapshot "Clean GPU" -Gpu -Zip <scratchpad folder>\CodeWiki-<version>-windows-x64.zip `
    -OldZip dist\old\CodeWiki-<previous>-windows-x64.zip -ReleaseTag v<version> -RealRelease -Only R1,U2 `
    -Report report\vm-test-<date>-<version>-published *> report\vm-test-<date>-<version>-published.log
```

R1 downloads the package in the guest and compares it with `-Zip` and with its `.sha256`. U2 installs the previous version and lets it update itself from the real GitHub.
