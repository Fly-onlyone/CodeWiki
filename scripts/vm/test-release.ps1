# Install the packaged release into a clean Windows guest, use it the way a
# user would, uninstall it, and keep the evidence. Run after
# scripts\package.ps1 and (once) scripts\vm\provision.ps1:
#   pwsh scripts\vm\test-release.ps1
#
# Every run starts from the VM's "clean" checkpoint. Results go to
# report\vm-test-<date>\: screenshots\, recording\session.mp4 and
# logs\results.json. The script decides only what a program can decide (exit
# codes, files, processes); whether a page was drawn correctly is for whoever
# reads the screenshots.
param(
    [string]$VmName = "Software Tester",
    # Package to test. Default: the newest zip in dist\.
    [string]$Zip,
    # Folder for the evidence. Default: report\vm-test-<yyyy-MM-dd>.
    [string]$Report,
    [string]$User = "tester",
    [string]$Password = "codewiki",
    [string]$Snapshot = "clean",
    # Give the guest a share of the host's graphics card. Needs the checkpoint
    # "clean-gpu" from provision-gpu.ps1, which is then the default.
    [switch]$Gpu,
    # With -Gpu, when the checkpoint holds an older graphics driver than the
    # host now has: copy the host's driver into the guest for this run (about
    # 2 GB) instead of stopping. The checkpoint itself is left as it is;
    # provision-gpu.ps1 -Force is what retakes it.
    [switch]$RefreshGpuDriver,
    # Leave the VM running at the end, for a look by hand.
    [switch]$KeepRunning,
    # A package of an older version. With it, case U2 installs that version
    # and has it update itself to the package under test, which a stand-in
    # for GitHub serves inside the guest (guest\update-server.ps1).
    [string]$OldZip,
    # With -OldZip: let the older version ask the real GitHub instead of the
    # stand-in. For the run after a release, with the published package as -Zip.
    [switch]$RealRelease,
    # A published release's tag (v0.1.0). With it, case R1 downloads that
    # release's package inside the guest and checks it against its .sha256
    # file and against -Zip, which must be the same package.
    [string]$ReleaseTag,
    # Memory for the guest in this run, in megabytes. Default: what the
    # checkpoint holds. Lower it on a host that has little free; a restored
    # checkpoint brings its own figure back, so it is set after each restore.
    [int]$MemoryMb,
    # Turn Windows' own protection in the guest up as far as its settings go
    # (guest\harden.ps1) before anything is installed, and make the whole run
    # under it. Case S1 records what is in force, case S2 what it stopped.
    [switch]$Hardened,
    # Also test the Claude Code plugin (cases CC0 to CC8): install Git for
    # Windows and Claude Code in the guest as their docs say, sign Claude Code
    # in with the access token of the host's own Claude login, install the
    # plugin as README says and have Claude call one of its tools. It makes
    # two short requests on the account's cheapest model. The token is taken
    # from the guest again before the run ends; see Copy-ClaudeLogin for what
    # is and is not handed over.
    [switch]$Plugin,
    # Run only these cases (their IDs), after the usual setup of the guest.
    # For one case that can stand alone, such as U2; most cases build on the
    # ones before them.
    [string[]]$Only,
    # Multiplies every wait; raise it on a slow host.
    [double]$Pace = 1.0
)
$ErrorActionPreference = "Stop"
# "pwsh -File ... -Only A1,C1" hands the list over as one text.
if ($Only) { $Only = @($Only | ForEach-Object { $_ -split "," } | ForEach-Object { $_.Trim() } | Where-Object { $_ }) }
. (Join-Path $PSScriptRoot "Vm.ps1")
Initialize-Vm -Name $VmName -User $User -Password $Password
# The host must not fall asleep under a run nobody is watching.
Start-KeepAwake

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not $Zip) {
    $newest = Get-ChildItem (Join-Path $root "dist") -Filter "CodeWiki-*-windows-x64.zip" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $newest) { throw "no package in dist\; run scripts\package.ps1 first" }
    $Zip = $newest.FullName
}
$Zip = (Resolve-Path $Zip).Path
if ($OldZip) { $OldZip = (Resolve-Path $OldZip).Path }
# The version a package's name carries.
function Get-ZipVersion {
    param([string]$Path)
    if ((Split-Path -Leaf $Path) -match '^CodeWiki-(.+)-windows-x64\.zip$') { $Matches[1] } else { throw "cannot read a version from $Path" }
}
if (-not $Report) { $Report = Join-Path $root ("report\vm-test-" + (Get-Date -Format "yyyy-MM-dd")) }
$shots = Join-Path $Report "screenshots"
$logs = Join-Path $Report "logs"
New-Item -ItemType Directory -Force $shots, $logs, (Join-Path $Report "recording") | Out-Null
$Report = (Resolve-Path $Report).Path
# Evidence of an earlier run into the same folder would mix with this one's.
Get-ChildItem $shots -Filter "*.png" | Remove-Item -Force
Get-ChildItem $logs -Filter "probe-*.json" | Remove-Item -Force

# The guest's folders. They are set once the guest is up, from what the guest
# says its profile folder is: that is not always C:\Users\<account name>.
$guestHome = $null
$guestUnpacked = $null
$guestTarget = $null
$guestRepo = $null
$powershell = "C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe"

$results = [System.Collections.Generic.List[object]]::new()
$shotCount = 0

function Wait-For { param([double]$Seconds) Start-Sleep -Milliseconds ([int]($Seconds * 1000 * $Pace)) }

# Take a screenshot named after the case and return its file name.
function Shot {
    param([string]$Name)
    $script:shotCount += 1
    $file = "{0:d2}-{1}.png" -f $script:shotCount, $Name
    Save-Screenshot (Join-Path $shots $file) | Out-Null
    $file
}

function Add-Result {
    param(
        [string]$Id, [string]$Title, [string]$Expect,
        [ValidateSet("pass", "fail", "note", "error")][string]$Status,
        [string]$Observed, [string[]]$Shots = @(), [string]$Output = ""
    )
    $results.Add([pscustomobject]@{
            id = $Id; title = $Title; expect = $Expect; status = $Status
            observed = $Observed; screenshots = $Shots; output = $Output
        })
    "{0,-5} {1,-4} {2}" -f $Status, $Id, $Observed
    $results | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $logs "results.json")
}

# Run one case; a thrown error becomes an "error" result and the run goes on.
function Test-Case {
    param([string]$Id, [string]$Title, [string]$Expect, [scriptblock]$Body)
    if ($Only -and $Only -notcontains $Id) { return }
    try {
        & $Body
    } catch {
        Add-Result -Id $Id -Title $Title -Expect $Expect -Status error `
            -Observed "the harness failed: $($_.Exception.Message)" -Shots @(Shot "$Id-error")
    }
}

# Run probe.ps1 in the guest; keep its JSON under logs\ and return it parsed.
# -Quick leaves out the slow parts; use it while the app is running.
function Get-Probe {
    param([string]$Name, [switch]$Quick)
    $arguments = "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
        "-File", "$($script:Vm.GuestDir)\probe.ps1", "-Target", $guestTarget
    if ($Quick) { $arguments += "-Quick" }
    $run = Invoke-GuestExe -Exe $powershell -TimeoutSec 300 -Arguments $arguments
    Set-Content (Join-Path $logs "probe-$Name.json") $run.Stdout
    # A probe cut off by its timeout prints nothing, and nothing must not be
    # read as "no processes, no files".
    if (-not $run.Stdout.Trim()) { throw "probe '$Name' printed nothing: $($run.Stderr.Trim())" }
    try { $run.Stdout | ConvertFrom-Json } catch { throw "probe '$Name' printed no JSON: $($run.Stderr.Trim())" }
}

# Seconds of processor time the app uses in ten seconds.
function Get-AppCpu {
    $run = Invoke-GuestScript -TimeoutSec 300 -Script @'
$p = Get-Process codewiki-app | Select-Object -First 1
$before = $p.TotalProcessorTime.TotalSeconds
Start-Sleep 10
$p.Refresh()
"cpu=$([math]::Round($p.TotalProcessorTime.TotalSeconds - $before, 2))"
'@
    if ($run.Output -match "cpu=([\d\.]+)") { [double]$Matches[1] } else { throw "could not measure the app's CPU use: $($run.Output)" }
}

# Set the app's motion setting ("system", "reduced" or "full") before it starts.
function Set-GuestMotion {
    param([string]$Value)
    Invoke-GuestScript -Script "New-Item -ItemType Directory -Force `"`$env:APPDATA\CodeWiki`" | Out-Null; Set-Content `"`$env:APPDATA\CodeWiki\motion.txt`" '$Value' -NoNewline" | Out-Null
}

# Start the desktop app with arguments in the logged-in user's desktop.
function Start-GuestApp {
    param([string[]]$Arguments = @())
    Start-GuestExe -Exe "$guestTarget\codewiki-app.exe" -Arguments $Arguments | Out-Null
    # Gives the window the keyboard focus and remembers where it is.
    $script:appWindow = Show-GuestWindow -ProcessName "codewiki-app"
}

function Stop-GuestApp {
    Invoke-GuestScript -Script 'Get-Process codewiki-app -ErrorAction SilentlyContinue | Stop-Process -Force' | Out-Null
    Wait-For 2
}

# Number of entries in a probe's file list; the list is null when the folder is gone.
function Get-FileCount { param($Files) @($Files | Where-Object { $_ }).Count }
function Get-App { param($Probe) @($Probe.processes | Where-Object { $_.name -eq "codewiki-app" }) }
function Get-Backend { param($Probe) @($Probe.processes | Where-Object { $_.name -eq "codewiki" }) }

# Open the Start menu, type the app's name and press Enter, as a user does.
function Start-FromStartMenu {
    Send-Keys "win"
    Wait-For 4
    Send-Text "CodeWiki"
    Wait-For 6
    $found = Shot "start-menu-search"
    Send-Keys "enter"
    Wait-For 20
    $found
}

# ---------------------------------------------------------------- A1, host
Test-Case "A1" "Package contents" "three programs, licence files, the scripts and their launchers, and the plugins" {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [System.IO.Compression.ZipFile]::OpenRead($Zip)
    try { $entries = @($archive.Entries | ForEach-Object { $_.FullName -replace "\\", "/" }) } finally { $archive.Dispose() }
    $needed = "codewiki.exe", "codewiki-mcp.exe", "codewiki-app.exe", "LICENSE", "NOTICE", "install.ps1", "uninstall.ps1", "install.cmd", "uninstall.cmd"
    $missing = @($needed | Where-Object { $entries -notcontains $_ })
    $plugins = @($entries | Where-Object { $_ -like "plugins/*" }).Count
    # Without it the plugins folder cannot be added to Claude Code as a marketplace.
    if ($entries -notcontains "plugins/.claude-plugin/marketplace.json") { $missing += "plugins/.claude-plugin/marketplace.json" }
    # The same for Codex.
    if ($entries -notcontains "plugins/.agents/plugins/marketplace.json") { $missing += "plugins/.agents/plugins/marketplace.json" }
    $unpacked = Join-Path ([System.IO.Path]::GetTempPath()) "cwvm-zip-$PID"
    Expand-Archive -Path $Zip -DestinationPath $unpacked -Force
    # Names of the DLLs each program asks Windows for, read from the file itself.
    $imports = foreach ($exe in "codewiki.exe", "codewiki-mcp.exe", "codewiki-app.exe") {
        $text = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes((Join-Path $unpacked $exe)))
        $dlls = [regex]::Matches($text, '(?i)\b[a-z0-9_\-\.]{2,40}\.dll\b') | ForEach-Object { $_.Value.ToLowerInvariant() } | Sort-Object -Unique
        "$exe`: " + ($dlls -join ", ")
    }
    Remove-Item -Recurse -Force $unpacked
    $size = [math]::Round((Get-Item $Zip).Length / 1MB, 1)
    $status = if ($missing.Count -eq 0 -and $plugins -gt 0) { "pass" } else { "fail" }
    Add-Result -Id "A1" -Title "Package contents" -Expect "three programs, licence files, the scripts and their launchers, and the plugins" -Status $status `
        -Observed "$(Split-Path -Leaf $Zip), $size MB, $($entries.Count) entries, $plugins under plugins/, missing: $(if ($missing) { $missing -join ', ' } else { 'none' })" `
        -Output ($imports -join "`n")
}

# ---------------------------------------------------------------- guest up
if ($Gpu -and -not $PSBoundParameters.ContainsKey("Snapshot")) { $Snapshot = "clean-gpu" }
"restoring checkpoint '$Snapshot'"
Restore-Guest -Snapshot $Snapshot
if ($MemoryMb) {
    # Startup and ceiling alike: a guest that grows later takes what the
    # host does not have.
    Set-VMMemory -VMName $VmName -StartupBytes ($MemoryMb * 1MB) -MaximumBytes ($MemoryMb * 1MB)
    "guest memory set to $MemoryMb MB for this run"
}
# A checkpoint does not keep the card, so it is attached after every restore.
if ($Gpu) { Enable-GuestGpu }
Start-Guest
Wait-Guest -TimeoutSec 900
if ($Gpu) {
    $driver = Test-GuestGpuDriver
    if (-not $driver.Match -and $RefreshGpuDriver) {
        # For this run only: the checkpoint keeps the driver it was taken
        # with, and the next restore brings that one back.
        "the guest holds the graphics driver '$($driver.GuestDrivers -join ', ')' and the host now has '$($driver.HostDriver)'; copying the host's into the guest for this run"
        Copy-GpuDriverToGuest | Out-Null
        Restart-Guest
        Wait-Guest -TimeoutSec 900
        $driver = Test-GuestGpuDriver
    }
    if (-not $driver.Match) {
        throw "the guest holds the graphics driver '$($driver.GuestDrivers -join ', ')' and the host now has '$($driver.HostDriver)'; run scripts\vm\provision-gpu.ps1 -Force, or pass -RefreshGpuDriver to copy the host's driver in for this run alone"
    }
    # Without a screen nothing is drawn, and every measurement would be of nothing.
    if (-not (Test-GuestDisplay)) { throw "the guest's desktop has no screen with the graphics card attached; see Test-GuestDisplay in Vm.ps1" }
}
$guestHome = (Invoke-GuestScript -Script '$env:USERPROFILE').Output.Trim()
if ($guestHome -notmatch '^[A-Za-z]:\\') { throw "could not read the guest's profile folder: '$guestHome'" }
$guestUnpacked = "$guestHome\Downloads\CodeWiki"
$guestTarget = "$guestHome\AppData\Local\Programs\CodeWiki"
$guestRepo = "$guestHome\repos\cw-core"
Wait-For 30
Start-Recording -Path (Join-Path $Report "recording\session.mp4") | Out-Null

New-GuestDirectory $script:Vm.GuestDir
New-GuestDirectory "$guestHome\repos"
Copy-ToGuest -Path (Join-Path $PSScriptRoot "guest\probe.ps1") -GuestDirectory $script:Vm.GuestDir
Copy-ToGuest -Path $Zip -GuestDirectory "$guestHome\Downloads"
Copy-ToGuest -Path (Join-Path $root "crates\cw-core") -GuestDirectory "$guestHome\repos"
$guestZip = "$guestHome\Downloads\$(Split-Path -Leaf $Zip)"

# ---------------------------------------------------------------- S1, protection
# Before anything of CodeWiki is in the guest, so that the unpacking, the
# install and every start happen under it. Nothing is excluded or allowed.
if ($Hardened) {
    Test-Case "S1" "Windows protection turned up" "every step of guest\harden.ps1 succeeds" {
        Copy-ToGuest -Path (Join-Path $PSScriptRoot "guest\harden.ps1") -GuestDirectory $script:Vm.GuestDir
        $run = Invoke-GuestExe -Background -TimeoutSec 300 -Exe $powershell -Arguments "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
            "-File", "$($script:Vm.GuestDir)\harden.ps1"
        Set-Content (Join-Path $logs "hardening.json") $run.Stdout
        if (-not $run.Stdout.Trim()) { throw "harden.ps1 printed nothing: $($run.Stderr.Trim())" }
        $state = $run.Stdout | ConvertFrom-Json
        $failed = @($state.failed)
        Add-Result -Id "S1" -Title "Windows protection turned up" -Expect "every step succeeds" `
            -Status $(if ($failed) { "fail" } else { "pass" }) -Output $run.Stdout `
            -Observed "Defender $($state.defenderMode), real-time $($state.realTimeProtection), tamper protection $($state.tamperProtection); cloud block level $($state.cloudBlockLevel) (6 is zero tolerance), all samples sent; unwanted applications $($state.unwantedApplications), network protection $($state.networkProtection), controlled folder access $($state.controlledFolderAccess) (1 is on); attack surface reduction rules blocking: $($state.reductionRulesBlocking) of $($state.reductionRules); exclusions: $(@($state.exclusions).Count); UAC prompt $($state.uacPrompt) (2 is always notify); SmartScreen '$($state.smartScreen)'; firewall $($state.firewall -join ', '); LSA protection $($state.lsaProtection); steps that failed: $(if ($failed) { $failed -join '; ' } else { 'none' })"
    }
}

Test-Case "B1" "Guest before install" "recorded only" {
    $probe = Get-Probe "before"
    $missing = @($probe.runtime | Where-Object { -not $_.present } | ForEach-Object { $_.name })
    $policy = ($probe.powershell.policy | ForEach-Object { "$($_.scope)=$($_.policy)" }) -join ", "
    Add-Result -Id "B1" -Title "Guest before install" -Expect "recorded only" -Status note -Shots @(Shot "desktop") `
        -Observed "$($probe.os.caption) $($probe.os.display) build $($probe.os.build); PowerShell $($probe.powershell.version), pwsh $(if ($probe.powershell.pwsh) { 'present' } else { 'absent' }); policy $policy; git $(if ($probe.git) { 'present' } else { 'absent' }); adapters: $(($probe.adapters | ForEach-Object { $_.name }) -join ', '); graphics card attached by the host: $([bool]$Gpu); runtime DLLs missing: $(if ($missing) { $missing -join ', ' } else { 'none' }); commands run in session $($probe.session.id), elevated $($probe.session.elevated)"
}

# ---------------------------------------------------------------- R1, published
if ($ReleaseTag) {
    Test-Case "R1" "Download the published package in the guest" "the zip and its .sha256 file come from GitHub and agree, and the zip is the one under test" {
        $name = Split-Path -Leaf $Zip
        $base = "https://github.com/Fly-onlyone/CodeWiki/releases/download/$ReleaseTag"
        $run = Invoke-GuestScript -TimeoutSec 900 -Script @"
`$ProgressPreference = 'SilentlyContinue'
`$dir = '$guestHome\Downloads\published'
New-Item -ItemType Directory -Force `$dir | Out-Null
Invoke-WebRequest -UseBasicParsing '$base/$name' -OutFile (Join-Path `$dir '$name')
Invoke-WebRequest -UseBasicParsing '$base/$name.sha256' -OutFile (Join-Path `$dir '$name.sha256')
"published: `$(((Get-Content (Join-Path `$dir '$name.sha256') -Raw) -split '\s+')[0])"
"downloaded: `$((Get-FileHash -Algorithm SHA256 (Join-Path `$dir '$name')).Hash.ToLowerInvariant())"
"bytes: `$((Get-Item (Join-Path `$dir '$name')).Length)"
"@
        $published = if ($run.Output -match "published: ([0-9a-f]{64})") { $Matches[1] } else { "" }
        $downloaded = if ($run.Output -match "downloaded: ([0-9a-f]{64})") { $Matches[1] } else { "" }
        $tested = (Get-FileHash -Algorithm SHA256 $Zip).Hash.ToLowerInvariant()
        $ok = $published -and $published -eq $downloaded -and $downloaded -eq $tested
        Add-Result -Id "R1" -Title "Download the published package in the guest" -Expect "three equal SHA-256 values" `
            -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
            -Observed "from $base; the .sha256 file says $published; the zip downloaded in the guest has $downloaded; the zip this run installs has $tested"
    }
}

# ---------------------------------------------------------------- C, install
Test-Case "C1" "Unpack the zip" "Expand-Archive in Windows PowerShell 5.1 gives every entry of the package" {
    $run = Invoke-GuestScript -Script @"
Expand-Archive -Path '$guestZip' -DestinationPath '$guestUnpacked' -Force
Get-ChildItem '$guestUnpacked' | ForEach-Object { `$_.Name }
"@
    $names = @($run.Output -split "`n" | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    $needed = "codewiki.exe", "codewiki-mcp.exe", "codewiki-app.exe", "LICENSE", "NOTICE", "install.ps1", "uninstall.ps1", "install.cmd", "uninstall.cmd", "plugins"
    $missing = @($needed | Where-Object { $names -notcontains $_ })
    Add-Result -Id "C1" -Title "Unpack the zip" -Expect "every entry of the package" -Status $(if ($missing) { "fail" } else { "pass" }) `
        -Observed "unpacked: $($names -join ', ')" -Output $run.Output
}

function Test-Installed {
    param($Probe)
    $files = @($Probe.install.files | ForEach-Object { $_.path })
    $problems = @()
    foreach ($exe in "codewiki.exe", "codewiki-mcp.exe", "codewiki-app.exe", "uninstall.ps1", "uninstall.cmd", "LICENSE", "NOTICE", "plugins") {
        if ($files -notcontains $exe) { $problems += "missing $exe" }
    }
    if ($Probe.userPath.targetCount -ne 1) { $problems += "user PATH holds the folder $($Probe.userPath.targetCount) times" }
    if (-not $Probe.shortcut) { $problems += "no Start menu shortcut" }
    elseif ($Probe.shortcut.target -ne "$guestTarget\codewiki-app.exe") { $problems += "shortcut points at $($Probe.shortcut.target)" }
    # Started in the profile, the app once took the profile as its repository.
    elseif ($Probe.shortcut.workingDirectory -ne $guestTarget) { $problems += "shortcut starts in $($Probe.shortcut.workingDirectory)" }
    $problems
}

# Windows refuses a plain install.ps1; that is its policy, and why install.cmd
# exists. Recorded, not judged.
Test-Case "C2" "install.ps1 started plainly" "refused by the execution policy Windows ships with" {
    $run = Invoke-GuestExe -Exe $powershell -TimeoutSec 120 -Arguments "-NoProfile", "-NonInteractive", "-File", "$guestUnpacked\install.ps1"
    $probe = Get-Probe "after-default-policy"
    $text = ($run.Stdout + "`n" + $run.Stderr).Trim()
    Add-Result -Id "C2" -Title "install.ps1 started plainly" -Expect "refused by the default execution policy; recorded" -Status note `
        -Observed $(if ($probe.install.exists) { "installed without any policy switch" } else { "refused: nothing was installed" }) -Output $text
}

# What a double-click on install.cmd runs: cmd.exe /c with the file.
Test-Case "C5" "install.cmd, as README says" "installs under the policy Windows ships with" {
    $run = Invoke-GuestExe -Exe "C:\Windows\System32\cmd.exe" -TimeoutSec 120 -Arguments "/c", "$guestUnpacked\install.cmd"
    $probe = Get-Probe "after-install-cmd"
    $problems = Test-Installed $probe
    Add-Result -Id "C5" -Title "install.cmd, as README says" -Expect "programs, PATH entry and shortcut in place" `
        -Status $(if ($problems -or $run.ExitCode -ne 0) { "fail" } else { "pass" }) -Output ($run.Stdout + "`n" + $run.Stderr).Trim() `
        -Observed "exit $($run.ExitCode); $(if ($problems) { $problems -join '; ' } else { 'all in place' }); shortcut working directory $($probe.shortcut.workingDirectory)"
}

Test-Case "C2b" "install.ps1 by right-click, Run with PowerShell" "installs" {
    # The command Explorer runs for that menu entry.
    $command = "if((Get-ExecutionPolicy ) -ne 'AllSigned') { Set-ExecutionPolicy -Scope Process Bypass }; & '$guestUnpacked\install.ps1'"
    $run = Invoke-GuestExe -Exe $powershell -TimeoutSec 120 -Arguments "-NoProfile", "-NonInteractive", "-Command", $command
    $probe = Get-Probe "after-run-with-powershell"
    $problems = Test-Installed $probe
    Add-Result -Id "C2b" -Title "install.ps1 by right-click, Run with PowerShell" -Expect "installs" `
        -Status $(if ($problems -or $run.ExitCode -ne 0) { "fail" } else { "pass" }) `
        -Observed "exit $($run.ExitCode), over the installation C5 made; $(if ($problems) { $problems -join '; ' } else { 'all in place' })" -Output ($run.Stdout + "`n" + $run.Stderr).Trim()
}

