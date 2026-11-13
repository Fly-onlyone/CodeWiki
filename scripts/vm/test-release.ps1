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
    # Multiplies every wait; raise it on a slow host.
    [double]$Pace = 1.0
)
$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "Vm.ps1")
Initialize-Vm -Name $VmName -User $User -Password $Password

$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not $Zip) {
    $newest = Get-ChildItem (Join-Path $root "dist") -Filter "CodeWiki-*-windows-x64.zip" -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $newest) { throw "no package in dist\; run scripts\package.ps1 first" }
    $Zip = $newest.FullName
}
$Zip = (Resolve-Path $Zip).Path
if (-not $Report) { $Report = Join-Path $root ("report\vm-test-" + (Get-Date -Format "yyyy-MM-dd")) }
$shots = Join-Path $Report "screenshots"
$logs = Join-Path $Report "logs"
New-Item -ItemType Directory -Force $shots, $logs, (Join-Path $Report "recording") | Out-Null
$Report = (Resolve-Path $Report).Path
# Evidence of an earlier run into the same folder would mix with this one's.
Get-ChildItem $shots -Filter "*.png" | Remove-Item -Force
Get-ChildItem $logs -Filter "probe-*.json" | Remove-Item -Force

$guestHome = "C:\Users\$User"
$guestUnpacked = "$guestHome\Downloads\CodeWiki"
$guestTarget = "$guestHome\AppData\Local\Programs\CodeWiki"
$guestRepo = "$guestHome\repos\cw-core"
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
Wait-For 30
Start-Recording -Path (Join-Path $Report "recording\session.mp4") | Out-Null

New-GuestDirectory $script:Vm.GuestDir
New-GuestDirectory "$guestHome\repos"
Copy-ToGuest -Path (Join-Path $PSScriptRoot "guest\probe.ps1") -GuestDirectory $script:Vm.GuestDir
Copy-ToGuest -Path $Zip -GuestDirectory "$guestHome\Downloads"
Copy-ToGuest -Path (Join-Path $root "crates\cw-core") -GuestDirectory "$guestHome\repos"
$guestZip = "$guestHome\Downloads\$(Split-Path -Leaf $Zip)"

