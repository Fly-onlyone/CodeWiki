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
    if (-not $driver.Match) {
        throw "the guest holds the graphics driver '$($driver.GuestDrivers -join ', ')' and the host now has '$($driver.HostDriver)'; run scripts\vm\provision-gpu.ps1 -Force"
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
Test-Case "C1" "Unpack the zip" "Expand-Archive in Windows PowerShell 5.1 gives all ten entries" {
    $run = Invoke-GuestScript -Script @"
Expand-Archive -Path '$guestZip' -DestinationPath '$guestUnpacked' -Force
Get-ChildItem '$guestUnpacked' | ForEach-Object { `$_.Name }
"@
    $names = @($run.Output -split "`n" | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    $needed = "codewiki.exe", "codewiki-mcp.exe", "codewiki-app.exe", "LICENSE", "NOTICE", "install.ps1", "uninstall.ps1", "install.cmd", "uninstall.cmd", "plugins"
    $missing = @($needed | Where-Object { $names -notcontains $_ })
    Add-Result -Id "C1" -Title "Unpack the zip" -Expect "all ten entries" -Status $(if ($missing) { "fail" } else { "pass" }) `
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
if ($Hardened) {
    Test-Case "S3" "The programs start under the protection" "recorded: whether Windows lets codewiki.exe start" {
        $ruleNames = @{
            "01443614-cd74-433a-b99e-2ecdc07bfc25" = "Block executable files from running unless they meet a prevalence, age, or trusted list criterion"
            "c1db55ab-c21a-4637-bb3f-a12568109d35" = "Use advanced protection against ransomware"
        }
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
        $named = @($auditedRules | ForEach-Object { if ($ruleNames[$_]) { "'$($ruleNames[$_])' ($_)" } else { $_ } })
        $status = if ($first -match "start: exit 0") { "pass" } elseif ($last -match "start: exit 0") { "note" } else { "fail" }
        $observed = switch ($status) {
            "pass" { "codewiki.exe started with every rule on block" }
            "note" { "Windows refused to start CodeWiki's programs ($((($first -split "`n") | Where-Object { $_ -match '^codewiki\S*: refused,' } | ForEach-Object { ($_ -split ':')[0] }) -join ', ')): 'Access is denied'. Set to audit so that the run can go on: $($named -join '; '). With that the programs start. These rules are off unless an administrator turns them on; where they are on, a new unsigned build does not run" }
            default { "Windows refuses to start CodeWiki's programs, and setting the rules that Defender names to audit did not change it; the cases after this cannot be judged. See the output" }
        }
        Add-Result -Id "S3" -Title "The programs start under the protection" -Expect "recorded" -Status $status -Output $log -Observed $observed
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

Test-Case "TR4" "The tray menu" "a right click opens it: what the app is doing, then the commands, Quit last" {
    $icon = Send-GuestTrayClick -Button right
    Wait-For 2
    $shot = Shot "tray-menu"
    $rows = @(Get-GuestMenu | Where-Object { $_.Kind -eq "MenuItem" })
    Send-Keys "esc"; Wait-For 1
    # The fourth row names the version, or the newer release when there is one.
    $wanted = "cw-core", "Idle", "MCP: *", "*", "Open CodeWiki", "Update wiki now", "Recent repositories", "Settings", "Check for updates", "Quit"
    $wrong = @()
    if ($rows.Count -ne $wanted.Count) { $wrong += "$($rows.Count) rows, not $($wanted.Count)" }
    for ($i = 0; $i -lt [math]::Min($rows.Count, $wanted.Count); $i++) {
        if ($rows[$i].Name -notlike $wanted[$i]) { $wrong += "row $($i + 1) is '$($rows[$i].Name)', not '$($wanted[$i])'" }
    }
    # What the app is doing can be read, not chosen.
    $chosen = @($rows | Select-Object -First 3 | Where-Object { $_.Enabled })
    if ($chosen) { $wrong += "a status row can be chosen: $(($chosen | ForEach-Object { $_.Name }) -join ', ')" }
    Add-Result -Id "TR4" -Title "The tray menu" -Expect "ten rows in order; the first three greyed" `
        -Status $(if ($wrong) { "fail" } else { "pass" }) -Shots @($shot) `
        -Observed "right-clicked the icon at ($($icon.X),$($icon.Y)); rows: $(($rows | ForEach-Object { "$($_.Name)$(if (-not $_.Enabled) { ' (greyed)' })" }) -join ' | ')$(if ($wrong) { '; wrong: ' + ($wrong -join '; ') })"
}

Test-Case "SI1" "Starting the app again shows the running one" "with the window hidden, the Start menu brings the same app back; no second app, no second backend" {
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
    $row = Send-GuestMenuClick "Quit"
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
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($shot) `
        -Observed "right-clicked the icon, clicked '$($row.Name)' at ($($row.X),$($row.Y)); left running: $apps app, $backends backend; backend.json left: $([bool]$probe.backend); tray icons named CodeWiki left: $icons"
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
        Sidebar = if ($run.Output -match "sidebar=(\S+)") { $Matches[1] } else { "" }
        Details = if ($run.Output -match "details=(\S+)") { $Matches[1] } else { "" }
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

# The Settings page fills the window: a list of eight categories on the left,
# the open one on the right. The rows are at x 92, y 109 + 36 n of the
# window's client area at 100 percent (settings_layout.rs), anchored to its
# top left corner. Each is clicked and photographed; the provider and the
# keys are left alone, so nothing is sent anywhere.
Test-Case "ST1" "The Settings page and its categories" "each row of the list opens its category" {
    Start-GuestApp -Arguments $guestRepo, "settings:"
    Wait-For 15
    $info = Get-GuestWindowInfo "codewiki-app"
    if (-not $info) { throw "the app has no window" }
    $taken = @()
    $row = 0
    foreach ($name in "appearance", "ai-model", "semantic-search", "backend", "repositories", "updates", "keyboard", "diagnostics") {
        $x = $info.Left + [int](92 * $info.Scale)
        $y = $info.Top + [int]((109 + 36 * $row) * $info.Scale)
        Send-RestedClick -X $x -Y $y -RestSeconds 0.6
        Wait-For 2
        $taken += Shot "settings-$name"
        $row += 1
    }
    Add-Result -Id "ST1" -Title "The Settings page and its categories" -Expect "eight categories, each opened by its row" -Status note -Shots $taken `
        -Observed "started on 'settings:' and clicked the eight rows of the list; read the screenshots: each must show its own heading (Appearance, AI model, Semantic search, Backend, Repositories, Updates, Keyboard, Diagnostics) with its row marked, cards on the right, and no sidebar or tabs. Updates must have the 'Daily check' switch, Appearance the 'Start with Windows' switch"
    Stop-GuestApp
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

Test-Case "D3" "MCP bridge over stdio" "initialize and tools/list answer; 24 tools" {
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
    Add-Result -Id "D3" -Title "MCP bridge over stdio" -Expect "initialize and tools/list answer; 24 tools" `
        -Status $(if ($tools -eq 24) { "pass" } elseif ($tools -gt 0) { "note" } else { "fail" }) -Output $run.Output -Observed "tools listed: $tools"
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
    Test-Case "CC0" "Install Git for Windows" "git and Git Bash are installed" {
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
    Test-Case "CC5" "Claude calls a CodeWiki tool" "the 24 tools are offered, get_overview is called without a permission prompt and answers from the wiki" {
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
        "plugin errors: $(@($event.plugin_errors).Count)"
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
        $ok = $run.Output -match "codewiki server: plugin:codewiki:codewiki=connected" -and $tools -eq 24 -and
            $run.Output -match "tool call: mcp__plugin_codewiki_codewiki__get_overview" -and $run.Output -match "result: error False, turns \d+, denials 0"
        Add-Result -Id "CC5" -Title "Claude calls a CodeWiki tool" -Expect "24 tools offered; get_overview called and answered" `
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
        $was = (Invoke-GuestScript -Script $version).Output.Trim()
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
            -Observed "installed $oldVersion (exit $($install.ExitCode)), codewiki --version said '$was'; update source: $(if ($RealRelease) { 'GitHub' } else { 'the stand-in in the guest' }); palette: 'update codewiki', Enter; 75 s later codewiki --version said '$now', app processes: $((Get-App $probe).Count), app windows: $($window.Count), left over in the install folder: $(if ($left) { $left } else { 'nothing' })"
        Stop-GuestApp
        Invoke-GuestExe -Exe $cmd -TimeoutSec 120 -Arguments "/c", "$guestTarget\uninstall.cmd", "-PurgeData" | Out-Null
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