Test-Case "C3" "install.ps1 with -ExecutionPolicy Bypass" "programs, PATH entry and shortcut in place, no elevation prompt" {
    $run = Invoke-GuestScript -Script "& '$powershell' -NoProfile -ExecutionPolicy Bypass -File '$guestUnpacked\install.ps1'"
    $shot = Shot "after-install"
    $probe = Get-Probe "installed"
    $problems = Test-Installed $probe
    Add-Result -Id "C3" -Title "install.ps1 with -ExecutionPolicy Bypass" -Expect "programs, PATH entry and shortcut in place" `
        -Status $(if ($problems -or $run.ExitCode -ne 0) { "fail" } else { "pass" }) -Shots @($shot) -Output $run.Output `
        -Observed "exit $($run.ExitCode); $(if ($problems) { $problems -join '; ' } else { 'all in place' }); shortcut working directory $($probe.shortcut.workingDirectory), arguments '$($probe.shortcut.arguments)'"
}

Test-Case "C4" "Install a second time" "nothing duplicated" {
    $run = Invoke-GuestScript -Script "& '$powershell' -NoProfile -ExecutionPolicy Bypass -File '$guestUnpacked\install.ps1'"
    $probe = Get-Probe "installed-twice"
    $problems = Test-Installed $probe
    Add-Result -Id "C4" -Title "Install a second time" -Expect "nothing duplicated" `
        -Status $(if ($problems -or $run.ExitCode -ne 0) { "fail" } else { "pass" }) -Output $run.Output `
        -Observed "exit $($run.ExitCode); $(if ($problems) { $problems -join '; ' } else { 'PATH entry once, files in place' })"
}

# ---------------------------------------------------------------- D, command line
# Under the protection S1 set, Windows may refuse to start the programs at
# all: two of the attack surface reduction rules judge a program by how many
# machines have seen it and for how long, and a new unsigned build has no
# such history. Whether it does is the first thing to know, so it is recorded
# here. The rules that refuse the programs are then set to audit, or no case
# after this one could be judged; nothing else is loosened.
$auditedRules = @()
$ruleNames = @{
    "01443614-cd74-433a-b99e-2ecdc07bfc25" = "Block executable files from running unless they meet a prevalence, age, or trusted list criterion"
    "c1db55ab-c21a-4637-bb3f-a12568109d35" = "Use advanced protection against ransomware"
}

# Try to start the installed programs, and when Windows refuses them, set the
# rules that Defender names to audit and try again. Returns Status ("started",
# "audited" when they start only with the rules in audit, "refused"), the
# programs that were refused at first (Programs), the rules now in audit by
# name (Rules), and what was tried (Log). It is asked after each install of a
# build: Defender judges each file by itself, so the build an update starts
# from can be refused where the one under test is not.
function Resolve-GuestRefusal {
    # The command-line program, then the MCP bridge with its input closed,
    # which makes it end at once. Not the app: it would open a window and
    # start a backend. One rule can let one program through and stop
    # another, so "started" needs both.
    $start = (@'
$refused = 0
try { $version = & "{{TARGET}}\codewiki.exe" --version 2>&1; "codewiki.exe: exit $LASTEXITCODE, $version" } catch { $refused += 1; "codewiki.exe: refused, $($_.Exception.Message)" }
try { $null | & "{{TARGET}}\codewiki-mcp.exe" 2>&1 | Out-Null; "codewiki-mcp.exe: exit $LASTEXITCODE" } catch { $refused += 1; "codewiki-mcp.exe: refused, $($_.Exception.Message)" }
if ($refused) { "start: refused, $refused of 2 programs" } else { "start: exit 0" }
$global:LASTEXITCODE = 0
'@).Replace("{{TARGET}}", $guestTarget)
    $blocks = @'
$events = @(Get-WinEvent -FilterHashtable @{ LogName = "Microsoft-Windows-Windows Defender/Operational"; Id = 1121 } -ErrorAction SilentlyContinue | Where-Object { $_.Message -match "codewiki" })
$rules = @($events | ForEach-Object { if ($_.Message -match "(?m)^\s*ID:\s*([0-9A-Fa-f-]{36})") { $Matches[1].ToLowerInvariant() } } | Sort-Object -Unique)
"rules: $($rules -join ',')"
"blocks: $($events.Count)"
'@
    $first = (Invoke-GuestScript -TimeoutSec 120 -Script $start).Output
    $last = $first
    $log = "first start:`n$first"
    foreach ($round in 1..3) {
        if ($last -match "start: exit 0") { break }
        $found = (Invoke-GuestScript -Background -TimeoutSec 180 -Script $blocks).Output
        $rules = if ($found -match "(?m)^rules: (\S+)") { @($Matches[1] -split ",") } else { @() }
        $log += "`n--- Defender, round ${round}:`n$found"
        if (-not $rules) { break }
        # The two rules that judge a program by its history go together:
        # which of them speaks first differs from start to start, and the
        # app, which is not started here, meets both.
        $rules = @(@($rules) + @($ruleNames.Keys) | Sort-Object -Unique)
        $script:auditedRules = @(@($auditedRules) + @($rules) | Sort-Object -Unique)
        $relaxed = Invoke-GuestExe -Background -TimeoutSec 300 -Exe $powershell -Arguments "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
            "-File", "$($script:Vm.GuestDir)\harden.ps1", "-Audit", ($auditedRules -join ",")
        Set-Content (Join-Path $logs "hardening-after-audit.json") $relaxed.Stdout
        Wait-For 5
        $last = (Invoke-GuestScript -TimeoutSec 120 -Script $start).Output
        $log += "`n--- start after setting to audit: $($rules -join ', ')`n$last"
    }
    [pscustomobject]@{
        Status   = if ($first -match "start: exit 0") { "started" } elseif ($last -match "start: exit 0") { "audited" } else { "refused" }
        Programs = @(($first -split "`n") | Where-Object { $_ -match '^codewiki\S*: refused,' } | ForEach-Object { ($_ -split ':')[0] })
        Rules    = @($auditedRules | ForEach-Object { if ($ruleNames[$_]) { "'$($ruleNames[$_])' ($_)" } else { $_ } })
        Log      = $log
    }
}

if ($Hardened) {
    Test-Case "S3" "The programs start under the protection" "recorded: whether Windows lets the programs start" {
        $tried = Resolve-GuestRefusal
        $status = @{ started = "pass"; audited = "note"; refused = "fail" }[$tried.Status]
        $observed = switch ($tried.Status) {
            "started" { "codewiki.exe and codewiki-mcp.exe started with every rule on block" }
            "audited" { "Windows refused to start CodeWiki's programs ($($tried.Programs -join ', ')): 'Access is denied'. Set to audit so that the run can go on: $($tried.Rules -join '; '). With that the programs start. These rules are off unless an administrator turns them on; where they are on, a build that Microsoft's cloud has not known for a while does not run" }
            default { "Windows refuses to start CodeWiki's programs, and setting the rules that Defender names to audit did not change it; the cases after this cannot be judged. See the output" }
        }
        Add-Result -Id "S3" -Title "The programs start under the protection" -Expect "recorded" -Status $status -Output $tried.Log -Observed $observed
    }
}

Test-Case "D1" "codewiki through PATH" "a new process finds codewiki and it runs" {
    $run = Invoke-GuestScript -TimeoutSec 90 -Script @'
"where: " + ((& where.exe codewiki 2>&1) -join " | ")
& cmd.exe /c "codewiki --version"
"version exit: $LASTEXITCODE"
& cmd.exe /c "codewiki --help" | Select-Object -First 12
'@
    $shot = Shot "cli"
    $ok = $run.Completed -and $run.Output -match "version exit: 0"
    Add-Result -Id "D1" -Title "codewiki through PATH" -Expect "found on PATH, exit 0" -Status $(if ($ok) { "pass" } else { "fail" }) `
        -Shots @($shot) -Output $run.Output -Observed $(if ($ok) { "codewiki found on PATH and ran" } else { "did not run; see output and screenshot" })
}

$noteName = $null
Test-Case "D2" "Generate a wiki without git" "init and update succeed; a second update changes no note" {
    $run = Invoke-GuestScript -TimeoutSec 600 -Script @"
Set-Location '$guestRepo'
& codewiki init 2>&1
"init exit: `$LASTEXITCODE"
& codewiki update 2>&1
"update exit: `$LASTEXITCODE"
`$before = Get-ChildItem obsidian -Recurse -File -Filter *.md | Get-FileHash
& codewiki update 2>&1
"second update exit: `$LASTEXITCODE"
`$after = Get-ChildItem obsidian -Recurse -File -Filter *.md | Get-FileHash
"notes: `$(@(`$after).Count)"
"changed by second update: `$(@(Compare-Object @(`$before) @(`$after) -Property Path, Hash).Count)"
"first note: `$((Get-ChildItem obsidian -Recurse -File -Filter *.md | Sort-Object Length -Descending | Select-Object -First 1).BaseName)"
`$global:LASTEXITCODE = 0
"@
    if ($run.Output -match "first note: (.+)") { $script:noteName = $Matches[1].Trim() }
    $notes = if ($run.Output -match "notes: (\d+)") { [int]$Matches[1] } else { 0 }
    $ok = $run.Output -match "init exit: 0" -and $run.Output -match "(?m)^update exit: 0" -and
        $run.Output -match "second update exit: 0" -and $run.Output -match "changed by second update: 0" -and $notes -gt 0
    Add-Result -Id "D2" -Title "Generate a wiki without git" -Expect "init and update succeed; second update changes nothing" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output -Observed "$notes notes written; see output for exit codes"
}

# ---------------------------------------------------------------- E, first launch
# Ask the guest for a quick probe and wait up to the given time for an answer.
# Returns null without one: while the app draws its moving backdrop on the
# processor, the guest can be too busy to open a command session.
function Get-ProbeWhenAble {
    param([string]$Name, [int]$TimeoutSec = 120)
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        try { return Get-Probe $Name -Quick } catch { Start-Sleep -Seconds 5 }
    }
    $null
}

# The repositories the backend has registered, from a probe.
function Get-Registered {
    param($Probe)
    $text = "$($Probe.backend.repos)".Trim()
    if ($text.StartsWith("[")) { @($text | ConvertFrom-Json) } else { @() }
}

# The repository the app shows, from what its log says since it last started.
# The window's own title is always "CodeWiki"; the name is drawn in the app.
function Get-OpenedRepo {
    $run = Invoke-GuestScript -Script "Get-Content `"`$env:LOCALAPPDATA\CodeWiki\logs\app.log`" -ErrorAction SilentlyContinue | Select-String 'started with|repository: ' | ForEach-Object { `$_.Line }"
    $lines = @($run.Output -split "`n" | Where-Object { $_.Trim() })
    $start = -1
    for ($i = 0; $i -lt $lines.Count; $i++) { if ($lines[$i] -match "started with") { $start = $i } }
    $opened = @($lines | Select-Object -Skip ([math]::Max($start, 0)) | Where-Object { $_ -match "repository: (\S+)" } | ForEach-Object { $Matches[1] })
    if ($start -ge 0 -and $opened) { $opened[-1] } else { "" }
}

$idleLoad = $null
# True once P1 finds the idle app keeping the guest's processors busy.
$starved = $false
$firstBackendPid = $null
$firstProbe = $null
$closedProbe = $null
$secondProbe = $null
# Nothing is registered yet and the shortcut starts in the install folder, so
# the app must show Home, not take a folder as the repository.
Test-Case "E1" "Start the app from the Start menu" "a CodeWiki window opens on Home; no folder is registered" {
    $script:idleLoad = Wait-GuestIdle
    $search = Start-FromStartMenu
    $shot = Shot "first-launch"
    $probe = Get-ProbeWhenAble "first-launch"
    $script:firstProbe = $probe
    if (-not $probe) {
        Add-Result -Id "E1" -Title "Start the app from the Start menu" -Expect "a CodeWiki window opens" -Status note -Shots @($search, $shot) `
            -Observed "the guest answered no command for two minutes while the app was open (see P1); whether a window opened is read from the screenshot"
        return
    }
    $app = Get-App $probe
    $window = @($app | Where-Object { $_.hasWindow })
    $backend = Get-Backend $probe
    $script:firstBackendPid = $probe.backend.pid
    $registered = @(Get-Registered $probe)
    Add-Result -Id "E1" -Title "Start the app from the Start menu" -Expect "a window on Home; no folder registered" `
        -Status $(if ($window -and $registered.Count -eq 0) { "pass" } else { "fail" }) -Shots @($search, $shot) `
        -Observed "codewiki-app processes: $($app.Count), with a window: $($window.Count), title '$($window.title -join ',')'; backend processes: $($backend.Count), backend has its own window: $(@($backend | Where-Object { $_.hasWindow }).Count -gt 0), console hosts under CodeWiki programs: $($probe.consoles.Count); repositories registered: $($probe.backend.repos)"
}

Test-Case "P1" "Idle processor use, motion as installed" "an idle window leaves the processors idle" {
    # Measured by the host: the guest may be too busy to measure itself.
    $load = Get-GuestProcessorLoad -Seconds 20
    $script:starved = $load -ge 90
    $shot = Shot "idle-default-motion"
    # Which Direct3D driver the app loaded: d3d10warp.dll draws on the
    # processor, a vendor's DLL (nv*, ati*, ig*) on a graphics card.
    $modules = "not asked, the guest is too busy"
    if (-not $starved) {
        $modules = (Invoke-GuestScript -TimeoutSec 60 -Script '((Get-Process codewiki-app).Modules | Where-Object { $_.ModuleName -match "^(d3d10warp|nv|ati|amd|ig)" } | ForEach-Object ModuleName) -join ", "').Output
    }
    Add-Result -Id "P1" -Title "Idle processor use, motion as installed" -Expect "an idle window leaves the processors idle" `
        -Status $(if ($load -le $idleLoad + 15) { "pass" } else { "fail" }) -Shots @($shot) `
        -Observed "the guest's processors were $load percent busy over 20 s with the app's window open and nothing happening; $idleLoad percent just before the app started; graphics drivers loaded by the app: $modules"
}

# The window from the Start menu has the keyboard focus. The palette's command
# opens Windows's folder dialog, which takes a typed path.
Test-Case "E3" "Open a folder from Home" "Open folder… asks for a folder, registers it and opens it" {
    if ($starved) { throw "the guest is too busy to be driven (P1)" }
    Send-Keys "ctrl+k"; Wait-For 2
    Send-Text "open folder"; Wait-For 2
    $a = Shot "palette-open-folder"
    Send-Keys "enter"; Wait-For 5
    $b = Shot "folder-dialog"
    Send-Text $guestRepo; Wait-For 1
    Send-Keys "enter"; Wait-For 10
    $c = Shot "folder-chosen"
    $probe = Get-Probe "folder-opened" -Quick
    if (-not (Get-Registered $probe | Where-Object { $_.name -eq "cw-core" })) {
        # The dialog may first have gone into the folder; Enter then picks it.
        Send-Keys "enter"; Wait-For 10
        $c = Shot "folder-chosen-again"
        $probe = Get-Probe "folder-opened-again" -Quick
    }
    $registered = @(Get-Registered $probe)
    $opened = Get-OpenedRepo
    $ok = [bool]($registered | Where-Object { $_.name -eq "cw-core" }) -and $opened -eq "cw-core"
    Add-Result -Id "E3" -Title "Open a folder from Home" -Expect "cw-core registered and shown" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($a, $b, $c) `
        -Observed "palette: 'open folder', Enter; typed $guestRepo, Enter; registered: $(($registered | ForEach-Object { $_.name }) -join ', '); the app's log says it opened '$opened'"
}

# ---------------------------------------------------------------- TR, the tray
# Closing the window hides it: the app stays in the tray, with its backend,
# until Quit. The first time, Windows shows the app's one notification. No
# app has been ended by force yet, so there is one tray icon and no leftover
# of a dead one, which Windows keeps until the pointer passes over it.
$windowBeforeHide = $null
$hiddenApp = $null
Test-Case "H1" "Close the window" "Alt+F4 hides it; the app and its backend stay; a notification says so, once" {
    $script:windowBeforeHide = Get-GuestWindowInfo "codewiki-app"
    Send-Keys "alt+f4"
    # The notification is on screen for a few seconds only.
    Wait-For 3
    $closed = Shot "after-alt-f4"
    $probe = Get-ProbeWhenAble "window-hidden" -TimeoutSec 240
    if (-not $probe) { throw "the guest answered no command for four minutes after Alt+F4" }
    $script:closedProbe = $probe
    $app = @(Get-App $probe)
    $script:hiddenApp = $app | Select-Object -First 1
    $windows = @($app | Where-Object { $_.hasWindow }).Count
    $sameBackend = [bool]($probe.backend -and $probe.backend.pidAlive -and $probe.backend.pid -eq $firstBackendPid)
    $ok = $app.Count -eq 1 -and $windows -eq 0 -and $sameBackend -and $probe.registry.trayHint
    Add-Result -Id "H1" -Title "Close the window" -Expect "one app process with no window; the same backend; tray_hint.txt written" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($closed) `
        -Observed "after Alt+F4: app processes $($app.Count), with a window $windows; backend pid $($probe.backend.pid) alive $($probe.backend.pidAlive) (it was $firstBackendPid before); tray_hint.txt written: $($probe.registry.trayHint); Windows has the app's name for notifications: $($probe.registry.appIdentity); the screenshot must show the notification 'CodeWiki is still running'"
}

Test-Case "TR1" "The tray icon says what the app is doing" "its tooltip names the repository, the state and the MCP status" {
    $icons = @(Get-GuestTrayIcons)
    $name = "$(($icons | Select-Object -First 1).Name)"
    $ok = $icons.Count -eq 1 -and $name -like "*CodeWiki - cw-core - Idle - MCP: no calls yet*"
    Add-Result -Id "TR1" -Title "The tray icon says what the app is doing" -Expect "one icon; tooltip 'CodeWiki - cw-core - Idle - MCP: no calls yet'" `
        -Status $(if ($ok) { "pass" } else { "fail" }) `
        -Observed "tray icons named CodeWiki: $($icons.Count); tooltip: '$name'; held by: $(($icons | ForEach-Object { $_.Class }) -join ', ') (Shell_TrayWnd is the taskbar, the other the flyout of hidden icons)"
}

# A tool call through the bridge, as an agent makes it. The bridge names
# itself to the backend; the app asks the backend every five seconds.
Test-Case "TR2" "The tray icon shows an agent's call" "after a tool call through the MCP bridge the tooltip says 'MCP: active'" {
    $run = Invoke-GuestScript -TimeoutSec 180 -Script @"
`$start = New-Object System.Diagnostics.ProcessStartInfo '$guestTarget\codewiki-mcp.exe'
`$start.RedirectStandardInput = `$true; `$start.RedirectStandardOutput = `$true; `$start.RedirectStandardError = `$true
`$start.UseShellExecute = `$false; `$start.CreateNoWindow = `$true; `$start.WorkingDirectory = '$guestRepo'
`$bridge = [System.Diagnostics.Process]::Start(`$start)
foreach (`$request in @(
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"vm-test","version":"0"}}}'
    '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_overview","arguments":{}}}'
)) { `$bridge.StandardInput.WriteLine(`$request) }
`$bridge.StandardInput.Flush()
# The bridge ends when its input does, so the input stays open until it answers.
`$answered = 'no answer'
`$deadline = (Get-Date).AddSeconds(90)
while ((Get-Date) -lt `$deadline) {
    `$reading = `$bridge.StandardOutput.ReadLineAsync()
    if (-not `$reading.Wait(90000) -or `$null -eq `$reading.Result) { break }
    try { `$message = `$reading.Result | ConvertFrom-Json } catch { continue }
    if (`$message.id -eq 2) { `$answered = "answered; an error: `$([bool]`$message.error -or [bool]`$message.result.isError)"; break }
}
`$bridge.StandardInput.Close()
if (-not `$bridge.WaitForExit(10000)) { `$bridge.Kill() }
"bridge: `$answered"
`$global:LASTEXITCODE = 0
"@
    Wait-For 8
    $name = "$((Get-GuestTrayIcons | Select-Object -First 1).Name)"
    $answered = $run.Output -match "bridge: answered; an error: False"
    Add-Result -Id "TR2" -Title "The tray icon shows an agent's call" -Expect "the bridge's get_overview is answered; the tooltip then ends in 'MCP: active'" `
        -Status $(if ($answered -and $name -like "*MCP: active*") { "pass" } else { "fail" }) -Output $run.Output `
        -Observed "$(($run.Output -split "`n" | Where-Object { $_ -match 'bridge:' }) -join '; '); tooltip 8 s later: '$name'"
}

Test-Case "TR3" "A click on the tray icon shows the window" "the hidden window is back, in front, where and as large as it was" {
    $icon = Send-GuestTrayClick -Button left
    Wait-For 4
    $info = Get-GuestWindowInfo "codewiki-app"
    $shot = Shot "tray-click-window-back"
    $probe = Get-Probe "tray-click" -Quick
    $app = @(Get-App $probe)
    $sameApp = $hiddenApp -and $app.Count -eq 1 -and $app[0].id -eq $hiddenApp.id
    $sameWindow = $info -and $windowBeforeHide -and $info.Left -eq $windowBeforeHide.Left -and $info.Top -eq $windowBeforeHide.Top -and
        $info.Width -eq $windowBeforeHide.Width -and $info.Height -eq $windowBeforeHide.Height
    $ok = $sameApp -and $sameWindow -and -not $info.Minimized -and $info.Foreground
    Add-Result -Id "TR3" -Title "A click on the tray icon shows the window" -Expect "the same app; its window as before, in front" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($shot) `
        -Observed "clicked '$($icon.Name)' at ($($icon.X),$($icon.Y)) in $($icon.Class); app processes $($app.Count), the one that was hidden: $([bool]$sameApp); window: $(if ($info) { "client area $($info.Width)x$($info.Height) at ($($info.Left),$($info.Top)), in front $($info.Foreground)" } else { 'none' }); before it was hidden: $(if ($windowBeforeHide) { "$($windowBeforeHide.Width)x$($windowBeforeHide.Height) at ($($windowBeforeHide.Left),$($windowBeforeHide.Top))" } else { 'not read' })"
}

# The menu is the app's own pop-up. It cannot be asked for its rows: it closes
# when another program starts, and every question to the guest starts one. So
# it is photographed (the host does that alone) and judged from the picture.
Test-Case "TR4" "The tray menu" "a right click opens it: what the app is doing, then the commands, Quit last" {
    $icon = Send-GuestTrayClick -Button right
    Wait-For 2
    $shot = Shot "tray-menu"
    Send-Keys "esc"; Wait-For 1
    Add-Result -Id "TR4" -Title "The tray menu" -Expect "ten rows in order; the first four greyed" -Status note -Shots @($shot) `
        -Observed "right-clicked the icon at ($($icon.X),$($icon.Y)); read the screenshot: the menu must show, greyed, the repository (cw-core), the state (Idle), the MCP line and the version, then 'Open CodeWiki', 'Update wiki now', 'Recent repositories', 'Settings', 'Check for updates' and, last, 'Quit'"
}

# Opening the icon's menu gives the keyboard to the icon's own window, which
# never shows, and it keeps it after the menu closes. Alt+F4 then went to
# that window, Windows destroyed it, and the icon was gone while the app ran
# on. The icon must outlive that. A dead icon stays drawn until the pointer
# moves over it, so the pointer is put on it before the icons are counted.
Test-Case "TR7" "Alt+F4 after the tray menu leaves the icon" "the menu is opened and closed with Escape, then Alt+F4: the icon is still there and the app runs" {
    $icon = Send-GuestTrayClick -Button right
    Wait-For 2
    Send-Keys "esc"; Wait-For 1
    Send-Keys "alt+f4"; Wait-For 3
    $again = Get-GuestTrayIcon
    if ($again) { Move-Pointer -X $again.X -Y $again.Y; Wait-For 2 }
    $left = @(Get-GuestTrayIcons).Count
    $shot = Shot "tray-after-menu-alt-f4"
    $probe = Get-Probe "tray-alt-f4" -Quick
    $apps = (Get-App $probe).Count
    Send-Keys "esc"; Wait-For 1
    Add-Result -Id "TR7" -Title "Alt+F4 after the tray menu leaves the icon" -Expect "one tray icon, one app" `
        -Status $(if ($left -eq 1 -and $apps -eq 1) { "pass" } else { "fail" }) -Shots @($shot) `
        -Observed "right-clicked the icon at ($($icon.X),$($icon.Y)), Escape, Alt+F4, pointer on the icon; tray icons named CodeWiki: $left; app processes: $apps"
}

Test-Case "SI1" "Starting the app again shows the running one" "with the window hidden, the Start menu brings the same app back; no second app, no second backend" {
    # The keyboard may be with the tray or its flyout: the window takes it
    # back first, as a click on it would.
    [void](Show-GuestWindow -ProcessName "codewiki-app")
    Send-Keys "alt+f4"
    Wait-For 4
    $hidden = Get-GuestWindowInfo "codewiki-app"
    $search = Start-FromStartMenu
    $shot = Shot "second-start-same-app"
    $probe = Get-Probe "second-start" -Quick
    $app = @(Get-App $probe)
    $info = Get-GuestWindowInfo "codewiki-app"
    $sameApp = $hiddenApp -and $app.Count -eq 1 -and $app[0].id -eq $hiddenApp.id
    $sameBackend = [bool]($probe.backend -and $probe.backend.pid -eq $firstBackendPid) -and (Get-Backend $probe).Count -eq 1
    $ok = -not $hidden -and $sameApp -and $sameBackend -and $info -and -not $info.Minimized
    Add-Result -Id "SI1" -Title "Starting the app again shows the running one" -Expect "one app, the same process; one backend, the same; its window on screen" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($search, $shot) `
        -Observed "Alt+F4 left a window: $([bool]$hidden); after the Start menu: app processes $($app.Count), the same process: $([bool]$sameApp); backend processes $((Get-Backend $probe).Count), pid $($probe.backend.pid) (it was $firstBackendPid); window on screen: $([bool]$info), in front: $($info.Foreground)"
}

Test-Case "TR5" "Quit from the tray menu" "the app and its backend end, and the icon goes" {
    $icon = Send-GuestTrayClick -Button right
    Wait-For 2
    $menu = Shot "tray-menu-before-quit"
    # Up from a menu that has just opened is its last row, and that is Quit.
    Send-Keys "up", "enter"
    $deadline = (Get-Date).AddSeconds(40)
    do {
        $probe = Get-Probe "tray-quit" -Quick
        $apps = (Get-App $probe).Count
        $backends = (Get-Backend $probe).Count
        if ($apps + $backends -gt 0) { Start-Sleep -Seconds 3 }
    } while ($apps + $backends -gt 0 -and (Get-Date) -lt $deadline)
    $shot = Shot "after-tray-quit"
    $icons = @(Get-GuestTrayIcons).Count
    $ok = $apps -eq 0 -and $backends -eq 0 -and -not $probe.backend -and $icons -eq 0
    Add-Result -Id "TR5" -Title "Quit from the tray menu" -Expect "no codewiki-app, no codewiki process, no backend.json, no tray icon" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($menu, $shot) `
        -Observed "right-clicked the icon at ($($icon.X),$($icon.Y)), pressed Up and Enter for the menu's last row, Quit; left running: $apps app, $backends backend; backend.json left: $([bool]$probe.backend); tray icons named CodeWiki left: $icons"
    if ($apps) { Stop-GuestApp }
}

# After a real Quit the next start brings up a new backend, and, the Start
# menu giving no folder, the app reopens the one opened in E3.
Test-Case "E4" "The last repository opens again" "started from the Start menu after Quit: a new backend, and the app shows cw-core" {
    # Without a graphics processor the moving backdrop takes every core (case
    # P1), and the guest then answers too slowly to be driven. In that case
    # the app runs with motion reduced from here on; otherwise as installed.
    if ($starved) { Set-GuestMotion "reduced" }
    $search = Start-FromStartMenu
    $again = Shot "second-launch"
    $probe2 = Get-Probe "second-launch" -Quick
    $script:secondProbe = $probe2
    $new = [bool]($probe2.backend -and $probe2.backend.pidAlive -and $probe2.backend.pid -ne $firstBackendPid)
    $opened = Get-OpenedRepo
    $ok = $new -and (Get-App $probe2).Count -eq 1 -and $opened -eq "cw-core"
    Add-Result -Id "E4" -Title "The last repository opens again" -Expect "a new backend; one app; it shows cw-core" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($search, $again) `
        -Observed "backend pid $($probe2.backend.pid) alive $($probe2.backend.pidAlive) (the one that ended was $firstBackendPid); app processes $((Get-App $probe2).Count); the app's log says it opened '$(if ($opened) { $opened } else { 'nothing' })' since it started"
}

Test-Case "E2" "Backend after first launch" "backend.json written, health answers, token required" {
    $source = if ($firstProbe) { "while the app was open" } else { "after the app was started again" }
    $backend = if ($firstProbe) { $firstProbe.backend } else { $secondProbe.backend }
    if (-not $backend) { throw "no probe has backend facts" }
    $ok = $backend.health -eq 200 -and $backend.reposWithoutToken -eq 401 -and $backend.pidAlive
    Add-Result -Id "E2" -Title "Backend after first launch" -Expect "health 200, 401 without token" -Status $(if ($ok) { "pass" } else { "fail" }) `
        -Observed "read ${source}: url $($backend.url), pid $($backend.pid) alive $($backend.pidAlive), health $($backend.health), /v1/repos without token $($backend.reposWithoutToken), repositories registered: $($backend.repos)"
}

Test-Case "P2" "Idle processor use after a restart of the app" "recorded, to compare with P1" {
    $load = Get-GuestProcessorLoad -Seconds 10
    $used = Get-AppCpu
    $motion = if ($starved) { "set to reduced" } else { "as installed" }
    Add-Result -Id "P2" -Title "Idle processor use after a restart of the app" -Expect "recorded, to compare with P1" -Status note `
        -Observed "with motion $motion the guest's processors were $load percent busy over 10 s, and the app used $used s of processor time in 10 s"
    Stop-GuestApp
}
# ---------------------------------------------------------------- W, the window
# The title bar is the app's own. Where its three buttons are, measured from
# the top right corner of the client area at 100% scaling: the bar is 34
# pixels high and each button 34 wide, close outermost (gpui-component's
# title_bar.rs).
function Get-TitleBarPoint {
    param($Info, [ValidateSet("close", "maximize", "minimize", "caption")][string]$What)
    $y = $Info.Top + [int](17 * $Info.Scale)
    if ($What -eq "caption") { return @(($Info.Left + [int]($Info.Width / 2)), $y) }
    $fromRight = @{ close = 17; maximize = 51; minimize = 85 }[$What]
    @(($Info.Right - [int]($fromRight * $Info.Scale)), $y)
}

function Format-Window {
    param($Info)
    if (-not $Info) { return "no window" }
    "client area $($Info.Width)x$($Info.Height) at ($($Info.Left),$($Info.Top)), minimized $($Info.Minimized), maximized $($Info.Maximized)"
}

# Press a part of the title bar after the pointer has rested on it, and say
# where. A button that answers only right after the pointer moved fails here.
function Send-TitleBarClick {
    param([string]$What, [switch]$Double)
    $info = Get-GuestWindowInfo "codewiki-app"
    if (-not $info -or $info.Minimized) { throw "the app has no window on screen to click in: $(Format-Window $info)" }
    $x, $y = Get-TitleBarPoint $info $What
    Send-RestedClick -X $x -Y $y -Double:$Double
    Wait-For 3
    "($x,$y)"
}

$windowAtStart = $null
Test-Case "W0" "The window opens clear of the taskbar" "the whole window is inside the part of the screen the taskbar leaves free" {
    if ($starved) { throw "the guest is too busy to be driven (P1)" }
    Start-GuestApp -Arguments $guestRepo, "_HOME"
    Wait-For 12
    $info = Get-GuestWindowInfo "codewiki-app"
    if (-not $info) { throw "the app has no window" }
    $script:windowAtStart = $info
    # A window's unseen border may reach a few pixels past an edge.
    $slack = [int](8 * $info.Scale)
    $inside = $info.Outer.Top -ge $info.Free.Top - $slack -and $info.Outer.Bottom -le $info.Free.Bottom + $slack -and
        $info.Outer.Left -ge $info.Free.Left - $slack -and $info.Outer.Right -le $info.Free.Right + $slack
    Add-Result -Id "W0" -Title "The window opens clear of the taskbar" -Expect "the whole window is inside the free part of the screen" `
        -Status $(if ($inside -and -not $info.Maximized) { "pass" } else { "fail" }) -Shots @(Shot "window-at-start") `
        -Observed "screen $($info.Screen.Width)x$($info.Screen.Height) at $([int]($info.Scale * 100)) percent; free of the taskbar: ($($info.Free.Left),$($info.Free.Top)) to ($($info.Free.Right),$($info.Free.Bottom)); the window: ($($info.Outer.Left),$($info.Outer.Top)) to ($($info.Outer.Right),$($info.Outer.Bottom)); $(Format-Window $info)"
}

Test-Case "W1" "Minimize button" "a click on it, after the pointer has rested there, minimizes the window" {
    $at = Send-TitleBarClick "minimize"
    $info = Get-GuestWindowInfo "codewiki-app"
    Add-Result -Id "W1" -Title "Minimize button" -Expect "the window is minimized" `
        -Status $(if ($info -and $info.Minimized) { "pass" } else { "fail" }) -Shots @(Shot "window-minimized") `
        -Observed "pointer rested 1.5 s at $at, then one click; after it: $(Format-Window $info)"
}

Test-Case "W2" "Back from minimized" "Alt+Tab brings the window back at its old size, with its page drawn" {
    Send-Keys "alt+tab"; Wait-For 4
    $info = Get-GuestWindowInfo "codewiki-app"
    if ($info -and $info.Minimized) {
        Send-Keys "alt+tab"; Wait-For 4
        $info = Get-GuestWindowInfo "codewiki-app"
    }
    $same = $info -and -not $info.Minimized -and $windowAtStart -and $info.Width -eq $windowAtStart.Width -and $info.Height -eq $windowAtStart.Height
    Add-Result -Id "W2" -Title "Back from minimized" -Expect "the window is back at its old size; read the screenshot for the page" `
        -Status $(if ($same) { "pass" } else { "fail" }) -Shots @(Shot "window-restored") `
        -Observed "after Alt+Tab: $(Format-Window $info); at the start: $(Format-Window $windowAtStart)"
}

Test-Case "W3" "Maximize button" "a click on it fills the free part of the screen and leaves the taskbar in view" {
    $at = Send-TitleBarClick "maximize"
    $info = Get-GuestWindowInfo "codewiki-app"
    $fills = $info -and $info.Maximized -and $info.Width -eq ($info.Free.Right - $info.Free.Left) -and $info.Bottom -le $info.Free.Bottom
    Add-Result -Id "W3" -Title "Maximize button" -Expect "maximized, as wide as the screen, not over the taskbar" `
        -Status $(if ($fills) { "pass" } else { "fail" }) -Shots @(Shot "window-maximized") `
        -Observed "pointer rested 1.5 s at $at, then one click; after it: $(Format-Window $info); free of the taskbar: ($($info.Free.Left),$($info.Free.Top)) to ($($info.Free.Right),$($info.Free.Bottom))"
}

Test-Case "W4" "Restore button" "a click on the same button puts the window back at its old size" {
    $at = Send-TitleBarClick "maximize"
    $info = Get-GuestWindowInfo "codewiki-app"
    $back = $info -and -not $info.Maximized -and $windowAtStart -and $info.Width -eq $windowAtStart.Width -and $info.Height -eq $windowAtStart.Height
    Add-Result -Id "W4" -Title "Restore button" -Expect "not maximized, old size" `
        -Status $(if ($back) { "pass" } else { "fail" }) -Shots @(Shot "window-unmaximized") `
        -Observed "pointer rested 1.5 s at $at, then one click; after it: $(Format-Window $info)"
}

Test-Case "W5" "Double-click on the title bar" "it maximizes the window, and a second one restores it" {
    $at = Send-TitleBarClick "caption" -Double
    $first = Get-GuestWindowInfo "codewiki-app"
    $shot = Shot "window-double-click"
    $at2 = Send-TitleBarClick "caption" -Double
    $second = Get-GuestWindowInfo "codewiki-app"
    $ok = $first -and $first.Maximized -and $second -and -not $second.Maximized
    Add-Result -Id "W5" -Title "Double-click on the title bar" -Expect "maximized, then restored" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($shot) `
        -Observed "double-click at $at, after a rest: $(Format-Window $first); again at ${at2}: $(Format-Window $second)"
}

Test-Case "W6" "Close button" "a click on it hides the window; the app and its backend stay" {
    $at = Send-TitleBarClick "close"
    $shot = Shot "window-closed"
    $probe = Get-Probe "window-closed" -Quick
    $app = @(Get-App $probe)
    $windows = @($app | Where-Object { $_.hasWindow }).Count
    $backendAlive = [bool]($probe.backend -and $probe.backend.pidAlive)
    Add-Result -Id "W6" -Title "Close button" -Expect "one codewiki-app process with no window; its backend alive" `
        -Status $(if ($app.Count -eq 1 -and $windows -eq 0 -and $backendAlive) { "pass" } else { "fail" }) -Shots @($shot) `
        -Observed "pointer rested 1.5 s at $at, then one click; app processes $($app.Count), with a window $windows; backend pid $($probe.backend.pid) alive $backendAlive"
    Stop-GuestApp
}

# Hiding loses nothing, so the X no longer asks about unsaved edits. Quit
# would lose them, and is refused.
Test-Case "W7" "Unsaved edits, the close button and Quit" "the X hides the window with the edits in it; a second start shows them again; Quit says to save or discard" {
    if (-not $noteName) { throw "D2 produced no note to edit" }
    Start-GuestApp -Arguments $guestRepo, "edit:$noteName"
    Wait-For 15
    Send-Click -X ($appWindow.Left + 520) -Y ($appWindow.Top + 160); Wait-For 1
    Send-Text "unsavedmarker "; Wait-For 3
    $typed = Shot "unsaved-edits-typed"
    $at = Send-TitleBarClick "close"
    $hidden = Get-GuestWindowInfo "codewiki-app"
    # A second start, with nothing to open, hands over to the hidden app,
    # which shows its window as it was.
    Start-GuestApp
    Wait-For 4
    $back = Shot "unsaved-edits-back"
    Send-Keys "ctrl+k"; Wait-For 2
    Send-Text "quit codewiki"; Wait-For 2
    Send-Keys "enter"; Wait-For 4
    $refused = Shot "quit-with-unsaved-edits"
    $info = Get-GuestWindowInfo "codewiki-app"
    $probe = Get-Probe "quit-refused" -Quick
    $apps = (Get-App $probe).Count
    $run = Invoke-GuestScript -Script "(Get-ChildItem '$guestRepo\obsidian' -Recurse -Filter *.md | Select-String -Pattern 'unsavedmarker' -List | ForEach-Object { `$_.Path }) -join ', '"
    $ok = -not $hidden -and $info -and $apps -eq 1 -and -not $run.Output.Trim()
    Add-Result -Id "W7" -Title "Unsaved edits, the close button and Quit" -Expect "hidden by the X; shown again by a second start; still running after Quit; nothing written" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($typed, $back, $refused) `
        -Observed "typed into the editor, then clicked close at ${at}: a window was left $([bool]$hidden); after a second start and palette 'quit codewiki', Enter: $(Format-Window $info), app processes $apps; the typed text in a file: $(if ($run.Output.Trim()) { $run.Output.Trim() } else { 'no' }); the second screenshot must show the typed text and the Save button, the third the message to save or discard"
    Stop-GuestApp
}

# ---------------------------------------------------------------- F, pages
$views = @(
    @{ name = "home"; args = @($guestRepo) }
    @{ name = "wiki-map"; args = @($guestRepo, "_HOME") }
    @{ name = "note"; args = @($guestRepo, $noteName) }
    @{ name = "graph"; args = @($guestRepo, "graph:") }
    @{ name = "health"; args = @($guestRepo, "health:") }
    @{ name = "tests"; args = @($guestRepo, "tests:") }
    @{ name = "settings"; args = @($guestRepo, "settings:") }
    # No "themes:" here: it opens the theme menu, and the click that gives the
    # window the focus closes a menu. T1 and T2 open it by keyboard instead.
)
foreach ($view in $views) {
    $id = "F1-$($view.name)"
    Test-Case $id "Open $($view.name)" "the window opens on that page" {
        if ($view.name -eq "note" -and -not $noteName) { throw "D2 produced no note to open" }
        Start-GuestApp -Arguments $view.args
        Wait-For 15
        $shot = Shot "view-$($view.name)"
        $probe = Get-Probe "view-$($view.name)" -Quick
        $app = Get-App $probe
        $window = @($app | Where-Object { $_.hasWindow })
        $observed = "app processes $($app.Count), with a window $($window.Count), title '$($window.title -join ',')', memory $($app.memoryMb -join ',') MB"
        Add-Result -Id $id -Title "Open $($view.name)" -Expect "the window opens on that page" `
            -Status $(if ($window) { "pass" } else { "fail" }) -Shots @($shot) -Observed $observed
        Stop-GuestApp
    }
}