Test-Case "B1" "Guest before install" "recorded only" {
    $probe = Get-Probe "before"
    $missing = @($probe.runtime | Where-Object { -not $_.present } | ForEach-Object { $_.name })
    $policy = ($probe.powershell.policy | ForEach-Object { "$($_.scope)=$($_.policy)" }) -join ", "
    Add-Result -Id "B1" -Title "Guest before install" -Expect "recorded only" -Status note -Shots @(Shot "desktop") `
        -Observed "$($probe.os.caption) $($probe.os.display) build $($probe.os.build); PowerShell $($probe.powershell.version), pwsh $(if ($probe.powershell.pwsh) { 'present' } else { 'absent' }); policy $policy; git $(if ($probe.git) { 'present' } else { 'absent' }); adapters: $(($probe.adapters | ForEach-Object { $_.name }) -join ', '); graphics card attached by the host: $([bool]$Gpu); runtime DLLs missing: $(if ($missing) { $missing -join ', ' } else { 'none' }); commands run in session $($probe.session.id), elevated $($probe.session.elevated)"
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
# Nothing is registered yet and the shortcut starts in the install folder, so
# the app must show its welcome page, not take a folder as the repository.
Test-Case "E1" "Start the app from the Start menu" "a CodeWiki window opens on the welcome page; no folder is registered" {
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
    Add-Result -Id "E1" -Title "Start the app from the Start menu" -Expect "a window on the welcome page; no folder registered" `
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
Test-Case "E3" "Open a folder from the welcome page" "Open folder… asks for a folder, registers it and opens it" {
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
    Add-Result -Id "E3" -Title "Open a folder from the welcome page" -Expect "cw-core registered and shown" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($a, $b, $c) `
        -Observed "palette: 'open folder', Enter; typed $guestRepo, Enter; registered: $(($registered | ForEach-Object { $_.name }) -join ', '); the app's log says it opened '$opened'"
}

Test-Case "H1" "Close the app, start it again" "backend outlives the app and is reused" {
    Send-Keys "alt+f4"
    Wait-For 5
    $closed = Shot "after-alt-f4"
    $probe = Get-ProbeWhenAble "app-closed" -TimeoutSec 240
    if (-not $probe) { throw "the guest answered no command for four minutes after Alt+F4" }
    $script:closedProbe = $probe
    if (-not $firstBackendPid) { $script:firstBackendPid = $probe.backend.pid }
    $appLeft = (Get-App $probe).Count
    $backendAlive = $probe.backend.pidAlive
    # Without a graphics processor the moving backdrop takes every core (case
    # P1), and the guest then answers too slowly to be driven. In that case
    # the app runs with motion reduced from here on; otherwise as installed.
    if ($starved) { Set-GuestMotion "reduced" }
    $search = Start-FromStartMenu
    $again = Shot "second-launch"
    $probe2 = Get-Probe "second-launch" -Quick
    $same = $probe2.backend.pid -eq $firstBackendPid
    $ok = $appLeft -eq 0 -and $backendAlive -and $same -and (Get-App $probe2).Count -eq 1
    Add-Result -Id "H1" -Title "Close the app, start it again" -Expect "backend outlives the app and is reused" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($closed, $again) `
        -Observed "after Alt+F4: app processes $appLeft, backend alive $backendAlive; after restart: backend pid $($probe2.backend.pid) (was $firstBackendPid), app processes $((Get-App $probe2).Count)"
    # The Start menu gives no folder: the app reopens the one opened in E3.
    $opened = Get-OpenedRepo
    Add-Result -Id "E4" -Title "The last repository opens again" -Expect "started from the Start menu, the app shows cw-core" `
        -Status $(if ($opened -eq "cw-core") { "pass" } else { "fail" }) -Shots @($again) `
        -Observed "the app's log says it opened '$(if ($opened) { $opened } else { 'nothing' })' since it started"
}

Test-Case "E2" "Backend after first launch" "backend.json written, health answers, token required" {
    $source = if ($firstProbe) { "while the app was open" } else { "after the app was closed" }
    $backend = if ($firstProbe) { $firstProbe.backend } else { $closedProbe.backend }
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
# ---------------------------------------------------------------- F, pages
$views = @(
    @{ name = "home"; args = @($guestRepo) }
    @{ name = "note"; args = @($guestRepo, $noteName) }
    @{ name = "graph"; args = @($guestRepo, "graph:") }
    @{ name = "health"; args = @($guestRepo, "health:") }
    @{ name = "tests"; args = @($guestRepo, "tests:") }
    @{ name = "settings"; args = @($guestRepo, "settings:") }
    @{ name = "themes"; args = @($guestRepo, "themes:") }
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

# ---------------------------------------------------------------- G, keyboard
Test-Case "G1" "Palette by keyboard" "Ctrl+K opens it, typing filters, Enter runs, Escape closes" {
    Start-GuestApp -Arguments $guestRepo
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
    $paletteX = $appWindow.Right - 344; $paletteY = $appWindow.Top + 61
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
Test-Case "G4" "Focus by keyboard" "Tab moves a visible focus; Enter and Space press the focused control" {
    Send-Keys "tab", "tab", "tab"; Wait-For 2
    $a = Shot "focus-tab-3"
    Send-Keys "tab", "tab", "tab"; Wait-For 2
    $b = Shot "focus-tab-6"
    Send-Keys "enter"; Wait-For 4
    $c = Shot "focus-enter"
    Send-Keys "tab"; Send-Keys "space"; Wait-For 4
    $d = Shot "focus-space"
    Add-Result -Id "G4" -Title "Focus by keyboard" -Expect "focus moves and controls fire" -Status note -Shots @($a, $b, $c, $d) `
        -Observed "keys sent: Tab x3, Tab x3, Enter, Tab, Space; read the screenshots"
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

# ---------------------------------------------------------------- K, uninstall
# uninstall.cmd, as a double-click runs it, while the app and backend run.
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
    Add-Result -Id "K1" -Title "Uninstall while the app runs" -Expect "processes stopped; programs, PATH entry and shortcut removed" `
        -Status $(if ($problems -or $run.ExitCode -ne 0) { "fail" } else { "pass" }) -Shots @($shot) -Output $run.Output `
        -Observed "exit $($run.ExitCode); running before: $($before.processes.Count) processes; $(if ($problems) { $problems -join '; ' } else { 'removed' }); left in the install folder: $(if ($left) { $left -join ', ' } else { 'nothing' }); data folder files: $(Get-FileCount $probe.dataDir.files); settings folder files: $(Get-FileCount $probe.prefsDir.files)"
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