# ---------------------------------------------------------------- helpers for the cases below
# End the app the way a user does: palette, "Quit CodeWiki", Enter. The app
# and its backend must be gone afterwards; one that stays is ended by force.
# Returns whether Quit ended them by itself. The window must have the focus.
function Exit-GuestApp {
    Send-Keys "ctrl+k"; Wait-For 2
    Send-Text "quit codewiki"; Wait-For 2
    Send-Keys "enter"
    $deadline = (Get-Date).AddSeconds(60)
    do {
        Start-Sleep -Seconds 3
        $probe = Get-Probe "quit" -Quick
        $left = (Get-App $probe).Count + (Get-Backend $probe).Count
    } while ($left -gt 0 -and (Get-Date) -lt $deadline)
    if ($left) { Stop-GuestApp }
    $left -eq 0
}

# The share of a screenshot's pixels (sampled) that differ clearly from
# another's. A diagram viewer over the window changes a large part of it.
function Get-ImageChange {
    param([string]$A, [string]$B)
    Add-Type -AssemblyName System.Drawing
    $first = New-Object System.Drawing.Bitmap $A
    $second = New-Object System.Drawing.Bitmap $B
    try {
        $width = [math]::Min($first.Width, $second.Width)
        $height = [math]::Min($first.Height, $second.Height)
        $changed = 0
        $total = 0
        for ($y = 0; $y -lt $height; $y += 8) {
            for ($x = 0; $x -lt $width; $x += 8) {
                $p = $first.GetPixel($x, $y)
                $q = $second.GetPixel($x, $y)
                if ([math]::Abs($p.R - $q.R) + [math]::Abs($p.G - $q.G) + [math]::Abs($p.B - $q.B) -gt 24) { $changed += 1 }
                $total += 1
            }
        }
        if ($total) { $changed / $total } else { 0 }
    } finally {
        $first.Dispose()
        $second.Dispose()
    }
}

# What a script that talks to the backend in the guest starts with: the
# address and token of the running backend (backend.json) and Invoke-Backend,
# which returns the answer's text, or "HTTP error: ..." with the body of a
# refusal. Get-RepoId finds a registered repository by its name.
$backendPrelude = @'
$ErrorActionPreference = 'Stop'
$info = Get-Content (Join-Path $env:LOCALAPPDATA 'CodeWiki\backend.json') -Raw | ConvertFrom-Json
function Invoke-Backend {
    param([string]$Method, [string]$Path, $Body = $null, [int]$Wait = 120)
    $request = @{ UseBasicParsing = $true; Method = $Method; Uri = ($info.url + $Path); TimeoutSec = $Wait
        Headers = @{ Authorization = ('Bearer ' + $info.token) } }
    if ($null -ne $Body) { $request.Body = ($Body | ConvertTo-Json -Depth 8 -Compress); $request.ContentType = 'application/json' }
    try { (Invoke-WebRequest @request).Content }
    catch {
        $detail = ''
        if ($_.Exception.Response) { try { $detail = (New-Object System.IO.StreamReader($_.Exception.Response.GetResponseStream())).ReadToEnd() } catch { } }
        "HTTP error: $($_.Exception.Message) $detail"
    }
}
function Get-RepoId {
    param([string]$Name)
    # Windows PowerShell 5.1 hands a JSON array on as one object: wrapped in
    # @(...) straight from the pipe, the filter saw the whole list and gave
    # every id. Kept in a variable first, it is taken apart.
    $list = Invoke-Backend GET '/v1/repos' | ConvertFrom-Json
    @($list | Where-Object { $_.name -eq $Name })[0].id
}
'@
function Invoke-GuestBackendScript {
    param([Parameter(Mandatory)][string]$Script, [int]$TimeoutSec = 300)
    Invoke-GuestScript -TimeoutSec $TimeoutSec -Script ($backendPrelude + "`n" + $Script)
}

# ---------------------------------------------------------------- AV1, the agent view
# `agent:<note>` starts the app on the note's agent view, the note as the MCP
# tool read_note gives it to a coding agent (main.rs, open_startup_view).
Test-Case "AV1" "The agent view of a note" "the window opens on the note's agent view, with the page drawn" {
    if (-not $noteName) { throw "D2 produced no note to open" }
    Start-GuestApp -Arguments $guestRepo, "agent:$noteName"
    Wait-For 15
    $shot = Shot "view-agent"
    $probe = Get-Probe "view-agent" -Quick
    $app = Get-App $probe
    $window = @($app | Where-Object { $_.hasWindow })
    Add-Result -Id "AV1" -Title "The agent view of a note" -Expect "the window opens on the note's agent view" `
        -Status $(if ($window) { "pass" } else { "fail" }) -Shots @($shot) `
        -Observed "started on 'agent:$noteName'; app processes $($app.Count), with a window $($window.Count); read the screenshot: the note is drawn, a line above it says what is left out (the walk-through of calls that rules wrote) and stays while the note scrolls, and the toolbar offers no Edit"
    Stop-GuestApp
}

# A page must appear without any input. Start-GuestApp clicks the window to
# give it the focus, which hid this: here the app is started and left alone.
# With motion reduced, a page once stayed a lone "CodeWiki" heading until the
# pointer moved.
foreach ($mode in "reduced", "system") {
    Test-Case "F2-$mode" "A page appears without input, motion $mode" "the health page is drawn with no input at all" {
        if ($starved -and $mode -eq "system") { throw "the guest is too busy to be driven with default motion (P1)" }
        Set-GuestMotion $mode
        Start-GuestExe -Exe "$guestTarget\codewiki-app.exe" -Arguments $guestRepo, "health:"
        Wait-For 25
        $shot = Shot "no-input-health-$mode"
        Add-Result -Id "F2-$mode" -Title "A page appears without input, motion $mode" -Expect "the health table is drawn" -Status note -Shots @($shot) `
            -Observed "started without the focus and given no input for 25 s; read the screenshot: a table of files is the page, a lone 'CodeWiki' heading is the defect"
        Stop-GuestApp
    }
}
# Back to what the rest of the run uses.
Set-GuestMotion $(if ($starved) { "reduced" } else { "system" })

Test-Case "L1" "The app writes a log" "logs\app.log names the graphics adapter" {
    $run = Invoke-GuestScript -Script "Get-Content `"`$env:LOCALAPPDATA\CodeWiki\logs\app.log`" -ErrorAction SilentlyContinue | Select-String 'graphics adapter|Using GPU|ERROR|WARN|panic' | Select-Object -Last 8 | ForEach-Object { `$_.Line }"
    $adapter = if ($run.Output -match "graphics adapter: (.+)") { $Matches[1].Trim() } else { "" }
    $backendLog = (Invoke-GuestScript -Script "Get-Content `"`$env:LOCALAPPDATA\CodeWiki\logs\backend.log`" -ErrorAction SilentlyContinue | Select-Object -First 3").Output
    Add-Result -Id "L1" -Title "The app writes a log" -Expect "logs\app.log names the graphics adapter" `
        -Status $(if ($adapter) { "pass" } else { "fail" }) -Output ($run.Output + "`n--- backend.log`n" + $backendLog) `
        -Observed $(if ($adapter) { "adapter: $adapter" } else { "no adapter line in app.log" })
}

# An installed app asks GitHub for the latest release once a day when it
# starts. It writes update_checked.txt only when GitHub answered.
Test-Case "U1" "Update check against GitHub" "GitHub answers; with nothing newer published, nothing is offered" {
    $run = Invoke-GuestScript -Script @'
"checked: $(Test-Path "$env:APPDATA\CodeWiki\update_checked.txt")"
"switch: $(if (Test-Path "$env:APPDATA\CodeWiki\update_check.txt") { Get-Content "$env:APPDATA\CodeWiki\update_check.txt" -Raw } else { '(not set)' })"
Get-Content "$env:LOCALAPPDATA\CodeWiki\logs\app.log" -ErrorAction SilentlyContinue | Select-String 'update|GitHub|release' | Select-Object -Last 6 | ForEach-Object { $_.Line }
'@
    $checked = $run.Output -match "checked: True"
    Add-Result -Id "U1" -Title "Update check against GitHub" -Expect "update_checked.txt is written: GitHub answered" `
        -Status $(if ($checked) { "pass" } else { "note" }) -Output $run.Output `
        -Observed $(if ($checked) { "GitHub answered the check; the Settings screenshot (F1-settings) shows what the app made of it" } else { "no answer from GitHub was recorded; the guest may have no network. See the output" })
}

# ---------------------------------------------------------------- G, keyboard
Test-Case "G1" "Palette by keyboard" "Ctrl+K opens it, typing filters, Enter runs, Escape closes" {
    # On the wiki map, which has the toolbar and tabs that G2 and N1 use; Home has neither.
    Start-GuestApp -Arguments $guestRepo, "_HOME"
    Wait-For 15
    $a = Shot "keys-before"
    Send-Keys "ctrl+k"; Wait-For 2
    $b = Shot "palette-open"
    Send-Text "health"; Wait-For 2
    $c = Shot "palette-filtered"
    Send-Keys "down", "up", "enter"; Wait-For 5
    $d = Shot "palette-ran"
    Send-Keys "ctrl+k"; Wait-For 2
    Send-Keys "esc"; Wait-For 2
    $e = Shot "palette-closed"
    Add-Result -Id "G1" -Title "Palette by keyboard" -Expect "opens, filters, runs, closes" -Status note -Shots @($a, $b, $c, $d, $e) `
        -Observed "keys sent: Ctrl+K, 'health', Down, Up, Enter, Ctrl+K, Escape; read the screenshots"
}

Test-Case "G2" "Tabs by keyboard" "Ctrl+T opens a tab, Ctrl+Tab switches, Ctrl+W closes" {
    Send-Keys "ctrl+t"; Wait-For 3
    $a = Shot "tab-new"
    Send-Keys "ctrl+tab"; Wait-For 3
    $b = Shot "tab-switched"
    Send-Keys "ctrl+w"; Wait-For 3
    $c = Shot "tab-closed"
    Add-Result -Id "G2" -Title "Tabs by keyboard" -Expect "new tab, switch, close" -Status note -Shots @($a, $b, $c) `
        -Observed "keys sent: Ctrl+T, Ctrl+Tab, Ctrl+W; read the screenshots"
}

Test-Case "N1" "Toolbar by mouse" "a click on the Ctrl+K button opens the palette; a click on + opens a tab" {
    # Distances from the window's top right corner, read off the screenshot
    # "keys-before": the Ctrl+K button and the + at the end of the tab bar.
    # (The window's own numbers include an unseen 8 pixel border.)
    $paletteX = $appWindow.Right - 376; $paletteY = $appWindow.Top + 61
    $tabX = $appWindow.Right - 33; $tabY = $appWindow.Top + 112
    Send-Click -X $paletteX -Y $paletteY; Wait-For 2
    $a = Shot "click-palette-button"
    Send-Keys "esc"; Wait-For 2
    Send-Click -X $tabX -Y $tabY; Wait-For 3
    $b = Shot "click-new-tab"
    Send-Keys "ctrl+w"; Wait-For 2
    $c = Shot "click-tab-closed"
    Add-Result -Id "N1" -Title "Toolbar by mouse" -Expect "the palette opens; a tab opens" -Status note -Shots @($a, $b, $c) `
        -Observed "clicked the Ctrl+K button at ($paletteX,$paletteY), pressed Escape, clicked + at ($tabX,$tabY), pressed Ctrl+W; read the screenshots"
}
# Tab must walk through the controls, the sidebar's first. It once went to the
# search box and stayed there: no other control was a tab stop.
Test-Case "G4" "Focus by keyboard" "Tab moves a visible focus from control to control; Enter and Space press the focused control" {
    Send-Keys "tab"; Wait-For 1
    Send-Keys "tab"; Wait-For 1
    Send-Keys "tab"; Wait-For 2
    $a = Shot "focus-tab-3"
    Send-Keys "tab"; Wait-For 1
    Send-Keys "tab"; Wait-For 1
    Send-Keys "tab"; Wait-For 2
    $b = Shot "focus-tab-6"
    Send-Keys "enter"; Wait-For 4
    $c = Shot "focus-enter"
    Send-Keys "tab"; Send-Keys "space"; Wait-For 4
    $d = Shot "focus-space"
    Add-Result -Id "G4" -Title "Focus by keyboard" -Expect "focus moves and controls fire" -Status note -Shots @($a, $b, $c, $d) `
        -Observed "keys sent: Tab x3, Tab x3, Enter, Tab, Space; read the screenshots: the highlight must be on a different control after three and after six Tabs, and Enter must open what was focused"
    Stop-GuestApp
}

# What the app remembers of its two side panes: "hidden", "shown", or nothing
# when the pane was never toggled.
function Get-GuestPanes {
    $run = Invoke-GuestScript -Script @'
foreach ($pane in "sidebar", "details") {
    $file = "$env:APPDATA\CodeWiki\$pane.txt"
    "$pane=$(if (Test-Path $file) { (Get-Content $file -Raw).Trim() } else { '(not set)' })"
}
'@
    @{
        Sidebar = if ($run.Output -match "(?m)^sidebar=(.+)$") { $Matches[1].Trim() } else { "" }
        Details = if ($run.Output -match "(?m)^details=(.+)$") { $Matches[1].Trim() } else { "" }
    }
}

# The sidebar and the details pane each hide and show. The shortcuts hide
# them; the buttons at the two ends of the toolbar show them again, after a
# restart that must have kept both hidden. The buttons are 28 pixels wide and
# sit 8 pixels inside the toolbar (`toolbar` in main.rs); with the sidebar
# hidden the toolbar starts at the window's left edge.
Test-Case "PN1" "Hide and show the side panes" "Ctrl+B and Ctrl+Alt+B hide the panes, a restart keeps them hidden, and the toolbar's two end buttons show them again" {
    Start-GuestApp -Arguments $guestRepo, "_HOME"
    Wait-For 15
    $a = Shot "panes-both-shown"
    Send-Keys "ctrl+b"; Wait-For 3
    $b = Shot "panes-sidebar-hidden"
    $afterFirst = Get-GuestPanes
    Send-Keys "ctrl+alt+b"; Wait-For 3
    $c = Shot "panes-both-hidden"
    $afterKeys = Get-GuestPanes
    Stop-GuestApp
    Start-GuestApp -Arguments $guestRepo, "_HOME"
    Wait-For 15
    $d = Shot "panes-hidden-after-restart"
    $info = Get-GuestWindowInfo "codewiki-app"
    if (-not $info) { throw "the app has no window" }
    $y = $info.Top + [int](61 * $info.Scale)
    $sidebarX = $info.Left + [int](22 * $info.Scale)
    $detailsX = $info.Right - [int](22 * $info.Scale)
    Send-RestedClick -X $sidebarX -Y $y; Wait-For 3
    $e = Shot "panes-sidebar-clicked"
    Send-RestedClick -X $detailsX -Y $y; Wait-For 3
    $f = Shot "panes-details-clicked"
    $afterClicks = Get-GuestPanes
    $hidden = $afterKeys.Sidebar -eq "hidden" -and $afterKeys.Details -eq "hidden"
    $shown = $afterClicks.Sidebar -eq "shown" -and $afterClicks.Details -eq "shown"
    Add-Result -Id "PN1" -Title "Hide and show the side panes" -Expect "both hidden after the shortcuts and after a restart; both shown after the clicks" `
        -Status $(if ($hidden -and $shown) { "pass" } else { "fail" }) -Shots @($a, $b, $c, $d, $e, $f) `
        -Observed "after Ctrl+B the app remembers sidebar '$($afterFirst.Sidebar)', details '$($afterFirst.Details)'; after Ctrl+Alt+B as well: sidebar '$($afterKeys.Sidebar)', details '$($afterKeys.Details)'; after clicks at ($sidebarX,$y) and ($detailsX,$y): sidebar '$($afterClicks.Sidebar)', details '$($afterClicks.Details)'; read the screenshots: the page must take the room of a hidden pane, and the fourth must show both still hidden"
    Stop-GuestApp
}

# "Open logs folder" hands the folder to Explorer. Explorer's windows are
# asked for from the user's desktop; the one on the logs folder is then closed.
Test-Case "L3" "Open the logs folder from the palette" "Explorer opens on the logs folder under the data folder" {
    Start-GuestApp -Arguments $guestRepo, "_HOME"
    Wait-For 15
    Send-Keys "ctrl+k"; Wait-For 2
    # The whole name: "logs" alone finds a note named after logs first, when
    # the wiki has one, and Enter then opens the note.
    Send-Text "open logs folder"; Wait-For 2
    $a = Shot "palette-logs"
    Send-Keys "enter"; Wait-For 8
    $b = Shot "logs-folder-open"
    $run = Invoke-GuestScript -Script @'
$shell = New-Object -ComObject Shell.Application
$open = @($shell.Windows() | Where-Object { $_.Document.Folder.Self.Path -like "*\CodeWiki\logs" })
"explorer windows on the logs folder: $($open.Count)"
foreach ($window in $open) { "path: $($window.Document.Folder.Self.Path)"; $window.Quit() }
'@
    $count = if ($run.Output -match "logs folder: (\d+)") { [int]$Matches[1] } else { -1 }
    Add-Result -Id "L3" -Title "Open the logs folder from the palette" -Expect "one Explorer window on %LOCALAPPDATA%\CodeWiki\logs" `
        -Status $(if ($count -ge 1) { "pass" } elseif ($count -eq 0) { "fail" } else { "error" }) -Shots @($a, $b) -Output $run.Output `
        -Observed $(if ($count -ge 0) { "Explorer windows on the logs folder: $count" } else { "the guest did not answer; read the second screenshot" })
    Stop-GuestApp
}

# The Settings page fills the window: a list of seven categories on the left,
# the open one on the right. The rows are at x 92, y 109 + 36 n of the
# window's client area at 100 percent (settings_layout.rs), anchored to its
# top left corner. Each is clicked and photographed; the embedder and the
# keys are left alone, so nothing is sent anywhere.
Test-Case "ST1" "The Settings page and its categories" "each row of the list opens its category" {
    Start-GuestApp -Arguments $guestRepo, "settings:"
    Wait-For 15
    $info = Get-GuestWindowInfo "codewiki-app"
    if (-not $info) { throw "the app has no window" }
    $taken = @()
    $row = 0
    foreach ($name in "appearance", "semantic-search", "backend", "repositories", "updates", "keyboard", "diagnostics") {
        $x = $info.Left + [int](92 * $info.Scale)
        $y = $info.Top + [int]((109 + 36 * $row) * $info.Scale)
        Send-RestedClick -X $x -Y $y -RestSeconds 0.6
        Wait-For 2
        $taken += Shot "settings-$name"
        $row += 1
    }
    Add-Result -Id "ST1" -Title "The Settings page and its categories" -Expect "seven categories, each opened by its row" -Status note -Shots $taken `
        -Observed "started on 'settings:' and clicked the seven rows of the list; read the screenshots: each must show its own heading (Appearance, Semantic search, Backend, Repositories, Updates, Keyboard, Diagnostics) with its row marked, cards on the right, and no sidebar or tabs. No row is named AI model. Semantic search must have the card 'Keys and addresses'. Updates must have the 'Daily check' switch, Appearance the 'Start with Windows' switch"
    Stop-GuestApp
}

# ---------------------------------------------------------------- Z1, zoom
# The whole app zooms with Ctrl+wheel (zoom.rs): 100, 110, 125 ... percent,
# kept in zoom.txt under %APPDATA%\CodeWiki. The wheel is the VM's own mouse
# device and Ctrl is held on its keyboard device, as a hand does: the app
# reads the modifier from the keyboard's state, which a message posted to the
# window with MK_CONTROL would not change.
function Send-CtrlWheel {
    param([Parameter(Mandatory)][int]$Notches)
    Invoke-Device Keyboard PressKey @{ KeyCode = [uint32]0x11 }
    Start-Sleep -Milliseconds 150
    Send-Scroll -Notches $Notches
    Start-Sleep -Milliseconds 150
    Invoke-Device Keyboard ReleaseKey @{ KeyCode = [uint32]0x11 }
    Start-Sleep -Milliseconds 300
}

Test-Case "Z1" "Zoom the whole app with Ctrl+wheel" "two notches up zoom in, two down return to 100 percent, Ctrl+0 resets, and zoom.txt follows" {
    $read = "if (Test-Path `$env:APPDATA\CodeWiki\zoom.txt) { (Get-Content `$env:APPDATA\CodeWiki\zoom.txt -Raw).Trim() } else { '(not set)' }"
    Invoke-GuestScript -Script 'Remove-Item (Join-Path $env:APPDATA "CodeWiki\zoom.txt") -ErrorAction SilentlyContinue' | Out-Null
    Start-GuestApp -Arguments $guestRepo, "_HOME"
    Wait-For 15
    $info = Get-GuestWindowInfo "codewiki-app"
    if (-not $info) { throw "the app has no window" }
    # The pointer rests in the middle of the page, where the wheel is read.
    Move-Pointer -X ($info.Left + [int]($info.Width / 2)) -Y ($info.Top + [int]($info.Height / 2))
    Wait-For 1
    $a = Shot "zoom-100"
    $start = (Invoke-GuestScript -Script $read).Output.Trim()
    Send-CtrlWheel 1; Wait-For 1
    Send-CtrlWheel 1; Wait-For 3
    $zoomed = (Invoke-GuestScript -Script $read).Output.Trim()
    $b = Shot "zoom-in"
    Send-CtrlWheel -1; Wait-For 1
    Send-CtrlWheel -1; Wait-For 3
    $back = (Invoke-GuestScript -Script $read).Output.Trim()
    $c = Shot "zoom-back"
    Send-CtrlWheel 1; Wait-For 2
    Send-Keys "ctrl+0"; Wait-For 3
    $reset = (Invoke-GuestScript -Script $read).Output.Trim()
    $d = Shot "zoom-reset"
    $level = 0
    $up = [int]::TryParse($zoomed, [ref]$level) -and $level -gt 100
    $ok = $up -and $back -eq "100" -and $reset -eq "100"
    Add-Result -Id "Z1" -Title "Zoom the whole app with Ctrl+wheel" -Expect "zoom.txt: above 100 after two notches up, 100 after two down, 100 after Ctrl+0" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($a, $b, $c, $d) `
        -Observed "zoom.txt at the start '$start'; after two notches up with Ctrl held '$zoomed'; after two down '$back'; after one up and Ctrl+0 '$reset'; read the screenshots: the second is larger than the first and shows the level chip in the title bar, the third and fourth look like the first (the title bar never changes size)"
    Stop-GuestApp
}

# ---------------------------------------------------------------- LR1, open the last repository
# Settings > Appearance > Startup > "Open the last repository" is kept in
# open_last.txt ("on") under %APPDATA%\CodeWiki (startup.rs). On, a start with
# no repository and no view opens the wiki map of the last repository; off, it
# opens Home. The switch is turned on through its file (the click's position is
# not read off a screenshot) and photographed in Settings.
Test-Case "LR1" "Open the last repository, on and off" "off: a plain start shows Home; on: after Quit a plain start shows cw-core's wiki map" {
    $file = "`$env:APPDATA\CodeWiki\open_last.txt"
    Invoke-GuestScript -Script "Remove-Item $file -ErrorAction SilentlyContinue" | Out-Null
    # Starting with the folder makes it the last repository.
    Start-GuestApp -Arguments $guestRepo, "settings:appearance"
    Wait-For 15
    $a = Shot "open-last-switch-off"
    $ended = Exit-GuestApp
    Invoke-GuestScript -Script "New-Item -ItemType Directory -Force (Split-Path $file) | Out-Null; Set-Content $file 'on' -NoNewline" | Out-Null
    Start-GuestApp -Arguments $guestRepo, "settings:appearance"
    Wait-For 15
    $b = Shot "open-last-switch-on"
    $endedOn = Exit-GuestApp
    # A plain start: no folder, no view, a working folder that is no repository.
    Start-GuestApp
    Wait-For 25
    $c = Shot "open-last-on-plain-start"
    $opened = Get-OpenedRepo
    $exitedAgain = Exit-GuestApp
    Invoke-GuestScript -Script "Remove-Item $file -ErrorAction SilentlyContinue" | Out-Null
    Start-GuestApp
    Wait-For 25
    $d = Shot "open-last-off-plain-start"
    $openedOff = Get-OpenedRepo
    Stop-GuestApp
    $ok = $ended -and $endedOn -and $exitedAgain -and $opened -eq "cw-core"
    Add-Result -Id "LR1" -Title "Open the last repository, on and off" -Expect "after Quit, a plain start opens cw-core with the switch on, and Home with it off" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($a, $b, $c, $d) `
        -Observed "Settings > Appearance photographed with the switch off, then on (open_last.txt written); Quit ended the app and its backend by itself: $ended, $endedOn, $exitedAgain; plain start with the switch on opened '$opened'; off, '$openedOff'; read the screenshots: the third is cw-core's wiki map, the fourth Home. open_last.txt removed again"
}

Test-Case "G3" "Edit a note and save with Ctrl+S" "the typed text reaches the file" {
    if (-not $noteName) { throw "D2 produced no note to edit" }
    Start-GuestApp -Arguments $guestRepo, "edit:$noteName"
    Wait-For 15
    $a = Shot "editor-open"
    # The click that gave the window the focus took it from the editor; a
    # click into the text gives it back. The marker lands in line 1.
    Send-Click -X ($appWindow.Left + 520) -Y ($appWindow.Top + 160); Wait-For 1
    Send-Text "vmtestmarker "; Wait-For 3
    $b = Shot "editor-typed"
    Send-Keys "ctrl+s"; Wait-For 5
    $c = Shot "editor-saved"
    $run = Invoke-GuestScript -Script "(Get-ChildItem '$guestRepo\obsidian' -Recurse -Filter *.md | Select-String -Pattern 'vmtestmarker' -List | ForEach-Object { `$_.Path }) -join ', '"
    $found = $run.Output.Trim()
    Add-Result -Id "G3" -Title "Edit a note and save with Ctrl+S" -Expect "the typed text reaches the file" `
        -Status $(if ($found) { "pass" } else { "fail" }) -Shots @($a, $b, $c) `
        -Observed $(if ($found) { "marker saved in $found" } else { "marker not found in any note after Ctrl+S" })
    Stop-GuestApp
}

Test-Case "H2" "Theme is remembered" "a theme chosen through the palette is still set after a restart" {
    $read = "if (Test-Path `$env:APPDATA\CodeWiki\theme.txt) { Get-Content `$env:APPDATA\CodeWiki\theme.txt -Raw } else { '(no file)' }"
    $before = (Invoke-GuestScript -Script $read).Output
    Start-GuestApp -Arguments $guestRepo
    Wait-For 15
    Send-Keys "ctrl+k"; Wait-For 2
    Send-Text "ember"; Wait-For 3
    $a = Shot "theme-search"
    Send-Keys "enter"; Wait-For 5
    $b = Shot "theme-applied"
    $after = (Invoke-GuestScript -Script $read).Output
    Stop-GuestApp
    Start-GuestApp -Arguments $guestRepo
    Wait-For 15
    $c = Shot "theme-after-restart"
    Stop-GuestApp
    Add-Result -Id "H2" -Title "Theme is remembered" -Expect "theme.txt changes and the restart shows the theme" `
        -Status $(if ($after -ne $before -and $after -ne "(no file)") { "pass" } else { "fail" }) -Shots @($a, $b, $c) `
        -Observed "theme.txt before: '$before', after: '$after'"
}

# ---------------------------------------------------------------- T, theme menu
$readTheme = "if (Test-Path `$env:APPDATA\CodeWiki\theme.txt) { Get-Content `$env:APPDATA\CodeWiki\theme.txt -Raw } else { '(no file)' }"
Test-Case "T1" "Theme menu by keyboard" "the menu hangs under the toolbar's theme button; Down and Enter choose the next theme" {
    $before = (Invoke-GuestScript -Script $readTheme).Output
    Start-GuestApp -Arguments $guestRepo, "_HOME"
    Wait-For 12
    Send-Keys "ctrl+k"; Wait-For 2
    Send-Text "choose theme"; Wait-For 2
    Send-Keys "enter"; Wait-For 3
    $a = Shot "theme-menu-open"
    Send-Keys "down"; Wait-For 1
    Send-Keys "enter"; Wait-For 4
    $b = Shot "theme-menu-chosen"
    $after = (Invoke-GuestScript -Script $readTheme).Output
    Stop-GuestApp
    Add-Result -Id "T1" -Title "Theme menu by keyboard" -Expect "theme.txt names the next theme" `
        -Status $(if ($after -ne $before -and $after -ne "(no file)") { "pass" } else { "fail" }) -Shots @($a, $b) `
        -Observed "palette: 'choose theme', Enter; then Down, Enter; theme.txt before: '$before', after: '$after'"
}

Test-Case "T2" "Theme menu on Home" "Home has no toolbar; the menu hangs from the window's top right corner and still chooses" {
    $before = (Invoke-GuestScript -Script $readTheme).Output
    Start-GuestApp -Arguments $guestRepo
    Wait-For 12
    Send-Keys "ctrl+k"; Wait-For 2
    Send-Text "choose theme"; Wait-For 2
    Send-Keys "enter"; Wait-For 3
    $a = Shot "theme-menu-on-home"
    Send-Keys "down"; Wait-For 1
    Send-Keys "enter"; Wait-For 4
    $b = Shot "theme-menu-on-home-chosen"
    $after = (Invoke-GuestScript -Script $readTheme).Output
    Stop-GuestApp
    Add-Result -Id "T2" -Title "Theme menu on Home" -Expect "theme.txt names the next theme; read the screenshot for where the menu hangs" `
        -Status $(if ($after -ne $before -and $after -ne "(no file)") { "pass" } else { "fail" }) -Shots @($a, $b) `
        -Observed "on Home, palette: 'choose theme', Enter; then Down, Enter; theme.txt before: '$before', after: '$after'"
}

# ---------------------------------------------------------------- HM, Home
# A folder can be away for a while (a drive, a share). The project must stay
# registered and be listed apart, not be forgotten.
Test-Case "HM1" "A project whose folder is gone stays on Home" "it is still registered and Home lists it under 'Folder not found'" {
    $gone = "$guestHome\repos\gone-demo"
    Invoke-GuestScript -Script "New-Item -ItemType Directory -Force '$gone\.git' | Out-Null; Set-Content '$gone\lib.rs' 'pub fn demo() {}'" | Out-Null
    Start-GuestApp -Arguments $gone
    Wait-For 12
    Stop-GuestApp
    Invoke-GuestScript -Script "Remove-Item -Recurse -Force '$gone'" | Out-Null
    Start-GuestApp -Arguments $guestRepo
    Wait-For 12
    $shot = Shot "home-folder-not-found"
    $probe = Get-Probe "home-folder-not-found" -Quick
    $entry = @(Get-Registered $probe | Where-Object { $_.name -eq "gone-demo" })
    Add-Result -Id "HM1" -Title "A project whose folder is gone stays on Home" -Expect "gone-demo is still registered, marked as not present" `
        -Status $(if ($entry.Count -eq 1 -and $entry[0].present -eq $false) { "pass" } else { "fail" }) -Shots @($shot) `
        -Observed "registered a folder, deleted it, started the app on another project; the backend lists: $((Get-Registered $probe | ForEach-Object { "$($_.name) (present $($_.present))" }) -join ', '); read the screenshot for the heading and the Remove button"
}

Test-Case "HM2" "Remove a project whose folder is gone" "a click on Remove takes it off the list" {
    # Where Remove is with one recent project and one missing, read off the
    # screenshot "home-folder-not-found": from the client area's top right corner.
    $info = Get-GuestWindowInfo "codewiki-app"
    if (-not $info) { throw "the app has no window" }
    $x = $info.Right - [int](284 * $info.Scale); $y = $info.Top + [int](566 * $info.Scale)
    Send-RestedClick -X $x -Y $y
    Wait-For 5
    $shot = Shot "home-project-removed"
    $probe = Get-Probe "home-project-removed" -Quick
    $left = @(Get-Registered $probe | Where-Object { $_.name -eq "gone-demo" })
    Stop-GuestApp
    Add-Result -Id "HM2" -Title "Remove a project whose folder is gone" -Expect "gone-demo is no longer registered; cw-core is" `
        -Status $(if ($left.Count -eq 0 -and (Get-Registered $probe | Where-Object { $_.name -eq "cw-core" })) { "pass" } else { "fail" }) -Shots @($shot) `
        -Observed "clicked Remove at ($x,$y); the backend now lists: $((Get-Registered $probe | ForEach-Object { $_.name }) -join ', ')"
}

# ---------------------------------------------------------------- DV1, the diagram viewer
# A drawn Mermaid diagram in a note is a link, and a click on it opens the
# diagram viewer over the window; Escape closes it (main.rs, open_diagram). The
# wiki of cw-core has no diagram: its notes are rules' notes of a single crate,
# and only a flow note has one (a "Crate Dependency Flow" needs two crates). So
# this case builds a small Rust workspace of three crates in the guest, writes
# its wiki, and opens the flow note. Where the diagram is on the page is not
# known to the script; it clicks down the column and tells the viewer by the
# change in the picture (Get-ImageChange).
Test-Case "DV1" "Open a diagram in the viewer, and close it" "a click on the flow note's diagram opens the viewer; Escape closes it and the note is as before" {
    $demo = "$guestHome\repos\diagram-demo"
    # The workspace's files, sent as one JSON text so that no quote has to be escaped twice.
    $demoFiles = [ordered]@{
        "Cargo.toml"              = "[workspace]`nmembers = [`"crates/*`"]`nresolver = `"2`"`n"
        "crates/alpha/Cargo.toml" = "[package]`nname = `"alpha`"`nversion = `"0.1.0`"`nedition = `"2021`"`n"
        "crates/alpha/src/lib.rs" = "/// The base of the other two crates.`npub fn base() -> u32 { 1 }`n"
        "crates/beta/Cargo.toml"  = "[package]`nname = `"beta`"`nversion = `"0.1.0`"`nedition = `"2021`"`n`n[dependencies]`nalpha = { path = `"../alpha`" }`n"
        "crates/beta/src/lib.rs"  = "/// Builds on alpha.`npub fn middle() -> u32 { alpha::base() + 1 }`n"
        "crates/gamma/Cargo.toml" = "[package]`nname = `"gamma`"`nversion = `"0.1.0`"`nedition = `"2021`"`n`n[dependencies]`nalpha = { path = `"../alpha`" }`nbeta = { path = `"../beta`" }`n"
        "crates/gamma/src/lib.rs" = "/// Builds on alpha and beta.`npub fn top() -> u32 { beta::middle() + alpha::base() }`n"
    }
    $encoded = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes(($demoFiles | ConvertTo-Json -Compress)))
    $setup = Invoke-GuestScript -TimeoutSec 300 -Script (@'
$demo = '{{DEMO}}'
Remove-Item -Recurse -Force $demo -ErrorAction SilentlyContinue
$files = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{{FILES}}')) | ConvertFrom-Json
foreach ($property in $files.PSObject.Properties) {
    $path = Join-Path $demo ($property.Name -replace '/', '\')
    New-Item -ItemType Directory -Force (Split-Path $path) | Out-Null
    [System.IO.File]::WriteAllText($path, $property.Value)
}
Set-Location $demo
& '{{CODEWIKI}}' init 2>&1
"init exit: $LASTEXITCODE"
& '{{CODEWIKI}}' update 2>&1
"update exit: $LASTEXITCODE"
$flow = Get-ChildItem (Join-Path $demo 'obsidian') -Recurse -Filter '*Dependency Flow.md' | Select-Object -First 1
"flow note: $(if ($flow) { $flow.BaseName } else { 'none' })"
$fence = [string][char]96 * 3 + 'mermaid'
"has diagram: $(if ($flow) { (Get-Content $flow.FullName -Raw).Contains($fence) } else { $false })"
$global:LASTEXITCODE = 0
'@).Replace('{{DEMO}}', $demo).Replace('{{FILES}}', $encoded).Replace('{{CODEWIKI}}', "$guestTarget\codewiki.exe")
    $flowName = if ($setup.Output -match "flow note: (.+)") { $Matches[1].Trim() } else { "none" }
    if ($flowName -eq "none" -or $setup.Output -notmatch "has diagram: True") {
        Add-Result -Id "DV1" -Title "Open a diagram in the viewer, and close it" -Expect "the demo wiki has a flow note with a diagram" -Status fail -Output $setup.Output `
            -Observed "the demo workspace's wiki has no flow note with a Mermaid diagram; see the output"
        return
    }
    # With both side panes hidden, the note's column is in the middle of the window.
    Invoke-GuestScript -Script 'New-Item -ItemType Directory -Force "$env:APPDATA\CodeWiki" | Out-Null; Set-Content "$env:APPDATA\CodeWiki\sidebar.txt" hidden -NoNewline; Set-Content "$env:APPDATA\CodeWiki\details.txt" hidden -NoNewline' | Out-Null
    Start-GuestApp -Arguments $demo, $flowName
    # The diagram is drawn off the UI thread: a placeholder shows first.
    Wait-For 25
    $info = Get-GuestWindowInfo "codewiki-app"
    if (-not $info) { throw "the app has no window" }
    $note = Shot "diagram-note"
    $base = Join-Path $shots $note
    $probeShot = Join-Path $logs "diagram-scan.png"
    $found = $null
    $attempts = 0
    $step = [int](40 * $info.Scale)
    foreach ($offset in -100, -250, 60) {
        $x = $info.Left + [int]($info.Width / 2) + [int]($offset * $info.Scale)
        for ($y = $info.Top + [int]($info.Height * 0.25); $y -lt $info.Bottom - $step; $y += $step) {
            Send-Click -X $x -Y $y
            $attempts += 1
            Wait-For 2.5
            Save-Screenshot $probeShot | Out-Null
            if ((Get-ImageChange $base $probeShot) -gt 0.02) {
                # Something opened. The viewer closes with Escape and leaves
                # the note as it was; a page that was navigated to does not.
                Send-Keys "esc"; Wait-For 2.5
                Save-Screenshot $probeShot | Out-Null
                if ((Get-ImageChange $base $probeShot) -lt 0.005) { $found = @($x, $y); break }
                # A link of the note was followed: start the note again.
                Stop-GuestApp
                Start-GuestApp -Arguments $demo, $flowName
                Wait-For 25
                Save-Screenshot $base | Out-Null
            }
        }
        if ($found) { break }
    }
    if (-not $found) {
        Add-Result -Id "DV1" -Title "Open a diagram in the viewer, and close it" -Expect "a click on the diagram opens the viewer" -Status note -Shots @($note) `
            -Observed "$attempts clicks down the note's column found nothing that opens over the window and closes with Escape; read the screenshot: whether the note shows a drawn diagram (a placeholder or the source means it was not drawn in time)"
    } else {
        Send-Click -X $found[0] -Y $found[1]
        Wait-For 8
        $open = Shot "diagram-viewer-open"
        Send-Keys "esc"; Wait-For 3
        $closed = Shot "diagram-viewer-closed"
        $after = Get-ImageChange $base (Join-Path $shots $closed)
        Add-Result -Id "DV1" -Title "Open a diagram in the viewer, and close it" -Expect "the viewer opens over the note and Escape closes it" `
            -Status $(if ($after -lt 0.005) { "pass" } else { "fail" }) -Shots @($note, $open, $closed) `
            -Observed "the flow note '$flowName' of a three-crate workspace built in the guest; a click at ($($found[0]),$($found[1])) after $attempts tries opened something that covers the window, and Escape brought back the note (change from before: $([math]::Round($after * 100, 2)) percent of sampled pixels); read the screenshots: the second must be the diagram viewer (header 'Scroll to zoom, drag to move', a percentage, zoom buttons), sharp after the wait, the third the note"
    }
    Stop-GuestApp
    # Put the guest back as the cases after this one expect: the panes shown,
    # cw-core the last repository, the demo not registered.
    Invoke-GuestScript -Script 'Set-Content "$env:APPDATA\CodeWiki\sidebar.txt" shown -NoNewline; Set-Content "$env:APPDATA\CodeWiki\details.txt" shown -NoNewline' | Out-Null
    Start-GuestApp -Arguments $guestRepo
    Wait-For 12
    Invoke-GuestBackendScript -Script (@'
$id = Get-RepoId 'diagram-demo'
if ($id) { Invoke-Backend DELETE "/v1/repos/$id" | Out-Null }
Remove-Item -Recurse -Force '{{DEMO}}' -ErrorAction SilentlyContinue
'@).Replace('{{DEMO}}', $demo) | Out-Null
    Stop-GuestApp
}

# ---------------------------------------------------------------- IC, icon
Test-Case "IC1" "The program carries the app's icon" "codewiki-app.exe holds icon group 1, which Windows shows for the file, the shortcut and the taskbar" {
    $run = Invoke-GuestScript -Script @"
Add-Type -Namespace VmTest -Name Res -MemberDefinition '
[DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)] public static extern IntPtr LoadLibraryEx(string file, IntPtr reserved, uint flags);
[DllImport("kernel32.dll")] public static extern IntPtr FindResource(IntPtr module, IntPtr name, IntPtr type);
[DllImport("kernel32.dll")] public static extern uint SizeofResource(IntPtr module, IntPtr resource);
[DllImport("kernel32.dll")] public static extern bool FreeLibrary(IntPtr module);'
foreach (`$name in 'codewiki-app.exe', 'codewiki.exe') {
    # 2: load as a data file. 14: RT_GROUP_ICON.
    `$module = [VmTest.Res]::LoadLibraryEx((Join-Path '$guestTarget' `$name), [IntPtr]::Zero, 2)
    `$group = [VmTest.Res]::FindResource(`$module, [IntPtr]1, [IntPtr]14)
    "`$name icon group 1: `$(`$group -ne [IntPtr]::Zero), `$([VmTest.Res]::SizeofResource(`$module, `$group)) bytes"
    [void][VmTest.Res]::FreeLibrary(`$module)
}
"@
    $has = $run.Output -match "codewiki-app\.exe icon group 1: True"
    Add-Result -Id "IC1" -Title "The program carries the app's icon" -Expect "codewiki-app.exe holds icon group 1" `
        -Status $(if ($has) { "pass" } else { "fail" }) -Output $run.Output `
        -Observed "$(($run.Output -split "`n" | ForEach-Object { $_.Trim() } | Where-Object { $_ }) -join '; '); the icon on screen is in the Start menu and taskbar screenshots"
}

# ---------------------------------------------------------------- I, reboot
Test-Case "I1" "Restart Windows" "nothing of CodeWiki starts by itself; PATH kept; the app starts again" {
    # Shut down and start again; the recording repeats its last picture meanwhile.
    Restart-Guest
    Wait-Guest -TimeoutSec 900
    Wait-For 40
    $a = Shot "after-reboot"
    $probe = Get-Probe "after-reboot"
    $running = $probe.processes.Count
    $search = Start-FromStartMenu
    $b = Shot "launch-after-reboot"
    $probe2 = Get-Probe "launch-after-reboot" -Quick
    $window = @(Get-App $probe2 | Where-Object { $_.hasWindow })
    $ok = $running -eq 0 -and $probe.userPath.processSees -ge 1 -and $window.Count -eq 1
    Add-Result -Id "I1" -Title "Restart Windows" -Expect "no autostart, PATH kept, app starts" -Status $(if ($ok) { "pass" } else { "fail" }) `
        -Shots @($a, $search, $b) `
        -Observed "CodeWiki processes right after boot: $running; a new process sees the folder on PATH: $($probe.userPath.processSees -ge 1); app window after Start menu launch: $($window.Count)"
}

# ---------------------------------------------------------------- AS, start with Windows
# I1 has just shown that nothing starts by itself. The setting is reached
# through the palette, which lists it for an installed copy.
function Set-GuestAutostart {
    Send-Keys "ctrl+k"; Wait-For 2
    Send-Text "start with windows"; Wait-For 2
    $listed = Shot "autostart-in-palette"
    Send-Keys "enter"; Wait-For 4
    $listed
}
$autostartCommand = "`"$guestTarget\codewiki-app.exe`" --tray"

Test-Case "AS1" "Turn on the start with Windows" "the palette's setting writes the value that starts the installed app for the tray" {
    $a = Set-GuestAutostart
    $b = Shot "autostart-on"
    $probe = Get-Probe "autostart-on" -Quick
    Add-Result -Id "AS1" -Title "Turn on the start with Windows" -Expect "the Run value CodeWiki is $autostartCommand" `
        -Status $(if ($probe.registry.autostart -eq $autostartCommand) { "pass" } else { "fail" }) -Shots @($a, $b) `
        -Observed "palette: 'start with windows', Enter; the Run value is now: '$($probe.registry.autostart)'; the second screenshot must show the settings page with the setting on"
}

Test-Case "AS2" "Windows starts the app in the tray" "after a restart of Windows the app runs with no window and its backend is up; a click on the tray icon shows the window" {
    Restart-Guest
    Wait-Guest -TimeoutSec 900
    Wait-For 60
    $a = Shot "after-reboot-autostart"
    $probe = Get-Probe "autostart-boot" -Quick
    $app = @(Get-App $probe)
    $windows = @($app | Where-Object { $_.hasWindow }).Count
    $backendAlive = [bool]($probe.backend -and $probe.backend.pidAlive -and $probe.backend.health -eq 200)
    $name = "$((Get-GuestTrayIcons | Select-Object -First 1).Name)"
    $icon = Send-GuestTrayClick -Button left
    Wait-For 5
    $info = Get-GuestWindowInfo "codewiki-app"
    $b = Shot "autostart-window-shown"
    # A window never shown before must still land inside the screen (W0).
    $slack = if ($info) { [int](8 * $info.Scale) } else { 0 }
    $inside = $info -and $info.Outer.Top -ge $info.Free.Top - $slack -and $info.Outer.Bottom -le $info.Free.Bottom + $slack -and
        $info.Outer.Left -ge $info.Free.Left - $slack -and $info.Outer.Right -le $info.Free.Right + $slack
    $opened = Get-OpenedRepo
    $ok = $app.Count -eq 1 -and $windows -eq 0 -and $backendAlive -and $name -like "*cw-core*" -and $inside -and $opened -eq "cw-core"
    Add-Result -Id "AS2" -Title "Windows starts the app in the tray" -Expect "one app with no window; backend healthy; the tray click shows the window inside the screen, on cw-core" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($a, $b) `
        -Observed "a minute after sign-in: app processes $($app.Count), with a window $windows; backend pid $($probe.backend.pid), health $($probe.backend.health); tooltip '$name'; the app's log says it opened '$opened'; after a click on the icon: $(Format-Window $info), inside the free part of the screen: $([bool]$inside); read the second screenshot for the title bar and the page"
}

Test-Case "AS3" "Turn the start with Windows off, and on again" "off removes the value; on writes it again" {
    $a = Set-GuestAutostart
    $off = (Get-Probe "autostart-off" -Quick).registry.autostart
    $b = Shot "autostart-off"
    Set-GuestAutostart | Out-Null
    # Left on: the uninstall (K1) must remove it.
    $on = (Get-Probe "autostart-on-again" -Quick).registry.autostart
    Add-Result -Id "AS3" -Title "Turn the start with Windows off, and on again" -Expect "no value, then the value again" `
        -Status $(if (-not $off -and $on -eq $autostartCommand) { "pass" } else { "fail" }) -Shots @($a, $b) `
        -Observed "after turning it off the Run value was '$off'; after turning it on again '$on'"
}

# The taskbar is Explorer's. When Explorer starts again, every program must
# put its icon back, or the app is running with no way to reach it.
Test-Case "TR6" "The tray icon comes back when Explorer restarts" "Explorer is ended and starts again; the app's icon is there again" {
    $before = @(Get-GuestTrayIcons).Count
    Invoke-GuestScript -Script 'Stop-Process -Name explorer -Force; Start-Sleep 12; if (-not (Get-Process explorer -ErrorAction SilentlyContinue)) { Start-Process explorer.exe }' | Out-Null
    Wait-For 10
    $after = @(Get-GuestTrayIcons)
    $shot = Shot "explorer-restarted"
    Add-Result -Id "TR6" -Title "The tray icon comes back when Explorer restarts" -Expect "one icon before, one after" `
        -Status $(if ($before -eq 1 -and $after.Count -eq 1) { "pass" } else { "fail" }) -Shots @($shot) `
        -Observed "tray icons named CodeWiki before Explorer was ended: $before; after it started again: $($after.Count), tooltip '$(($after | Select-Object -First 1).Name)'"
}

Test-Case "D3" "MCP bridge over stdio" "initialize and tools/list answer; 27 tools" {
    $run = Invoke-GuestScript -TimeoutSec 120 -Script @"
Set-Location '$guestRepo'
`$requests = @(
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"vm-test","version":"0"}}}'
    '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
)
`$answers = `$requests | & codewiki-mcp.exe 2>&1
"mcp exit: `$LASTEXITCODE"
foreach (`$line in `$answers) {
    try { `$message = "`$line" | ConvertFrom-Json } catch { "not json: `$line"; continue }
    if (`$message.id -eq 1) { "server: `$(`$message.result.serverInfo.name) `$(`$message.result.serverInfo.version)" }
    if (`$message.id -eq 2) { "tools: `$(@(`$message.result.tools).Count)" }
}
`$global:LASTEXITCODE = 0
"@
    $tools = if ($run.Output -match "tools: (\d+)") { [int]$Matches[1] } else { 0 }
    Add-Result -Id "D3" -Title "MCP bridge over stdio" -Expect "initialize and tools/list answer; 27 tools" `
        -Status $(if ($tools -eq 27) { "pass" } elseif ($tools -gt 0) { "note" } else { "fail" }) -Output $run.Output -Observed "tools listed: $tools"
}

# D2 wrote the wiki and D3 showed that the bridge answers. The three tools that
# let a coding agent write a note are called here the way an agent calls them,
# over stdio, with no agent: guest\mcp-write.ps1 asks which note waits, reads
# its brief, tries a submission on an old base and one for a section that is
# not the agent's (both refused), then submits the explanation and reads the
# note's file. The note stays changed for the cases after it.
Test-Case "D4" "Write a note through the MCP bridge" "list_waiting_notes, get_note_brief and submit_note over stdio save an explanation into the note's file" {
    Copy-ToGuest -Path (Join-Path $PSScriptRoot "guest\mcp-write.ps1") -GuestDirectory $script:Vm.GuestDir
    $run = Invoke-GuestScript -TimeoutSec 600 -Script "& '$powershell' -NoProfile -ExecutionPolicy Bypass -File '$($script:Vm.GuestDir)\mcp-write.ps1' -Mcp '$guestTarget\codewiki-mcp.exe' -Repo '$guestRepo'"
    $line = { param($key) if ($run.Output -match "(?m)^${key}: (.*)$") { $Matches[1].Trim() } else { "" } }
    $note = & $line "note"
    $stale = & $line "stale base"
    $wrong = & $line "wrong section"
    $submit = & $line "submit"
    $held = (& $line "file holds the text") -eq "True"
    $again = (& $line "brief again, section written by an agent") -eq "True"
    $ok = $note -and $stale -match "Not saved" -and $wrong -match "Not saved" -and $submit -match "^Saved " -and $held -and $again
    Add-Result -Id "D4" -Title "Write a note through the MCP bridge" -Expect "a stale base and a section that is not the agent's are refused; the submission is saved; the note's file holds the text" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
        -Observed "note '$note'; old base: '$stale'; wrong section: '$wrong'; submission: '$submit'; the vault file holds the submitted text: $held; the brief now shows the section as written by an agent: $again"
}

# Each program keeps a log of its own. By now the app, a backend and the MCP
# bridge have all run.
Test-Case "L2" "Every program writes its own log" "app.log, backend.log and mcp.log are in the logs folder, none empty" {
    $run = Invoke-GuestScript -Script @'
foreach ($name in "app", "backend", "mcp") {
    $file = "$env:LOCALAPPDATA\CodeWiki\logs\$name.log"
    if (Test-Path $file) {
        "${name}: $((Get-Item $file).Length) bytes"
        Get-Content $file -Tail 3 | ForEach-Object { "    $_" }
    } else { "${name}: missing" }
}
"folder: $((Get-ChildItem "$env:LOCALAPPDATA\CodeWiki\logs" -ErrorAction SilentlyContinue | ForEach-Object { $_.Name }) -join ', ')"
'@
    $sizes = @{}
    foreach ($name in "app", "backend", "mcp") {
        $sizes[$name] = if ($run.Output -match "(?m)^${name}: (\d+) bytes") { [int]$Matches[1] } else { 0 }
    }
    $missing = @($sizes.Keys | Where-Object { $sizes[$_] -eq 0 } | Sort-Object)
    Add-Result -Id "L2" -Title "Every program writes its own log" -Expect "three logs, none empty" `
        -Status $(if (-not $run.Output.Trim()) { "error" } elseif ($missing) { "fail" } else { "pass" }) -Output $run.Output `
        -Observed $(if (-not $run.Output.Trim()) { "the guest did not answer" } elseif ($missing) { "missing or empty: $($missing -join ', ')" } else { "app.log $($sizes.app) bytes, backend.log $($sizes.backend) bytes, mcp.log $($sizes.mcp) bytes" })
}


# ---------------------------------------------------------------- ER, error reports
# A package built with CODEWIKI_SENTRY_DSN carries a DSN and reports only once
# the user turns "Send error reports" on, which writes error_reports.txt in
# %APPDATA%\CodeWiki (crates/cw-telemetry). These cases turn it on through
# that file, send one test report and one panic with the hidden command, and
# turn it off again. Whether the reports arrived, with readable stack frames,
# is read in Sentry afterwards: project fly-theonlyone/codewiki, environment
# production, tag program:sentry-test.
Test-Case "ER1" "Error reports from the command line" "off by default; with the switch on, a test report and a panic are sent" {
    $run = Invoke-GuestScript -TimeoutSec 120 -Script @'
$consent = Join-Path $env:APPDATA "CodeWiki\error_reports.txt"
Remove-Item $consent -ErrorAction SilentlyContinue
"switch off:"
& codewiki sentry-test 2>&1
"off exit: $LASTEXITCODE"
New-Item -ItemType Directory -Force (Split-Path $consent) | Out-Null
Set-Content $consent "on" -NoNewline
"switch on:"
& codewiki sentry-test 2>&1
"on exit: $LASTEXITCODE"
& codewiki sentry-test --panic 2>&1
"panic exit: $LASTEXITCODE"
Remove-Item $consent -ErrorAction SilentlyContinue
$global:LASTEXITCODE = 0
'@
    $off = $run.Output -match "off exit: 1" -and $run.Output -match "error reports are off"
    $sent = $run.Output -match "on exit: 0" -and $run.Output -match "sent a test report"
    $panicked = $run.Output -match "panic exit: 101" -and $run.Output -match "test panic, sent on request"
    $status = if (-not $off) { "fail" } elseif ($sent -and $panicked) { "pass" } elseif ($run.Output -match "on exit: 1") { "note" } else { "fail" }
    $observed = if ($status -eq "note") { "this package carries no DSN: it refuses with the switch on too, as a build without CODEWIKI_SENTRY_DSN must" }
        else { "refused while off: $off; test report sent with the switch on: $sent; panic reported and ended with 101: $panicked. Look in Sentry for the two events of environment production" }
    Add-Result -Id "ER1" -Title "Error reports from the command line" -Expect "refused while off; sent while on" `
        -Status $status -Output $run.Output -Observed $observed
}

Test-Case "ER2" "The switch in Settings" "Settings > Diagnostics shows 'Send error reports', off, and can be turned on" {
    Invoke-GuestScript -Script 'Remove-Item (Join-Path $env:APPDATA "CodeWiki\error_reports.txt") -ErrorAction SilentlyContinue' | Out-Null
    Start-GuestApp -Arguments $guestRepo, "settings:diagnostics"
    Wait-For 15
    $shot = Shot "settings-error-reports"
    Add-Result -Id "ER2" -Title "The switch in Settings" -Expect "a card 'Error reports' with the switch 'Send error reports' off and not locked, and the text that says what is sent" -Status note -Shots @($shot) `
        -Observed "started on 'settings:diagnostics'; read the screenshot: the card 'Error reports' must show the switch off, with no lock note under it in a package that carries a DSN ('This build has no error reporting' in one that does not)"
    Stop-GuestApp
}

# ---------------------------------------------------------------- CC, Claude Code plugin
# The plugin is tested with the real Claude Code: installed as its docs say,
# then the plugin as README says, from the installation's own plugins folder.
# CodeWiki is installed and $guestRepo holds a wiki by now (C5, D2).
if ($Plugin) {
    $market = "$guestTarget\plugins"

    # Hand the guest the access token of the host's own Claude login, and
    # nothing else of it. Not the refresh token: with it Claude Code in the
    # guest could renew the login, and a renewal there can end the login on
    # the host. Not the tokens of other services, which the same file holds.
    # The token goes over PowerShell Direct into the guest user's .claude
    # folder and is never printed; Remove-ClaudeLogin deletes it. Take no
    # checkpoint of the guest in between. Returns why it could not be done,
    # or nothing.
    function Copy-ClaudeLogin {
        $folder = if ($env:CLAUDE_CONFIG_DIR) { $env:CLAUDE_CONFIG_DIR } else { Join-Path $env:USERPROFILE ".claude" }
        $file = Join-Path $folder ".credentials.json"
        if (-not (Test-Path $file)) { return "the host has no saved Claude login ($file)" }
        $login = (Get-Content $file -Raw | ConvertFrom-Json).claudeAiOauth
        if (-not $login.accessToken) { return "the host's saved login holds no access token" }
        $minutes = [int](([DateTimeOffset]::FromUnixTimeMilliseconds([long]$login.expiresAt) - [DateTimeOffset]::UtcNow).TotalMinutes)
        if ($minutes -lt 20) { return "the host's access token ends in $minutes minutes; use Claude Code on the host once, so that it renews the token, and run again" }
        $reduced = @{ claudeAiOauth = [ordered]@{
                accessToken      = $login.accessToken
                expiresAt        = $login.expiresAt
                scopes           = $login.scopes
                subscriptionType = $login.subscriptionType
                rateLimitTier    = $login.rateLimitTier
            }
        } | ConvertTo-Json -Depth 4 -Compress
        $run = Invoke-InGuest -ArgumentList $reduced -ScriptBlock {
            param($json)
            $folder = Join-Path $env:USERPROFILE ".claude"
            New-Item -ItemType Directory -Force $folder | Out-Null
            [System.IO.File]::WriteAllText((Join-Path $folder ".credentials.json"), $json, (New-Object System.Text.UTF8Encoding $false))
            "written"
        }
        if ($run.Output -notcontains "written") { return "it could not be written in the guest: $($run.Errors -join '; ')" }
    }

    function Remove-ClaudeLogin {
        $run = Invoke-InGuest -ScriptBlock {
            $file = Join-Path $env:USERPROFILE ".claude\.credentials.json"
            Remove-Item $file -Force -ErrorAction SilentlyContinue
            if (Test-Path $file) { "still there" } else { "gone" }
        }
        "the Claude login in the guest: $(if ($run.Output) { $run.Output -join ' ' } else { "no answer ($($run.Errors -join '; '))" })"
    }

    # Claude Code runs its commands and hooks in Git Bash when Git for Windows
    # is there and in PowerShell when it is not, and it clones a marketplace
    # named by its repository with git. Its users nearly all have git, so the
    # plugin is tested with it. The checkpoint stays without git: this comes
    # after every case that needs a guest without it (B1, D2). The installer
    # is the one git-scm.com hands out, run with the rights that "Yes" at its
    # UAC prompt gives it.
    # After a restart of Windows the guest's network takes a while, and the
    # host's own (a VPN that reconnects) can drop it for a moment: names did
    # not resolve when this block's first download was asked for. Wait for
    # them, so that a slow network is not taken for a failed install.
    function Wait-GuestNetwork {
        param([int]$TimeoutSec = 240)
        $deadline = (Get-Date).AddSeconds($TimeoutSec)
        do {
            $run = Invoke-GuestScript -TimeoutSec 60 -Script 'try { [void][System.Net.Dns]::GetHostAddresses("api.github.com"); [void][System.Net.Dns]::GetHostAddresses("claude.ai"); "names resolve" } catch { "no names yet" }'
            if ($run.Output -match "names resolve") { return $true }
            Start-Sleep -Seconds 10
        } while ((Get-Date) -lt $deadline)
        $false
    }

    Test-Case "CC0" "Install Git for Windows" "git and Git Bash are installed" {
        if (-not (Wait-GuestNetwork)) { throw "the guest resolves no names: it has no network for the downloads" }
        $run = Invoke-GuestScript -Background -TimeoutSec 1200 -Script @'
$ProgressPreference = "SilentlyContinue"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$release = Invoke-RestMethod "https://api.github.com/repos/git-for-windows/git/releases/latest" -UseBasicParsing
$asset = $release.assets | Where-Object { $_.name -match '^Git-[\d\.]+-64-bit\.exe$' } | Select-Object -First 1
if (-not $asset) { throw "release $($release.tag_name) has no 64-bit installer" }
$file = Join-Path $env:TEMP $asset.name
Invoke-WebRequest $asset.browser_download_url -OutFile $file -UseBasicParsing
$signature = Get-AuthenticodeSignature $file
"installer: $($asset.name), signature $($signature.Status)"
if ($signature.Status -ne "Valid") { throw "the installer's signature is $($signature.Status)" }
$setup = Start-Process $file -ArgumentList "/VERYSILENT", "/NORESTART", "/NOCANCEL", "/SP-" -Wait -PassThru
"setup exit: $($setup.ExitCode)"
Remove-Item $file -Force
"git: $(& "$env:ProgramFiles\Git\cmd\git.exe" --version 2>&1)"
"bash: $(Test-Path "$env:ProgramFiles\Git\bin\bash.exe")"
$global:LASTEXITCODE = 0
'@
        $ok = $run.Output -match "setup exit: 0" -and $run.Output -match "git: git version" -and $run.Output -match "bash: True"
        $version = if ($run.Output -match "git: git version (\S+)") { $Matches[1] } else { "none" }
        Add-Result -Id "CC0" -Title "Install Git for Windows" -Expect "git and Git Bash are installed" `
            -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
            -Observed $(if ($ok) { "Git for Windows $version from its latest release at GitHub, installer signature valid, silent install" } else { "not installed; see the output" })
    }

    Test-Case "CC1" "Install Claude Code as its docs say" "claude.exe is installed for the user, carries Anthropic's signature and prints its version" {
        if (-not (Wait-GuestNetwork)) { throw "the guest resolves no names: it has no network for the downloads" }
        $run = Invoke-GuestScript -TimeoutSec 900 -Script @'
irm https://claude.ai/install.ps1 | iex
$claude = "$env:USERPROFILE\.local\bin\claude.exe"
"installed: $(Test-Path $claude)"
if (Test-Path $claude) {
    $signature = Get-AuthenticodeSignature $claude
    "signature: $($signature.Status), Anthropic: $($signature.SignerCertificate.Subject -match 'Anthropic')"
    "version: $(& $claude --version 2>&1)"
}
"git in the guest: $(Test-Path "$env:ProgramFiles\Git\bin\bash.exe")"
$global:LASTEXITCODE = 0
'@
        $ok = $run.Output -match "installed: True" -and $run.Output -match "signature: Valid, Anthropic: True" -and $run.Output -match "version: \d"
        $version = if ($run.Output -match "version: (.+)") { $Matches[1].Trim() } else { "none" }
        Add-Result -Id "CC1" -Title "Install Claude Code as its docs say" -Expect "installed, signed by Anthropic, prints its version" `
            -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
            -Observed "'irm https://claude.ai/install.ps1 | iex' as the user: version $version; Git Bash in the guest: $(if ($run.Output -match 'git in the guest: True') { 'yes' } else { 'no, so Claude Code runs commands and hooks in PowerShell' })"
    }

    $signedIn = $false
    Test-Case "CC2" "Claude Code signs in with the host's login" "a request is answered" {
        $problem = Copy-ClaudeLogin
        if ($problem) {
            Add-Result -Id "CC2" -Title "Claude Code signs in with the host's login" -Expect "a request is answered" -Status fail `
                -Observed "the host's login could not be handed over: $problem. The owner has to log in by hand in the guest ('claude', then '/login') before CC5 and CC6 can run"
            return
        }
        $run = Invoke-GuestScript -TimeoutSec 300 -Script @'
$env:ENABLE_CLAUDEAI_MCP_SERVERS = "false"
$answer = "$(& "$env:USERPROFILE\.local\bin\claude.exe" -p "Reply with the single word: ready" --output-format json --model haiku --max-turns 1 2>&1)"
"claude exit: $LASTEXITCODE"
try {
    $result = $answer | ConvertFrom-Json
    "error: $($result.is_error)"
    "answer: $($result.result)"
    "model: $(($result.modelUsage.PSObject.Properties | ForEach-Object { $_.Name }) -join ', ')"
} catch { "not json: $($answer.Substring(0, [Math]::Min(300, $answer.Length)))" }
$global:LASTEXITCODE = 0
'@
        $script:signedIn = $run.Output -match "claude exit: 0" -and $run.Output -match "(?i)answer: \W*ready"
        Add-Result -Id "CC2" -Title "Claude Code signs in with the host's login" -Expect "a request is answered" `
            -Status $(if ($signedIn) { "pass" } else { "fail" }) -Output $run.Output `
            -Observed $(if ($signedIn) { "the guest got the host's access token only (no refresh token, no other service's token) and Claude answered 'ready'" } else { "the token was handed over but Claude did not answer; see the output. The owner has to log in by hand in the guest ('claude', then '/login')" })
    }

    # The marketplace is the plugins folder of the installation: nothing is
    # cloned, so the guest needs no git, and the folder holds the plugin only.
    Test-Case "CC3" "Install the plugin as README says" "the installation's plugins folder is added as a marketplace and codewiki@codewiki is installed and enabled" {
        $run = Invoke-GuestScript -TimeoutSec 600 -Script (@'
$claude = "$env:USERPROFILE\.local\bin\claude.exe"
"--- claude plugin validate"; & $claude plugin validate '{{MARKET}}' 2>&1; "validate exit: $LASTEXITCODE"
"--- claude plugin marketplace add"; & $claude plugin marketplace add '{{MARKET}}' 2>&1; "add exit: $LASTEXITCODE"
"--- claude plugin install codewiki@codewiki"; & $claude plugin install codewiki@codewiki 2>&1; "install exit: $LASTEXITCODE"
"--- claude plugin list"; & $claude plugin list 2>&1; "list exit: $LASTEXITCODE"
$global:LASTEXITCODE = 0
'@).Replace("{{MARKET}}", $market)
        $ok = $run.Output -match "add exit: 0" -and $run.Output -match "install exit: 0" -and
            $run.Output -match "codewiki@codewiki" -and $run.Output -match "(?s)--- claude plugin list.*enabled"
        $warnings = if ($run.Output -match "Found (\d+) warnings?") { $Matches[1] } else { "0" }
        Add-Result -Id "CC3" -Title "Install the plugin as README says" -Expect "marketplace added, plugin installed and enabled" `
            -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
            -Observed "'claude plugin marketplace add $market' and 'claude plugin install codewiki@codewiki': $(if ($ok) { 'installed and enabled' } else { 'not installed; see the output' }); 'claude plugin validate' gave $warnings warnings"
    }

    # The account's own connectors (mail, drive and the like) come with the
    # login. They are not this test's business: they are asked to stay out, and
    # only lines that name codewiki are kept.
    Test-Case "CC4" "The plugin's MCP server connects" "claude mcp list shows plugin:codewiki:codewiki as connected" {
        $run = Invoke-GuestScript -TimeoutSec 300 -Script (@'
Set-Location '{{REPO}}'
$env:ENABLE_CLAUDEAI_MCP_SERVERS = "false"
$lines = @(& "$env:USERPROFILE\.local\bin\claude.exe" mcp list 2>&1 | ForEach-Object { "$_" })
"mcp exit: $LASTEXITCODE"
"servers listed: $(@($lines | Where-Object { $_ -match ' - ' }).Count)"
$lines | Where-Object { $_ -match "codewiki" }
$global:LASTEXITCODE = 0
'@).Replace("{{REPO}}", $guestRepo)
        $ok = $run.Output -match "plugin:codewiki:codewiki.*Connected"
        Add-Result -Id "CC4" -Title "The plugin's MCP server connects" -Expect "plugin:codewiki:codewiki is connected" `
            -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
            -Observed $(if ($ok) { "'claude mcp list' in the repository: plugin:codewiki:codewiki is connected (README names it so)" } else { "not connected; see the output" })
    }

    # One real session: Claude is asked to call get_overview. The session's
    # events say which servers connected, which tools were offered, what was
    # called, and what the session-start hook added.
    $session = ""
    Test-Case "CC5" "Claude calls a CodeWiki tool" "the 27 tools are offered, get_overview is called without a permission prompt and answers from the wiki" {
        if (-not $signedIn) { throw "Claude Code is not signed in (CC2)" }
        $run = Invoke-GuestScript -TimeoutSec 600 -Script (@'
Set-Location '{{REPO}}'
$env:ENABLE_CLAUDEAI_MCP_SERVERS = "false"
$out = "$env:TEMP\claude-plugin-run.jsonl"
& "$env:USERPROFILE\.local\bin\claude.exe" -p "Call the get_overview tool of the codewiki MCP server once. Then answer with one line: the title of the wiki's home note." --allowedTools "mcp__plugin_codewiki_codewiki__*" --output-format stream-json --verbose --max-turns 6 --model haiku 2>&1 | Out-File -Encoding utf8 $out
"claude exit: $LASTEXITCODE"
function Cut { param([string]$Text, [int]$Length) $flat = $Text -replace "\s+", " "; $flat.Substring(0, [Math]::Min($Length, $flat.Length)) }
foreach ($line in Get-Content $out) {
    try { $event = $line | ConvertFrom-Json } catch { "not json: $(Cut $line 200)"; continue }
    if ($event.type -eq "system" -and $event.subtype -eq "init") {
        "claude code: $($event.claude_code_version), model $($event.model), permission mode $($event.permissionMode)"
        "codewiki server: $(($event.mcp_servers | Where-Object { $_.name -match 'codewiki' } | ForEach-Object { "$($_.name)=$($_.status)" }) -join ', ')"
        "other servers in the session: $(@($event.mcp_servers | Where-Object { $_.name -notmatch 'codewiki' }).Count)"
        "codewiki plugin loaded: $([bool]($event.plugins | Where-Object { $_.name -eq 'codewiki' }))"
        $errors = @($event.plugin_errors | Where-Object { $_ })
        "plugin errors: $($errors.Count)$(if ($errors) { ': ' + (Cut ($errors | ConvertTo-Json -Compress -Depth 4) 400) })"
        "codewiki tools offered: $(@($event.tools | Where-Object { $_ -like 'mcp__plugin_codewiki_*' }).Count)"
    } elseif ($event.type -eq "system" -and $event.subtype -eq "hook_response") {
        "briefing: $(Cut "$($event.output)" 600)"
    } elseif ($event.type -eq "system" -and $event.subtype -eq "informational") {
        "notice: $(Cut "$($event.content)" 400)"
    } elseif ($event.type -eq "assistant") {
        foreach ($part in $event.message.content) { if ($part.type -eq "tool_use") { "tool call: $($part.name)" } }
    } elseif ($event.type -eq "user") {
        foreach ($part in $event.message.content) {
            if ($part.type -eq "tool_result") { "tool result (error '$($part.is_error)'): $(Cut ($part.content | ConvertTo-Json -Compress -Depth 4) 300)" }
        }
    } elseif ($event.type -eq "result") {
        "result: error $($event.is_error), turns $($event.num_turns), denials $(@($event.permission_denials).Count)"
        "answer: $(Cut "$($event.result)" 300)"
    }
}
Remove-Item $out -Force
$global:LASTEXITCODE = 0
'@).Replace("{{REPO}}", $guestRepo)
        $script:session = $run.Output
        $tools = if ($run.Output -match "codewiki tools offered: (\d+)") { [int]$Matches[1] } else { 0 }
        $answer = if ($run.Output -match "(?m)^answer: (.+)$") { $Matches[1].Trim() } else { "" }
        $ok = $run.Output -match "codewiki server: plugin:codewiki:codewiki=connected" -and $tools -eq 27 -and
            $run.Output -match "tool call: mcp__plugin_codewiki_codewiki__get_overview" -and $run.Output -match "result: error False, turns \d+, denials 0"
        Add-Result -Id "CC5" -Title "Claude calls a CodeWiki tool" -Expect "27 tools offered; get_overview called and answered" `
            -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
            -Observed "tools offered: $tools; get_overview called: $($run.Output -match 'tool call: mcp__plugin_codewiki_codewiki__get_overview'); Claude's answer: '$answer' (the home note's title is expected)"
    }

    Test-Case "CC6" "The session-start briefing" "the hook adds CodeWiki's lines to the session" {
        if (-not $session) { throw "no session ran (CC5)" }
        $briefing = if ($session -match "(?m)^briefing: (.+)$") { $Matches[1].Trim() } else { "" }
        $notice = if ($session -match "(?m)^notice: (.+)$") { $Matches[1].Trim() } else { "" }
        Add-Result -Id "CC6" -Title "The session-start briefing" -Expect "the hook's lines start with 'CodeWiki:'" `
            -Status $(if ($briefing -match "^CodeWiki:") { "pass" } else { "fail" }) `
            -Observed "briefing: $(if ($briefing) { $briefing } else { 'none' }); Claude Code's notice about the hook: $(if ($notice) { $notice } else { 'none' })"
    }

    Test-Case "CC7" "Remove the plugin as README says" "the plugin and the marketplace are gone" {
        $run = Invoke-GuestScript -TimeoutSec 300 -Script @'
$claude = "$env:USERPROFILE\.local\bin\claude.exe"
"--- claude plugin uninstall codewiki@codewiki"; & $claude plugin uninstall codewiki@codewiki 2>&1; "uninstall exit: $LASTEXITCODE"
"--- claude plugin marketplace remove codewiki"; & $claude plugin marketplace remove codewiki 2>&1; "remove exit: $LASTEXITCODE"
"--- claude plugin list"; $list = @(& $claude plugin list 2>&1 | ForEach-Object { "$_" }); $list; "list exit: $LASTEXITCODE"
"still listed: $([bool]($list -match 'codewiki@codewiki'))"
$global:LASTEXITCODE = 0
'@
        $ok = $run.Output -match "uninstall exit: 0" -and $run.Output -match "remove exit: 0" -and $run.Output -match "still listed: False"
        Add-Result -Id "CC7" -Title "Remove the plugin as README says" -Expect "plugin and marketplace gone" `
            -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
            -Observed $(if ($ok) { "'claude plugin uninstall' and 'claude plugin marketplace remove' both succeeded; the plugin is no longer listed" } else { "not removed cleanly; see the output" })
    }

    # README's other way, for a build from source: the repository at GitHub
    # as the marketplace. Claude Code clones it, which is why it needs git.
    # It installs the plugin as the repository's default branch has it.
    Test-Case "CC8" "The repository as marketplace" "claude plugin marketplace add Fly-onlyone/CodeWiki clones it, and the plugin installs from it" {
        $run = Invoke-GuestScript -TimeoutSec 900 -Script @'
$claude = "$env:USERPROFILE\.local\bin\claude.exe"
"--- claude plugin marketplace add Fly-onlyone/CodeWiki"; & $claude plugin marketplace add Fly-onlyone/CodeWiki 2>&1; "add exit: $LASTEXITCODE"
"--- claude plugin install codewiki@codewiki"; & $claude plugin install codewiki@codewiki 2>&1; "install exit: $LASTEXITCODE"
"--- claude plugin list"; $list = @(& $claude plugin list 2>&1 | ForEach-Object { "$_" }); $list
"listed: $([bool]($list -match 'codewiki@codewiki'))"
"--- removal"; & $claude plugin uninstall codewiki@codewiki 2>&1; & $claude plugin marketplace remove codewiki 2>&1; "removal exit: $LASTEXITCODE"
$global:LASTEXITCODE = 0
'@
        $ok = $run.Output -match "add exit: 0" -and $run.Output -match "install exit: 0" -and $run.Output -match "listed: True"
        Add-Result -Id "CC8" -Title "The repository as marketplace" -Expect "cloned, and the plugin installs from it" `
            -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
            -Observed $(if ($ok) { "the repository was cloned as the marketplace and codewiki@codewiki installed from it; removed again" } else { "it did not install; see the output" })
    }

    # Whatever the cases did: the token does not stay in the guest.
    Remove-ClaudeLogin
}

# ---------------------------------------------------------------- SE, a stand-in embedding server
# CodeWiki runs no model: it asks a server for vectors (README, Search). The
# stand-in is guest\embed-server.ps1, which lists one model and answers
# POST /v1/embeddings with small deterministic vectors. It is chosen as the
# llama.cpp server (llamacpp.url), because that is the embedder that refuses a
# model its list does not hold (cw-embed, check_listed). The settings are
# changed through PUT /v1/settings, the call the Settings page makes; the page
# is photographed with them in force.
$stubPort = 9931
$stubModel = "vm-embed-stub"
$stubUrl = "http://localhost:$stubPort"
$stubFolder = "$($script:Vm.GuestDir)\embed"
$stubUp = $false

function Start-EmbedStub {
    New-GuestDirectory $stubFolder
    Copy-ToGuest -Path (Join-Path $PSScriptRoot "guest\embed-server.ps1") -GuestDirectory $script:Vm.GuestDir
    Invoke-GuestScript -Script "Remove-Item '$stubFolder\requests.log' -ErrorAction SilentlyContinue" | Out-Null
    Start-GuestExe -Exe "C:\Windows\System32\conhost.exe" -Arguments "--headless", $powershell, "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "$($script:Vm.GuestDir)\embed-server.ps1", "-Folder", $stubFolder, "-Port", "$stubPort", "-Model", $stubModel
    Wait-For 6
    $script:stubUp = $true
    (Invoke-GuestScript -Script "try { (Invoke-WebRequest -UseBasicParsing '$stubUrl/v1/models').Content } catch { 'no answer: ' + `$_.Exception.Message }").Output
}

function Stop-EmbedStub {
    Invoke-GuestScript -Script "try { Invoke-WebRequest -UseBasicParsing '$stubUrl/stop' | Out-Null } catch { }" | Out-Null
    $script:stubUp = $false
}

# How many embeddings requests the stand-in has answered, and with how many texts.
function Get-EmbedStubCount {
    $run = Invoke-GuestScript -Script "`$lines = @(Get-Content '$stubFolder\requests.log' -ErrorAction SilentlyContinue | Where-Object { `$_ -match 'POST /v1/embeddings -> 200' }); `$texts = 0; foreach (`$l in `$lines) { if (`$l -match 'texts=(\d+)') { `$texts += [int]`$Matches[1] } }; `"posts=`$(`$lines.Count) texts=`$texts`""
    if ($run.Output -match "posts=(\d+) texts=(\d+)") { [pscustomobject]@{ Posts = [int]$Matches[1]; Texts = [int]$Matches[2] } } else { throw "could not read the stand-in's log: $($run.Output)" }
}

# Choose an embedder through the backend, as the Settings page does, and
# leave the app open on Settings > Semantic search with it in force. The app
# is started first, to have a backend that knows cw-core; it is started again
# afterwards so that its page is drawn from the new settings. Returns the
# backend's answer to the change.
function Set-GuestEmbedder {
    param([Parameter(Mandatory)][string]$Provider, [string]$Model, [string]$Url)
    Stop-GuestApp
    Start-GuestApp -Arguments $guestRepo, "settings:search"
    Wait-For 20
    $body = @{ embed = @{ provider = $Provider } }
    if ($Model) { $body.embed.model = $Model }
    if ($Url) { $body[$Provider] = @{ url = $Url } }
    $encoded = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes(($body | ConvertTo-Json -Depth 5 -Compress)))
    $set = Invoke-GuestBackendScript -Script (@'
$body = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{{BODY}}')) | ConvertFrom-Json
Invoke-Backend PUT '/v1/settings' $body
'@).Replace('{{BODY}}', $encoded)
    Stop-GuestApp
    Start-GuestApp -Arguments $guestRepo, "settings:search"
    Wait-For 20
    $set.Output
}

# Guest text for a job that is waited for: starts nothing, only polls.
$jobWait = @'
$deadline = (Get-Date).AddSeconds({{SECONDS}})
do {
    Start-Sleep -Seconds 3
    $job = (Invoke-Backend GET '/v1/embed/job' | ConvertFrom-Json).job
} while ($job -and $job.phase -eq 'embedding' -and (Get-Date) -lt $deadline)
'job: ' + ($job | ConvertTo-Json -Compress)
'@

# The job object of a "job: {...}" line.
function Get-JobLine {
    param([string]$Output)
    if ($Output -match "(?m)^job: (\{.*\})\s*$") { $Matches[1] | ConvertFrom-Json } else { $null }
}

Test-Case "SE1" "A stand-in embedding server: models and vectors" "Settings shows the server's model, a test answers, a reembed stores vectors for every document, and a search embeds the query" {
    $listing = Start-EmbedStub
    $set = Set-GuestEmbedder -Provider "llamacpp" -Model $stubModel -Url $stubUrl
    $shot = Shot "settings-search-stand-in"
    $run = Invoke-GuestBackendScript -TimeoutSec 600 -Script (@'
'models: ' + (Invoke-Backend GET '/v1/settings/embed/models')
'test: ' + (Invoke-Backend POST '/v1/settings/test/embed')
'reembed: ' + (Invoke-Backend POST '/v1/embed/reembed')
'@ + "`n" + $jobWait.Replace('{{SECONDS}}', "300"))
    $job = Get-JobLine $run.Output
    $before = Get-EmbedStubCount
    $search = Invoke-GuestBackendScript -Script @'
$id = Get-RepoId 'cw-core'
$answer = Invoke-Backend GET "/v1/repos/$id/search?q=settings&limit=5"
if ($answer.StartsWith('HTTP error')) { "search: $answer" }
$parsed = if ($answer.StartsWith('[')) { $answer | ConvertFrom-Json } else { @() }
$hits = @($parsed)
"hits: $($hits.Count)"
'@
    $after = Get-EmbedStubCount
    $listed = $run.Output -match '"reachable":\s*true' -and $run.Output -match [regex]::Escape($stubModel)
    $tested = $run.Output -match '"ok":\s*true' -and $run.Output -match '"dimensions":\s*16'
    $stored = $job -and $job.phase -eq "done" -and $job.docs_total -gt 0 -and $job.docs_done -eq $job.docs_total -and $before.Texts -ge $job.docs_done
    $hits = if ($search.Output -match "hits: (\d+)") { [int]$Matches[1] } else { 0 }
    $queried = $after.Posts -gt $before.Posts
    $ok = $listed -and $tested -and $stored -and $hits -gt 0 -and $queried
    Add-Result -Id "SE1" -Title "A stand-in embedding server: models and vectors" -Expect "the model is listed; the test answers with 16 dimensions; the job is done with every document embedded; a search sends the query to the server" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($shot) -Output ("stand-in's /v1/models: $listing`nsettings change: $set`n" + $run.Output + "`n" + $search.Output) `
        -Observed "llama.cpp chosen with the model '$stubModel' at $stubUrl; the server's list shows the model: $listed; Test answered: $tested; job $(if ($job) { "$($job.phase), $($job.docs_done) of $($job.docs_total) documents" } else { 'not read' }), the stand-in answered $($before.Posts) requests with $($before.Texts) texts; a search for 'settings' gave $hits hits and the stand-in then had $($after.Posts) requests (a query is embedded): $queried; read the screenshot: Settings > Semantic search names llama.cpp and $stubModel with no warning"
    if ($Only -and $Only -notcontains "SE2") {
        Set-GuestEmbedder -Provider "local" | Out-Null
        Stop-EmbedStub
        Stop-GuestApp
    }
}

Test-Case "SE2" "A model the server does not list is refused" "llama.cpp does not list the chosen model: the test, the job and the page say so, and no embeddings request is sent" {
    if (-not $stubUp) { Start-EmbedStub | Out-Null }
    $missing = "vm-embed-missing"
    $set = Set-GuestEmbedder -Provider "llamacpp" -Model $missing -Url $stubUrl
    $shot = Shot "settings-search-model-not-listed"
    $before = Get-EmbedStubCount
    $run = Invoke-GuestBackendScript -TimeoutSec 300 -Script (@'
'test: ' + (Invoke-Backend POST '/v1/settings/test/embed')
'reembed: ' + (Invoke-Backend POST '/v1/embed/reembed')
'@ + "`n" + $jobWait.Replace('{{SECONDS}}', "120"))
    $after = Get-EmbedStubCount
    $job = Get-JobLine $run.Output
    $message = "llama.cpp does not list $missing; choose a model it serves"
    $tested = $run.Output -match '"ok":\s*false' -and $run.Output.Contains($message)
    $failed = $job -and $job.phase -eq "failed" -and "$($job.error)".Contains($message)
    $quiet = $after.Posts -eq $before.Posts
    Add-Result -Id "SE2" -Title "A model the server does not list is refused" -Expect "'$message' from the test and the job; no embeddings request reaches the server" `
        -Status $(if ($tested -and $failed -and $quiet) { "pass" } else { "fail" }) -Shots @($shot) -Output ("settings change: $set`n" + $run.Output) `
        -Observed "chose '$missing', which the stand-in does not list; Test refused with the message: $tested; the job ended $(if ($job) { $job.phase } else { 'unread' }) with it: $failed; embeddings requests at the stand-in before $($before.Posts), after $($after.Posts); read the screenshot: the page shows a warning that llama.cpp does not list $missing, and keeps it selected"
    Set-GuestEmbedder -Provider "local" | Out-Null
    Stop-EmbedStub
    Stop-GuestApp
}

# ---------------------------------------------------------------- OL, a real Ollama
# README names Ollama and `ollama pull embeddinggemma` as one way to serve
# embeddings. These cases do exactly that in the guest, which has a network.
$ollamaExe = "$guestHome\AppData\Local\Programs\Ollama\ollama.exe"
$ollamaDownload = "https://ollama.com/download/OllamaSetup.exe"

# Ask the guest which Ollama version answers on its port; empty when none does.
function Get-OllamaVersion {
    (Invoke-GuestScript -TimeoutSec 60 -Script "try { (Invoke-RestMethod -TimeoutSec 5 'http://localhost:11434/api/version').version } catch { }").Output.Trim()
}

# OllamaSetup.exe is an Inno Setup installer that needs no administrator
# (github.com/ollama/ollama, app/ollama.iss: PrivilegesRequired=lowest,
# DefaultDirName={localappdata}\Programs\Ollama; docs/windows.mdx: "The Ollama
# install does not require Administrator, and installs in your home directory
# by default", and the installer takes /DIR). The download is
# https://ollama.com/download/OllamaSetup.exe, which redirects to the newest
# release's asset on GitHub (about 1.8 GB). Inno Setup's own switches make it
# silent: /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-. The docs name no
# silent switch of Ollama's own. When the installer ends it starts the tray
# app, which serves on http://localhost:11434.
Test-Case "OL1" "Install Ollama silently" "OllamaSetup.exe from ollama.com installs for the user, and the server answers on port 11434" {
    $download = Invoke-GuestScript -Background -TimeoutSec 2400 -Script (@'
$ProgressPreference = 'SilentlyContinue'
$free = [math]::Round((Get-PSDrive C).Free / 1GB, 1)
"free disk GB: $free"
if ($free -lt 8) { 'result: not enough disk'; return }
$folder = '{{HOME}}\Downloads\ollama'
New-Item -ItemType Directory -Force $folder | Out-Null
$setup = Join-Path $folder 'OllamaSetup.exe'
$watch = Get-Date
Invoke-WebRequest -UseBasicParsing '{{URL}}' -OutFile $setup
"downloaded: $([math]::Round((Get-Item $setup).Length / 1MB)) MB in $([int]((Get-Date) - $watch).TotalSeconds) s"
$signature = Get-AuthenticodeSignature $setup
"signature: $($signature.Status) $($signature.SignerCertificate.Subject)"
'result: downloaded'
'@).Replace('{{HOME}}', $guestHome).Replace('{{URL}}', $ollamaDownload)
    if ($download.Output -notmatch "result: downloaded") {
        Add-Result -Id "OL1" -Title "Install Ollama silently" -Expect "the installer downloads" -Status fail -Output $download.Output `
            -Observed "the installer was not downloaded: $(($download.Output -split "`n" | Select-Object -Last 2) -join '; ')"
        return
    }
    $setup = "$guestHome\Downloads\ollama\OllamaSetup.exe"
    $watch = Get-Date
    # Not Invoke-GuestExe: the installer starts the tray app, which would keep
    # the output pipe it waits on open for as long as the app runs.
    Start-GuestExe -Exe $setup -Arguments "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-"
    $installed = $false
    $deadline = (Get-Date).AddMinutes(15)
    while ((Get-Date) -lt $deadline -and -not $installed) {
        Start-Sleep -Seconds 10
        $state = (Invoke-GuestScript -TimeoutSec 60 -Script "`"setup running: `$(@(Get-Process 'OllamaSetup*' -ErrorAction SilentlyContinue).Count)`"; `"exe: `$(Test-Path '$ollamaExe')`"").Output
        $installed = $state -match "setup running: 0" -and $state -match "exe: True"
    }
    $seconds = [int]((Get-Date) - $watch).TotalSeconds
    # The tray app is up a few seconds after the install, but its server took
    # longer than 15 s to answer in the first run.
    $version = ""
    $deadline = (Get-Date).AddSeconds(120)
    while (-not $version -and (Get-Date) -lt $deadline) {
        Wait-For 10
        $version = Get-OllamaVersion
    }
    $facts = (Invoke-GuestScript -TimeoutSec 120 -Script (@'
$exe = '{{EXE}}'
"file version: $((Get-Item $exe -ErrorAction SilentlyContinue).VersionInfo.ProductVersion)"
"on user PATH: $(([Environment]::GetEnvironmentVariable('Path', 'User') -split ';') -contains (Split-Path $exe))"
"processes: $((Get-Process | Where-Object { $_.ProcessName -like '*ollama*' } | ForEach-Object { $_.ProcessName }) -join ', ')"
$events = @(Get-WinEvent -FilterHashtable @{ LogName = 'Microsoft-Windows-Windows Defender/Operational'; Id = 1121, 1122, 1116, 1117 } -ErrorAction SilentlyContinue | Where-Object { $_.Message -match 'ollama' })
"Defender events naming Ollama: $($events.Count)"
foreach ($event in $events | Select-Object -First 4) { "  event $($event.Id): $(($event.Message -split "`r?`n" | Where-Object { $_ -match '^\s*(ID|Path|Name):' } | ForEach-Object { $_.Trim() }) -join ' | ')" }
'@).Replace('{{EXE}}', $ollamaExe)).Output
    $shot = Shot "ollama-installed"
    Add-Result -Id "OL1" -Title "Install Ollama silently" -Expect "the installer runs with /VERYSILENT as the user, ollama.exe is in place and the server answers on 11434" `
        -Status $(if ($installed -and $version) { "pass" } elseif ($installed) { "note" } else { "fail" }) -Shots @($shot) -Output ($download.Output + "`n--- after the install`n" + $facts) `
        -Observed "$ollamaDownload; $(($download.Output -split "`n" | Where-Object { $_ -match '^(downloaded|signature)' }) -join '; '); /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- ended after $seconds s with ollama.exe in place: $installed; the server answers on 11434 with version '$version' (empty: it did not start by itself; see the output for what runs and for any Defender event that names Ollama)"
}

Test-Case "OL2" "ollama pull embeddinggemma" "the model is pulled as README says and the server lists it" {
    if (-not (Get-OllamaVersion)) {
        # The tray app did not start, or the guest was restored since OL1.
        Start-GuestExe -Exe "C:\Windows\System32\conhost.exe" -Arguments "--headless", $ollamaExe, "serve"
        Wait-For 15
    }
    if (-not (Get-OllamaVersion)) { throw "no Ollama answers on port 11434; run OL1 first" }
    $run = Invoke-GuestScript -Background -TimeoutSec 2700 -Script (@'
$watch = Get-Date
$text = & '{{EXE}}' pull embeddinggemma 2>&1 | Out-String
$seconds = [int]((Get-Date) - $watch).TotalSeconds
"pull exit: $LASTEXITCODE after $seconds s"
$clean = ($text -replace '[^\x20-\x7E\r\n]', '') -split "`r?`n|`r" | Where-Object { $_.Trim() } | Select-Object -Last 4
"pull output ends: $($clean -join ' | ')"
$tags = (Invoke-RestMethod -TimeoutSec 20 'http://localhost:11434/api/tags').models | Where-Object { $_.name -like 'embeddinggemma*' }
foreach ($tag in $tags) { "model: $($tag.name), $([math]::Round($tag.size / 1MB)) MB" }
$folder = Join-Path $env:USERPROFILE '.ollama\models'
"models folder: $([math]::Round(((Get-ChildItem $folder -Recurse -File -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum) / 1MB)) MB"
$global:LASTEXITCODE = 0
'@).Replace('{{EXE}}', $ollamaExe)
    $ok = $run.Output -match "pull exit: 0" -and $run.Output -match "model: embeddinggemma"
    Add-Result -Id "OL2" -Title "ollama pull embeddinggemma" -Expect "pull exits 0 and /api/tags lists embeddinggemma" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
        -Observed "$(($run.Output -split "`n" | Where-Object { $_ -match '^(pull exit|model:|models folder)' }) -join '; ')"
}

Test-Case "OL3" "Semantic search through Ollama" "Ollama and embeddinggemma chosen: the page lists the model, a reembed stores every document's vector, and a search by meaning answers" {
    if (-not (Get-OllamaVersion)) { throw "no Ollama answers on port 11434; run OL1 and OL2 first" }
    $set = Set-GuestEmbedder -Provider "ollama" -Model "embeddinggemma"
    $shot = Shot "settings-search-ollama"
    $watch = Get-Date
    $run = Invoke-GuestBackendScript -TimeoutSec 1500 -Script (@'
'models: ' + (Invoke-Backend GET '/v1/settings/embed/models')
'test: ' + (Invoke-Backend POST '/v1/settings/test/embed' $null 600)
'reembed: ' + (Invoke-Backend POST '/v1/embed/reembed')
'@ + "`n" + $jobWait.Replace('{{SECONDS}}', "900"))
    $seconds = [int]((Get-Date) - $watch).TotalSeconds
    $job = Get-JobLine $run.Output
    # A search by meaning: the words of the query are not the words of the
    # code's notes. `codewiki search` is full text alone; the backend's route
    # adds the vectors. A hit that only the route finds comes from them.
    $query = "protecting stored credentials"
    $meaning = Invoke-GuestBackendScript -TimeoutSec 600 -Script (@'
$id = Get-RepoId 'cw-core'
$answer = Invoke-Backend GET "/v1/repos/$id/search?q={{QUERYURL}}&limit=10"
if ($answer.StartsWith('HTTP error')) { "search: $answer" }
$parsed = if ($answer.StartsWith('[')) { $answer | ConvertFrom-Json } else { @() }
$hits = @($parsed)
"hybrid hits: $(@($hits | ForEach-Object { $_.key }) -join ' ; ')"
Set-Location '{{REPO}}'
# In 5.1 a line on stderr through 2>&1 is an error record, which 'Stop' ends the script on.
$ErrorActionPreference = 'Continue'
$lexical = @(& '{{CODEWIKI}}' search '{{QUERY}}' --limit 10 2>&1 | ForEach-Object { "$_" } | Where-Object { $_ -match '^\[' } | ForEach-Object { ($_ -replace '^\[[^\]]*\]\s*', '') -split ' . ' | Select-Object -First 1 })
"lexical hits: $($lexical -join ' ; ')"
$global:LASTEXITCODE = 0
'@).Replace("{{QUERYURL}}", ($query -replace " ", "%20")).Replace("{{QUERY}}", $query).Replace('{{REPO}}', $guestRepo).Replace('{{CODEWIKI}}', "$guestTarget\codewiki.exe")
    $hybrid = if ($meaning.Output -match "(?m)^hybrid hits: (.*)$") { @($Matches[1] -split " ; " | Where-Object { $_.Trim() }) } else { @() }
    $lexical = if ($meaning.Output -match "(?m)^lexical hits: (.*)$") { @($Matches[1] -split " ; " | Where-Object { $_.Trim() }) } else { @() }
    $onlyVectors = @($hybrid | Where-Object { $lexical -notcontains $_ })
    $memory = (Invoke-GuestScript -TimeoutSec 120 -Script @'
$processes = @(Get-Process | Where-Object { $_.ProcessName -like '*ollama*' -or $_.ProcessName -eq 'runner' })
foreach ($p in $processes) { "process: $($p.ProcessName) $([math]::Round($p.WorkingSet64 / 1MB)) MB" }
"total: $([math]::Round((($processes | Measure-Object WorkingSet64 -Sum).Sum) / 1MB)) MB of $([math]::Round((Get-CimInstance Win32_OperatingSystem).TotalVisibleMemorySize / 1KB)) MB"
"loaded: $((Invoke-RestMethod -TimeoutSec 10 'http://localhost:11434/api/ps').models | ForEach-Object { $_.name + ' ' + [math]::Round($_.size / 1MB) + ' MB' })"
'@).Output
    # Back to what the other cases expect: no Ollama, search by words alone.
    Set-GuestEmbedder -Provider "local" | Out-Null
    Stop-GuestApp
    $stopped = (Invoke-GuestScript -TimeoutSec 120 -Script @'
Get-Process | Where-Object { $_.ProcessName -like '*ollama*' -or $_.ProcessName -eq 'runner' } | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep 3
"left: $(@(Get-Process | Where-Object { $_.ProcessName -like '*ollama*' }).Count)"
'@).Output
    $listed = $run.Output -match '"reachable":\s*true' -and $run.Output -match 'embeddinggemma'
    $tested = $run.Output -match '"ok":\s*true'
    $stored = $job -and $job.phase -eq "done" -and $job.docs_total -gt 0 -and $job.docs_done -eq $job.docs_total
    $ok = $listed -and $tested -and $stored -and $hybrid.Count -gt 0
    Add-Result -Id "OL3" -Title "Semantic search through Ollama" -Expect "the model is listed; the test answers; the job is done with every document embedded; the route finds hits" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($shot) -Output ("settings change: $set`n" + $run.Output + "`n" + $meaning.Output + "`n--- memory while the model was loaded`n$memory`n--- after stopping Ollama`n$stopped") `
        -Observed "Ollama chosen with embeddinggemma; the server's list shows it: $listed; Test answered: $tested; job $(if ($job) { "$($job.phase), $($job.docs_done) of $($job.docs_total) documents" } else { 'not read' }) in $seconds s (test, reembed and wait); search '$query': $($hybrid.Count) hits with vectors, $($lexical.Count) by words alone, $($onlyVectors.Count) found only with the vectors$(if ($onlyVectors) { ': ' + ($onlyVectors -join ', ') }); Ollama's memory with the model loaded: $(($memory -split "`n" | Where-Object { $_ -match '^total:' }) -join ''); $stopped; read the screenshot: Settings > Semantic search names Ollama and embeddinggemma"
}

# ---------------------------------------------------------------- K, uninstall
# uninstall.cmd, as a double-click runs it, while the app and backend run and
# the start with Windows is on (AS3).
Test-Case "K1" "Uninstall while the app runs" "processes stopped; programs, PATH entry and shortcut removed" {
    $before = Get-Probe "before-uninstall" -Quick
    $exe = Invoke-GuestExe -Exe "C:\Windows\System32\cmd.exe" -TimeoutSec 120 -Arguments "/c", "$guestTarget\uninstall.cmd"
    $run = [pscustomobject]@{ ExitCode = $exe.ExitCode; Output = ($exe.Stdout + "`n" + $exe.Stderr).Trim() }
    Wait-For 3
    $shot = Shot "after-uninstall"
    $probe = Get-Probe "uninstalled"
    $left = @($probe.install.files | Where-Object { $_ } | ForEach-Object { $_.path })
    $problems = @()
    if ($probe.processes.Count) { $problems += "$($probe.processes.Count) CodeWiki processes still run" }
    if ($left | Where-Object { $_ -like "*.exe" }) { $problems += "programs left behind" }
    if ($probe.userPath.targetCount) { $problems += "PATH entry left" }
    if ($probe.shortcut) { $problems += "shortcut left" }
    if ($probe.registry.autostart) { $problems += "the start with Windows is left: $($probe.registry.autostart)" }
    if ($probe.registry.appIdentity) { $problems += "the app's name for notifications is left in the registry" }
    Add-Result -Id "K1" -Title "Uninstall while the app runs" -Expect "processes stopped; programs, PATH entry, shortcut, start with Windows and notification name removed" `
        -Status $(if ($problems -or $run.ExitCode -ne 0) { "fail" } else { "pass" }) -Shots @($shot) -Output $run.Output `
        -Observed "exit $($run.ExitCode); running before: $($before.processes.Count) processes; start with Windows before: '$($before.registry.autostart)', notification name before: $($before.registry.appIdentity); $(if ($problems) { $problems -join '; ' } else { 'removed' }); left in the install folder: $(if ($left) { $left -join ', ' } else { 'nothing' }); data folder files: $(Get-FileCount $probe.dataDir.files); settings folder files: $(Get-FileCount $probe.prefsDir.files)"
}

Test-Case "K2" "Reinstall, then uninstall with -PurgeData" "the data and settings folders are removed" {
    $cmd = "C:\Windows\System32\cmd.exe"
    $install = Invoke-GuestExe -Exe $cmd -TimeoutSec 120 -Arguments "/c", "$guestUnpacked\install.cmd"
    $run = Invoke-GuestExe -Exe $cmd -TimeoutSec 120 -Arguments "/c", "$guestTarget\uninstall.cmd", "-PurgeData"
    $probe = Get-Probe "purged"
    $dataLeft = Get-FileCount $probe.dataDir.files
    $prefsLeft = @($probe.prefsDir.files | Where-Object { $_ } | ForEach-Object { $_.path })
    $ok = $dataLeft -eq 0 -and $prefsLeft.Count -eq 0 -and $run.ExitCode -eq 0 -and $install.ExitCode -eq 0
    Add-Result -Id "K2" -Title "Reinstall, then uninstall with -PurgeData" -Expect "the data and settings folders are removed" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Output ($install.Stdout + "`n" + $install.Stderr + "`n" + $run.Stdout + "`n" + $run.Stderr).Trim() `
        -Observed "install exit $($install.ExitCode), uninstall exit $($run.ExitCode); files left in %LOCALAPPDATA%\CodeWiki: $dataLeft; left in %APPDATA%\CodeWiki: $(if ($prefsLeft) { $prefsLeft -join ', ' } else { 'nothing' })"
}

# ---------------------------------------------------------------- U2, self-update
# Ends the app and removes the installation U2 made, data included.
function Remove-UpdateTestInstall {
    Stop-GuestApp
    Invoke-GuestExe -Exe "C:\Windows\System32\cmd.exe" -TimeoutSec 120 -Arguments "/c", "$guestTarget\uninstall.cmd", "-PurgeData" | Out-Null
}

if ($OldZip) {
    Test-Case "U2" "The app updates itself" "an older installed version is offered the newer one, installs it and comes back as it" {
        $oldVersion = Get-ZipVersion $OldZip
        $newVersion = Get-ZipVersion $Zip
        $guestOld = "$guestHome\Downloads\old"
        $cmd = "C:\Windows\System32\cmd.exe"
        $version = "(& '$guestTarget\codewiki.exe' --version 2>&1 | Out-String).Trim()"
        New-GuestDirectory $guestOld
        Copy-ToGuest -Path $OldZip -GuestDirectory $guestOld
        Invoke-GuestScript -Script "Expand-Archive -Path '$guestOld\$(Split-Path -Leaf $OldZip)' -DestinationPath '$guestOld\CodeWiki' -Force" | Out-Null
        $install = Invoke-GuestExe -Exe $cmd -TimeoutSec 120 -Arguments "/c", "$guestOld\CodeWiki\install.cmd"
        # The older build is other files than the one under test, and Defender
        # judges each file by itself.
        $oldStart = if ($Hardened) { Resolve-GuestRefusal } else { $null }
        $was = (Invoke-GuestScript -Script $version).Output.Trim()
        # What 0.2.0 left in the data folder of a user of its AI writer and of its
        # built-in model: the sections `llm`, `anthropic` and `cliproxy` of
        # settings.json (a secret is {dpapi, hint}; these two are not keys), other
        # settings, a section no version knows, and the `cuda` folder of the
        # built-in model's worker. The 0.3.0 backend and updater must clean the
        # first and last and leave the rest (case U3).
        $legacy = @'
{
  "version": 1,
  "llm": { "provider": "anthropic", "model": "vm-test-model" },
  "anthropic": { "api_key": { "dpapi": "dm10ZXN0LW5vdC1hLWtleQ==", "hint": "-key" } },
  "cliproxy": { "url": "http://127.0.0.1:8317", "api_key": { "dpapi": "dm10ZXN0LW5vdC1hLWtleQ==", "hint": "-key" } },
  "embed": { "provider": "local" },
  "ollama": { "host": "http://127.0.0.1:11999" },
  "future": { "kept": true }
}
'@
        $encodedLegacy = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($legacy))
        Invoke-GuestScript -Script (@'
$data = Join-Path $env:LOCALAPPDATA 'CodeWiki'
New-Item -ItemType Directory -Force $data | Out-Null
[System.IO.File]::WriteAllText((Join-Path $data 'settings.json'), [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{{JSON}}')))
New-Item -ItemType Directory -Force '{{TARGET}}\cuda' | Out-Null
# The worker is what marks the folder as the app's own: recover_and_clean in
# cw-update leaves a cuda folder without it alone.
Set-Content '{{TARGET}}\cuda\codewiki-embed-cuda.exe' 'not a program'
Set-Content '{{TARGET}}\cuda\vmtest.dll' 'not a library'
'@).Replace('{{JSON}}', $encodedLegacy).Replace('{{TARGET}}', $guestTarget) | Out-Null
        $script:legacySeeded = $true
        $serve = "$($script:Vm.GuestDir)\update"
        if (-not $RealRelease) {
            # The stand-in for GitHub: it serves the package under test.
            New-GuestDirectory $serve
            Copy-ToGuest -Path $Zip -GuestDirectory $serve
            Copy-ToGuest -Path (Join-Path $PSScriptRoot "guest\update-server.ps1") -GuestDirectory $script:Vm.GuestDir
            Invoke-GuestScript -Script @"
`$zip = Get-ChildItem '$serve' -Filter 'CodeWiki-*-windows-x64.zip' | Select-Object -First 1
`$hash = (Get-FileHash -Algorithm SHA256 `$zip.FullName).Hash.ToLowerInvariant()
[System.IO.File]::WriteAllText(`$zip.FullName + '.sha256', `$hash + '  ' + `$zip.Name + [char]10)
[Environment]::SetEnvironmentVariable('CODEWIKI_UPDATE_URL', 'http://localhost:8765', 'User')
"@ | Out-Null
            Start-GuestExe -Exe "C:\Windows\System32\conhost.exe" -Arguments "--headless", $powershell, "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "$($script:Vm.GuestDir)\update-server.ps1", "-Folder", $serve
            Wait-For 6
        }
        # The app checks when it starts; Home then offers the update.
        Start-GuestApp -Arguments $guestRepo
        Wait-For 25
        $a = Shot "update-offered"
        Send-Keys "ctrl+k"; Wait-For 2
        Send-Text "update codewiki"; Wait-For 3
        $b = Shot "update-in-palette"
        Send-Keys "enter"
        Wait-For 75
        $c = Shot "update-done"
        $now = (Invoke-GuestScript -Script $version).Output.Trim()
        $probe = Get-Probe "updated" -Quick
        $window = @(Get-App $probe | Where-Object { $_.hasWindow })
        $left = (Invoke-GuestScript -Script "(Get-ChildItem '$guestTarget' -Recurse -Force | Where-Object { `$_.Name -match '\.old\d*$' -or `$_.Name -eq '.update' } | ForEach-Object { `$_.Name }) -join ', '").Output.Trim()
        $asked = ""
        if (-not $RealRelease) {
            $asked = (Invoke-GuestScript -Script "Get-Content '$serve\requests.log' -ErrorAction SilentlyContinue | Select-Object -Last 12").Output
            Invoke-GuestScript -Script "[Environment]::SetEnvironmentVariable('CODEWIKI_UPDATE_URL', `$null, 'User'); try { Invoke-WebRequest -UseBasicParsing 'http://localhost:8765/stop' | Out-Null } catch { }" | Out-Null
        }
        $log = (Invoke-GuestScript -Script "Get-Content `"`$env:LOCALAPPDATA\CodeWiki\logs\app.log`" -ErrorAction SilentlyContinue | Select-String 'update|ERROR|WARN' | Select-Object -Last 12 | ForEach-Object { `$_.Line }").Output
        # One app: the new one must take the place of the one it replaced,
        # not hand over to it while it ends.
        $ok = $install.ExitCode -eq 0 -and $was -match [regex]::Escape($oldVersion) -and $now -match [regex]::Escape($newVersion) -and $window.Count -eq 1 -and (Get-App $probe).Count -eq 1 -and -not $left
        Add-Result -Id "U2" -Title "The app updates itself" -Expect "codewiki --version changes from $oldVersion to $newVersion; the app is running again; no .old file or .update folder is left" `
            -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($a, $b, $c) -Output ("requests the stand-in saw:`n" + $asked + "`n--- app.log`n" + $log).Trim() `
            -Observed "installed $oldVersion (exit $($install.ExitCode))$(if ($oldStart -and $oldStart.Status -ne 'started') { ", which Windows refused to start until the rules $($oldStart.Rules -join '; ') were in audit" }), codewiki --version said '$was'; update source: $(if ($RealRelease) { 'GitHub' } else { 'the stand-in in the guest' }); palette: 'update codewiki', Enter; 75 s later codewiki --version said '$now', app processes: $((Get-App $probe).Count), app windows: $($window.Count), left over in the install folder: $(if ($left) { $left } else { 'nothing' })"
        $script:updatedInstall = $ok
        # U3 looks at what the update left, then clears up.
        if ($Only -and $Only -notcontains "U3") { Remove-UpdateTestInstall }
    }

    # What 0.3.0 did to the data folder and the install folder that 0.2.0 left
    # (commits 9b57fc9 and b2744d9: retired writer settings and the built-in
    # model). Two parts. The update has just started the new backend on the
    # seeded settings.json: the writer's sections must be gone, the rest kept.
    # Then the app is quit, a built-in embedder choice as 0.2.0 saved it and
    # downloaded model files are put in place, and the next start must move the
    # choice to `local` and delete the models.
    Test-Case "U3" "The update cleans up what the old version left" "the retired writer settings, the cuda folder and the built-in model are gone; other settings stay; the app runs" {
        if (-not $script:legacySeeded) { throw "U2 did not seed the old settings; run U2 and U3 together" }
        try {
            $read = (Invoke-GuestScript -Script (@'
$data = Join-Path $env:LOCALAPPDATA 'CodeWiki'
'cuda folder: ' + (Test-Path '{{TARGET}}\cuda')
'settings.json: ' + (Test-Path (Join-Path $data 'settings.json'))
'--- file'
Get-Content (Join-Path $data 'settings.json') -Raw -ErrorAction SilentlyContinue
'--- backend.log'
Get-Content (Join-Path $env:LOCALAPPDATA 'CodeWiki\logs\backend.log') -ErrorAction SilentlyContinue | Select-String 'retired|settings|built-in|embedder' | Select-Object -Last 6 | ForEach-Object { $_.Line }
'@).Replace('{{TARGET}}', $guestTarget)).Output
            $file = if ($read -match "(?s)--- file\r?\n(.*?)\r?\n--- backend\.log") { $Matches[1] } else { "" }
            $doc = try { $file | ConvertFrom-Json } catch { $null }
            $names = if ($doc) { @($doc.PSObject.Properties.Name) } else { @() }
            $retiredLeft = @("llm", "anthropic", "cliproxy" | Where-Object { $names -contains $_ })
            $keyLeft = $file -match "dm10ZXN0|vm-test-model|8317"
            $kept = $doc -and $doc.ollama.host -eq "http://127.0.0.1:11999" -and $doc.embed.provider -eq "local" -and $doc.future.kept -eq $true
            $cudaGone = $read -match "cuda folder: False"
            $probe = Get-Probe "after-update" -Quick
            $running = (Get-App $probe).Count -eq 1 -and [bool]($probe.backend -and $probe.backend.health -eq 200)
            $first = $doc -and -not $retiredLeft -and -not $keyLeft -and $kept -and $cudaGone -and $running

            # Part two: the built-in model's choice and files.
            [void](Show-GuestWindow -ProcessName "codewiki-app")
            $quit = Exit-GuestApp
            $builtin = '{ "version": 1, "embed": { "provider": "builtin", "model": "gte-modernbert-base", "device": "cuda" }, "ollama": { "host": "http://127.0.0.1:11999" } }'
            $encodedBuiltin = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($builtin))
            Invoke-GuestScript -Script (@'
$data = Join-Path $env:LOCALAPPDATA 'CodeWiki'
[System.IO.File]::WriteAllText((Join-Path $data 'settings.json'), [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{{JSON}}')))
New-Item -ItemType Directory -Force (Join-Path $data 'models\vmtest-model'), (Join-Path $data 'models\.partial') | Out-Null
Set-Content (Join-Path $data 'models\vmtest-model\manifest.json') '{}'
Set-Content (Join-Path $data 'models\vmtest-model\weights.bin') 'not a model'
Set-Content (Join-Path $data 'models\.partial\download.bin') 'not a model'
Set-Content (Join-Path $data 'models\keep.txt') 'a file the built-in store did not write'
'@).Replace('{{JSON}}', $encodedBuiltin) | Out-Null
            Start-GuestApp -Arguments $guestRepo
            Wait-For 25
            $shot = Shot "after-builtin-migration"
            $after = (Invoke-GuestScript -Script @'
$data = Join-Path $env:LOCALAPPDATA 'CodeWiki'
'--- file'
Get-Content (Join-Path $data 'settings.json') -Raw -ErrorAction SilentlyContinue
'--- models'
(Get-ChildItem (Join-Path $data 'models') -Force -ErrorAction SilentlyContinue | ForEach-Object { $_.Name }) -join ', '
'--- backend.log'
Get-Content (Join-Path $data 'logs\backend.log') -ErrorAction SilentlyContinue | Select-String 'built-in|embedder' | Select-Object -Last 4 | ForEach-Object { $_.Line }
'@).Output
            $file2 = if ($after -match "(?s)--- file\r?\n(.*?)\r?\n--- models") { $Matches[1] } else { "" }
            $doc2 = try { $file2 | ConvertFrom-Json } catch { $null }
            $models = if ($after -match "(?m)^--- models\r?\n(.*)$") { $Matches[1].Trim() } else { "?" }
            $moved = $doc2 -and $doc2.embed.provider -eq "local" -and $null -eq $doc2.embed.model -and $null -eq $doc2.embed.device -and $doc2.ollama.host -eq "http://127.0.0.1:11999"
            $deleted = $models -notmatch "vmtest-model|\.partial" -and $models -match "keep\.txt"
            $ok2 = $quit -and $moved -and $deleted
            Add-Result -Id "U3" -Title "The update cleans up what the old version left" -Expect "writer sections, cuda folder, built-in choice and models are gone; other settings stay; the app runs" `
                -Status $(if ($first -and $ok2) { "pass" } else { "fail" }) -Shots @($shot) -Output ($read + "`n=== after the built-in choice`n" + $after) `
                -Observed "after the update from 0.2.0: sections left of llm/anthropic/cliproxy: $(if ($retiredLeft) { $retiredLeft -join ', ' } else { 'none' }); seeded values left in the file: $([bool]$keyLeft); ollama.host, embed.provider and the unknown section kept: $([bool]$kept); cuda folder removed: $([bool]$cudaGone); app running with a healthy backend: $running. Then Quit (ended by itself: $quit), a saved built-in choice (provider builtin, model, device cuda) and model files written, and a plain start: embed.provider is '$($doc2.embed.provider)', model '$($doc2.embed.model)', device '$($doc2.embed.device)', ollama.host kept: $([bool]($doc2.ollama.host)); files left in models: '$models' (keep.txt is not the built-in store's and stays)"
        } finally {
            Remove-UpdateTestInstall
        }
    }
}

# Defender writes down each thing it stops: a detection (1116, 1117), an
# attack surface reduction rule (1121), controlled folder access (1123, 1127),
# network protection (1126). Read at the end, for the whole run.
if ($Hardened) {
    Test-Case "S2" "What Windows protection stopped" "nothing of CodeWiki" {
        $run = Invoke-GuestScript -Background -TimeoutSec 180 -Script @'
$ids = 1006, 1007, 1008, 1015, 1116, 1117, 1118, 1119, 1121, 1122, 1123, 1126, 1127
$events = @(Get-WinEvent -FilterHashtable @{ LogName = "Microsoft-Windows-Windows Defender/Operational"; Id = $ids } -ErrorAction SilentlyContinue)
"recorded: $($events.Count)"
# The same block is written down every time it happens: one line for each
# kind, with how often.
$kinds = [ordered]@{}
foreach ($event in $events | Sort-Object TimeCreated) {
    $what = ($event.Message -split "`r?`n" | ForEach-Object { $_.Trim() } | Where-Object { $_ -match "^(ID|Path|Name|Process Name|Threat|Destination):" }) -join " | "
    $kind = "event $($event.Id): $what"
    if ($kinds.Contains($kind)) { $kinds[$kind] += 1 } else { $kinds[$kind] = 1 }
}
foreach ($kind in $kinds.Keys) { "$($kinds[$kind])x $kind" }
'@
        if ($run.Output -notmatch "recorded: (\d+)") { throw "the guest did not answer: $($run.Stderr)" }
        $count = [int]$Matches[1]
        # 1122 is a rule in audit: it says what the rule would have stopped.
        $ours = @($run.Output -split "`n" | Where-Object { $_ -match "event \d+:" -and $_ -match "codewiki" })
        $audited = @($ours | Where-Object { $_ -match "event 1122:" })
        # A block of CodeWiki that S3 did not already record is new.
        $new = @($ours | Where-Object { $line = $_; $line -notmatch "event 1122:" -and -not ($auditedRules | Where-Object { $line -match [regex]::Escape($_) }) })
        Add-Result -Id "S2" -Title "What Windows protection stopped" -Expect "nothing of CodeWiki beyond what S3 recorded" `
            -Status $(if ($new) { "fail" } elseif ($count) { "note" } else { "pass" }) -Output $run.Output `
            -Observed "Defender recorded $count events in this run. Kinds that name CodeWiki: $($ours.Count), of which $($audited.Count) are rules in audit (what they would have stopped) and $($new.Count) are blocks that S3 did not record. See the output for each"
    }
}

Test-Case "M1" "Windows event log" "no crash of a CodeWiki program was logged" {
    $probe = Get-Probe "final"
    $events = @($probe.events)
    Add-Result -Id "M1" -Title "Windows event log" -Expect "no crash of a CodeWiki program was logged" `
        -Status $(if ($events.Count) { "fail" } else { "pass" }) -Shots @(Shot "final") `
        -Observed "$($events.Count) Application-log entries name CodeWiki" -Output (($events | ForEach-Object { "$($_.time) $($_.provider) $($_.id): $($_.message)" }) -join "`n`n")
}

# ---------------------------------------------------------------- wrap up
Stop-Recording
Get-VM -Name $VmName | Select-Object Name, Generation, ProcessorCount, @{ n = "MemoryMb"; e = { $_.MemoryStartup / 1MB } }, Version |
    ConvertTo-Json | Set-Content (Join-Path $logs "vm.json")
if (-not $KeepRunning) { Stop-Guest }

$counts = $results | Group-Object status | ForEach-Object { "$($_.Count) $($_.Name)" }
"done: $($counts -join ', '); evidence in $Report"
