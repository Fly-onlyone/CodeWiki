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
    # The setup program (CodeWiki-<version>-windows-x64-setup.exe) to test in
    # cases IS1 to IS8. Default: the one beside the package, named for the
    # same version, else the newest in that folder. Without one, those cases
    # do not run; a release run always gives it.
    [string]$Setup,
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
    # With -Hardened: exempt CodeWiki's own files from the attack surface
    # reduction rules, as an administrator does for an app those rules do not
    # know yet (a new, unsigned release: its prevalence and age are nil). The
    # antivirus still scans them; every other rule stays as harden.ps1 set it.
    # Case SX records the exceptions.
    [switch]$ExceptCodeWiki,
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
    # Leave these cases out (their IDs; wildcards allowed, as "OL*"): for the
    # main pass of a release, whose real embedding servers run alone.
    [string[]]$Skip,
    # Multiplies every wait; raise it on a slow host.
    [double]$Pace = 1.0
)
$ErrorActionPreference = "Stop"
# "pwsh -File ... -Only A1,C1" hands the list over as one text.
if ($Only) { $Only = @($Only | ForEach-Object { $_ -split "," } | ForEach-Object { $_.Trim() } | Where-Object { $_ }) }
if ($Skip) { $Skip = @($Skip | ForEach-Object { $_ -split "," } | ForEach-Object { $_.Trim() } | Where-Object { $_ }) }
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
if ($Setup) {
    $Setup = (Resolve-Path $Setup).Path
} else {
    $beside = Split-Path -Parent $Zip
    $sameVersion = Join-Path $beside (((Split-Path -Leaf $Zip) -replace '\.zip$', '') + "-setup.exe")
    if (Test-Path -LiteralPath $sameVersion) {
        $Setup = $sameVersion
    } else {
        $newestSetup = Get-ChildItem $beside -Filter "CodeWiki-*-windows-x64-setup.exe" -ErrorAction SilentlyContinue |
            Sort-Object LastWriteTime -Descending | Select-Object -First 1
        if ($newestSetup) { $Setup = $newestSetup.FullName }
    }
}
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
Get-ChildItem $shots -Directory | Remove-Item -Recurse -Force
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
    if ($Skip | Where-Object { $Id -like $_ }) { return }
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
    # The agent that the session-start hook has explain waiting notes.
    if ($entries -notcontains "plugins/claude-code/agents/note-writer.md") { $missing += "plugins/claude-code/agents/note-writer.md" }
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
$guestSetup = $null
if ($Setup) {
    Copy-ToGuest -Path $Setup -GuestDirectory "$guestHome\Downloads"
    $guestSetup = "$guestHome\Downloads\$(Split-Path -Leaf $Setup)"
}

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

if ($Hardened -and $ExceptCodeWiki) {
    # Exceptions to the attack surface reduction rules only (not antivirus
    # exclusions), by path with wildcards: the install folder under its long
    # and its 8.3 name (programs, plugins and the update's .update folder),
    # the setup program in Downloads, and the temporary copies Inno Setup
    # runs (is-*.tmp for setup, _iu*.tmp for the uninstaller).
    Test-Case "SX" "Exceptions for CodeWiki" "CodeWiki's own files are exempt from the attack surface reduction rules; nothing else is" {
        $paths = @(
            "C:\Users\*\AppData\Local\Programs\CodeWiki\*",
            "C:\Users\*\Downloads\CodeWiki-*-windows-x64-setup.exe",
            "C:\Users\*\AppData\Local\Temp\is-*.tmp\*",
            "C:\Users\*\AppData\Local\Temp\_iu*.tmp"
        )
        $run = Invoke-InGuest -ArgumentList (, $paths) -ScriptBlock {
            param($paths)
            foreach ($path in $paths) { Add-MpPreference -AttackSurfaceReductionOnlyExclusions $path }
            $preference = Get-MpPreference
            "asr exceptions: " + (@($preference.AttackSurfaceReductionOnlyExclusions) -join " | ")
            "antivirus exclusions: " + (@(@($preference.ExclusionPath) + @($preference.ExclusionProcess) | Where-Object { $_ }).Count)
        }
        $listed = "$($run.Output | Where-Object { $_ -like 'asr exceptions:*' })"
        $missing = @($paths | Where-Object { $listed -notlike "*$_*" })
        Add-Result -Id "SX" -Title "Exceptions for CodeWiki" -Expect "the four paths listed as attack surface reduction exceptions; no antivirus exclusion" `
            -Status $(if (-not $missing -and ($run.Output -match 'antivirus exclusions: 0')) { "pass" } else { "fail" }) -Output (($run.Output + $run.Errors) -join "`n") `
            -Observed "$listed; $(($run.Output | Where-Object { $_ -like 'antivirus exclusions:*' }) -join '')$(if ($missing) { "; not taken: $($missing -join ', ')" })"
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

# The app gives Windows its name and logo with its first Windows notification,
# H1's: the values under its AppUserModelId key and the picture they name.
Test-Case "NI1" "Windows knows the app's notification name and icon" "after the first Windows notification: DisplayName 'CodeWiki' and an IconUri naming notification-icon.png in the data folder, which exists" {
    $probe = if ($closedProbe) { $closedProbe } else { Get-Probe "notification-identity" -Quick }
    $name = "$($probe.registry.appIdentityName)"
    $icon = "$($probe.registry.appIdentityIcon)"
    $expected = "$guestHome\AppData\Local\CodeWiki\notification-icon.png"
    $ok = $probe.registry.appIdentity -and $name -eq "CodeWiki" -and $icon -ieq $expected -and $probe.registry.notificationIcon
    Add-Result -Id "NI1" -Title "Windows knows the app's notification name and icon" -Expect "DisplayName 'CodeWiki'; IconUri '$expected'; the file exists" `
        -Status $(if ($ok) { "pass" } else { "fail" }) `
        -Observed "key present: $($probe.registry.appIdentity); DisplayName '$name'; IconUri '$icon'; notification-icon.png in the data folder: $($probe.registry.notificationIcon); H1's screenshot shows the notification with this name and logo"
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
    $ok = -not $hidden -and $sameApp -and $sameBackend -and $info -and -not $info.Minimized -and $info.Foreground
    Add-Result -Id "SI1" -Title "Starting the app again shows the running one" -Expect "one app, the same process; one backend, the same; its window on screen and in front" `
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
    @{ name = "review"; args = @($guestRepo, "review:") }
    # The toolbar shows "To explain (N)"; the page lists the notes that wait.
    @{ name = "waiting"; args = @($guestRepo, "waiting:") }
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

# ---------------------------------------------------------------- helpers for notices, recovery and the look
# Screenshots at a steady pace, for what shows only for a while: the banner
# while the backend is away, a toast for six seconds. The frames go to a
# folder of their own; the case keeps a contact sheet of them, four to a row.
# Returns the sheet's file name, or the folder's when ffmpeg is missing.
function Save-ShotBurst {
    param([Parameter(Mandatory)][string]$Name, [double]$Seconds = 30, [double]$Every = 2)
    $script:shotCount += 1
    $base = "{0:d2}-{1}" -f $script:shotCount, $Name
    $folder = Join-Path $shots $base
    New-Item -ItemType Directory -Force $folder | Out-Null
    $deadline = (Get-Date).AddSeconds($Seconds * $Pace)
    $count = 0
    do {
        $count += 1
        Save-Screenshot (Join-Path $folder ("frame-{0:d2}.png" -f $count)) | Out-Null
        Start-Sleep -Milliseconds ([int]($Every * 1000))
    } while ((Get-Date) -lt $deadline)
    $ffmpeg = Find-Ffmpeg
    if (-not $ffmpeg) { return $base }
    $rows = [math]::Ceiling($count / 4)
    & $ffmpeg -y -loglevel error -framerate 1 -i (Join-Path $folder "frame-%02d.png") -vf "scale=640:-1,tile=4x$rows" -frames:v 1 (Join-Path $shots "$base.png") 2>&1 | Out-Null
    if (Test-Path (Join-Path $shots "$base.png")) { "$base.png" } else { $base }
}

# The number of lines in the app's log, and the lines after such a mark that
# match a pattern (a regular expression without quotes).
function Get-AppLogMark {
    $run = Invoke-GuestScript -Script "@(Get-Content `"`$env:LOCALAPPDATA\CodeWiki\logs\app.log`" -ErrorAction SilentlyContinue).Count"
    if ($run.Output -match '(\d+)') { [int]$Matches[1] } else { 0 }
}
function Get-AppLogSince {
    param([int]$Mark, [string]$Pattern = ".")
    $run = Invoke-GuestScript -Script "@(Get-Content `"`$env:LOCALAPPDATA\CodeWiki\logs\app.log`" -ErrorAction SilentlyContinue) | Select-Object -Skip $Mark | Select-String -Pattern '$Pattern' | ForEach-Object { `$_.Line }"
    @($run.Output -split "`r?`n" | Where-Object { $_.Trim() })
}

# The backend that backend.json names: its pid, whether it runs and what its
# health route answers (0 for no answer). Null without a backend.json.
function Get-GuestBackendState {
    $run = Invoke-GuestScript -TimeoutSec 60 -Script @'
$file = Join-Path $env:LOCALAPPDATA 'CodeWiki\backend.json'
if (-not (Test-Path $file)) { 'backend=none'; return }
$json = Get-Content $file -Raw | ConvertFrom-Json
$alive = [bool](Get-Process -Id $json.pid -ErrorAction SilentlyContinue)
$health = 0
try { $health = [int](Invoke-WebRequest -UseBasicParsing -TimeoutSec 3 "$($json.url)/v1/health").StatusCode } catch { }
"backend=$($json.pid);$alive;$health"
'@
    if ($run.Output -match 'backend=(\d+);(True|False);(\d+)') {
        [pscustomobject]@{ Pid = [int]$Matches[1]; Alive = $Matches[2] -eq "True"; Health = [int]$Matches[3] }
    }
}

# Wait until a backend other than -Not answers its health route.
function Wait-NewBackend {
    param([int]$Not, [int]$TimeoutSec = 75)
    $deadline = (Get-Date).AddSeconds($TimeoutSec * $Pace)
    do {
        $state = Get-GuestBackendState
        if ($state -and $state.Pid -ne $Not -and $state.Alive -and $state.Health -eq 200) { return $state }
        Start-Sleep -Seconds 3
    } while ((Get-Date) -lt $deadline)
    $state
}

# Start a backend from outside the app, as a coding agent's session or a user
# at a terminal does: `codewiki serve` as a program of its own on the desktop
# (Start-GuestExe), in a console without a window. Not from Invoke-GuestScript,
# which ends its task by force and the backend with it; not through the MCP
# bridge, which waits for a request before it connects. Returns the backend
# once it answers, or null.
function Start-GuestBackendOutside {
    $old = Get-GuestBackendState
    Start-GuestExe -Exe "C:\Windows\System32\conhost.exe" -Arguments "--headless", "$guestTarget\codewiki.exe", "serve" -WorkingDirectory $guestRepo
    Wait-NewBackend -Not $(if ($old -and $old.Alive) { -1 } else { 0 }) -TimeoutSec 60
}

# End the backend the way a crash would: no request, no clean-up.
function Stop-GuestBackend {
    Invoke-GuestScript -Script 'Get-Process codewiki -ErrorAction SilentlyContinue | Stop-Process -Force' | Out-Null
    Wait-For 1
}

# Where a point of the app's window is on the guest's screen: X and Y are in
# the app's own pixels from the top left of the client area (the title bar is
# its top 34), as the layout in main.rs gives them.
function Get-AppPoint {
    param([Parameter(Mandatory)]$Info, [double]$X, [double]$Y)
    @(($Info.Left + [int]($X * $Info.Scale)), ($Info.Top + [int]($Y * $Info.Scale)))
}

# The app's layout on a repository page, in its own pixels from the top left
# of the client area (main.rs): a 34 high title bar, a 280 wide sidebar, then
# the toolbar, the tab bar (44) and the page; the details pane is 260 wide.
# Everything here is placed from the left, so it holds for any window width,
# except a negative X, which counts from the right edge. Read off screenshots
# of the release's window (2026-11-25).
$layout = @{
    neutral     = @(600, 17)    # the middle of the title bar: hovers nothing
    sidebarRow  = @(124, 248)   # Core > Overview, a sidebar row that is not open
    sidebarRow2 = @(124, 560)   # Operations > Build and Test, with the window at 992x688
    homeIcon    = @(333, 61)    # the toolbar's Home icon button
    searchField = @(432, 61)    # the toolbar's search field, just after its icon (a click on the icon does not focus it)
    tab         = @(330, 112)   # the first tab
    noteLink    = @(354, 540)   # the link 'cw-core' of _HOME, with the window at 992x688
    detailsButton = @(-130, 433) # "Open in Obsidian" in the details pane of _HOME
    detailsBlank = @(-130, 600) # empty space in the details pane, below that button
    reviewAll   = @(311, 158)   # the review queue's header checkbox (select all)
    reviewRow3  = @(388, 304)   # the queue's third row, its label
    approve     = @(620, 157)   # "Approve N" / "Approve and write", its left part
    reject      = @(762, 157)   # "Reject" beside "Approve and write"
}
function Get-LayoutPoint {
    param([Parameter(Mandatory)]$Info, [Parameter(Mandatory)][string]$Name)
    $x, $y = $layout[$Name]
    if ($x -lt 0) { $x = $Info.Width / $Info.Scale + $x }
    Get-AppPoint -Info $Info -X $x -Y $y
}

# ---------------------------------------------------------------- BG1 and HV1, the look
# The background graph is drawn behind the glass, beside the page; off, the
# glass shows the plain backdrop. Settings > Appearance > Background graph
# writes backdrop.txt; the case writes it, then starts the app again.
Test-Case "BG1" "The background graph, on and off" "with the graph on the window shows notes and links behind the glass; off it does not; neither keeps the processors busy" {
    Invoke-GuestScript -Script "Remove-Item `"`$env:APPDATA\CodeWiki\backdrop.txt`" -ErrorAction SilentlyContinue" | Out-Null
    Start-GuestApp -Arguments $guestRepo, "_HOME"
    Wait-For 15
    Move-Pointer -X ($appWindow.Left + 400) -Y ($appWindow.Top + 17)
    Wait-For 3
    $onShot = Shot "background-graph-on"
    $onLoad = Get-GuestProcessorLoad -Seconds 15
    Stop-GuestApp
    Invoke-GuestScript -Script "Set-Content `"`$env:APPDATA\CodeWiki\backdrop.txt`" 'off' -NoNewline" | Out-Null
    Start-GuestApp -Arguments $guestRepo, "_HOME"
    Wait-For 15
    Move-Pointer -X ($appWindow.Left + 400) -Y ($appWindow.Top + 17)
    Wait-For 3
    $offShot = Shot "background-graph-off"
    $offLoad = Get-GuestProcessorLoad -Seconds 15
    Stop-GuestApp
    Invoke-GuestScript -Script "Remove-Item `"`$env:APPDATA\CodeWiki\backdrop.txt`" -ErrorAction SilentlyContinue" | Out-Null
    $change = Get-ImageChange (Join-Path $shots $onShot) (Join-Path $shots $offShot)
    $idle = if ($null -ne $idleLoad) { $idleLoad } else { 10 }
    # The graph is faint by design (soft dots and lines behind the glass): on
    # a software adapter, where it stands still, about one percent of the
    # window changes.
    $ok = $change -ge 0.005 -and $onLoad -le $idle + 15 -and $offLoad -le $idle + 15
    Add-Result -Id "BG1" -Title "The background graph, on and off" -Expect "the two pictures differ where the graph is; the guest's processors stay near idle both ways" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($onShot, $offShot) `
        -Observed "share of the window that changed between graph on and off: $([math]::Round($change * 100, 1)) percent; the guest's processors over 15 s: $onLoad percent with the graph, $offLoad percent without ($idle before the app started); read the first screenshot: dots and lines behind the glass beside the page; the second: none"
}

# Hover, press and selection, made clearly visible in 0.4.0, are judged by eye
# from these pictures: each is taken after the pointer has rested a second.
Test-Case "HV1" "Hover, press and selection" "a hovered row, button, tab and link each look different from at rest; a pressed row darkens; the open note's row is marked" {
    Start-GuestApp -Arguments $guestRepo, "_HOME"
    Wait-For 15
    # The same window on both checkpoints' screens (1024x768 and 1600x900),
    # so that the note's text wraps alike and the link is where it was measured.
    $info = Set-GuestWindowClient -ProcessName "codewiki-app" -Left 16 -Top 12 -Width 992 -Height 688
    if (-not $info) { throw "the app has no window" }
    Wait-For 3
    # In two runs the note's text was shown selected once the pointer passed
    # over it after the start's click on the title bar, with no button down;
    # it did not happen by hand. A click on empty space first leaves no
    # selection to mistake for a hover.
    $x, $y = Get-LayoutPoint -Info $info -Name "detailsBlank"
    Send-RestedClick -X $x -Y $y
    Wait-For 1
    $taken = @()
    foreach ($target in "neutral", "sidebarRow", "homeIcon", "detailsButton", "tab", "noteLink") {
        $x, $y = Get-LayoutPoint -Info $info -Name $target
        Move-Pointer -X $x -Y $y
        Start-Sleep -Milliseconds 1200
        $taken += Shot "hover-$target"
    }
    $pressed = "not taken"
    $x, $y = Get-LayoutPoint -Info $info -Name "sidebarRow"
    Move-Pointer -X $x -Y $y
    Start-Sleep -Milliseconds 1200
    try {
        Set-MouseButton -Down $true
        Start-Sleep -Milliseconds 400
        $taken += Shot "press-sidebarRow"
        $pressed = "taken"
    } catch {
        $pressed = "the VM's mouse could not hold its button: $($_.Exception.Message)"
    } finally {
        try { Set-MouseButton -Down $false } catch { }
    }
    Start-Sleep -Milliseconds 1500
    $taken += Shot "selected-row"
    Stop-GuestApp
    Add-Result -Id "HV1" -Title "Hover, press and selection" -Expect "each hovered control differs from the first picture; the pressed row is darker; the open note's row carries the bar" -Status note -Shots $taken `
        -Observed "window placed at 992x688; pointer rested 1.2 s on: the title bar (rest), a sidebar row, the Home icon, 'Open in Obsidian', the first tab and the link 'cw-core' of _HOME; the row held down: $pressed; read the screenshots: the row fills and shifts right with its icon grown, the Home icon grows and brightens, the outlined button tints, the tab tints, the link gets a solid underline; the pressed row fills darker; in the last the row of the note opened by the press has a glowing bar at its left edge"
}

# ---------------------------------------------------------------- TB1, CH1 and WT1, a narrow window (0.5.0)
# In a window about 1000 px wide, or at 125% zoom, the toolbar ran past the
# window's right edge, the search hint was cut ("Searcl"), the Code health
# Score column wrapped its head over the file path, and the title bar named
# the last repository on Home. The window is placed as in HV1, at 992x688,
# which fits both checkpoints' screens.
function Set-GuestZoom {
    param([string]$Percent)
    if ($Percent -eq "100") {
        Invoke-GuestScript -Script "Remove-Item `"`$env:APPDATA\CodeWiki\zoom.txt`" -ErrorAction SilentlyContinue" | Out-Null
    } else {
        Invoke-GuestScript -Script "New-Item -ItemType Directory -Force `"`$env:APPDATA\CodeWiki`" | Out-Null; Set-Content `"`$env:APPDATA\CodeWiki\zoom.txt`" '$Percent' -NoNewline" | Out-Null
    }
}
function Show-NarrowWindow {
    param([string[]]$Arguments, [string]$Name)
    Start-GuestApp -Arguments $Arguments
    Wait-For 12
    $info = Set-GuestWindowClient -ProcessName "codewiki-app" -Left 16 -Top 12 -Width 992 -Height 688
    if (-not $info) { throw "the app has no window" }
    Wait-For 3
    Shot $Name
}

Test-Case "TB1" "The toolbar in a narrow window" "at 100% and 125% zoom every toolbar control is inside the window, and the search field keeps its magnifier" {
    $taken = @()
    try {
        foreach ($zoom in "100", "125") {
            Stop-GuestApp
            Set-GuestZoom $zoom
            $taken += Show-NarrowWindow -Arguments $guestRepo, "_HOME" -Name "toolbar-narrow-$zoom"
        }
    } finally {
        Stop-GuestApp
        Set-GuestZoom "100"
    }
    Add-Result -Id "TB1" -Title "The toolbar in a narrow window" -Expect "every toolbar control inside the window at 100% and 125%; labels give way to icons first" -Status note -Shots $taken `
        -Observed "window placed at 992x688 on _HOME, at 100% and at 125% zoom (zoom.txt); read the screenshots: the toolbar ends before the details pane with Sync, Review and To explain visible; at 125% Agent view, Edit, Ctrl+K and Sync are icons, Review and To explain show their icon and count; the search field shows its magnifier and a whole hint or none, never a cut one"
}

Test-Case "CH1" "Code health in a narrow window" "the Score column's head stays on one line and its label does not overlap the file path" {
    Stop-GuestApp
    $shot = Show-NarrowWindow -Arguments $guestRepo, "health:" -Name "health-narrow"
    Stop-GuestApp
    Add-Result -Id "CH1" -Title "Code health in a narrow window" -Expect "the Score head on one line; each score's bar and number clear of the path" -Status note -Shots @($shot) `
        -Observed "window placed at 992x688 on health:; read the screenshot: the table's head reads 'Score' on one line, and every row's bar and number end before the next column's file path"
}

Test-Case "WT1" "The window title on Home" "a note of the repository shows 'CodeWiki — cw-core' in the title bar, Home shows 'CodeWiki' alone" {
    Stop-GuestApp
    $noteShot = Show-NarrowWindow -Arguments $guestRepo, "_HOME" -Name "title-note"
    Stop-GuestApp
    # Not $home: PowerShell's $HOME is read-only, whatever the case.
    $homeShot = Show-NarrowWindow -Arguments $guestRepo, "home:" -Name "title-home"
    Stop-GuestApp
    Add-Result -Id "WT1" -Title "The window title on Home" -Expect "the note's title bar names cw-core; Home's names no repository" -Status note -Shots @($noteShot, $homeShot) `
        -Observed "started on _HOME, then on home: with cw-core open behind it; the title is drawn by the app (Windows keeps 'CodeWiki' as the window text either way), so it is read off the screenshots: the first title bar reads 'CodeWiki — cw-core', the second 'CodeWiki'"
}

# ---------------------------------------------------------------- TT1, tab tools (0.5.0)
# A right-clicked sidebar row once kept a strong fill for good: the press gave
# it keyboard focus. Now a right-click opens a menu (Open in new tab, Copy
# note path) and marks the row only while the menu is open. A tab has a menu
# too (Close, Close others, to the right, to the left, all, Reveal in sidebar,
# Copy note path); middle-click closes a tab; Ctrl+click or middle-click opens
# a note in a new tab. Settings > Appearance > Tabs writes tabs.txt ("off":
# no strip, one view) and new_tab.txt ("on": every note in a tab of its own).
# The window is placed as in HV1, so the layout points hold.
function Show-TabWindow {
    Start-GuestApp -Arguments $guestRepo, "_HOME"
    Wait-For 15
    $info = Set-GuestWindowClient -ProcessName "codewiki-app" -Left 16 -Top 12 -Width 992 -Height 688
    if (-not $info) { throw "the app has no window" }
    Wait-For 3
    $info
}
function Set-GuestTabPref {
    param([Parameter(Mandatory)][string]$Name, [string]$Value)
    $file = "`$env:APPDATA\CodeWiki\$Name.txt"
    if ($Value) {
        Invoke-GuestScript -Script "New-Item -ItemType Directory -Force (Split-Path $file) | Out-Null; Set-Content $file '$Value' -NoNewline" | Out-Null
    } else {
        Invoke-GuestScript -Script "Remove-Item $file -ErrorAction SilentlyContinue" | Out-Null
    }
}

# The link 'cw-core' of _HOME is [[01-Core/cw-core/_index|cw-core/]]: a vault
# path without .md, as Obsidian writes it. Until 0.5.0 the backend did not
# find it ("not found: note '01-Core/cw-core/_index'", query.rs find_entry).
Test-Case "LK1" "A link by folder path" "a click on the link 'cw-core' of _HOME opens the cw-core folder's map" {
    Stop-GuestApp
    $info = Show-TabWindow
    $x, $y = Get-LayoutPoint -Info $info -Name "noteLink"
    Send-RestedClick -X $x -Y $y
    Wait-For 4
    $shot = Shot "link-folder-map"
    Stop-GuestApp
    Add-Result -Id "LK1" -Title "A link by folder path" -Expect "the cw-core folder's map opens; no 'not found' banner" -Status note -Shots @($shot) `
        -Observed "window placed at 992x688 on _HOME; clicked the link 'cw-core' ([[01-Core/cw-core/_index|cw-core/]]); read the screenshot: the page shows the map of cw-core (its notes listed, the tab named after it), and no red 'not found: note' banner"
}

Test-Case "TT1" "Tab tools" "right-click menus on a sidebar note and a tab; the row's mark goes when its menu closes; Ctrl+click and middle-click open new tabs; Close others leaves one; tabs off hides the strip; new tabs on opens every note in its own tab" {
    $taken = @()
    try {
        Stop-GuestApp
        Set-GuestTabPref "tabs"; Set-GuestTabPref "new_tab"
        $info = Show-TabWindow
        $x, $y = Get-LayoutPoint -Info $info -Name "sidebarRow"
        Send-Click -X $x -Y $y -Button right; Wait-For 2
        $taken += Shot "tabs-sidebar-menu"
        Send-Keys "esc"; Wait-For 1
        $nx, $ny = Get-LayoutPoint -Info $info -Name "neutral"
        Move-Pointer -X $nx -Y $ny; Wait-For 2
        $taken += Shot "tabs-sidebar-menu-closed"
        # A Ctrl+click on Operations > Build and Test, then a middle-click on
        # Core > Overview: two new tabs, the last one shown. (The link
        # 'cw-core' of _HOME is LK1's.)
        $lx, $ly = Get-LayoutPoint -Info $info -Name "sidebarRow2"
        Send-Click -X $lx -Y $ly -Hold "ctrl"; Wait-For 3
        Send-Click -X $x -Y $y -Button middle; Wait-For 3
        Move-Pointer -X $nx -Y $ny; Wait-For 1
        $taken += Shot "tabs-three-open"
        $tx, $ty = Get-LayoutPoint -Info $info -Name "tab"
        Send-Click -X $tx -Y $ty -Button right; Wait-For 2
        $taken += Shot "tabs-tab-menu"
        # The menu's second row: Close others.
        Send-Keys "down"; Wait-For 1
        Send-Keys "enter"; Wait-For 3
        Move-Pointer -X $nx -Y $ny; Wait-For 1
        $taken += Shot "tabs-closed-others"
        Stop-GuestApp

        Set-GuestTabPref "tabs" "off"
        $info = Show-TabWindow
        Send-Click -X $lx -Y $ly -Hold "ctrl"; Wait-For 3
        Move-Pointer -X $nx -Y $ny; Wait-For 1
        $taken += Shot "tabs-off"
        Stop-GuestApp
        Set-GuestTabPref "tabs"

        Set-GuestTabPref "new_tab" "on"
        $info = Show-TabWindow
        Send-Click -X $x -Y $y; Wait-For 3
        Send-Click -X $lx -Y $ly; Wait-For 3
        Move-Pointer -X $nx -Y $ny; Wait-For 1
        $taken += Shot "tabs-new-tab-on"
        Stop-GuestApp
        Start-GuestApp -Arguments $guestRepo, "settings:appearance"
        Wait-For 12
        $taken += Shot "tabs-settings"
    } finally {
        Stop-GuestApp
        Set-GuestTabPref "tabs"; Set-GuestTabPref "new_tab"
    }
    Add-Result -Id "TT1" -Title "Tab tools" -Expect "both menus open at the pointer; no row keeps a mark after its menu closes; three tabs, then one; no strip with tabs off; a tab per note with new tabs on" -Status note -Shots $taken `
        -Observed "window placed at 992x688 on _HOME. Right-clicked Core > Overview, Escape, pointer to the title bar; Ctrl+clicked Operations > Build and Test, middle-clicked Core > Overview; right-clicked the first tab, Down, Enter; then tabs.txt = off and a Ctrl+click on Build and Test; then new_tab.txt = on and clicks on Core > Overview and Build and Test; both files removed after. Read the screenshots: (1) a menu at the pointer with 'Open in new tab' and 'Copy note path', the row tinted; (2) no menu and no row filled except the open note's; (3) three tabs: Wiki map, Build and Test, Overview, the last selected; (4) a menu under the first tab: Close, Close others, Close to the right, Close all, Reveal in sidebar, Copy note path; (5) one tab, Wiki map; (6) no tab strip, the page shows Build and Test; (7) three tabs: Wiki map, Overview, Build and Test; (8) Settings > Appearance with a Tabs card: 'Tabs' on and 'Open notes in a new tab' off (it may need the page scrolled: the card is under Zoom)"
}

# ---------------------------------------------------------------- AC1, how many notes a session explains (0.5.0)
# Settings > Repositories > Auto explaining > "Notes per session" writes
# auto-explain-count.txt in the data folder: a number or "all"; missing
# means 20. The session-start hook orders one note-writer per 20 notes, all
# at once (briefing.rs, launch_order). The hook is run here as Claude Code
# runs it, with the host's JSON on stdin, so it needs no login.
Test-Case "AC1" "Notes per session" "the dropdown shows the saved count; the hook orders one note-writer per 20 notes of min(count, waiting), 20 when no count is saved" {
    $readWaiting = @'
$id = Get-RepoId 'cw-core'
$waiting = (Invoke-Backend GET "/v1/repos/$id/explanations?all=true" | ConvertFrom-Json).waiting
"waiting=$([int]$waiting.behind + [int]$waiting.unexplained)"
'@
    if (-not (Get-GuestBackendState | Where-Object { $_.Health -eq 200 })) { Start-GuestBackendOutside | Out-Null }
    $counted = Invoke-GuestBackendScript -Script $readWaiting
    if ($counted.Output -notmatch "waiting=(\d+)") { throw "could not count the waiting notes: $($counted.Output)" }
    $waiting = [int]$Matches[1]
    if ($waiting -eq 0) { throw "no note of cw-core waits, so no count can be judged" }
    $switchBefore = (Invoke-GuestScript -Script "if (Test-Path `"`$env:LOCALAPPDATA\CodeWiki\auto-explain.txt`") { (Get-Content `"`$env:LOCALAPPDATA\CodeWiki\auto-explain.txt`" -Raw).Trim() } else { '(none)' }").Output.Trim()
    $shot = $null
    try {
        # The dropdown is hidden while the switch is off.
        Invoke-GuestScript -Script "New-Item -ItemType Directory -Force `"`$env:LOCALAPPDATA\CodeWiki`" | Out-Null; Set-Content `"`$env:LOCALAPPDATA\CodeWiki\auto-explain.txt`" 'on' -NoNewline; Set-Content `"`$env:LOCALAPPDATA\CodeWiki\auto-explain-count.txt`" '50' -NoNewline" | Out-Null
        Stop-GuestApp
        Start-GuestApp -Arguments $guestRepo, "settings:repositories"
        Wait-For 12
        $shot = Shot "notes-per-session"
        Stop-GuestApp
        $run = Invoke-GuestScript -TimeoutSec 300 -Script (@'
Set-Location '{{REPO}}'
$file = "$env:LOCALAPPDATA\CodeWiki\auto-explain-count.txt"
foreach ($count in '5', '50', 'all', 'none') {
    if ($count -eq 'none') { Remove-Item $file -ErrorAction SilentlyContinue } else { Set-Content $file $count -NoNewline }
    $text = (('{"source":"startup"}' | & '{{EXE}}' hook session-start) -join ' ') -replace '\s+', ' '
    $order = if ($text -match 'The user turned on auto explaining: (.*?) Do not write') { $Matches[1] } else { '(no order)' }
    "order ${count}: $order"
}
Remove-Item $file -ErrorAction SilentlyContinue
'@).Replace("{{REPO}}", $guestRepo).Replace("{{EXE}}", "$guestTarget\codewiki.exe")
    } finally {
        Invoke-GuestScript -Script "Remove-Item `"`$env:LOCALAPPDATA\CodeWiki\auto-explain-count.txt`" -ErrorAction SilentlyContinue" | Out-Null
        if ($switchBefore -eq "(none)") {
            Invoke-GuestScript -Script "Remove-Item `"`$env:LOCALAPPDATA\CodeWiki\auto-explain.txt`" -ErrorAction SilentlyContinue" | Out-Null
        } else {
            Invoke-GuestScript -Script "Set-Content `"`$env:LOCALAPPDATA\CodeWiki\auto-explain.txt`" '$switchBefore' -NoNewline" | Out-Null
        }
    }
    # What briefing.rs says for n notes: one agent up to 20, else one per 20.
    $expected = { param([int]$n)
        $agents = [math]::Ceiling($n / 20)
        if ($agents -le 1) { "agent in the background to explain up to $n waiting notes" } else { "launch $agents ``codewiki:note-writer`` agents" }
    }
    $checks = foreach ($case in @(@("5", 5), @("50", 50), @("all", $waiting), @("none", 20))) {
        $name, $limit = $case
        $want = & $expected ([math]::Min([int]$limit, $waiting))
        $line = @($run.Output -split "`n" | Where-Object { $_ -like "order ${name}: *" })[0]
        [pscustomobject]@{ Name = $name; Want = $want; Ok = [bool]($line -and $line.Contains($want)) }
    }
    $failed = @($checks | Where-Object { -not $_.Ok })
    Add-Result -Id "AC1" -Title "Notes per session" -Expect "the dropdown reads 50; orders for 5, 50, all and no file match one note-writer per 20 notes of min(count, $waiting waiting)" `
        -Status $(if ($failed.Count -eq 0) { "pass" } else { "fail" }) -Shots @($shot) -Output $run.Output `
        -Observed "$waiting notes of cw-core wait; expected in the order: $(($checks | ForEach-Object { "$($_.Name) -> '$($_.Want)' $(if ($_.Ok) { 'found' } else { 'MISSING' })" }) -join '; '); the switch is back to '$switchBefore' and the count file is gone; read the screenshot: Settings > Repositories shows the switch on and 'Notes per session' set to 50 under it"
}

# ---------------------------------------------------------------- RC1, NW1 and RC2, the backend lost and found
# The app asks its backend for its status every five seconds. Two silent
# polls mean it is lost: a banner says so, and the app starts a new backend
# itself. When it answers, a toast says "Reconnected", or, with the window
# hidden, a Windows notification (connection.rs, main.rs take_attempt).
Test-Case "RC1" "The backend ends while the window is open" "a banner says so, the app starts a new backend by itself, and a toast says 'Reconnected'" {
    Start-GuestApp -Arguments $guestRepo, "_HOME"
    Wait-For 20
    $before = Get-GuestBackendState
    if (-not $before) { throw "no backend.json while the app is open" }
    $mark = Get-AppLogMark
    Stop-GuestBackend
    $sheet = Save-ShotBurst "backend-lost-and-back" -Seconds 40 -Every 2
    $after = Wait-NewBackend -Not $before.Pid
    $lines = Get-AppLogSince $mark "toast|Windows notification|backend"
    $toast = [bool]($lines | Where-Object { $_ -match "toast backend" })
    $probe = Get-Probe "backend-back" -Quick
    $shot = Shot "backend-back"
    $ok = $after -and $after.Pid -ne $before.Pid -and $after.Health -eq 200 -and $toast -and (Get-App $probe).Count -eq 1 -and (Get-Registered $probe | Where-Object { $_.name -eq "cw-core" })
    Add-Result -Id "RC1" -Title "The backend ends while the window is open" -Expect "a new backend answers; the log says 'toast backend'; the app still shows cw-core" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($sheet, $shot) -Output ($lines -join "`n") `
        -Observed "backend $($before.Pid) ended by force; then backend $($after.Pid) alive $($after.Alive), health $($after.Health); the app's log: toast 'backend' $toast; app processes $((Get-App $probe).Count); read the contact sheet (a frame each 2 s): the banner 'Backend not reachable, reconnecting…' above the page, then the toast 'Reconnected' at the bottom right"
}

# The same with the window closed to the tray: the notice goes to Windows. It
# is clicked in the notification centre (Win+N), as a user who missed it does;
# the click brings the window back in front.
Test-Case "NW1" "A Windows notification while the window is hidden" "the backend is lost and found with the window in the tray: a Windows notification 'Reconnected'; a click on it in the notification centre shows the window in front" {
    if (-not (Get-GuestWindowInfo "codewiki-app")) { Start-GuestApp -Arguments $guestRepo, "_HOME"; Wait-For 15 }
    [void](Show-GuestWindow -ProcessName "codewiki-app")
    Send-Keys "alt+f4"
    Wait-For 4
    $before = Get-GuestBackendState
    $mark = Get-AppLogMark
    Stop-GuestBackend
    $after = Wait-NewBackend -Not $before.Pid
    Wait-For 3
    $lines = Get-AppLogSince $mark "toast|Windows notification|notification was clicked"
    $sent = [bool]($lines | Where-Object { $_ -match "Windows notification backend" })
    # The notification centre closes as soon as another program starts, and
    # every question to the guest starts one: it cannot be read through UI
    # Automation. Its newest card is at a fixed place from the screen's top
    # right corner (the panel's header, then the app's name, then the card),
    # so it is photographed and clicked there; the app's log says whether the
    # click reached it.
    Send-Keys "win+n"
    Wait-For 4
    $centre = Shot "notification-centre"
    $screen = Get-GuestScreenSize
    Send-RestedClick -X ($screen.Width - 181) -Y 150
    Wait-For 5
    $clicked = [bool](Get-AppLogSince $mark "notification was clicked")
    $info = Get-GuestWindowInfo "codewiki-app"
    $shot = Shot "after-notification-click"
    # The notification centre stays open when the click missed the card.
    if (-not $clicked) { Send-Keys "esc"; Wait-For 1 }
    $status = if (-not $sent) { "fail" } elseif (-not $clicked) { "note" } elseif ($info -and $info.Foreground -and -not $info.Minimized) { "pass" } else { "fail" }
    Add-Result -Id "NW1" -Title "A Windows notification while the window is hidden" -Expect "the log says 'Windows notification backend'; a click on the notification in the notification centre reaches the app and shows the window in front" `
        -Status $status -Shots @($centre, $shot) -Output ($lines -join "`n") `
        -Observed "backend $($before.Pid) ended with the window in the tray; new backend $($after.Pid) health $($after.Health); the app sent a Windows notification: $sent; Win+N, click on the newest card at ($($screen.Width - 181),150): the app's log says it was clicked: $clicked; window $(if ($info) { "on screen, in front $($info.Foreground)" } else { 'not on screen' })$(if (-not $clicked) { '; the click did not reach the app (not verified by the rig): read the notification centre screenshot, which must show CodeWiki and Reconnected' })"
    Stop-GuestApp
}

# A start when no backend can start (its program is held away for the start):
# the app shows its offline page and goes on looking. A backend that another
# program starts later, here `codewiki serve` as a coding agent's session or a terminal does,
# is found and taken, with a toast "Connected".
Test-Case "RC2" "A start with no backend, then one appears" "the offline page; then a backend started outside the app is taken by itself, with a toast 'Connected'" {
    Stop-GuestApp
    Stop-GuestBackend
    $held = "$guestTarget\codewiki.exe.vmhold"
    Invoke-GuestScript -Script "Rename-Item -LiteralPath '$guestTarget\codewiki.exe' -NewName 'codewiki.exe.vmhold'" | Out-Null
    try {
        Start-GuestApp -Arguments $guestRepo, "_HOME"
        Wait-For 20
        $offline = Shot "offline-page"
        $mark = Get-AppLogMark
    } finally {
        Invoke-GuestScript -Script "if (Test-Path -LiteralPath '$held') { Rename-Item -LiteralPath '$held' -NewName 'codewiki.exe' }" | Out-Null
    }
    $started = Start-GuestBackendOutside
    $sheet = Save-ShotBurst "backend-appears" -Seconds 40 -Every 2
    $state = Get-GuestBackendState
    $lines = Get-AppLogSince $mark "toast|backend"
    $toast = [bool]($lines | Where-Object { $_ -match "toast backend" })
    $probe = Get-Probe "backend-found" -Quick
    $shot = Shot "backend-found"
    $ok = $state -and $state.Alive -and $state.Health -eq 200 -and $toast -and (Get-App $probe).Count -eq 1
    Add-Result -Id "RC2" -Title "A start with no backend, then one appears" -Expect "the offline page; the log says 'toast backend' once the bridge's backend answers" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($offline, $sheet, $shot) -Output ($lines -join "`n") `
        -Observed "codewiki.exe held away for the start; codewiki serve started outside the app brought up backend $(if ($started) { "$($started.Pid), health $($started.Health)" } else { 'none' }); afterwards backend $($state.Pid) alive $($state.Alive), health $($state.Health); the app's log: toast 'backend' $toast; read the first screenshot: 'The backend is not reachable.' with 'Try again'; the contact sheet: the page comes back and the toast 'Connected' shows"
    Stop-GuestApp
}

# ---------------------------------------------------------------- RV1, RV2 and SG1, reviews and search
# A note edited by hand is not rewritten when the code under it changes: the
# sync proposes the change instead (cw-vault lifecycle). Three such notes are
# made here, and a sync from outside the app (the backend's update route, as
# a coding agent's sync_wiki does) raises them while the app is open.
$reviewRepoId = $null
Test-Case "RV1" "New reviews arrive while the window is open" "three notes edited by hand, the code under them changed and a sync run from outside: the toast '3 new reviews in cw-core'" {
    Start-GuestApp -Arguments $guestRepo, "_HOME"
    Wait-For 20
    $mark = Get-AppLogMark
    $run = Invoke-GuestBackendScript -TimeoutSec 600 -Script (@'
$id = Get-RepoId 'cw-core'
"repo: $id"
$repo = '{{REPO}}'
$notes = Invoke-Backend GET "/v1/repos/$id/notes" | ConvertFrom-Json
$all = @($notes)
$n = 0
foreach ($source in 'src/paths.rs', 'src/hash.rs', 'src/logs.rs') {
    $own = @($all | Where-Object { @($_.sources).Count -eq 1 -and @($_.sources)[0] -eq $source })
    if (-not $own) { "no note of its own for $source"; continue }
    $n += 1
    $note = $own[0]
    Add-Content -LiteralPath (Join-Path $repo ('obsidian\' + ($note.path -replace '/', '\'))) -Value "`nA line written by hand in the release test.`n"
    Add-Content -LiteralPath (Join-Path $repo ($source -replace '/', '\')) -Value "`n/// A function the release test adds, so that the note's draft changes.`npub fn vm_probe_$n() {}`n"
    "edited: $($note.path) for $source"
}
'update: ' + (Invoke-Backend POST "/v1/repos/$id/update" $null 600)
$deadline = (Get-Date).AddSeconds(180)
do {
    Start-Sleep -Seconds 3
    $status = Invoke-Backend GET '/v1/status' | ConvertFrom-Json
} while ($status.updating -and (Get-Date) -lt $deadline)
"pending: $($status.proposals.$id)"
'@).Replace('{{REPO}}', $guestRepo)
    $script:reviewRepoId = if ($run.Output -match "(?m)^repo: (\S+)") { $Matches[1] } else { $null }
    $sheet = Save-ShotBurst "new-reviews" -Seconds 20 -Every 2
    $lines = Get-AppLogSince $mark "toast|Windows notification"
    $toast = [bool]($lines | Where-Object { $_ -match "toast review:" })
    $pending = if ($run.Output -match "(?m)^pending: (\d+)") { [int]$Matches[1] } else { 0 }
    $edited = @([regex]::Matches($run.Output, "(?m)^edited: ")).Count
    $ok = $edited -eq 3 -and $pending -ge 3 -and $toast
    Add-Result -Id "RV1" -Title "New reviews arrive while the window is open" -Expect "3 notes edited; 3 or more proposals pending; the log says 'toast review:<repo>'" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($sheet) -Output ($run.Output + "`n" + ($lines -join "`n")) `
        -Observed "notes edited by hand with their code changed: $edited; proposals pending after the sync: $pending; the app's log: a review toast $toast; read the contact sheet: the toast '$pending new reviews in cw-core' with 'Open review' at the bottom right, and 'Review ($pending)' in the toolbar"
}

# The queue sits beside the open proposal. The header's box selects every
# row; a Ctrl+click takes one out again; "Approve 2" writes two notes. The
# third is opened alone and rejected.
Test-Case "RV2" "Approve two proposals at once, reject the third" "the queue's boxes select several; 'Approve 2' writes both notes; the last one is rejected; nothing waits afterwards" {
    if (-not $reviewRepoId) { throw "RV1 made no proposals" }
    if (-not (Get-GuestWindowInfo "codewiki-app")) { Start-GuestApp -Arguments $guestRepo, "review:"; Wait-For 15 }
    else {
        [void](Show-GuestWindow -ProcessName "codewiki-app")
        Send-Keys "ctrl+k"; Wait-For 2
        Send-Text "review"; Wait-For 2
        Send-Keys "enter"; Wait-For 6
    }
    $info = Get-GuestWindowInfo "codewiki-app"
    $page = Shot "review-page"
    $x, $y = Get-LayoutPoint -Info $info -Name "reviewAll"
    Send-RestedClick -X $x -Y $y; Wait-For 2
    $x, $y = Get-LayoutPoint -Info $info -Name "reviewRow3"
    Send-Click -X $x -Y $y -Hold "ctrl"; Wait-For 2
    $two = Shot "review-two-selected"
    $x, $y = Get-LayoutPoint -Info $info -Name "approve"
    Send-RestedClick -X $x -Y $y; Wait-For 8
    $approved = Shot "review-approved-two"
    $x, $y = Get-LayoutPoint -Info $info -Name "reject"
    Send-RestedClick -X $x -Y $y; Wait-For 8
    $rejected = Shot "review-rejected-last"
    $run = Invoke-GuestBackendScript -Script (@'
$list = Invoke-Backend GET "/v1/repos/{{ID}}/proposals" | ConvertFrom-Json
$all = @($list)
foreach ($state in 'pending', 'approved', 'rejected') { "${state}: $(@($all | Where-Object { $_.status -eq $state }).Count)" }
'@).Replace('{{ID}}', $reviewRepoId)
    $count = { param($state) if ($run.Output -match "(?m)^${state}: (\d+)") { [int]$Matches[1] } else { -1 } }
    $ok = (& $count "pending") -eq 0 -and (& $count "approved") -eq 2 -and (& $count "rejected") -eq 1
    Add-Result -Id "RV2" -Title "Approve two proposals at once, reject the third" -Expect "2 approved, 1 rejected, none pending" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($page, $two, $approved, $rejected) -Output $run.Output `
        -Observed "the header's box, then Ctrl+click on the third row, 'Approve 2', then 'Reject' on the one left: proposals pending $(& $count 'pending'), approved $(& $count 'approved'), rejected $(& $count 'rejected'); read the screenshots: the queue beside the open proposal, '2 of 3 selected' with 'Approve 2', the toast 'Approved 2', then the empty review page"
}

# Search suggests while the user types: the field under the toolbar lists
# notes, symbols and files; Down and Enter open the first.
Test-Case "SG1" "Search suggests while typing" "typing in the toolbar's search field lists hits under it; Down and Enter open the first note" {
    if (-not (Get-GuestWindowInfo "codewiki-app")) { Start-GuestApp -Arguments $guestRepo, "_HOME"; Wait-For 15 }
    else { [void](Show-GuestWindow -ProcessName "codewiki-app") }
    $info = Get-GuestWindowInfo "codewiki-app"
    $x, $y = Get-LayoutPoint -Info $info -Name "searchField"
    Send-RestedClick -X $x -Y $y; Wait-For 1
    $empty = Shot "search-field"
    Send-Text "paths"; Wait-For 3
    $listed = Shot "search-suggestions"
    Send-Keys "down"; Wait-For 1
    Send-Keys "enter"; Wait-For 5
    $opened = Shot "search-opened"
    $shown = Get-ImageChange (Join-Path $shots $empty) (Join-Path $shots $listed)
    $moved = Get-ImageChange (Join-Path $shots $listed) (Join-Path $shots $opened)
    $ok = $shown -ge 0.01 -and $moved -ge 0.05
    Add-Result -Id "SG1" -Title "Search suggests while typing" -Expect "a list appears under the field; the page changes to the chosen note" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($empty, $listed, $opened) `
        -Observed "typed 'paths' into the search field: $([math]::Round($shown * 100, 1)) percent of the window changed; Down, Enter: $([math]::Round($moved * 100, 1)) percent changed; read the screenshots: groups 'Notes', 'Symbols', 'Files' under the field, then the Paths note"
    Stop-GuestApp
}

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
    # Distances from the client area's top right corner: the Ctrl+K button and
    # the + at the end of the tab bar. Since 0.4.0 "Review (N)" and "To
    # explain (N)" stand right of Ctrl+K, about 500 pixels from the edge (read
    # off the 0.4.0 run's screenshots; a digit more in N moves it 8).
    $client = Get-GuestWindowInfo "codewiki-app"
    if (-not $client) { throw "the app has no window" }
    $paletteX = $client.Right - [int](502 * $client.Scale); $paletteY = $client.Top + [int](61 * $client.Scale)
    $tabX = $client.Right - [int](25 * $client.Scale); $tabY = $client.Top + [int](112 * $client.Scale)
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
        -Observed "started on 'settings:' and clicked the seven rows of the list; read the screenshots: each must show its own heading (Appearance, Semantic search, Backend, Repositories, Updates, Keyboard, Diagnostics) with its row marked, cards on the right, and no sidebar or tabs. No row is named AI model. Semantic search shows the embedder in use ('Word forms: no model needed' here; the card 'Keys and addresses' comes with a server, see SE1). Updates must have the 'Daily check' switch. Appearance has Theme (with 'Background graph'), Motion and Zoom; its Startup card, with 'Start with Windows', is below the fold since 0.4.0 (AS1 to AS3 turn that switch through the palette)"
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
    # With both side panes hidden, the note's column is in the middle of the
    # window. The background graph (0.4.0) is off: with a graphics card it
    # drifts, the window then never matches its earlier picture, and every
    # closed viewer looked like a link that had been followed.
    Invoke-GuestScript -Script 'New-Item -ItemType Directory -Force "$env:APPDATA\CodeWiki" | Out-Null; Set-Content "$env:APPDATA\CodeWiki\sidebar.txt" hidden -NoNewline; Set-Content "$env:APPDATA\CodeWiki\details.txt" hidden -NoNewline; Set-Content "$env:APPDATA\CodeWiki\backdrop.txt" off -NoNewline' | Out-Null
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
    Invoke-GuestScript -Script 'Set-Content "$env:APPDATA\CodeWiki\sidebar.txt" shown -NoNewline; Set-Content "$env:APPDATA\CodeWiki\details.txt" shown -NoNewline; Remove-Item "$env:APPDATA\CodeWiki\backdrop.txt" -ErrorAction SilentlyContinue' | Out-Null
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
    $first = & $line "claim first"
    $second = & $line "claim second"
    $claimed = $first -match "^2 notes; kept: True" -and $second -match "told of another agent: True; notes of the first among them: 0$"
    $ok = $note -and $stale -match "Not saved" -and $wrong -match "Not saved" -and $submit -match "^Saved " -and $held -and $again -and $claimed
    Add-Result -Id "D4" -Title "Write a note through the MCP bridge" -Expect "a stale base and a section that is not the agent's are refused; the submission is saved; the note's file holds the text; a claiming listing keeps its notes from the next one" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
        -Observed "note '$note'; old base: '$stale'; wrong section: '$wrong'; submission: '$submit'; the vault file holds the submitted text: $held; the brief now shows the section as written by an agent: $again; first claiming listing: $first; second: $second"
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

    # VM plugin runs must not spend allowance on note batches: with auto
    # explaining on (its default), the session-start hook would order Claude to
    # explain up to 20 notes in every session. The switch is a file in the
    # guest's data directory; the script sets no CODEWIKI_DATA_DIR there.
    Invoke-GuestScript -Script "New-Item -ItemType Directory -Force `"`$env:LOCALAPPDATA\CodeWiki`" | Out-Null; Set-Content `"`$env:LOCALAPPDATA\CodeWiki\auto-explain.txt`" 'off' -NoNewline" | Out-Null

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

    # A folder kept from coding agents (`codewiki agents off`): a real session
    # asks for a wiki there and gets none. The folder is a new one beside the
    # demo repository, never registered, so "no wiki, no entry" is a before and
    # after of nothing; the demo repository is registered and holds a wiki
    # already, where the same check would prove nothing. It must run while the
    # plugin is installed, so it sits before CC7.
    Test-Case "CC9" "A folder kept from coding agents gets no wiki" "agents off, then a live session asks for a wiki: no obsidian folder, no repos.json entry, and the tool's result or Claude's answer says CodeWiki is turned off" {
        if (-not $signedIn) { throw "Claude Code is not signed in (CC2)" }
        $kept = "$guestHome\repos\cw-kept"
        $run = Invoke-GuestScript -TimeoutSec 600 -Script (@'
$kept = '{{KEPT}}'
$cli = '{{CLI}}'
$dataDir = "$env:LOCALAPPDATA\CodeWiki"
$reposFile = Join-Path $dataDir "repos.json"
function Cut { param([string]$Text, [int]$Length) $flat = $Text -replace "\s+", " "; $flat.Substring(0, [Math]::Min($Length, $flat.Length)) }
function Test-Registered { if (Test-Path $reposFile) { (Get-Content $reposFile -Raw) -match "cw-kept" } else { $false } }
try {
    New-Item -ItemType Directory -Force "$kept\src" | Out-Null
    Set-Content "$kept\Cargo.toml" "[package]`nname = `"cw-kept`"`nversion = `"0.1.0`"`nedition = `"2021`"`n"
    Set-Content "$kept\src\lib.rs" "pub fn add(a: i32, b: i32) -> i32 {`n    a + b`n}`n"
    "registered before: $(Test-Registered)"
    "obsidian before: $(Test-Path "$kept\obsidian")"
    "--- codewiki agents off"
    $off = @(& $cli agents off $kept 2>&1 | ForEach-Object { "$_" }); $off
    "agents off exit: $LASTEXITCODE"
    "--- codewiki agents status"
    $status = @(& $cli agents status $kept 2>&1 | ForEach-Object { "$_" }); $status
    "kept: $([bool]($status -match 'kept from coding agents by'))"
    Set-Location $kept
    $env:ENABLE_CLAUDEAI_MCP_SERVERS = "false"
    $out = "$env:TEMP\claude-kept-run.jsonl"
    & "$env:USERPROFILE\.local\bin\claude.exe" -p "Make the wiki for this project: call the sync_wiki tool of the codewiki MCP server once. Then answer with one line: what CodeWiki answered." --permission-mode default --allowedTools "mcp__plugin_codewiki_codewiki__*" --output-format stream-json --verbose --max-turns 6 --model haiku 2>&1 | Out-File -Encoding utf8 $out
    "claude exit: $LASTEXITCODE"
    foreach ($line in Get-Content $out) {
        try { $event = $line | ConvertFrom-Json } catch { "not json: $(Cut $line 200)"; continue }
        if ($event.type -eq "system" -and $event.subtype -eq "init") {
            "codewiki server: $(($event.mcp_servers | Where-Object { $_.name -match 'codewiki' } | ForEach-Object { "$($_.name)=$($_.status)" }) -join ', ')"
        } elseif ($event.type -eq "system" -and $event.subtype -eq "hook_response") {
            "briefing: $(Cut "$($event.output)" 600)"
        } elseif ($event.type -eq "assistant") {
            foreach ($part in $event.message.content) { if ($part.type -eq "tool_use") { "tool call: $($part.name)" } }
        } elseif ($event.type -eq "user") {
            foreach ($part in $event.message.content) {
                if ($part.type -eq "tool_result") { "tool result: $(Cut ($part.content | ConvertTo-Json -Compress -Depth 4) 300)" }
            }
        } elseif ($event.type -eq "result") {
            "result: error $($event.is_error), turns $($event.num_turns)"
            "answer: $(Cut "$($event.result)" 400)"
        }
    }
    Remove-Item $out -Force -ErrorAction SilentlyContinue
    "obsidian after: $(Test-Path "$kept\obsidian")"
    "registered after: $(Test-Registered)"
} finally {
    Set-Location $env:USERPROFILE
    "--- codewiki agents on"
    & $cli agents on $kept 2>&1 | ForEach-Object { "$_" }
    "agents on exit: $LASTEXITCODE"
    # The feature failed if the folder got registered: forget it the way the
    # product does (DELETE /v1/repos/{id}); if the backend is not there, take
    # the entry out of repos.json. Before the folder goes.
    if (Test-Registered) {
        "registration found: removing it"
        try {
            $list = Get-Content $reposFile -Raw | ConvertFrom-Json
            $mine = @($list | Where-Object { $_.path -match "cw-kept" })
            $info = Get-Content (Join-Path $dataDir "backend.json") -Raw | ConvertFrom-Json
            foreach ($entry in $mine) {
                try {
                    $answer = Invoke-WebRequest -UseBasicParsing -Method DELETE -Uri ($info.url + "/v1/repos/" + $entry.id) -Headers @{ Authorization = ("Bearer " + $info.token) } -TimeoutSec 60
                    "backend delete $($entry.id): $($answer.StatusCode)"
                } catch { "backend delete $($entry.id) failed: $(Cut "$($_.Exception.Message)" 200)" }
            }
        } catch { "backend not reachable: $(Cut "$($_.Exception.Message)" 200)" }
        if (Test-Registered) {
            try {
                $list = Get-Content $reposFile -Raw | ConvertFrom-Json
                $rest = @($list | Where-Object { $_.path -notmatch "cw-kept" })
                [IO.File]::WriteAllText($reposFile, (ConvertTo-Json -InputObject $rest -Depth 4), (New-Object Text.UTF8Encoding $false))
                "repos.json edited by hand: $($rest.Count) entries left"
            } catch { "repos.json edit failed: $(Cut "$($_.Exception.Message)" 200)" }
        }
        "leftover registration: $(Test-Registered)"
    } else { "leftover registration: False" }
    Remove-Item $kept -Recurse -Force -ErrorAction SilentlyContinue
    "removed: $(-not (Test-Path $kept))"
}
$global:LASTEXITCODE = 0
'@).Replace("{{KEPT}}", $kept).Replace("{{CLI}}", "$guestTarget\codewiki.exe")
        if (-not $run.Output) { throw "the guest script printed nothing (a script that did not parse or timed out)" }
        $answer = if ($run.Output -match "(?m)^answer: (.+)$") { $Matches[1].Trim() } else { "" }
        $toolCalled = $run.Output -match "tool call: mcp__plugin_codewiki_codewiki__"
        $toolRefused = $run.Output -match "(?im)^tool result: .*turned off"
        $hookQuiet = $run.Output -notmatch "(?m)^briefing: CodeWiki:"
        $clean = $run.Output -match "obsidian before: False" -and $run.Output -match "registered before: False" -and
            $run.Output -match "obsidian after: False" -and $run.Output -match "registered after: False"
        # The refusal reached the session when the tool's result carried it or
        # Claude's answer mentions it; a model may answer in other words, so
        # its answer is kept as an observed value, not required.
        $refusalSeen = $toolRefused -or ($answer -match "(?i)turned off")
        $ok = $run.Output -match "agents off exit: 0" -and $run.Output -match "kept: True" -and
            $clean -and $refusalSeen -and $hookQuiet -and $run.Output -match "agents on exit: 0" -and $run.Output -match "removed: True"
        $leftover = if ($run.Output -match "leftover registration: True") { "; a repos.json entry for cw-kept is LEFT in the guest and could not be removed (see the output)" } else { "" }
        Add-Result -Id "CC9" -Title "A folder kept from coding agents gets no wiki" -Expect "no obsidian folder, no repos.json entry, the tool's result or Claude's answer says CodeWiki is turned off, the hook adds no briefing" `
            -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
            -Observed "'codewiki agents off' on a new folder, then Claude asked for a wiki there: a CodeWiki tool was called: $toolCalled, the tool's answer said 'turned off': $toolRefused; obsidian folder or repos.json entry before/after: none/none: $clean; session-start hook silent: $hookQuiet; Claude's answer: '$answer'; the folder is removed and 'agents on' run again either way (a registration the case caused is removed first)$leftover"
    }

    # Auto explaining, live, once: the switch on, a real session in the demo
    # repository gets the hook's order to launch the note-writer agent, and the
    # agent writes explanations through submit_note. The agent runs on the
    # account's Sonnet and explains up to 20 notes; the switch is off again
    # afterwards, as for every other session of the run. The order says "in
    # the background", but `claude -p` ends with its answer and takes a
    # background agent with it (the first run, 2026-11-25): the prompt asks
    # for the agent in the foreground.
    Test-Case "AE1" "Auto explaining in a live session" "with the switch on, the session-start hook orders the note-writer agent, the session launches it, and notes get explanations by claude-code" {
        if (-not $signedIn) { throw "Claude Code is not signed in (CC2)" }
        $count = @'
$id = Get-RepoId 'cw-core'
$notes = Invoke-Backend GET "/v1/repos/$id/notes" | ConvertFrom-Json
$all = @($notes)
"explained by claude-code: $(@($all | Where-Object { $_.explained_by -eq 'claude-code' }).Count)"
$waiting = (Invoke-Backend GET "/v1/repos/$id/explanations?all=true" | ConvertFrom-Json).waiting
"waiting: $($waiting.behind) behind, $($waiting.unexplained) never explained"
'@
        # The counts come from the backend: have one running.
        if (-not (Get-GuestBackendState | Where-Object { $_.Health -eq 200 })) { Start-GuestBackendOutside | Out-Null }
        $before = Invoke-GuestBackendScript -Script $count
        # Since 0.5.0 the order carries "Notes per session"; five keeps the
        # live session short.
        Invoke-GuestScript -Script "Set-Content `"`$env:LOCALAPPDATA\CodeWiki\auto-explain.txt`" 'on' -NoNewline; Set-Content `"`$env:LOCALAPPDATA\CodeWiki\auto-explain-count.txt`" '5' -NoNewline" | Out-Null
        try {
            $run = Invoke-GuestScript -TimeoutSec 1800 -Script (@'
Set-Location '{{REPO}}'
$env:ENABLE_CLAUDEAI_MCP_SERVERS = "false"
$out = "$env:TEMP\claude-auto-explain.jsonl"
& "$env:USERPROFILE\.local\bin\claude.exe" -p "Do what the CodeWiki session-start lines ask about auto explaining, but run that agent in the foreground, not in the background, and wait for its result: this one-shot session ends with your answer, and a background agent would end with it. Then answer with one line: the agent's own last line." --permission-mode default --allowedTools "Agent,Task,Read,Grep,Glob,Skill,mcp__plugin_codewiki_codewiki__*" --output-format stream-json --verbose --max-turns 12 --model haiku 2>&1 | Out-File -Encoding utf8 $out
"claude exit: $LASTEXITCODE"
function Cut { param([string]$Text, [int]$Length) $flat = $Text -replace "\s+", " "; $flat.Substring(0, [Math]::Min($Length, $flat.Length)) }
$submits = 0
foreach ($line in Get-Content $out) {
    try { $event = $line | ConvertFrom-Json } catch { continue }
    if ($event.type -eq "system" -and $event.subtype -eq "hook_response") {
        "briefing: $(Cut "$($event.output)" 1600)"
    } elseif ($event.type -eq "assistant") {
        foreach ($part in $event.message.content) {
            if ($part.type -ne "tool_use") { continue }
            if ($part.name -match '^(Agent|Task)$') { "agent launched: $($part.input.subagent_type)" }
            elseif ($part.name -match 'submit_note$') { $submits += 1 }
        }
    } elseif ($event.type -eq "result") {
        "result: error $($event.is_error), turns $($event.num_turns), denials $(@($event.permission_denials).Count)"
        "answer: $(Cut "$($event.result)" 300)"
    }
}
"submit_note calls seen: $submits"
Remove-Item $out -Force
$global:LASTEXITCODE = 0
'@).Replace("{{REPO}}", $guestRepo)
        } finally {
            Invoke-GuestScript -Script "Set-Content `"`$env:LOCALAPPDATA\CodeWiki\auto-explain.txt`" 'off' -NoNewline; Remove-Item `"`$env:LOCALAPPDATA\CodeWiki\auto-explain-count.txt`" -ErrorAction SilentlyContinue" | Out-Null
        }
        $after = Invoke-GuestBackendScript -Script $count
        $explained = { param($text) if ($text -match "explained by claude-code: (\d+)") { [int]$Matches[1] } else { -1 } }
        $gained = if ((& $explained $before.Output) -ge 0 -and (& $explained $after.Output) -ge 0) { (& $explained $after.Output) - (& $explained $before.Output) } else { 0 }
        $ordered = $run.Output -match "(?m)^briefing: .*note-writer"
        # The count: five, or fewer when fewer wait.
        $countOrdered = if ($run.Output -match "(?m)^briefing: .*explain up to (\d+) waiting notes") { [int]$Matches[1] } else { -1 }
        $countOk = $countOrdered -ge 1 -and $countOrdered -le 5
        $launched = $run.Output -match "(?m)^agent launched: codewiki:note-writer"
        $answer = if ($run.Output -match "(?m)^answer: (.+)$") { $Matches[1].Trim() } else { "" }
        Add-Result -Id "AE1" -Title "Auto explaining in a live session" -Expect "the briefing orders codewiki:note-writer; the session launches it; more notes explained by claude-code afterwards" `
            -Status $(if ($ordered -and $countOk -and $launched -and $gained -gt 0) { "pass" } else { "fail" }) -Output ($before.Output + "`n" + $run.Output + "`n" + $after.Output) `
            -Observed "the briefing orders the agent: $ordered, for up to $countOrdered notes with 'Notes per session' at 5 (1 to 5 expected); the session launched codewiki:note-writer: $launched; notes explained by claude-code before $(& $explained $before.Output), after $(& $explained $after.Output); $(($run.Output -split "`n" | Where-Object { $_ -match 'submit_note calls seen' }) -join ''); Claude's answer: '$answer'; the switch is off again"
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
    # as the marketplace. Claude Code clones it, which is why it needs git;
    # `--sparse` keeps the clone to the marketplace file and the plugin.
    # It installs the plugin as the repository's default branch has it.
    Test-Case "CC8" "The repository as marketplace" "claude plugin marketplace add Fly-onlyone/CodeWiki --sparse .claude-plugin plugins/claude-code checks out only the plugin, and the plugin installs from it" {
        $run = Invoke-GuestScript -TimeoutSec 900 -Script @'
$claude = "$env:USERPROFILE\.local\bin\claude.exe"
"--- claude plugin marketplace add Fly-onlyone/CodeWiki --sparse .claude-plugin plugins/claude-code"; & $claude plugin marketplace add Fly-onlyone/CodeWiki --sparse .claude-plugin plugins/claude-code 2>&1; "add exit: $LASTEXITCODE"
$m = "$env:USERPROFILE\.claude\plugins\marketplaces\codewiki"
"--- the clone"; Get-ChildItem $m -Name -Force
"whole: $(Test-Path "$m\crates")"
"plugin: $(Test-Path "$m\plugins\claude-code\.claude-plugin\plugin.json")"
"--- claude plugin install codewiki@codewiki"; & $claude plugin install codewiki@codewiki 2>&1; "install exit: $LASTEXITCODE"
"--- claude plugin list"; $list = @(& $claude plugin list 2>&1 | ForEach-Object { "$_" }); $list
"listed: $([bool]($list -match 'codewiki@codewiki'))"
"--- removal"; & $claude plugin uninstall codewiki@codewiki 2>&1; & $claude plugin marketplace remove codewiki 2>&1; "removal exit: $LASTEXITCODE"
$global:LASTEXITCODE = 0
'@
        $ok = $run.Output -match "add exit: 0" -and $run.Output -match "whole: False" -and $run.Output -match "plugin: True" -and
            $run.Output -match "install exit: 0" -and $run.Output -match "listed: True"
        Add-Result -Id "CC8" -Title "The repository as marketplace" -Expect "only the marketplace file and the plugin folder checked out, and the plugin installs from it" `
            -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
            -Observed $(if ($ok) { "only the marketplace file and the plugin folder were checked out, and codewiki@codewiki installed from them; removed again" } else { "the clone held the whole repository, or the plugin did not install; see the output" })
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

# While Settings > Semantic search is on screen and the chosen server does not
# answer, the page says so and asks again by itself, every 3 s at first and
# then less often (settings_view.rs). When the server is back, the page lists
# its models and a toast says "Connected to Llama".
Test-Case "SR1" "The embedding server comes back by itself" "with the stand-in stopped the page says it does not answer; once the stand-in runs again the page lists its model and a toast says 'Connected to Llama'" {
    if (-not $stubUp) { Start-EmbedStub | Out-Null }
    Set-GuestEmbedder -Provider "llamacpp" -Model $stubModel -Url $stubUrl | Out-Null
    Stop-EmbedStub
    Stop-GuestApp
    Start-GuestApp -Arguments $guestRepo, "settings:search"
    Wait-For 12
    $down = Shot "settings-search-server-down"
    $mark = Get-AppLogMark
    $listing = Start-EmbedStub
    $sheet = Save-ShotBurst "settings-search-server-back" -Seconds 40 -Every 2
    $lines = Get-AppLogSince $mark "toast|Windows notification"
    $toast = [bool]($lines | Where-Object { $_ -match "toast embed-server" })
    $models = Invoke-GuestBackendScript -Script "'models: ' + (Invoke-Backend GET '/v1/settings/embed/models')"
    $reachable = $models.Output -match '"reachable":\s*true'
    Add-Result -Id "SR1" -Title "The embedding server comes back by itself" -Expect "the log says 'toast embed-server' after the stand-in starts; the server answers again" `
        -Status $(if ($toast -and $reachable) { "pass" } else { "fail" }) -Shots @($down, $sheet) -Output ("stand-in's /v1/models: $listing`n" + $models.Output + "`n" + ($lines -join "`n")) `
        -Observed "the stand-in stopped and the app started on Settings > Semantic search; the stand-in started again: the app's log says toast 'embed-server' $toast; the backend reaches the server: $reachable; read the first screenshot: a warning that ends 'CodeWiki tries again on its own.' with Refresh; the contact sheet: the toast 'Connected to Llama' and a Model list instead of the text field"
    Stop-GuestApp
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

# ---------------------------------------------------------------- real embedding servers, shared helpers
# Every real server (Ollama, llama.cpp, LM Studio, Lemonade, mistral.rs) is
# judged the same way by Measure-RealEmbedder, so that the report can compare
# them: the same five searches, the same repository (cw-core), the same
# samples of memory, processor and graphics card, the same units. One record
# per server is appended to logs\embedders.json.
#
# The searches are phrased by meaning, not by the words of cw-core's code
# (secret.rs, logs.rs and paths.rs, settings.rs, hash.rs), so that vectors have
# something to find that word forms do not.
$embedQueries = @(
    "protecting stored credentials",
    "where log files are kept",
    "reading user settings from disk",
    "hashing text into a vector",
    "what happens when a setting is missing"
)
# What the install and pull cases learned, by "<provider>" and
# "<provider>/<model>": version, installMb, downloadS, installS, modelMb, pullS.
$embedFacts = @{}
$embedRecords = [System.Collections.Generic.List[object]]::new()
$embedLocalRecorded = $false
$embedSamples = "$($script:Vm.GuestDir)\embed-samples"
$embedLogs = "$($script:Vm.GuestDir)\embed-logs"
$embedModels = "$($script:Vm.GuestDir)\models"

function Set-EmbedFact {
    param([string]$Key, [string]$Name, $Value)
    if (-not $script:embedFacts.ContainsKey($Key)) { $script:embedFacts[$Key] = @{} }
    $script:embedFacts[$Key][$Name] = $Value
}
function Get-EmbedFact {
    param([string]$Key, [string]$Name)
    if ($script:embedFacts.ContainsKey($Key) -and $script:embedFacts[$Key].ContainsKey($Name)) { $script:embedFacts[$Key][$Name] } else { $null }
}

# The number in the first "<label> N" of a guest output, or $null.
function Get-OutputNumber {
    param([string]$Output, [string]$Pattern)
    if ($Output -match $Pattern) { [double]::Parse($Matches[1], [Globalization.CultureInfo]::InvariantCulture) } else { $null }
}

# Guest text that stops a case when the disk is nearly full. A case's own
# installer is deleted when it has installed, to keep the guest's disk free.
function Get-GuestFreeDiskText {
    param([double]$MinGb)
    (@'
$free = [math]::Round((Get-PSDrive C).Free / 1GB, 1)
"free disk GB: $free"
if ($free -lt {{MIN}}) { 'result: not enough disk'; return }
'@).Replace('{{MIN}}', "$MinGb")
}

# Guest text that downloads a file and says how big it was and how long it took.
function Get-GuestDownloadText {
    param([string]$Url, [string]$Path)
    (@'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
New-Item -ItemType Directory -Force (Split-Path -Parent '{{PATH}}') | Out-Null
$watch = Get-Date
Invoke-WebRequest -UseBasicParsing '{{URL}}' -OutFile '{{PATH}}'
"downloaded: $([math]::Round((Get-Item '{{PATH}}').Length / 1MB, 1)) MB in $([int]((Get-Date) - $watch).TotalSeconds) s"
'@).Replace('{{PATH}}', $Path).Replace('{{URL}}', $Url)
}

# Guest text that lists what Defender stopped or reported with a word in it.
function Get-GuestDefenderText {
    param([string]$Word)
    (@'
$events = @(Get-WinEvent -FilterHashtable @{ LogName = 'Microsoft-Windows-Windows Defender/Operational'; Id = 1121, 1122, 1116, 1117 } -ErrorAction SilentlyContinue | Where-Object { $_.Message -match '{{WORD}}' })
"Defender events naming {{WORD}}: $($events.Count)"
foreach ($event in $events | Select-Object -First 4) { "  event $($event.Id): $(($event.Message -split "`r?`n" | Where-Object { $_ -match '^\s*(ID|Path|Name):' } | ForEach-Object { $_.Trim() }) -join ' | ')" }
'@).Replace('{{WORD}}', $Word)
}

# Size of folders in the guest, in MB (0 for one that is not there).
function Get-GuestFolderMb {
    param([string[]]$Path)
    $list = (($Path | ForEach-Object { "'$_'" }) -join ", ")
    $run = Invoke-GuestScript -TimeoutSec 180 -Script ("`$total = 0; foreach (`$p in @($list)) { `$total += (Get-ChildItem `$p -Recurse -File -Force -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum }; `"mb: `$([math]::Round(`$total / 1MB, 1))`"")
    Get-OutputNumber $run.Output "mb: ([\d\.]+)"
}

# A server that stays up for the cases after its own: started behind
# conhost --headless by guest\serve-logged.ps1, which keeps its output in
# <Log>.out and <Log>.err.
function Start-LoggedServer {
    param([Parameter(Mandatory)][string]$Exe, [Parameter(Mandatory)][string]$Log, [string]$ArgLine = "")
    New-GuestDirectory $embedLogs
    Copy-ToGuest -Path (Join-Path $PSScriptRoot "guest\serve-logged.ps1") -GuestDirectory $script:Vm.GuestDir
    Invoke-GuestScript -Script "Remove-Item '$Log.out', '$Log.err' -ErrorAction SilentlyContinue" | Out-Null
    Start-GuestExe -Exe "C:\Windows\System32\conhost.exe" -Arguments "--headless", $powershell, "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "$($script:Vm.GuestDir)\serve-logged.ps1", "-Exe", $Exe, "-Log", $Log, "-ArgLine", $ArgLine
}

# Wait for a URL in the guest to answer; gives up early when no process whose
# program path matches -ServerProcess runs any more. Returns the guest's text:
# "answered after N s: ..." or "no answer ...", then the end of the server's log.
function Wait-GuestServer {
    param([Parameter(Mandatory)][string]$Url, [Parameter(Mandatory)][string]$ServerProcess, [int]$Seconds = 120, [string]$Log = "")
    (Invoke-GuestScript -TimeoutSec ($Seconds + 120) -Script (@'
$watch = Get-Date
$deadline = $watch.AddSeconds({{SECONDS}})
$answered = $false
while ((Get-Date) -lt $deadline) {
    try {
        $text = (Invoke-WebRequest -UseBasicParsing -TimeoutSec 5 '{{URL}}').Content
        "answered after $([int]((Get-Date) - $watch).TotalSeconds) s: $($text.Substring(0, [math]::Min(300, $text.Length)))"
        $answered = $true
        break
    } catch { }
    $running = @(Get-CimInstance Win32_Process | Where-Object { $(if ($_.ExecutablePath) { $_.ExecutablePath } else { $_.Name }) -match '{{PATTERN}}' }).Count
    if (-not $running -and ((Get-Date) - $watch).TotalSeconds -gt 20) { "the server is not running after $([int]((Get-Date) - $watch).TotalSeconds) s"; break }
    Start-Sleep -Seconds 3
}
if (-not $answered) { "no answer after $([int]((Get-Date) - $watch).TotalSeconds) s" }
if ('{{LOG}}') { foreach ($file in '{{LOG}}.out', '{{LOG}}.err') { if (Test-Path $file) { "--- $file (last lines)"; Get-Content $file -Tail 12 } } }
'@).Replace('{{SECONDS}}', "$Seconds").Replace('{{URL}}', $Url).Replace('{{PATTERN}}', $ServerProcess).Replace('{{LOG}}', $Log)).Output
}

# The id a server lists for a model whose id is not known beforehand (llama.cpp
# names it after the file, LM Studio after its own catalog). Reads each URL as
# OpenAI's /v1/models (data[].id), or Ollama's and Lemonade's variants, and
# returns the first id that matches -Match, with the guest's text.
function Resolve-GuestModelId {
    param([Parameter(Mandatory)][string[]]$Url, [Parameter(Mandatory)][string]$Match)
    $list = (($Url | ForEach-Object { "'$_'" }) -join ", ")
    $run = Invoke-GuestScript -TimeoutSec 120 -Script (@'
foreach ($url in @({{URLS}})) {
    try {
        $answer = Invoke-RestMethod -TimeoutSec 20 $url
        $ids = @()
        if ($answer.data) { $ids = @($answer.data | ForEach-Object { $_.id }) }
        elseif ($answer.models) { $ids = @($answer.models | ForEach-Object { if ($_.id) { $_.id } else { $_.name } }) }
        "ids from ${url}: $($ids -join ' ; ')"
    } catch { "no answer from ${url}: $($_.Exception.Message)" }
}
'@).Replace('{{URLS}}', $list)
    $id = $null
    foreach ($line in $run.Output -split "`n") {
        if ($line -match "^ids from \S+?: (.*)$") {
            $found = @($Matches[1] -split " ; " | Where-Object { $_ -match $Match })
            if ($found.Count) { $id = $found[0].Trim(); break }
        }
    }
    [pscustomobject]@{ Id = $id; Output = $run.Output }
}

# The part of a measuring script that runs the five searches; fills $searches.
$guestSearchText = @'
$queries = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{{QUERIES}}')) | ConvertFrom-Json
$repo = Get-RepoId 'cw-core'
$searches = @()
foreach ($q in $queries) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $answer = Invoke-Backend GET ("/v1/repos/$repo/search?q=" + [uri]::EscapeDataString($q) + '&limit=5')
    $ms = [int]$watch.ElapsedMilliseconds
    $failed = $answer.StartsWith('HTTP error')
    $parsed = if ($answer.StartsWith('[')) { $answer | ConvertFrom-Json } else { @() }
    $keys = [string[]]@($parsed | ForEach-Object { $_.key })
    $searches += [pscustomobject]@{ q = $q; ms = $ms; keys = $keys; error = $(if ($failed) { $answer } else { '' }) }
}
'@

# Guest text that prints $out as RESULT64 lines (base64, in pieces, so that no
# line is cut by the width of the guest's output file).
$guestResultText = @'
$json = $out | ConvertTo-Json -Depth 6 -Compress
$b64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($json))
for ($i = 0; $i -lt $b64.Length; $i += 3000) { 'RESULT64: ' + $b64.Substring($i, [math]::Min(3000, $b64.Length - $i)) }
'@

function Read-GuestResult {
    param([string]$Output)
    $b64 = (($Output -split "`n") | ForEach-Object { if ($_ -match '^RESULT64: (\S+)') { $Matches[1] } }) -join ""
    if (-not $b64) { return $null }
    [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($b64)) | ConvertFrom-Json
}

function Get-MedianValue {
    param($Values)
    $sorted = @($Values | Where-Object { $null -ne $_ } | Sort-Object)
    if (-not $sorted.Count) { return $null }
    $middle = [int][math]::Floor($sorted.Count / 2)
    if ($sorted.Count % 2) { $sorted[$middle] } else { ($sorted[$middle - 1] + $sorted[$middle]) / 2 }
}

# What the sampler (guest\sampler.ps1) wrote, as figures: the server's memory
# at its highest and when idle, the guest's processor load while the model
# loaded and the job ran, its least free memory, and whether a graphics engine
# worked.
function Get-SampleStats {
    param([string[]]$Lines)
    $invariant = [Globalization.CultureInfo]::InvariantCulture
    $number = { param($text) if ($text -eq "na") { $null } else { [double]::Parse($text, $invariant) } }
    $rows = @()
    foreach ($line in $Lines) {
        $f = $line.Trim() -split ","
        if ($f.Count -ge 7) {
            $rows += [pscustomobject]@{ Phase = $f[0]; Ws = (& $number $f[2]); Cpu = (& $number $f[3]); Free = (& $number $f[4]); Gpu3d = (& $number $f[5]); GpuCompute = (& $number $f[6]) }
        }
    }
    $working = @($rows | Where-Object { $_.Phase -in "load", "job" })
    $idle = @($rows | Where-Object { $_.Phase -eq "idle" } | Select-Object -Last 5)
    $bench = @($rows | Where-Object { $_.Phase -eq "bench" })
    $benchCpu = @($bench | ForEach-Object { $_.Cpu } | Where-Object { $null -ne $_ })
    $gpuValues = @(@($working) + @($bench) | ForEach-Object { $_.Gpu3d; $_.GpuCompute } | Where-Object { $null -ne $_ })
    $gpuPeak = if ($gpuValues.Count) { ($gpuValues | Measure-Object -Maximum).Maximum } else { $null }
    $cpuValues = @($working | ForEach-Object { $_.Cpu } | Where-Object { $null -ne $_ })
    $freeValues = @($rows | ForEach-Object { $_.Free } | Where-Object { $null -ne $_ })
    [pscustomobject]@{
        Samples      = $rows.Count
        MemoryPeakMb = if ($rows.Count) { [math]::Round(($rows | Measure-Object Ws -Maximum).Maximum, 0) } else { $null }
        MemoryIdleMb = if ($idle.Count) { [math]::Round(($idle | Measure-Object Ws -Average).Average, 0) } else { $null }
        CpuAvg       = if ($cpuValues.Count) { [math]::Round(($cpuValues | Measure-Object -Average).Average, 1) } else { $null }
        CpuPeak      = if ($cpuValues.Count) { [math]::Round(($cpuValues | Measure-Object -Maximum).Maximum, 1) } else { $null }
        FreeMinMb    = if ($freeValues.Count) { [math]::Round(($freeValues | Measure-Object -Minimum).Minimum, 0) } else { $null }
        CpuBenchAvg  = if ($benchCpu.Count) { [math]::Round(($benchCpu | Measure-Object -Average).Average, 1) } else { $null }
        CpuBenchPeak = if ($benchCpu.Count) { [math]::Round(($benchCpu | Measure-Object -Maximum).Maximum, 1) } else { $null }
        MemoryBenchPeakMb = if ($bench.Count) { [math]::Round(($bench | Measure-Object Ws -Maximum).Maximum, 0) } else { $null }
        Gpu          = if ($null -eq $gpuPeak) { "not available" } elseif ($gpuPeak -ge 5) { "yes (busiest engine $([math]::Round($gpuPeak, 0)) %)" } else { "no (busiest engine $([math]::Round($gpuPeak, 0)) %)" }
    }
}

# Choose the server and its model through Set-GuestEmbedder (and photograph
# Settings > Semantic search), then judge it: the model list, the test call
# (its "ms" is the first call, which loads the model), the embedding job of
# cw-core from the test to done, five searches by meaning with the vectors in
# use and again with the word-form embedder "local", and, from a sampler in the
# guest, the server's memory, the guest's processor load, its free memory and
# whether a graphics engine worked. It then resets the embedder to "local",
# stops the server and checks that nothing of it runs. A record is appended
# to logs\embedders.json and a result to the report.
#   -ServerProcess  a regular expression for the program path (or name) of the
#                   server's processes, for the samples and for stopping them.
#   -Model          what Settings is given; -ModelFamily and -Quantisation say
#                   what it is, for the table's "same model" columns.
#   -InstallFolder  guest folders whose size is "install MB".
#   -StopScript     guest text that ends the server politely before it is killed.
function Measure-RealEmbedder {
    param(
        [Parameter(Mandatory)][string]$Id,
        [Parameter(Mandatory)][ValidateSet("ollama", "lmstudio", "llamacpp", "lemonade", "mistralrs")][string]$Provider,
        [Parameter(Mandatory)][string]$Model,
        [string]$Url,
        [Parameter(Mandatory)][string]$ServerProcess,
        [Parameter(Mandatory)][int]$Port,
        [string]$Title = "Semantic search through $Provider",
        [string]$ModelFamily = $Model,
        [string]$Quantisation = "unknown",
        [string]$ServerVersion,
        [string[]]$InstallFolder = @(),
        [string]$StopScript = "",
        [string]$Note = "",
        [int]$JobSeconds = 1800
    )
    $factKey = "$Provider/$Model"
    $set = Set-GuestEmbedder -Provider $Provider -Model $Model -Url $Url
    $shot = Shot "settings-search-$Id"

    # The sampler runs beside the work, in its own task.
    New-GuestDirectory $embedSamples
    Copy-ToGuest -Path (Join-Path $PSScriptRoot "guest\sampler.ps1") -GuestDirectory $script:Vm.GuestDir
    Invoke-GuestScript -Script "Remove-Item '$embedSamples\*' -Force -ErrorAction SilentlyContinue" | Out-Null
    Start-GuestExe -Exe "C:\Windows\System32\conhost.exe" -Arguments "--headless", $powershell, "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "$($script:Vm.GuestDir)\sampler.ps1", "-Folder", $embedSamples, "-Pattern", $ServerProcess
    Wait-For 4

    $queriesB64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes((ConvertTo-Json -InputObject ([string[]]$embedQueries) -Compress)))
    # Each server's own embeddings route, as cw-embed calls it: Ollama's
    # /api/embed, every other server's OpenAI-style /v1/embeddings.
    $benchUrl = if ($Provider -eq "ollama") { "http://127.0.0.1:11434/api/embed" } else { "http://127.0.0.1:$Port/v1/embeddings" }
    $measure = Invoke-GuestBackendScript -TimeoutSec ($JobSeconds + 1800) -Script ((@'
$folder = '{{FOLDER}}'
function Set-Phase { param([string]$Name) Set-Content (Join-Path $folder 'phase.txt') $Name -NoNewline }
$out = [ordered]@{}
Set-Phase 'load'
$models = Invoke-Backend GET '/v1/settings/embed/models'
$out.models = $models.Substring(0, [math]::Min(1500, $models.Length))
$watch = [Diagnostics.Stopwatch]::StartNew()
$test = Invoke-Backend POST '/v1/settings/test/embed' $null 900
$out.testWallMs = [int]$watch.ElapsedMilliseconds
$out.test = $test.Substring(0, [math]::Min(1500, $test.Length))
Set-Phase 'job'
$jobWatch = [Diagnostics.Stopwatch]::StartNew()
# A good test starts the job by itself. When none runs, ask for one.
$job = (Invoke-Backend GET '/v1/embed/job' | ConvertFrom-Json).job
$out.reembed = ''
if (-not ($job -and $job.phase -eq 'embedding')) { $out.reembed = Invoke-Backend POST '/v1/embed/reembed'; Start-Sleep -Seconds 2 }
$deadline = (Get-Date).AddSeconds({{JOBSECONDS}})
do {
    Start-Sleep -Seconds 2
    $job = (Invoke-Backend GET '/v1/embed/job' | ConvertFrom-Json).job
} while ($job -and $job.phase -eq 'embedding' -and (Get-Date) -lt $deadline)
$out.jobSeconds = [math]::Round($jobWatch.Elapsed.TotalSeconds, 1)
$out.job = $job
Set-Phase 'idle'
Start-Sleep -Seconds 20
# The server alone, the same work for every server: 64 fixed texts of about
# 500 characters in batches of 16 straight to its own embeddings route, once
# to warm up and then three timed rounds; then ten single short queries. The
# job above starts when the embedder is chosen, before any clock here, so its
# documents per second are not comparable; these are.
Set-Phase 'bench'
$benchUrl = '{{BENCHURL}}'
$benchModel = '{{BENCHMODEL}}'
$words = 'index', 'vector', 'module', 'parser', 'backend', 'settings', 'search', 'note', 'graph', 'token', 'cache', 'server', 'client', 'update', 'window', 'theme', 'secret', 'log'
$texts = New-Object 'string[]' 64
for ($i = 0; $i -lt 64; $i++) {
    $picked = New-Object 'string[]' 70
    for ($k = 0; $k -lt 70; $k++) { $picked[$k] = $words[($i * 7 + $k * 3) % $words.Count] }
    $texts[$i] = "Paragraph $i. " + ($picked -join ' ') + '.'
}
function Send-Batch {
    param([string[]]$Batch)
    $json = (New-Object PSObject -Property @{ model = $benchModel; input = $Batch }) | ConvertTo-Json -Compress -Depth 4
    Invoke-RestMethod -UseBasicParsing -Method Post -Uri $benchUrl -ContentType 'application/json' -Body ([Text.Encoding]::UTF8.GetBytes($json)) -TimeoutSec 900 | Out-Null
}
$roundsMs = @()
$queryMs = @()
$benchError = ''
try {
    Send-Batch $texts[0..15]
    for ($r = 0; $r -lt 3; $r++) {
        $watch = [Diagnostics.Stopwatch]::StartNew()
        for ($b = 0; $b -lt 64; $b += 16) { Send-Batch $texts[$b..($b + 15)] }
        $roundsMs += [int]$watch.ElapsedMilliseconds
    }
    for ($q = 0; $q -lt 10; $q++) {
        $watch = [Diagnostics.Stopwatch]::StartNew()
        Send-Batch @("where are the settings of query $q stored")
        $queryMs += [int]$watch.ElapsedMilliseconds
    }
} catch { $benchError = $_.Exception.Message }
$out.bench = [ordered]@{ url = $benchUrl; texts = 64; batch = 16; roundsMs = [int[]]$roundsMs; queryMs = [int[]]$queryMs; error = $benchError }
Set-Phase 'search'
'@ + "`n" + $guestSearchText + @'

$out.searches = $searches
'@ + "`n" + $guestResultText).Replace('{{FOLDER}}', $embedSamples).Replace('{{JOBSECONDS}}', "$JobSeconds").Replace('{{QUERIES}}', $queriesB64).Replace('{{BENCHURL}}', $benchUrl).Replace('{{BENCHMODEL}}', $Model))
    $data = Read-GuestResult $measure.Output
    $samples = Invoke-GuestScript -TimeoutSec 120 -Script "New-Item -ItemType File -Force '$embedSamples\stop.flag' | Out-Null; Start-Sleep 4; Get-Content '$embedSamples\samples.csv' -ErrorAction SilentlyContinue"
    $stats = Get-SampleStats ($samples.Output -split "`n")

    # The same five searches with the word-form embedder, for comparison.
    Set-GuestEmbedder -Provider "local" | Out-Null
    $localRun = Invoke-GuestBackendScript -TimeoutSec 600 -Script (($guestSearchText + "`n" + '$out = [ordered]@{ searches = $searches }' + "`n" + $guestResultText).Replace('{{QUERIES}}', $queriesB64))
    $local = Read-GuestResult $localRun.Output
    Stop-GuestApp

    # Stop the server and everything it started, and check that it is gone.
    $stopped = (Invoke-GuestScript -TimeoutSec 300 -Script ($StopScript + "`n" + @'
Start-Sleep 2
$pattern = '{{PATTERN}}'
function Get-Server { @(Get-CimInstance Win32_Process | Where-Object { $(if ($_.ExecutablePath) { $_.ExecutablePath } else { $_.Name }) -match $pattern }) }
foreach ($p in Get-Server) { Stop-Process -Id $p.ProcessId -Force -ErrorAction SilentlyContinue }
Start-Sleep 4
$left = Get-Server
"left: $($left.Count) $(($left | ForEach-Object { $_.Name }) -join ', ')"
"listening on {{PORT}}: $(@(Get-NetTCPConnection -State Listen -LocalPort {{PORT}} -ErrorAction SilentlyContinue).Count)"
'@.Replace('{{PATTERN}}', $ServerProcess).Replace('{{PORT}}', "$Port"))).Output

    if (-not $data) {
        Add-Result -Id $Id -Title $Title -Expect "the measuring script answers" -Status fail -Shots @($shot) -Output ("settings change: $set`n" + $measure.Output + "`n--- after stopping`n$stopped") `
            -Observed "the guest's measuring script printed no result (it did not parse, or ran out of time): $(($measure.Output -split "`n" | Select-Object -Last 2) -join '; ')"
        return
    }
    $test = try { $data.test | ConvertFrom-Json } catch { $null }
    $job = $data.job
    $listed = "$($data.models)" -match '"reachable":\s*true' -and "$($data.models)".Contains($Model.Split(":")[0])
    $tested = $test -and $test.ok -eq $true
    $stored = $job -and $job.phase -eq "done" -and $job.docs_total -gt 0 -and $job.docs_done -eq $job.docs_total
    $searches = @($data.searches)
    $localSearches = if ($local) { @($local.searches) } else { @() }
    $found = 0; $onlyVectors = 0; $withHits = 0
    foreach ($search in $searches) {
        $keys = @($search.keys)
        $baseline = @($localSearches | Where-Object { $_.q -eq $search.q } | ForEach-Object { $_.keys })
        $found += $keys.Count
        if ($keys.Count) { $withHits += 1 }
        $onlyVectors += @($keys | Where-Object { $baseline -notcontains $_ }).Count
    }
    $medianVectors = Get-MedianValue ($searches | ForEach-Object { $_.ms })
    $medianLocal = Get-MedianValue ($localSearches | ForEach-Object { $_.ms })
    $jobSeconds = [double]$data.jobSeconds
    $docs = if ($job) { [int]$job.docs_done } else { $null }
    $bench = $data.bench
    $benchRoundMs = if ($bench) { Get-MedianValue @($bench.roundsMs) } else { $null }
    $benchTextsPerS = if ($benchRoundMs -gt 0) { [math]::Round(64 / ($benchRoundMs / 1000), 1) } else { $null }
    $benchQueryMs = if ($bench) { Get-MedianValue @($bench.queryMs) } else { $null }
    $benchText = if ($benchTextsPerS) { "bench $benchTextsPerS texts/s (64 texts of ~500 characters, batches of 16, warm, median of 3 rounds: $(@($bench.roundsMs) -join '/') ms), single query median $benchQueryMs ms, guest CPU during it avg $($stats.CpuBenchAvg) %, peak $($stats.CpuBenchPeak) %" } else { "bench failed: $(if ($bench) { $bench.error } else { 'no result' })" }
    $installMb = if ($InstallFolder.Count) { Get-GuestFolderMb $InstallFolder } else { Get-EmbedFact $Provider "installMb" }
    $version = if ($ServerVersion) { $ServerVersion } else { Get-EmbedFact $Provider "version" }
    $gone = $stopped -match "left: 0" -and $stopped -match "listening on ${Port}: 0"
    $status = if ($listed -and $tested -and $stored -and $withHits -eq $searches.Count) { "pass" } elseif ($listed -and $tested -and $stored) { "note" } else { "fail" }

    # The word-form baseline is recorded once per run, before the first server.
    if (-not $script:embedLocalRecorded -and $local) {
        $embedRecords.Add([ordered]@{
                provider = "local"; model = "word forms (no model)"; status = "baseline"
                searchMedianMs = $medianLocal; queries = @($localSearches | ForEach-Object { [ordered]@{ q = $_.q; ms = $_.ms; keys = @($_.keys) } })
            })
        $script:embedLocalRecorded = $true
    }
    $embedRecords.Add([ordered]@{
            case                  = $Id
            provider              = $Provider
            serverVersion         = $version
            model                 = $Model
            modelFamily           = $ModelFamily
            quantisation          = $Quantisation
            installMb             = $installMb
            modelMb               = Get-EmbedFact $factKey "modelMb"
            downloadS             = Get-EmbedFact $Provider "downloadS"
            installS              = Get-EmbedFact $Provider "installS"
            pullS                 = Get-EmbedFact $factKey "pullS"
            firstCallMs           = if ($test) { $test.ms } else { $null }
            dimensions            = if ($test) { $test.dimensions } else { $null }
            docs                  = $docs
            jobS                  = $jobSeconds
            docsPerS              = if ($jobSeconds -gt 0 -and $docs) { [math]::Round($docs / $jobSeconds, 2) } else { $null }
            benchTextsPerS        = $benchTextsPerS
            benchRoundMs          = @($(if ($bench) { $bench.roundsMs }))
            benchQueryMedianMs    = $benchQueryMs
            benchError            = if ($bench) { $bench.error } else { "no result" }
            cpuBenchAvgPercent    = $stats.CpuBenchAvg
            cpuBenchPeakPercent   = $stats.CpuBenchPeak
            serverMemoryBenchPeakMb = $stats.MemoryBenchPeakMb
            searchMedianMs        = $medianVectors
            searchMedianMsLocal   = $medianLocal
            hitsFound             = $found
            hitsNotInLocal        = $onlyVectors
            serverMemoryPeakMb    = $stats.MemoryPeakMb
            serverMemoryIdleMb    = $stats.MemoryIdleMb
            cpuAvgPercent         = $stats.CpuAvg
            cpuPeakPercent        = $stats.CpuPeak
            gpuUsed               = $stats.Gpu
            guestFreeMemoryMinMb  = $stats.FreeMinMb
            queries               = @($searches | ForEach-Object { [ordered]@{ q = $_.q; ms = $_.ms; keys = @($_.keys) } })
            serverGone            = [bool]$gone
            status                = $status
            notes                 = (@($Note, "samples: $($stats.Samples)") | Where-Object { $_ }) -join "; "
        })
    ConvertTo-Json -InputObject ([object[]]$embedRecords.ToArray()) -Depth 8 | Set-Content (Join-Path $logs "embedders.json")

    $lines = @($searches | ForEach-Object { "  $($_.q): $($_.ms) ms, top $(@($_.keys).Count): $(@($_.keys) -join ' ; ')$(if ($_.error) { ' ERROR ' + $_.error })" })
    $localLines = @($localSearches | ForEach-Object { "  $($_.q): $($_.ms) ms, top $(@($_.keys).Count): $(@($_.keys) -join ' ; ')" })
    Add-Result -Id $Id -Title $Title -Expect "the model is listed; the test answers; the job is done with every document embedded; the five searches find hits" `
        -Status $status -Shots @($shot) -Output ("settings change: $set`n" + "models: $($data.models)`ntest: $($data.test)`njob: $($job | ConvertTo-Json -Compress)`nreembed asked for: '$($data.reembed)'`n--- searches with $Provider`n$($lines -join "`n")`n--- searches with local`n$($localLines -join "`n")`n--- samples`n$($samples.Output)`n--- after stopping the server`n$stopped") `
        -Observed ("$Provider ($version) with $Model ($Quantisation); listed: $listed; test answered: $tested, first call $(if ($test) { "$($test.ms) ms, $($test.dimensions) dimensions" } else { 'none' }); job $(if ($job) { "$($job.phase), $($job.docs_done) of $($job.docs_total) documents" } else { 'not read' }) $jobSeconds s after the test (the job starts when the embedder is chosen, so this is no speed); search median $medianVectors ms with vectors, $medianLocal ms with local; $withHits of $($searches.Count) searches had hits, $onlyVectors of $found hits are not in local's top 5; server memory peak $($stats.MemoryPeakMb) MB, idle $($stats.MemoryIdleMb) MB; guest CPU avg $($stats.CpuAvg) %, peak $($stats.CpuPeak) %; GPU: $($stats.Gpu); guest free memory min $($stats.FreeMinMb) MB; $benchText; server gone after stopping: $gone. $Note Read the screenshot: Settings > Semantic search names $Provider and $Model")
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

# Start `ollama serve` when no server answers (the tray app did not start, or
# the guest was restored since OL1, or an earlier case stopped Ollama).
function Start-OllamaServe {
    if (-not (Get-OllamaVersion)) {
        Start-GuestExe -Exe "C:\Windows\System32\conhost.exe" -Arguments "--headless", $ollamaExe, "serve"
        Wait-For 15
    }
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
    Set-EmbedFact "ollama" "version" $version
    Set-EmbedFact "ollama" "downloadS" (Get-OutputNumber $download.Output "downloaded: \d+ MB in (\d+) s")
    Set-EmbedFact "ollama" "installS" $seconds
    Add-Result -Id "OL1" -Title "Install Ollama silently" -Expect "the installer runs with /VERYSILENT as the user, ollama.exe is in place and the server answers on 11434" `
        -Status $(if ($installed -and $version) { "pass" } elseif ($installed) { "note" } else { "fail" }) -Shots @($shot) -Output ($download.Output + "`n--- after the install`n" + $facts) `
        -Observed "$ollamaDownload; $(($download.Output -split "`n" | Where-Object { $_ -match '^(downloaded|signature)' }) -join '; '); /VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP- ended after $seconds s with ollama.exe in place: $installed; the server answers on 11434 with version '$version' (empty: it did not start by itself; see the output for what runs and for any Defender event that names Ollama)"
}

Test-Case "OL2" "ollama pull embeddinggemma" "the model is pulled as README says and the server lists it" {
    Start-OllamaServe
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
    Set-EmbedFact "ollama/embeddinggemma" "pullS" (Get-OutputNumber $run.Output "pull exit: \d+ after (\d+) s")
    Set-EmbedFact "ollama/embeddinggemma" "modelMb" (Get-OutputNumber $run.Output "model: embeddinggemma\S*, (\d+) MB")
    Add-Result -Id "OL2" -Title "ollama pull embeddinggemma" -Expect "pull exits 0 and /api/tags lists embeddinggemma" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
        -Observed "$(($run.Output -split "`n" | Where-Object { $_ -match '^(pull exit|model:|models folder)' }) -join '; ')"
}

Test-Case "OL3" "Semantic search through Ollama" "Ollama and embeddinggemma chosen: the page lists the model, a reembed stores every document's vector, and the five searches by meaning answer" {
    $version = Get-OllamaVersion
    if (-not $version) { throw "no Ollama answers on port 11434; run OL1 and OL2 first" }
    # embeddinggemma is the model README names for Ollama. Quantisation: the
    # library's default tag; Ollama's page for it states none.
    Measure-RealEmbedder -Id "OL3" -Title "Semantic search through Ollama" -Provider ollama -Model "embeddinggemma" -ServerVersion $version `
        -ServerProcess '\\Ollama\\|\\ollama[^\\]*\.exe$' -Port 11434 -ModelFamily "embeddinggemma" -Quantisation "library default (Ollama's page names none)" `
        -InstallFolder @("$guestHome\AppData\Local\Programs\Ollama") `
        -Note "README's model for Ollama; not the same model as the nomic-embed-text cases."
}

# The same model as llama.cpp and LM Studio run (nomic-embed-text v1.5), so
# that the figures compare servers. https://ollama.com/library/nomic-embed-text:
# `ollama pull nomic-embed-text`, 274 MB, the default tag is v1.5 and the same
# size as the tag 137m-v1.5-fp16, that is 16-bit floats, not a quantisation.
Test-Case "OL4" "ollama pull nomic-embed-text" "the model is pulled and the server lists it" {
    Start-OllamaServe
    if (-not (Get-OllamaVersion)) { throw "no Ollama answers on port 11434; run OL1 first" }
    $run = Invoke-GuestScript -Background -TimeoutSec 2700 -Script (@'
$watch = Get-Date
$text = & '{{EXE}}' pull nomic-embed-text 2>&1 | Out-String
$seconds = [int]((Get-Date) - $watch).TotalSeconds
"pull exit: $LASTEXITCODE after $seconds s"
$clean = ($text -replace '[^\x20-\x7E\r\n]', '') -split "`r?`n|`r" | Where-Object { $_.Trim() } | Select-Object -Last 4
"pull output ends: $($clean -join ' | ')"
$tags = (Invoke-RestMethod -TimeoutSec 20 'http://localhost:11434/api/tags').models | Where-Object { $_.name -like 'nomic-embed-text*' }
foreach ($tag in $tags) { "model: $($tag.name), $([math]::Round($tag.size / 1MB)) MB" }
$global:LASTEXITCODE = 0
'@).Replace('{{EXE}}', $ollamaExe)
    $ok = $run.Output -match "pull exit: 0" -and $run.Output -match "model: nomic-embed-text"
    Set-EmbedFact "ollama/nomic-embed-text" "pullS" (Get-OutputNumber $run.Output "pull exit: \d+ after (\d+) s")
    Set-EmbedFact "ollama/nomic-embed-text" "modelMb" (Get-OutputNumber $run.Output "model: nomic-embed-text\S*, (\d+) MB")
    Add-Result -Id "OL4" -Title "ollama pull nomic-embed-text" -Expect "pull exits 0 and /api/tags lists nomic-embed-text" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
        -Observed "$(($run.Output -split "`n" | Where-Object { $_ -match '^(pull exit|model:)' }) -join '; ')"
}

Test-Case "OL5" "Semantic search through Ollama with nomic-embed-text" "Ollama and nomic-embed-text chosen: the model is listed, a reembed stores every document's vector, and the five searches answer" {
    Start-OllamaServe
    $version = Get-OllamaVersion
    if (-not $version) { throw "no Ollama answers on port 11434; run OL1 and OL4 first" }
    Measure-RealEmbedder -Id "OL5" -Title "Semantic search through Ollama with nomic-embed-text" -Provider ollama -Model "nomic-embed-text" -ServerVersion $version `
        -ServerProcess '\\Ollama\\|\\ollama[^\\]*\.exe$' -Port 11434 -ModelFamily "nomic-embed-text v1.5" -Quantisation "F16 (the default tag is 137m-v1.5-fp16)" `
        -InstallFolder @("$guestHome\AppData\Local\Programs\Ollama") `
        -Note "Same model family as the llama.cpp and LM Studio cases (v1.5)."
}

# ---------------------------------------------------------------- LC, llama.cpp's llama-server
# README: "start `llama-server` with [a GGUF embedding model]. For long texts,
# start `llama-server` with `-ub 2048 -b 2048`". Run `-Only D2,LC1,LC2,LC3`.
# Sources (read 2026-11-21):
#  - https://github.com/ggml-org/llama.cpp/releases: each build is a release
#    tagged b<number>, with the asset llama-b<number>-bin-win-cpu-x64.zip (the
#    "latest" release of the repository is a different tag, v1.0.0, with one
#    asset, so the newest releases are listed and the first with that asset is
#    taken).
#  - https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md:
#    --embedding ("restrict to only support embedding use case; use only with
#    dedicated embedding models"), --port (default 9931), -ub and -b.
#  - https://huggingface.co/nomic-ai/nomic-embed-text-v1.5-GGUF: not gated,
#    nomic-embed-text-v1.5.Q8_0.gguf is 146,146,432 bytes (Apache-2.0).
# The zip needs the Visual C++ runtime, which a clean Windows lacks: in the
# first run of these cases (2026-11-21) `llama-server --version` ended with
# 0xC0000135 (a DLL not found), and Lemonade's own llama-server failed to start
# the same way. Install-GuestVcRuntime installs Microsoft's redistributable
# first, as a user of llama.cpp on such a machine must.
$script:vcRuntime = $null
function Install-GuestVcRuntime {
    if ($script:vcRuntime) { return $script:vcRuntime }
    # https://learn.microsoft.com/cpp/windows/latest-supported-vc-redist: the
    # permanent link to the newest x64 redistributable. A Burn bundle: /install
    # /quiet /norestart; it needs an administrator, so it runs in the elevated
    # PowerShell Direct session. Exit 3010 means installed, restart wanted;
    # 1638 means a newer one is already there.
    $run = Invoke-GuestScript -Background -TimeoutSec 900 -Script @'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$before = Test-Path "$env:SystemRoot\System32\vcruntime140.dll"
"vcruntime140.dll before: $before"
$setup = Join-Path $env:TEMP 'vc_redist.x64.exe'
$watch = Get-Date
Invoke-WebRequest -UseBasicParsing 'https://aka.ms/vs/17/release/vc_redist.x64.exe' -OutFile $setup
"downloaded: $([math]::Round((Get-Item $setup).Length / 1MB, 1)) MB in $([int]((Get-Date) - $watch).TotalSeconds) s"
$watch = Get-Date
$process = Start-Process $setup -ArgumentList '/install', '/quiet', '/norestart' -Wait -PassThru
"install exit: $($process.ExitCode) after $([int]((Get-Date) - $watch).TotalSeconds) s"
Remove-Item $setup -Force -ErrorAction SilentlyContinue
"vcruntime140.dll after: $(Test-Path "$env:SystemRoot\System32\vcruntime140.dll")"
'@
    $script:vcRuntime = "Visual C++ runtime: " + (($run.Output -split "`n" | Where-Object { $_ -match '^(vcruntime140.dll|downloaded|install exit)' } | ForEach-Object { $_.Trim() }) -join '; ')
    $script:vcRuntime
}
$llamaFolder = "$guestHome\llama.cpp"
$llamaServer = "$llamaFolder\llama-server.exe"
$llamaModelId = $null
$nomicUrl = "https://huggingface.co/nomic-ai/nomic-embed-text-v1.5-GGUF/resolve/main/nomic-embed-text-v1.5.Q8_0.gguf"
$nomicFile = "$embedModels\nomic-embed-text-v1.5.Q8_0.gguf"
$llamaPattern = '\\llama\.cpp\\|\\llama-server\.exe$'

Test-Case "LC1" "Install llama.cpp's llama-server" "the newest Windows CPU build of llama.cpp unpacks and llama-server.exe reports its version" {
    $vc = Install-GuestVcRuntime
    $run = Invoke-GuestScript -Background -TimeoutSec 1800 -Script ((Get-GuestFreeDiskText 3) + "`n" + (@'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$folder = '{{FOLDER}}'
$downloads = '{{HOME}}\Downloads\llamacpp'
$releases = Invoke-RestMethod -UseBasicParsing -Headers @{ 'User-Agent' = 'codewiki-vm-test' } 'https://api.github.com/repos/ggml-org/llama.cpp/releases?per_page=10'
$asset = $null
$tag = ''
foreach ($release in $releases) {
    $found = @($release.assets | Where-Object { $_.name -match '^llama-b\d+-bin-win-cpu-x64\.zip$' })
    if ($found.Count) { $asset = $found[0]; $tag = $release.tag_name; break }
}
if (-not $asset) { 'result: no Windows CPU asset in the newest 10 releases'; return }
"release: $tag, asset: $($asset.name), $([math]::Round($asset.size / 1MB, 1)) MB"
New-Item -ItemType Directory -Force $downloads, $folder | Out-Null
$zip = Join-Path $downloads $asset.name
$watch = Get-Date
Invoke-WebRequest -UseBasicParsing $asset.browser_download_url -OutFile $zip
"downloaded: $([math]::Round((Get-Item $zip).Length / 1MB, 1)) MB in $([int]((Get-Date) - $watch).TotalSeconds) s"
$watch = Get-Date
Expand-Archive -Path $zip -DestinationPath $folder -Force
"extracted in $([int]((Get-Date) - $watch).TotalSeconds) s"
Remove-Item $zip -Force
$exe = Get-ChildItem $folder -Recurse -Filter llama-server.exe | Select-Object -First 1
if (-not $exe) { 'result: no llama-server.exe in the archive'; return }
"exe: $($exe.FullName)"
$signature = Get-AuthenticodeSignature $exe.FullName
"signature: $($signature.Status) $($signature.SignerCertificate.Subject)"
$version = @(& $exe.FullName --version 2>&1 | ForEach-Object { "$_" })
"version exit: $LASTEXITCODE"
"version output: $($version -join ' | ')"
"installed MB: $([math]::Round(((Get-ChildItem $folder -Recurse -File | Measure-Object Length -Sum).Sum) / 1MB, 1))"
'@).Replace('{{FOLDER}}', $llamaFolder).Replace('{{HOME}}', $guestHome) + "`n" + (Get-GuestDefenderText "llama") + "`n" + "'result: installed'")
    $exePath = if ($run.Output -match "(?m)^exe: (.+)$") { $Matches[1].Trim() } else { $null }
    if ($exePath) { $script:llamaServer = $exePath }
    $version = if ($run.Output -match "version: (\d+) \((\w+)\)") { "b$($Matches[1]) ($($Matches[2]))" } elseif ($run.Output -match "release: (b\d+)") { $Matches[1] } else { $null }
    Set-EmbedFact "llamacpp" "version" $version
    Set-EmbedFact "llamacpp" "installMb" (Get-OutputNumber $run.Output "installed MB: ([\d\.]+)")
    Set-EmbedFact "llamacpp" "downloadS" (Get-OutputNumber $run.Output "downloaded: [\d\.]+ MB in (\d+) s")
    Set-EmbedFact "llamacpp" "installS" (Get-OutputNumber $run.Output "extracted in (\d+) s")
    $ok = $run.Output -match "result: installed" -and $run.Output -match "version exit: 0"
    Add-Result -Id "LC1" -Title "Install llama.cpp's llama-server" -Expect "the Windows CPU zip downloads, unpacks, and llama-server.exe --version exits 0" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Output ("$vc`n" + $run.Output) `
        -Observed "$vc; $(($run.Output -split "`n" | Where-Object { $_ -match '^(release|downloaded|extracted|signature|version exit|installed MB|Defender)' }) -join '; ')"
}

Test-Case "LC2" "llama-server with nomic-embed-text v1.5" "README's flags start llama-server on port 9931 and it lists the model" {
    if (-not (Test-Path (Join-Path $PSScriptRoot "guest\serve-logged.ps1"))) { throw "guest\serve-logged.ps1 is missing" }
    $download = Invoke-GuestScript -Background -TimeoutSec 1800 -Script ((Get-GuestFreeDiskText 2) + "`n" + (@'
$file = '{{FILE}}'
if ((Test-Path $file) -and (Get-Item $file).Length -gt 100MB) { 'downloaded: already there in 0 s' } else {
{{DOWNLOAD}}
}
"model MB: $([math]::Round((Get-Item $file).Length / 1MB, 1))"
'@).Replace('{{FILE}}', $nomicFile).Replace('{{DOWNLOAD}}', (Get-GuestDownloadText $nomicUrl $nomicFile)))
    $log = "$embedLogs\llama-server"
    # README's flags: --embedding for an embedding model, -ub 2048 -b 2048 for long texts.
    Start-LoggedServer -Exe $llamaServer -Log $log -ArgLine "-m $nomicFile --embedding --port 9931 -ub 2048 -b 2048"
    $wait = Wait-GuestServer -Url "http://127.0.0.1:9931/v1/models" -ServerProcess $llamaPattern -Seconds 180 -Log $log
    $resolved = Resolve-GuestModelId -Url "http://127.0.0.1:9931/v1/models" -Match "nomic"
    $script:llamaModelId = $resolved.Id
    Set-EmbedFact "llamacpp/$($resolved.Id)" "modelMb" (Get-OutputNumber $download.Output "model MB: ([\d\.]+)")
    Set-EmbedFact "llamacpp/$($resolved.Id)" "pullS" (Get-OutputNumber $download.Output "downloaded: .* in (\d+) s")
    $shot = Shot "llama-server-running"
    Add-Result -Id "LC2" -Title "llama-server with nomic-embed-text v1.5" -Expect "llama-server -m <Q8_0 GGUF> --embedding --port 9931 -ub 2048 -b 2048 answers and lists the model" `
        -Status $(if ($wait -match "answered after" -and $resolved.Id) { "pass" } else { "fail" }) -Shots @($shot) -Output ($download.Output + "`n" + $wait + "`n" + $resolved.Output) `
        -Observed "$nomicUrl; $(($download.Output -split "`n" | Where-Object { $_ -match '^(downloaded|model MB)' }) -join '; '); $(($wait -split "`n" | Select-Object -First 1)); the server lists the model as '$($resolved.Id)'"
}

Test-Case "LC3" "Semantic search through llama.cpp" "llama-server and nomic-embed-text chosen: the model is listed, a reembed stores every document's vector, and the five searches answer" {
    if (-not $llamaModelId) { $llamaModelId = (Resolve-GuestModelId -Url "http://127.0.0.1:9931/v1/models" -Match "nomic").Id }
    if (-not $llamaModelId) { throw "llama-server lists no model on port 9931; run LC1 and LC2 first" }
    Measure-RealEmbedder -Id "LC3" -Title "Semantic search through llama.cpp" -Provider llamacpp -Model $llamaModelId `
        -ServerProcess $llamaPattern -Port 9931 -ModelFamily "nomic-embed-text v1.5" -Quantisation "Q8_0 (GGUF)" -InstallFolder @($llamaFolder) `
        -Note "CPU build; -ub 2048 -b 2048 as README says."
}

# ---------------------------------------------------------------- Llama for Windows
# README names "Llama for Windows" beside llama-server (port 9931, "custom serve
# arguments"). github.com/ggml-org/Llama-Windows is a WinUI tray app that runs
# `llama serve`, a router (docs\superpowers\specs\2026-11-21-local-embedders-design.md).
# No case: see the report. Its serving is llama-server with a different front,
# which LC1 to LC3 test, and its window cannot be driven reliably by this rig.

# ---------------------------------------------------------------- LS, LM Studio
# Run `-Only D2,LS1,LS2,LS3`. Sources (read 2026-11-21):
#  - https://lmstudio.ai/docs/developer/core/headless: "irm https://lmstudio.ai/install.ps1 | iex"
#    installs llmster, the daemon, and `lms`; `lms daemon up`, `lms server start`.
#    The script itself (https://lmstudio.ai/install.ps1) installs to
#    $HOME/.lmstudio/bin and runs `lms bootstrap -y` to add it to the user's PATH.
#  - https://lmstudio.ai/docs/cli/serve/server-start: `lms server start --port <n>`
#    (port 1234 by default, "last used" otherwise).
#  - https://lmstudio.ai/docs/cli/local-models/get: `lms get <model>[@<quantization>]`,
#    `--gguf`; /load: `lms load <model_key>`, `--identifier`; /import:
#    `lms import <file> --user-repo <user>/<repo> -y --copy`.
# Not verified: the catalog name LM Studio gives the nomic-embed-text v1.5
# model to `lms get` (the docs show no embedding example), `-y` on `lms get`
# and `lms load`, and which quantization it picks. LS2 therefore tries three
# names, with and without -y, logs `lms get --help` and `lms ls`, and falls
# back to `lms import` of the same Q8_0 file that llama.cpp uses; it says which
# way worked, and the quantization LM Studio lists.
$lmsExe = "$guestHome\.lmstudio\bin\lms.exe"
$lmsPattern = '\\\.lmstudio\\|\\llmster[^\\]*\.exe$|\\lms\.exe$'

Test-Case "LS1" "Install LM Studio's llmster" "the install script from lmstudio.ai installs lms for the user" {
    $run = Invoke-GuestScript -TimeoutSec 1800 -Script ((Get-GuestFreeDiskText 4) + "`n" + (@'
$watch = Get-Date
# The command as the docs give it, in its own PowerShell: the script calls exit when it fails.
$child = @(& powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; irm https://lmstudio.ai/install.ps1 | iex" 2>&1 | ForEach-Object { "$_" })
$code = $LASTEXITCODE
"as written exit: $code"
"as written output ends: $((($child | Where-Object { $_.Trim() }) | Select-Object -Last 10) -join ' | ')"
$lms = @("$env:USERPROFILE\.lmstudio\bin\lms.exe", "$env:LOCALAPPDATA\lm-studio\bin\lms.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $lms) {
    # The script unpacks into %TEMP%\llmster-install-<32 hex digits> with .NET's
    # ZipFile, which takes no path longer than 260 characters. Under a profile
    # such as "C:\Users\Software Tester" that failed on 2026-11-21 with a
    # DirectoryNotFoundException for a file deep in the archive. Once more,
    # with a short TEMP.
    New-Item -ItemType Directory -Force 'C:\t' | Out-Null
    $child = @(& powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "`$env:TEMP = 'C:\t'; `$env:TMP = 'C:\t'; [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; irm https://lmstudio.ai/install.ps1 | iex" 2>&1 | ForEach-Object { "$_" })
    $code = $LASTEXITCODE
    "with TEMP=C:\t exit: $code"
    "with TEMP=C:\t output ends: $((($child | Where-Object { $_.Trim() }) | Select-Object -Last 10) -join ' | ')"
}
"installer exit: $code"
"installed in $([int]((Get-Date) - $watch).TotalSeconds) s"
$lms = @("$env:USERPROFILE\.lmstudio\bin\lms.exe", "$env:LOCALAPPDATA\lm-studio\bin\lms.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
"lms: $lms"
if (-not $lms) { 'result: no lms'; return }
"version: $((@(& $lms --version 2>&1 | ForEach-Object { "$_" })) -join ' | ')"
$home1 = Split-Path -Parent (Split-Path -Parent $lms)
"installed MB: $([math]::Round(((Get-ChildItem $home1 -Recurse -File -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum) / 1MB, 1))"
"on user PATH: $(([Environment]::GetEnvironmentVariable('Path', 'User') -split ';') -contains (Split-Path $lms))"
'@) + "`n" + (Get-GuestDefenderText "lmstudio|llmster|lms.exe") + "`n" + "'result: installed'")
    if ($run.Output -match "(?m)^lms: (.+)$") { $script:lmsExe = $Matches[1].Trim() }
    Set-EmbedFact "lmstudio" "version" $(if ($run.Output -match "(?m)^version: (.+)$") { $Matches[1].Trim() } else { $null })
    Set-EmbedFact "lmstudio" "installMb" (Get-OutputNumber $run.Output "installed MB: ([\d\.]+)")
    Set-EmbedFact "lmstudio" "installS" (Get-OutputNumber $run.Output "installed in (\d+) s")
    $ok = $run.Output -match "result: installed" -and $run.Output -match "installer exit: 0"
    Add-Result -Id "LS1" -Title "Install LM Studio's llmster" -Expect "irm https://lmstudio.ai/install.ps1 | iex exits 0 and lms.exe is in place" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Output $run.Output `
        -Observed "$(($run.Output -split "`n" | Where-Object { $_ -match '^(as written exit|with TEMP=C:\\t exit|installer exit|installed in|lms:|version:|installed MB|on user PATH|Defender)' }) -join '; ') (the script downloads and unpacks in one step: no separate download time)"
}

Test-Case "LS2" "lms get and load nomic-embed-text" "lms downloads an embedding model and loads it" {
    if (-not (Test-Path (Join-Path $PSScriptRoot "guest\serve-logged.ps1"))) { throw "guest\serve-logged.ps1 is missing" }
    Start-GuestExe -Exe $lmsExe -Arguments "daemon", "up"
    Wait-For 20
    $run = Invoke-GuestScript -TimeoutSec 3600 -Script ((Get-GuestFreeDiskText 3) + "`n" + (@'
$lms = '{{LMS}}'
$log = '{{LOG}}'
New-Item -ItemType Directory -Force (Split-Path -Parent $log) | Out-Null
# One lms call with a time limit, so that a prompt nobody answers cannot hang the run.
function Invoke-Lms {
    param([string[]]$Arguments, [int]$Seconds = 240)
    Remove-Item "$log.out", "$log.err" -ErrorAction SilentlyContinue
    $p = Start-Process -FilePath $lms -ArgumentList $Arguments -NoNewWindow -PassThru -RedirectStandardOutput "$log.out" -RedirectStandardError "$log.err"
    if (-not $p.WaitForExit($Seconds * 1000)) { $p.Kill(); "  timed out after $Seconds s" } else { "  exit: $($p.ExitCode)" }
    foreach ($file in "$log.out", "$log.err") { if (Test-Path $file) { Get-Content $file -Tail 8 | ForEach-Object { '  | ' + ($_ -replace '[^\x20-\x7E]', '') } } }
}
function Get-Listing { (@(& $lms ls 2>&1 | ForEach-Object { "$_" }) -join "`n") }
"--- lms get --help"
Invoke-Lms @('get', '--help') 30
$route = 'none'
$watch = Get-Date
foreach ($name in 'nomic-ai/nomic-embed-text-v1.5-GGUF@Q8_0', 'nomic-embed-text-v1.5@q8_0', 'nomic-embed-text-v1.5') {
    foreach ($yes in @('-y', '')) {
        if ((Get-Listing) -match '(?i)nomic') { break }
        $arguments = @('get', $name, '--gguf'); if ($yes) { $arguments += $yes }
        "--- lms $($arguments -join ' ')"
        Invoke-Lms $arguments 300
    }
    if ((Get-Listing) -match '(?i)nomic') { $route = "lms get $name"; break }
}
$getSeconds = [int]((Get-Date) - $watch).TotalSeconds
if ($route -eq 'none') {
    "--- falling back to lms import of the Q8_0 file"
    $file = '{{FILE}}'
    if (-not ((Test-Path $file) -and (Get-Item $file).Length -gt 100MB)) {
{{DOWNLOAD}}
    }
    Invoke-Lms @('import', $file, '--user-repo', 'nomic-ai/nomic-embed-text-v1.5-GGUF', '-y', '--copy') 300
    if ((Get-Listing) -match '(?i)nomic') { $route = 'lms import' }
}
"get seconds: $getSeconds"
"route: $route"
$listing = Get-Listing
"--- lms ls"
$listing
$key = ($listing -split "`n" | ForEach-Object { if ($_ -match '^\s*(\S*nomic\S*)') { $Matches[1] } } | Select-Object -First 1)
"key: $key"
$quant = ($listing -split "`n" | Where-Object { $_ -match '(?i)nomic' } | ForEach-Object { if ($_ -match '(?i)\b(Q\d\w*|F16|F32|BF16)\b') { $Matches[1] } } | Select-Object -First 1)
"quantization: $quant"
if ($key) {
    "--- lms load $key"
    $watch = Get-Date
    Invoke-Lms @('load', $key) 600
    "load seconds: $([int]((Get-Date) - $watch).TotalSeconds)"
}
"--- lms ps"
Invoke-Lms @('ps') 60
$models = Join-Path (Split-Path -Parent (Split-Path -Parent $lms)) 'models'
"model MB: $([math]::Round(((Get-ChildItem $models -Recurse -File -ErrorAction SilentlyContinue | Where-Object { $_.FullName -match 'nomic' } | Measure-Object Length -Sum).Sum) / 1MB, 1))"
'@).Replace('{{LMS}}', $lmsExe).Replace('{{LOG}}', "$embedLogs\lms").Replace('{{FILE}}', $nomicFile).Replace('{{DOWNLOAD}}', (Get-GuestDownloadText $nomicUrl $nomicFile)))
    $key = if ($run.Output -match "(?m)^key: (\S+)") { $Matches[1] } else { $null }
    $route = if ($run.Output -match "(?m)^route: (.+)$") { $Matches[1].Trim() } else { "unknown" }
    $quant = if ($run.Output -match "(?m)^quantization: (\S+)") { $Matches[1] } else { "not shown by lms ls" }
    $script:lmsQuantisation = $quant
    Set-EmbedFact "lmstudio/nomic" "modelMb" (Get-OutputNumber $run.Output "model MB: ([\d\.]+)")
    Set-EmbedFact "lmstudio/nomic" "pullS" (Get-OutputNumber $run.Output "get seconds: (\d+)")
    $loaded = $run.Output -match "(?s)--- lms ps.*nomic"
    $shot = Shot "lmstudio-model-loaded"
    Add-Result -Id "LS2" -Title "lms get and load nomic-embed-text" -Expect "lms get downloads nomic-embed-text v1.5, lms load loads it and lms ps lists it" `
        -Status $(if ($route -like "lms get*" -and $loaded) { "pass" } elseif ($key -and $loaded) { "note" } else { "fail" }) -Shots @($shot) -Output $run.Output `
        -Observed "model reached the machine by: $route (anything but 'lms get ...' means the catalog name or the -y flag was not accepted, see the output); model key '$key', quantization $quant, loaded: $loaded; $(($run.Output -split "`n" | Where-Object { $_ -match '^(get seconds|load seconds|model MB)' }) -join '; ')"
}

Test-Case "LS3" "Semantic search through LM Studio" "LM Studio's server and nomic-embed-text chosen: the model is listed, a reembed stores every document's vector, and the five searches answer" {
    Start-GuestExe -Exe $lmsExe -Arguments "server", "start", "--port", "1234"
    $wait = Wait-GuestServer -Url "http://127.0.0.1:1234/v1/models" -ServerProcess $lmsPattern -Seconds 120
    $resolved = Resolve-GuestModelId -Url "http://127.0.0.1:1234/v1/models" -Match "nomic"
    if (-not $resolved.Id) { throw "LM Studio lists no nomic model on port 1234; run LS1 and LS2 first. $($resolved.Output) $wait" }
    Set-EmbedFact "lmstudio/$($resolved.Id)" "modelMb" (Get-EmbedFact "lmstudio/nomic" "modelMb")
    Set-EmbedFact "lmstudio/$($resolved.Id)" "pullS" (Get-EmbedFact "lmstudio/nomic" "pullS")
    $quant = if ($script:lmsQuantisation) { $script:lmsQuantisation } else { "unknown (run LS2)" }
    Measure-RealEmbedder -Id "LS3" -Title "Semantic search through LM Studio" -Provider lmstudio -Model $resolved.Id `
        -ServerProcess $lmsPattern -Port 1234 -ModelFamily "nomic-embed-text v1.5" -Quantisation $quant `
        -StopScript ("& '$lmsExe' server stop 2>&1 | Out-Null; & '$lmsExe' daemon down 2>&1 | Out-Null") `
        -Note "llmster headless; runtime as LM Studio chose it. The model's quantization is what lms get picked, see LS2."
}

# ---------------------------------------------------------------- LM, Lemonade Server
# Run `-Only D2,LM1,LM2,LM3`. Sources (read 2026-11-21):
#  - https://github.com/lemonade-sdk/lemonade/blob/main/docs/guide/install/windows.md:
#    "Download and run the latest Lemonade MSI installer",
#    https://github.com/lemonade-sdk/lemonade/releases/latest/download/lemonade.msi (11 MB).
#  - src/cpp/installer/Product.wxs.in and .github/actions/install-lemonade-server-msi/action.yml
#    in that repository: `msiexec /i <msi> /qn /norestart` installs per user to
#    %LOCALAPPDATA%\lemonade_server (bin\lemonade.exe, bin\LemonadeServer.exe,
#    which embeds the server and a tray icon), adds bin to the user's PATH, and
#    the installer starts LemonadeServer.exe when it ends (AutoLaunchLemonade).
#  - docs/guide/cli.md: the CLI is `lemonade` (README's `lemonade-server pull` is the
#    older name), `lemonade pull <model>`, `lemonade load <model>`, `lemonade status`,
#    `lemonade unload`, port 13305 by default.
#  - src/cpp/resources/server_models.json: "nomic-embed-text-v1-GGUF" is
#    nomic-ai/nomic-embed-text-v1-GGUF:Q4_K_S, recipe llamacpp, about 80 MB. It is
#    v1, not v1.5: another model than the llama.cpp, LM Studio and Ollama cases.
# Not verified: that LemonadeServer.exe answers without a click after the
# install (LM2 starts it when `lemonade status` fails), the folder Lemonade
# keeps models in (LM2 looks for the file), and that loading the model fetches
# a llama.cpp backend (it is timed as "load" in LM2's output).
$lemonadeExe = "$guestHome\AppData\Local\lemonade_server\bin\lemonade.exe"
$lemonadeServerExe = "$guestHome\AppData\Local\lemonade_server\bin\LemonadeServer.exe"
$lemonadePattern = 'lemonade|lemond|\\llama-server\.exe$'

Test-Case "LM1" "Install Lemonade Server silently" "lemonade.msi installs for the user with /qn, and lemonade.exe reports its version" {
    $download = Invoke-GuestScript -Background -TimeoutSec 1200 -Script ((Get-GuestFreeDiskText 2) + "`n" + (Get-GuestDownloadText "https://github.com/lemonade-sdk/lemonade/releases/latest/download/lemonade.msi" "$guestHome\Downloads\lemonade\lemonade.msi") + @'

$signature = Get-AuthenticodeSignature '{{HOME}}\Downloads\lemonade\lemonade.msi'
"signature: $($signature.Status) $($signature.SignerCertificate.Subject)"
'result: downloaded'
'@.Replace('{{HOME}}', $guestHome))
    if ($download.Output -notmatch "result: downloaded") {
        Add-Result -Id "LM1" -Title "Install Lemonade Server silently" -Expect "the installer downloads" -Status fail -Output $download.Output `
            -Observed "the installer was not downloaded: $(($download.Output -split "`n" | Select-Object -Last 2) -join '; ')"
        return
    }
    $msi = "$guestHome\Downloads\lemonade\lemonade.msi"
    $watch = Get-Date
    # Not Invoke-GuestExe: the installer starts the server, which would keep the output pipe open.
    Start-GuestExe -Exe "C:\Windows\System32\msiexec.exe" -Arguments "/i", $msi, "/qn", "/norestart", "/L*V", "$guestHome\Downloads\lemonade\install.log"
    $installed = $false
    $deadline = (Get-Date).AddMinutes(10)
    while ((Get-Date) -lt $deadline -and -not $installed) {
        Start-Sleep -Seconds 10
        $state = (Invoke-GuestScript -TimeoutSec 60 -Script "`"msi running: `$(@(Get-CimInstance Win32_Process -Filter `"Name = 'msiexec.exe'`" | Where-Object { `$_.CommandLine -match 'lemonade.msi' }).Count)`"; `"exe: `$(Test-Path '$lemonadeExe')`"").Output
        $installed = $state -match "msi running: 0" -and $state -match "exe: True"
    }
    $seconds = [int]((Get-Date) - $watch).TotalSeconds
    Wait-For 10
    $facts = (Invoke-GuestScript -TimeoutSec 180 -Script (@'
$exe = '{{EXE}}'
$ErrorActionPreference = 'Continue'
"version: $((@(& $exe --version 2>&1 | ForEach-Object { "$_" })) -join ' | ')"
"installed MB: $([math]::Round(((Get-ChildItem (Split-Path -Parent (Split-Path -Parent $exe)) -Recurse -File -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum) / 1MB, 1))"
"on user PATH: $(([Environment]::GetEnvironmentVariable('Path', 'User') -split ';') -contains (Split-Path $exe))"
"status: $((@(& $exe status 2>&1 | ForEach-Object { "$_" }) | Select-Object -First 3) -join ' | ')"
"processes: $((Get-Process | Where-Object { $_.ProcessName -match 'lemon' } | ForEach-Object { $_.ProcessName }) -join ', ')"
Remove-Item '{{HOME}}\Downloads\lemonade\lemonade.msi' -Force -ErrorAction SilentlyContinue
"msi deleted: $(-not (Test-Path '{{HOME}}\Downloads\lemonade\lemonade.msi'))"
'@.Replace('{{EXE}}', $lemonadeExe).Replace('{{HOME}}', $guestHome) + "`n" + (Get-GuestDefenderText "lemonade"))).Output
    $shot = Shot "lemonade-installed"
    Set-EmbedFact "lemonade" "version" $(if ($facts -match "(?m)^version: (.+)$") { $Matches[1].Trim() } else { $null })
    Set-EmbedFact "lemonade" "installMb" (Get-OutputNumber $facts "installed MB: ([\d\.]+)")
    Set-EmbedFact "lemonade" "downloadS" (Get-OutputNumber $download.Output "downloaded: [\d\.]+ MB in (\d+) s")
    Set-EmbedFact "lemonade" "installS" $seconds
    Add-Result -Id "LM1" -Title "Install Lemonade Server silently" -Expect "msiexec /i lemonade.msi /qn /norestart ends with lemonade.exe in place and a version" `
        -Status $(if ($installed -and $facts -match "version: \S") { "pass" } elseif ($installed) { "note" } else { "fail" }) -Shots @($shot) -Output ($download.Output + "`n--- after the install`n" + $facts) `
        -Observed "https://github.com/lemonade-sdk/lemonade/releases/latest/download/lemonade.msi; $(($download.Output -split "`n" | Where-Object { $_ -match '^(downloaded|signature)' }) -join '; '); msiexec /qn ended after $seconds s with lemonade.exe in place: $installed; $(($facts -split "`n" | Where-Object { $_ -match '^(version|installed MB|on user PATH|status|processes|msi deleted|Defender)' }) -join '; ')"
}

Test-Case "LM2" "lemonade pull nomic-embed-text-v1-GGUF" "the model is pulled as README says and loaded" {
    # Lemonade's llama-server needs the Visual C++ runtime as well (see LC1).
    $vc = Install-GuestVcRuntime
    $up =(Invoke-GuestScript -TimeoutSec 120 -Script "`$ErrorActionPreference = 'Continue'; (@(& '$lemonadeExe' status 2>&1 | ForEach-Object { `"`$_`" }) -join ' | '); `"status exit: `$LASTEXITCODE`"; `$global:LASTEXITCODE = 0").Output
    if ($up -notmatch "status exit: 0") {
        # The installer's own start did not happen or did not last.
        Start-GuestExe -Exe $lemonadeServerExe
        Wait-For 25
    }
    $run = Invoke-GuestScript -Background -TimeoutSec 3000 -Script (@'
$ErrorActionPreference = 'Continue'
$lemonade = '{{EXE}}'
$watch = Get-Date
$text = @(& $lemonade pull nomic-embed-text-v1-GGUF 2>&1 | ForEach-Object { "$_" })
"pull exit: $LASTEXITCODE after $([int]((Get-Date) - $watch).TotalSeconds) s"
"pull output ends: $((($text | Where-Object { $_.Trim() }) | Select-Object -Last 4) -join ' | ')"
$watch = Get-Date
$text = @(& $lemonade load nomic-embed-text-v1-GGUF 2>&1 | ForEach-Object { "$_" })
"load exit: $LASTEXITCODE after $([int]((Get-Date) - $watch).TotalSeconds) s"
"load output ends: $((($text | Where-Object { $_.Trim() }) | Select-Object -Last 4) -join ' | ')"
"--- lemonade list (nomic lines)"
@(& $lemonade list 2>&1 | ForEach-Object { "$_" }) | Where-Object { $_ -match 'nomic' }
"--- lemonade backends"
@(& $lemonade backends 2>&1 | ForEach-Object { "$_" }) | Select-Object -First 12
foreach ($root in '{{HOME}}\.cache', '{{HOME}}\AppData\Local\lemonade_server', '{{HOME}}\.lemonade') {
    foreach ($file in Get-ChildItem $root -Recurse -File -Filter '*nomic-embed-text-v1*.gguf' -ErrorAction SilentlyContinue) { "model file: $($file.FullName), $([math]::Round($file.Length / 1MB, 1)) MB" }
}
$global:LASTEXITCODE = 0
'@.Replace('{{EXE}}', $lemonadeExe).Replace('{{HOME}}', $guestHome))
    Set-EmbedFact "lemonade/nomic-embed-text-v1-GGUF" "pullS" (Get-OutputNumber $run.Output "pull exit: \d+ after (\d+) s")
    Set-EmbedFact "lemonade/nomic-embed-text-v1-GGUF" "modelMb" (Get-OutputNumber $run.Output "model file: [^\n]*, ([\d\.]+) MB")
    $ok = $run.Output -match "pull exit: 0" -and $run.Output -match "load exit: 0"
    Add-Result -Id "LM2" -Title "lemonade pull nomic-embed-text-v1-GGUF" -Expect "pull and load exit 0" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Output ("$vc`nstatus before: $up`n" + $run.Output) `
        -Observed "$vc; $(($run.Output -split "`n" | Where-Object { $_ -match '^(pull exit|load exit|model file)' }) -join '; ') (the load may fetch a llama.cpp backend: its time is in the load figure)"
}

Test-Case "LM3" "Semantic search through Lemonade" "Lemonade and nomic-embed-text-v1-GGUF chosen: the model is listed, a reembed stores every document's vector, and the five searches answer" {
    $resolved = Resolve-GuestModelId -Url "http://127.0.0.1:13305/v1/models", "http://127.0.0.1:13305/api/v1/models" -Match "nomic"
    if (-not $resolved.Id) { throw "Lemonade lists no nomic model on port 13305; run LM1 and LM2 first. $($resolved.Output)" }
    Set-EmbedFact "lemonade/$($resolved.Id)" "modelMb" (Get-EmbedFact "lemonade/nomic-embed-text-v1-GGUF" "modelMb")
    Set-EmbedFact "lemonade/$($resolved.Id)" "pullS" (Get-EmbedFact "lemonade/nomic-embed-text-v1-GGUF" "pullS")
    Measure-RealEmbedder -Id "LM3" -Title "Semantic search through Lemonade" -Provider lemonade -Model $resolved.Id `
        -ServerProcess $lemonadePattern -Port 13305 -ModelFamily "nomic-embed-text v1 (not v1.5)" -Quantisation "Q4_K_S (GGUF)" `
        -InstallFolder @("$guestHome\AppData\Local\lemonade_server") `
        -StopScript ("& '$lemonadeExe' unload 2>&1 | Out-Null") `
        -Note "README's model for Lemonade: v1, Q4_K_S, not the model of the llama.cpp, LM Studio and Ollama cases."
}

# ---------------------------------------------------------------- MR, mistral.rs
# Run `-Only D2,MR1,MR2,MR3`. Sources (read 2026-11-21):
#  - https://github.com/EricLBuehler/mistral.rs README, Quick Start, Windows:
#    `irm https://mistralrs.dev/install.ps1 | iex`; `mistralrs serve -m <model>`
#    (port 1234, web UI at /ui).
#  - https://mistralrs.dev/install.ps1: `-Yes` or the environment variable
#    MISTRALRS_INSTALL_YES=1 skips the menu; the prebuilt zip
#    mistralrs-cpu-x86_64-pc-windows-msvc.zip (32 MB, a GitHub release asset) is
#    unpacked to %USERPROFILE%\.mistralrs\mistralrs.exe, with a launcher
#    mistralrs.cmd in %USERPROFILE%\.local\bin.
#  - https://huggingface.co/Qwen/Qwen3-Embedding-0.6B: not gated, model.safetensors
#    1,191,586,416 bytes (BF16), Apache-2.0; listed in mistral.rs's supported models.
# Not verified: whether a processor-only guest of 3072 MB holds it (peak memory is
# what the case measures) and how long the first load takes; MR2 waits up to 25
# minutes and MR3 up to 40 for the job, and both report rather than hang.
$mistralExe = "$guestHome\.mistralrs\mistralrs.exe"
$mistralPattern = '\\\.mistralrs\\|\\mistralrs[^\\]*\.exe$'
$mistralModel = "Qwen/Qwen3-Embedding-0.6B"
$mistralModelId = $null

Test-Case "MR1" "Install mistral.rs" "README's installer command installs mistralrs.exe for the user" {
    # The installer checks the program it unpacked; without the Visual C++
    # runtime that check ended with 0xC0000135 on 2026-11-21 (see LC1).
    $vc = Install-GuestVcRuntime
    $run = Invoke-GuestScript -TimeoutSec 1800 -Script ((Get-GuestFreeDiskText 5) + "`n" + (@'
$watch = Get-Date
# README's command in its own PowerShell, with the installer's switch for "no menu".
$child = @(& powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; `$env:MISTRALRS_INSTALL_YES = '1'; irm https://mistralrs.dev/install.ps1 | iex" 2>&1 | ForEach-Object { "$_" })
"installer exit: $LASTEXITCODE"
"installed in $([int]((Get-Date) - $watch).TotalSeconds) s"
"installer output ends: $((($child | Where-Object { $_.Trim() }) | Select-Object -Last 8) -join ' | ')"
$exe = '{{EXE}}'
"exe: $(Test-Path $exe)"
if (-not (Test-Path $exe)) { 'result: no mistralrs.exe'; return }
"version: $((@(& $exe --version 2>&1 | ForEach-Object { "$_" })) -join ' | ')"
"installed MB: $([math]::Round(((Get-ChildItem (Split-Path -Parent $exe) -Recurse -File | Measure-Object Length -Sum).Sum) / 1MB, 1))"
$signature = Get-AuthenticodeSignature $exe
"signature: $($signature.Status) $($signature.SignerCertificate.Subject)"
'@).Replace('{{EXE}}', $mistralExe) + "`n" + (Get-GuestDefenderText "mistralrs") + "`n" + "'result: installed'")
    Set-EmbedFact "mistralrs" "version" $(if ($run.Output -match "(?m)^version: (.+)$") { $Matches[1].Trim() } else { $null })
    Set-EmbedFact "mistralrs" "installMb" (Get-OutputNumber $run.Output "installed MB: ([\d\.]+)")
    Set-EmbedFact "mistralrs" "installS" (Get-OutputNumber $run.Output "installed in (\d+) s")
    $ok = $run.Output -match "result: installed" -and $run.Output -match "exe: True"
    Add-Result -Id "MR1" -Title "Install mistral.rs" -Expect "irm https://mistralrs.dev/install.ps1 | iex installs mistralrs.exe" `
        -Status $(if ($ok) { "pass" } else { "fail" }) -Output ("$vc`n" + $run.Output) `
        -Observed "$vc; $(($run.Output -split "`n" | Where-Object { $_ -match '^(installer exit|installed in|exe:|version:|installed MB|signature|Defender)' }) -join '; ') (the script downloads and unpacks in one step: no separate download time)"
}

Test-Case "MR2" "mistralrs serve -m Qwen/Qwen3-Embedding-0.6B" "README's command starts the server on port 1234 and it lists the model" {
    if (-not (Test-Path (Join-Path $PSScriptRoot "guest\serve-logged.ps1"))) { throw "guest\serve-logged.ps1 is missing" }
    $log = "$embedLogs\mistralrs"
    $watch = Get-Date
    Start-LoggedServer -Exe $mistralExe -Log $log -ArgLine "serve -m $mistralModel"
    # The first start downloads the model (about 1.2 GB) and loads it.
    $wait = Wait-GuestServer -Url "http://127.0.0.1:1234/v1/models" -ServerProcess $mistralPattern -Seconds 1500 -Log $log
    $script:mistralQuant = "F32 (mistral.rs's choice on the processor)"
    $asWritten = "as written: $(($wait -split "`n" | Select-Object -First 1))"
    if ($wait -notmatch "answered after" -and $wait -match "does not fit") {
        # On 2026-11-22 README's command loaded the model as F32 (2272 MiB) and
        # refused: "This model does not fit on the devices [cpu (avail:
        # 1037MB)]". Its docs (reference/cli/serve.md) name --isq: "Quantize
        # the weights at load time (in-situ quantization, ISQ): a bit width".
        # Once more, 8-bit, which a machine with little free memory needs.
        $asWritten += " ($(($wait -split "`n" | Where-Object { $_ -match 'does not fit' } | Select-Object -First 1).Trim()))"
        $watch = Get-Date
        Start-LoggedServer -Exe $mistralExe -Log $log -ArgLine "serve -m $mistralModel --isq 8"
        $wait = Wait-GuestServer -Url "http://127.0.0.1:1234/v1/models" -ServerProcess $mistralPattern -Seconds 1500 -Log $log
        $script:mistralQuant = "8-bit ISQ (--isq 8): README's command did not fit"
        $asWritten += "; with --isq 8: $(($wait -split "`n" | Select-Object -First 1))"
    }
    $seconds = [int]((Get-Date) - $watch).TotalSeconds
    $resolved = Resolve-GuestModelId -Url "http://127.0.0.1:1234/v1/models" -Match "Qwen3-Embedding"
    $script:mistralModelId = $resolved.Id
    $modelMb = Get-GuestFolderMb @("$guestHome\.cache\huggingface\hub\models--Qwen--Qwen3-Embedding-0.6B")
    Set-EmbedFact "mistralrs/$($resolved.Id)" "modelMb" $modelMb
    Set-EmbedFact "mistralrs/$($resolved.Id)" "pullS" $seconds
    $memory = (Invoke-GuestScript -TimeoutSec 60 -Script "`$free = [math]::Round((Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory / 1KB); `"guest free memory MB: `$free`"").Output
    $shot = Shot "mistralrs-running"
    Add-Result -Id "MR2" -Title "mistralrs serve -m Qwen/Qwen3-Embedding-0.6B" -Expect "the server answers on 1234 and lists the model, within 25 minutes" `
        -Status $(if ($wait -match "answered after" -and $resolved.Id) { if ($script:mistralQuant -match "ISQ") { "note" } else { "pass" } } else { "fail" }) -Shots @($shot) -Output ($asWritten + "`n" + $wait + "`n" + $resolved.Output + "`n" + $memory) `
        -Observed "$asWritten (load, $seconds s from the last start; model on disk $modelMb MB); serving as $($script:mistralQuant); the server lists the model as '$($resolved.Id)'; $memory"
}

Test-Case "MR3" "Semantic search through mistral.rs" "mistral.rs and Qwen3-Embedding-0.6B chosen: the model is listed, a reembed stores every document's vector, and the five searches answer" {
    if (-not $mistralModelId) { $mistralModelId = (Resolve-GuestModelId -Url "http://127.0.0.1:1234/v1/models" -Match "Qwen3-Embedding").Id }
    if (-not $mistralModelId) { throw "mistral.rs lists no Qwen3-Embedding model on port 1234; run MR1 and MR2 first" }
    Measure-RealEmbedder -Id "MR3" -Title "Semantic search through mistral.rs" -Provider mistralrs -Model $mistralModelId `
        -ServerProcess $mistralPattern -Port 1234 -ModelFamily "Qwen3-Embedding-0.6B" -Quantisation $(if ($script:mistralQuant) { $script:mistralQuant } else { "unknown" }) `
        -InstallFolder @("$guestHome\.mistralrs") -JobSeconds 2400 `
        -Note "README's model for mistral.rs, 0.6B parameters, 1024 dimensions: not the model of the llama.cpp, LM Studio and Ollama cases. Processor build; 3072 MB guest."
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
    if ($probe.registry.notificationIcon) { $problems += "notification-icon.png is left in the data folder" }
    Add-Result -Id "K1" -Title "Uninstall while the app runs" -Expect "processes stopped; programs, PATH entry, shortcut, start with Windows, notification name and its icon file removed" `
        -Status $(if ($problems -or $run.ExitCode -ne 0) { "fail" } else { "pass" }) -Shots @($shot) -Output $run.Output `
        -Observed "exit $($run.ExitCode); running before: $($before.processes.Count) processes; start with Windows before: '$($before.registry.autostart)', notification name before: $($before.registry.appIdentity), its icon file before: $($before.registry.notificationIcon);$(if ($problems) { $problems -join '; ' } else { 'removed' }); left in the install folder: $(if ($left) { $left -join ', ' } else { 'nothing' }); data folder files: $(Get-FileCount $probe.dataDir.files); settings folder files: $(Get-FileCount $probe.prefsDir.files)"
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

# ---------------------------------------------------------------- IS, the setup program
# The setup program (scripts\setup\codewiki.iss) installs per user, runs
# install.ps1 from the package it carries, and uninstalls through the
# uninstall.ps1 it installed. Each case starts from no installation and puts
# the guest back to none, so that the next case, and U2 after them, find what
# they expect. They need the guest only, not the zip's unpacked folder, except
# IS5 and IS6, which make an install.cmd installation first.
$setupEntryKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{7DEA7959-917F-46EC-80A8-625E2A9B24F5}_is1'
$setupHold = "$($script:Vm.GuestDir)\hold"
$setupShortcut = "$guestHome\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\CodeWiki.lnk"

# What is in the guest of the installation the setup program makes, one
# "name=value" line each, as a table. Lower-case names; the Apps entry's
# values are e_DisplayName and so on.
function Get-SetupState {
    $run = Invoke-GuestScript -TimeoutSec 180 -Script (@'
$t = '{{TARGET}}'
$key = '{{KEY}}'
"folder=$(Test-Path -LiteralPath $t)"
$names = @()
if (Test-Path -LiteralPath $t) { $names = @(Get-ChildItem -LiteralPath $t -Force | ForEach-Object { $_.Name }) }
"names=$($names -join '|')"
$user = [Environment]::GetEnvironmentVariable('Path', 'User')
$same = @($user -split ';' | Where-Object { $_ -and $_.Trim().TrimEnd('\').ToLowerInvariant() -eq $t.TrimEnd('\').ToLowerInvariant() })
"pathcount=$($same.Count)"
$lnk = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\CodeWiki.lnk'
"lnk=$(Test-Path -LiteralPath $lnk)"
$entry = $null
if (Test-Path -LiteralPath $key) { $entry = Get-ItemProperty -LiteralPath $key }
"entry=$([bool]$entry)"
if ($entry) {
    foreach ($n in 'DisplayName', 'DisplayVersion', 'Publisher', 'UninstallString', 'QuietUninstallString') { "e_$n=$($entry.$n)" }
}
$runValue = (Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -ErrorAction SilentlyContinue).CodeWiki
"run=$runValue"
$data = Join-Path $env:LOCALAPPDATA 'CodeWiki'
$prefs = Join-Path $env:APPDATA 'CodeWiki'
"datadir=$(Test-Path -LiteralPath $data)"
"prefsdir=$(Test-Path -LiteralPath $prefs)"
"datamarker=$(Test-Path -LiteralPath (Join-Path $data 'marker.txt'))"
"prefsmarker=$(Test-Path -LiteralPath (Join-Path $prefs 'marker.txt'))"
"aumid=$(Test-Path -LiteralPath 'HKCU:\Software\Classes\AppUserModelId\CodeWiki.App')"
"notifyicon=$(Test-Path -LiteralPath (Join-Path $data 'notification-icon.png'))"
"procs=$(@(Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -like 'codewiki*' }).Count)"
# What a new shell would find: the machine PATH and the user PATH as stored.
$env:Path = [Environment]::GetEnvironmentVariable('Path', 'Machine') + ';' + $user
$found = Get-Command codewiki -ErrorAction SilentlyContinue
"command=$($found.Source)"
if ($found) { "version=$((& $found.Source --version 2>&1 | Out-String).Trim() -replace '\s+', ' ')" }
'@).Replace('{{TARGET}}', $guestTarget).Replace('{{KEY}}', $setupEntryKey)
    $state = @{}
    foreach ($line in @($run.Output -split "`r?`n")) {
        if ($line -match '^([A-Za-z_]+)=(.*)$') { $state[$Matches[1]] = $Matches[2] }
    }
    # Empty output is a script that did not run, never "nothing installed".
    if (-not $state.ContainsKey("folder")) { throw "could not read the guest's installation: $($run.Output) $($run.Stderr)" }
    $state["names"] = @($state["names"] -split '\|' | Where-Object { $_ })
    $state
}

# What a setup-installed CodeWiki must look like. Returns the problems.
function Test-SetupInstalled {
    param($State, [string]$Version)
    $problems = @()
    foreach ($need in "codewiki.exe", "codewiki-mcp.exe", "codewiki-app.exe", "plugins", "uninstall.ps1", "unins000.exe") {
        if ($State.names -notcontains $need) { $problems += "missing $need" }
    }
    if ($State.names -contains "install.ps1") { $problems += "install.ps1 is in the folder" }
    if ($State.pathcount -ne "1") { $problems += "user PATH holds the folder $($State.pathcount) times" }
    if ($State.lnk -ne "True") { $problems += "no Start menu shortcut" }
    if ($State.entry -ne "True") {
        $problems += "no Apps entry"
    } else {
        if ($State.e_DisplayVersion -ne $Version) { $problems += "DisplayVersion is '$($State.e_DisplayVersion)'" }
        if ($State.e_DisplayName -ne "CodeWiki") { $problems += "DisplayName is '$($State.e_DisplayName)'" }
        if ($State.e_Publisher -ne "CodeWiki") { $problems += "Publisher is '$($State.e_Publisher)'" }
        if ($State.e_QuietUninstallString -ne "`"$guestTarget\unins000.exe`" /SILENT") { $problems += "QuietUninstallString is '$($State.e_QuietUninstallString)'" }
    }
    if ($State.version -notmatch [regex]::Escape($Version)) { $problems += "codewiki --version said '$($State.version)' (found as '$($State.command)')" }
    $problems
}

# One line of what the guest holds, for a result.
function Format-SetupState {
    param($State)
    "folder $($State.folder) [$($State.names -join ', ')], user PATH entries $($State.pathcount), shortcut $($State.lnk), Apps entry $($State.entry)$(if ($State.entry -eq 'True') { " (name '$($State.e_DisplayName)', version '$($State.e_DisplayVersion)', publisher '$($State.e_Publisher)')" }), Run value '$($State.run)', data folder $($State.datadir), settings folder $($State.prefsdir), CodeWiki processes $($State.procs)"
}

# Run the setup program with switches and wait for it; the exit code is Setup's.
function Invoke-GuestSetup {
    param([string[]]$Arguments, [int]$TimeoutSec = 240)
    Invoke-GuestExe -Exe $guestSetup -TimeoutSec $TimeoutSec -Arguments $Arguments
}

# The uninstaller starts a copy of itself and may return before that copy has
# finished; wait until no uninstaller process is left.
function Wait-GuestUninstaller {
    Invoke-GuestScript -TimeoutSec 240 -Script @'
Start-Sleep -Seconds 3
$deadline = (Get-Date).AddSeconds(150)
do {
    $left = @(Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -match '^_?unins' })
    if ($left.Count) { Start-Sleep -Seconds 1 }
} while ($left.Count -and (Get-Date) -lt $deadline)
"uninstallers left: $($left.Count)"
'@ | Out-Null
}

# Uninstall through the Apps entry, as Settings > Apps does, and wait for it.
# -Quiet runs QuietUninstallString (/SILENT), else UninstallString with
# /VERYSILENT. Returns the run (ExitCode) and the log's text.
function Invoke-GuestUninstall {
    param([string]$Log, [switch]$Quiet, [string[]]$Extra = @())
    $state = Get-SetupState
    $command = if ($Quiet) { $state.e_QuietUninstallString } else { $state.e_UninstallString }
    if ($command -notmatch '^"([^"]+)"\s*(.*)$') { throw "the Apps entry has no usable uninstall command: '$command'" }
    $exe = $Matches[1]
    $arguments = @($Matches[2] -split '\s+' | Where-Object { $_ }) + $Extra
    if (-not $Quiet) { $arguments += "/VERYSILENT" }
    $arguments += "/SUPPRESSMSGBOXES", "/NORESTART", "/LOG=$Log"
    $run = Invoke-GuestExe -Exe $exe -TimeoutSec 240 -Arguments $arguments
    Wait-GuestUninstaller
    $text = (Invoke-GuestScript -Script "Get-Content -LiteralPath '$Log' -ErrorAction SilentlyContinue").Output
    [pscustomobject]@{ ExitCode = $run.ExitCode; Command = $command; Arguments = ($arguments -join " "); Stdout = $run.Stdout; Stderr = $run.Stderr; Log = $text }
}

# The guest's own text of a setup log.
function Get-GuestLog {
    param([string]$Log)
    (Invoke-GuestScript -Script "Get-Content -LiteralPath '$Log' -ErrorAction SilentlyContinue").Output
}

# Every Apps entry named CodeWiki, the user's and the machine's: the path of
# each one's registry key, as PowerShell writes it ($setupEntryKey for the
# setup program's).
function Get-SetupEntryKey {
    $run = Invoke-GuestScript -Script @'
foreach ($root in 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall', 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall', 'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall') {
    Get-ChildItem -LiteralPath $root -ErrorAction SilentlyContinue | ForEach-Object {
        if ((Get-ItemProperty -LiteralPath $_.PSPath -ErrorAction SilentlyContinue).DisplayName -eq 'CodeWiki') { "entry=$root\$($_.PSChildName)" }
    }
}
"entries read"
'@
    # Empty output is a script that did not run, never "no entry".
    if ($run.Output -notmatch 'entries read') { throw "could not read the guest's Apps entries: $($run.Output) $($run.Stderr)" }
    @($run.Output -split "`r?`n" | Where-Object { $_ -match '^entry=' } | ForEach-Object { $_.Substring(6).Trim() })
}

# Back to no installation, by whatever means: end the programs and the file
# holder, uninstall with the installed script, delete what is left, the data
# and settings folders, the Run value, the Apps entry and the PATH entry.
function Reset-SetupGuest {
    $run = Invoke-GuestScript -TimeoutSec 280 -Script (@'
$t = '{{TARGET}}'
$key = '{{KEY}}'
[System.IO.File]::WriteAllText('{{HOLD}}.release', 'release')
Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object { $_.CommandLine -like '*hold-file.ps1*' } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -like 'codewiki*' } | Stop-Process -Force -ErrorAction SilentlyContinue
# An uninstaller a case left waiting on one of its questions (IS9).
Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -match '^_?unins|^_iu' } | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 2
if (Test-Path -LiteralPath (Join-Path $t 'uninstall.cmd')) {
    & "$env:SystemRoot\System32\cmd.exe" /c (Join-Path $t 'uninstall.cmd') -PurgeData | Out-Null
}
if (Test-Path -LiteralPath $key) { Remove-Item -LiteralPath $key -Recurse -Force -ErrorAction SilentlyContinue }
Remove-Item -LiteralPath 'HKCU:\Software\Classes\AppUserModelId\CodeWiki.App' -Recurse -Force -ErrorAction SilentlyContinue
if (Test-Path -LiteralPath $t) { Remove-Item -LiteralPath $t -Recurse -Force -ErrorAction SilentlyContinue }
foreach ($folder in (Join-Path $env:LOCALAPPDATA 'CodeWiki'), (Join-Path $env:APPDATA 'CodeWiki')) {
    if (Test-Path -LiteralPath $folder) { Remove-Item -LiteralPath $folder -Recurse -Force -ErrorAction SilentlyContinue }
}
Remove-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'CodeWiki' -ErrorAction SilentlyContinue
$user = [Environment]::GetEnvironmentVariable('Path', 'User')
$kept = @($user -split ';' | Where-Object { $_ -and $_.Trim().TrimEnd('\').ToLowerInvariant() -ne $t.TrimEnd('\').ToLowerInvariant() })
[Environment]::SetEnvironmentVariable('Path', ($kept -join ';'), 'User')
Remove-Item -LiteralPath (Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\CodeWiki.lnk') -Force -ErrorAction SilentlyContinue
"left: folder $(Test-Path -LiteralPath $t), entry $(Test-Path -LiteralPath $key)"
'@).Replace('{{TARGET}}', $guestTarget).Replace('{{KEY}}', $setupEntryKey).Replace('{{HOLD}}', $setupHold)
    # A reset that leaves CodeWiki behind would show up as a failure of the next case.
    if ($run.Output -notmatch 'left: folder (True|False), entry (True|False)') { throw "could not reset the guest's installation: $($run.Output) $($run.Stderr)" }
    if ($Matches[1] -eq 'True' -or $Matches[2] -eq 'True') { throw "the guest was not put back to no installation; left: $($Matches[0]) ($($run.Output))" }
}

# Hold a file open with no sharing, in a PowerShell that stays alive until a
# marker file appears: the file can then be neither replaced nor deleted.
$holdScript = @'
param([string]$Path, [string]$Marker)
$stream = [System.IO.File]::Open($Path, 'Open', 'Read', 'None')
try {
    [System.IO.File]::WriteAllText("$Marker.ready", "held")
    $deadline = (Get-Date).AddMinutes(20)
    while (-not (Test-Path -LiteralPath "$Marker.release") -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 300 }
} finally {
    $stream.Close()
}
'@
function Start-GuestFileHold {
    param([string]$GuestPath)
    $encoded = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($holdScript))
    Invoke-GuestScript -Script (@'
$file = '{{DIR}}\hold-file.ps1'
[System.IO.File]::WriteAllText($file, [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{{BODY}}')), (New-Object System.Text.UTF8Encoding $false))
Remove-Item -LiteralPath '{{HOLD}}.ready', '{{HOLD}}.release' -Force -ErrorAction SilentlyContinue
'@).Replace('{{DIR}}', $script:Vm.GuestDir).Replace('{{BODY}}', $encoded).Replace('{{HOLD}}', $setupHold) | Out-Null
    Start-GuestExe -Exe "C:\Windows\System32\conhost.exe" -Arguments "--headless", $powershell, "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass",
        "-File", "$($script:Vm.GuestDir)\hold-file.ps1", "-Path", $GuestPath, "-Marker", $setupHold
    $run = Invoke-GuestScript -TimeoutSec 90 -Script (@'
$deadline = (Get-Date).AddSeconds(45)
while (-not (Test-Path -LiteralPath '{{HOLD}}.ready') -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 500 }
"held: $(Test-Path -LiteralPath '{{HOLD}}.ready')"
'@).Replace('{{HOLD}}', $setupHold)
    if ($run.Output -notmatch 'held: True') { throw "the file $GuestPath was not held open: $($run.Output) $($run.Stderr)" }
}
function Stop-GuestFileHold {
    Invoke-GuestScript -TimeoutSec 90 -Script (@'
[System.IO.File]::WriteAllText('{{HOLD}}.release', 'release')
$deadline = (Get-Date).AddSeconds(30)
do {
    $left = @(Get-CimInstance Win32_Process -ErrorAction SilentlyContinue | Where-Object { $_.CommandLine -like '*hold-file.ps1*' })
    if ($left.Count) { Start-Sleep -Milliseconds 500 }
} while ($left.Count -and (Get-Date) -lt $deadline)
"holders left: $($left.Count)"
'@).Replace('{{HOLD}}', $setupHold) | Out-Null
}

# The installation install.cmd makes from the unpacked zip, which C1 unpacks;
# unpack it here when C1 did not run.
function Install-GuestWithCmd {
    $have = (Invoke-GuestScript -Script "Test-Path -LiteralPath '$guestUnpacked\install.cmd'").Output
    if ($have -notmatch 'True') {
        Invoke-GuestScript -Script "Expand-Archive -Path '$guestZip' -DestinationPath '$guestUnpacked' -Force" | Out-Null
    }
    Invoke-GuestExe -Exe "C:\Windows\System32\cmd.exe" -TimeoutSec 120 -Arguments "/c", "$guestUnpacked\install.cmd"
}

# The setup wizard's windows as Windows itself lists them. UI Automation found
# no button in the wizard on the first run (a "TWizardForm" window, buttons
# "Install" and "Finish" assumed), although the screenshot shows the Ready
# page with its Install button; so the wizard is read through the window
# tree instead: every visible top-level window with a title ("top=class;
# process;title"), and every child window of the wizard (title "Setup -
# CodeWiki ..." or class TWizardForm) with its class, visible, enabled, place
# in screen pixels and text, asked with WM_GETTEXT because GetWindowText
# does not read another program's controls.
$setupWizardScript = @'
Add-Type -Namespace VmTest -Name Wizard -MemberDefinition '
[StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
public delegate bool EachWindow(IntPtr window, IntPtr data);
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
[DllImport("user32.dll")] public static extern bool EnumWindows(EachWindow each, IntPtr data);
[DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, EachWindow each, IntPtr data);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
[DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr window);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr window, System.Text.StringBuilder name, int size);
[DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr SendMessageTimeout(IntPtr window, uint message, IntPtr wParam, System.Text.StringBuilder lParam, uint flags, uint timeout, out IntPtr result);
public static string ClassOf(IntPtr w) { System.Text.StringBuilder s = new System.Text.StringBuilder(256); GetClassName(w, s, 256); return s.ToString(); }
public static string TextOf(IntPtr w) { System.Text.StringBuilder s = new System.Text.StringBuilder(1024); IntPtr r; SendMessageTimeout(w, 0x000D, (IntPtr)1024, s, 0x0002, 2000, out r); return s.ToString(); }
public static uint ProcessOf(IntPtr w) { uint p; GetWindowThreadProcessId(w, out p); return p; }
public static string Place(IntPtr w) { Rect r; GetWindowRect(w, out r); return r.Left + "," + r.Top + "," + r.Right + "," + r.Bottom; }
public static System.Collections.Generic.List<IntPtr> Tops() { System.Collections.Generic.List<IntPtr> all = new System.Collections.Generic.List<IntPtr>(); EnumWindows(delegate(IntPtr w, IntPtr d) { if (IsWindowVisible(w)) { all.Add(w); } return true; }, IntPtr.Zero); return all; }
public static System.Collections.Generic.List<IntPtr> Children(IntPtr parent) { System.Collections.Generic.List<IntPtr> all = new System.Collections.Generic.List<IntPtr>(); EnumChildWindows(parent, delegate(IntPtr w, IntPtr d) { all.Add(w); return true; }, IntPtr.Zero); return all; }
'
# Without this, a scaled screen gives scaled numbers, and clicks miss.
[void][VmTest.Wizard]::SetProcessDPIAware()
foreach ($top in [VmTest.Wizard]::Tops()) {
    $class = [VmTest.Wizard]::ClassOf($top)
    $title = [VmTest.Wizard]::TextOf($top) -replace '\s+', ' '
    $process = (Get-Process -Id ([VmTest.Wizard]::ProcessOf($top)) -ErrorAction SilentlyContinue).ProcessName
    if ($title) { "top=$class;$process;$title" }
    # The wizard, and the uninstaller's message boxes: Inno's own is titled
    # "CodeWiki Uninstall", the [Code] section's MsgBox only "Uninstall" (seen
    # 2026-11-26), and both belong to the uninstaller's process.
    $ofUninstaller = $class -eq '#32770' -and ($title -like '*CodeWiki*' -or $title -eq 'Uninstall' -or $process -match '^_?unins|^_iu')
    if ($class -ne 'TWizardForm' -and $title -notlike 'Setup - CodeWiki*' -and -not $ofUninstaller) { continue }
    foreach ($child in [VmTest.Wizard]::Children($top)) {
        "control=$([VmTest.Wizard]::ClassOf($child));$([int][VmTest.Wizard]::IsWindowVisible($child));$([int][VmTest.Wizard]::IsWindowEnabled($child));$([VmTest.Wizard]::Place($child));$([VmTest.Wizard]::TextOf($child) -replace '\s+', ' ')"
    }
}
'@

# What the guest shows of the setup wizard: Tops (one "class;process;title"
# text per top-level window with a title) and Controls (the wizard's child
# windows, each with Class, Visible, Enabled, Text and the point in its
# middle, X and Y, where a click lands).
function Get-SetupWizard {
    $run = Invoke-GuestScript -TimeoutSec 90 -Script $setupWizardScript
    $tops = @()
    $controls = @()
    foreach ($line in @($run.Output -split "`r?`n")) {
        if ($line -match '^top=(.*)$') {
            $tops += $Matches[1].Trim()
        } elseif ($line -match '^control=([^;]*);(\d);(\d);(-?\d+),(-?\d+),(-?\d+),(-?\d+);(.*)$') {
            $controls += [pscustomobject]@{
                Class = $Matches[1]; Visible = $Matches[2] -eq "1"; Enabled = $Matches[3] -eq "1"
                Left = [int]$Matches[4]; Top = [int]$Matches[5]; Right = [int]$Matches[6]; Bottom = [int]$Matches[7]
                X = [int](([int]$Matches[4] + [int]$Matches[6]) / 2); Y = [int](([int]$Matches[5] + [int]$Matches[7]) / 2)
                Text = $Matches[8].Trim()
            }
        }
    }
    [pscustomobject]@{ Tops = $tops; Controls = $controls; Raw = $run.Output }
}

# One line of what the wizard showed, for a result or an error.
function Format-SetupWizard {
    param($Wizard)
    if (-not $Wizard) { return "nothing read" }
    "top-level windows: $($Wizard.Tops -join ' | '); wizard controls shown: $((@($Wizard.Controls | Where-Object { $_.Visible -and $_.Text }) | ForEach-Object { "$($_.Class) '$($_.Text)'$(if (-not $_.Enabled) { ' (disabled)' })" }) -join ', ')"
}

# A button of the setup wizard by its text (an & before the access key is
# left out); null while the wizard shows none. What was read last stays in
# $script:setupWizardSeen.
function Find-SetupButton {
    param([string]$Pattern)
    $wizard = Get-SetupWizard
    $script:setupWizardSeen = $wizard
    @($wizard.Controls | Where-Object {
            $_.Class -match 'Button' -and $_.Visible -and $_.Enabled -and $_.Right -gt $_.Left -and ($_.Text -replace '&', '') -match $Pattern
        }) | Select-Object -First 1
}

if ($Setup) {
    $setupVersion = Get-ZipVersion $Zip
    $setupSilent = "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"

    Test-Case "IS1" "Silent install with the setup program" "exit 0; the three programs, plugins, uninstall.ps1 and unins000.exe in the folder, no install.ps1; the PATH entry once; the shortcut; the Apps entry (CodeWiki, the package's version, CodeWiki); codewiki --version answers; the shortcut starts one window" {
        Reset-SetupGuest
        try {
            $log = "C:\Users\Public\cwvm-is1.log"
            $run = Invoke-GuestSetup -Arguments ($setupSilent + "/LOG=$log")
            $state = Get-SetupState
            $problems = @(Test-SetupInstalled $state $setupVersion)
            if ($run.ExitCode -ne 0) { $problems += "setup exited with $($run.ExitCode)" }
            $setupLog = Get-GuestLog $log
            # The shortcut, as the Start menu would run it.
            Start-GuestExe -Exe "C:\Windows\explorer.exe" -Arguments $setupShortcut
            $window = Show-GuestWindow -ProcessName "codewiki-app"
            Wait-For 8
            $shot = Shot "setup-silent-app"
            $probe = Get-Probe "setup-silent" -Quick
            $windows = @(Get-App $probe | Where-Object { $_.hasWindow })
            if (@(Get-App $probe).Count -ne 1 -or $windows.Count -ne 1) { $problems += "the shortcut started $(@(Get-App $probe).Count) app processes, $($windows.Count) with a window" }
            Add-Result -Id "IS1" -Title "Silent install with the setup program" -Expect "exit 0, the files, PATH entry, shortcut and Apps entry in place, the version answers, the shortcut starts one window" `
                -Status $(if ($problems) { "fail" } else { "pass" }) -Shots @($shot) -Output ("setup log $log`n" + $setupLog + "`n--- stdout/stderr`n" + ($run.Stdout + "`n" + $run.Stderr).Trim()).Trim() `
                -Observed "setup.exe $($setupSilent -join ' ') /LOG: exit $($run.ExitCode); $(Format-SetupState $state); codewiki --version: '$($state.version)'; $(if ($problems) { $problems -join '; ' } else { 'all in place' }); app started from the shortcut: $(@(Get-App $probe).Count) app process, $($windows.Count) window"
        } finally {
            Reset-SetupGuest
        }
    }

    Test-Case "IS2" "Uninstall from the Apps entry" "exit 0 with the app running; the folder, PATH entry, shortcut, Run value and Apps entry gone; the data and settings folders stay" {
        Reset-SetupGuest
        try {
            $install = Invoke-GuestSetup -Arguments ($setupSilent + "/LOG=C:\Users\Public\cwvm-is2-install.log")
            if ($install.ExitCode -ne 0) { throw "the silent install failed first: exit $($install.ExitCode)" }
            # What a user has by then: the start with Windows on (what AS1 turns on),
            # the app's name and logo for notifications (what its first Windows
            # notification writes, NI1), the app running, and data and settings
            # to keep.
            Invoke-GuestScript -Script (@'
New-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'CodeWiki' -Value '{{RUN}}' -PropertyType String -Force | Out-Null
New-Item -ItemType Directory -Force (Join-Path $env:LOCALAPPDATA 'CodeWiki'), (Join-Path $env:APPDATA 'CodeWiki') | Out-Null
Set-Content -Path (Join-Path $env:LOCALAPPDATA 'CodeWiki\marker.txt') -Value 'data'
Set-Content -Path (Join-Path $env:APPDATA 'CodeWiki\marker.txt') -Value 'settings'
$icon = Join-Path $env:LOCALAPPDATA 'CodeWiki\notification-icon.png'
Copy-Item -LiteralPath '{{TARGET}}\codewiki-app.exe' -Destination $icon -Force
$key = 'HKCU:\Software\Classes\AppUserModelId\CodeWiki.App'
New-Item -Path $key -Force | Out-Null
New-ItemProperty -Path $key -Name 'DisplayName' -Value 'CodeWiki' -PropertyType String -Force | Out-Null
New-ItemProperty -Path $key -Name 'IconUri' -Value $icon -PropertyType ExpandString -Force | Out-Null
'@).Replace('{{RUN}}', $autostartCommand).Replace('{{TARGET}}', $guestTarget) | Out-Null
            Start-GuestApp -Arguments $guestRepo
            Wait-For 15
            $before = Get-SetupState
            $running = Shot "setup-uninstall-app-running"
            $log = "C:\Users\Public\cwvm-is2.log"
            $run = Invoke-GuestUninstall -Log $log -Quiet
            Wait-For 3
            $shot = Shot "setup-uninstalled"
            $after = Get-SetupState
            $problems = @()
            if ($before.procs -eq "0") { $problems += "the app was not running before the uninstall" }
            if ($before.run -ne $autostartCommand) { $problems += "the Run value was not in place before ('$($before.run)')" }
            if ($before.aumid -ne "True" -or $before.notifyicon -ne "True") { $problems += "the notification name or icon was not in place before (key $($before.aumid), icon $($before.notifyicon))" }
            if ($after.aumid -eq "True") { $problems += "the notification name is left in the registry" }
            if ($after.notifyicon -eq "True") { $problems += "notification-icon.png is left in the data folder" }
            if ($run.ExitCode -ne 0) { $problems += "uninstall exited with $($run.ExitCode)" }
            if ($after.folder -eq "True") { $problems += "the folder is left: $($after.names -join ', ')" }
            if ($after.pathcount -ne "0") { $problems += "PATH entry left" }
            if ($after.lnk -eq "True") { $problems += "shortcut left" }
            if ($after.run) { $problems += "Run value left: $($after.run)" }
            if ($after.entry -eq "True") { $problems += "Apps entry left" }
            if ($after.procs -ne "0") { $problems += "$($after.procs) CodeWiki processes still run" }
            if ($after.datamarker -ne "True" -or $after.prefsmarker -ne "True") { $problems += "data or settings were deleted (data marker $($after.datamarker), settings marker $($after.prefsmarker))" }
            Add-Result -Id "IS2" -Title "Uninstall from the Apps entry" -Expect "exit 0; programs, folder, PATH entry, shortcut, Run value and Apps entry gone; data and settings stay" `
                -Status $(if ($problems) { "fail" } else { "pass" }) -Shots @($running, $shot) -Output ("$($run.Command) $($run.Arguments)`n" + $run.Log).Trim() `
                -Observed "ran QuietUninstallString '$($run.Command)' plus $($run.Arguments) with the app running (processes before: $($before.procs)): exit $($run.ExitCode); after: $(Format-SetupState $after); $(if ($problems) { $problems -join '; ' } else { 'removed, data and settings kept' })"
        } finally {
            Reset-SetupGuest
        }
    }

    Test-Case "IS3" "Uninstall refused while a program is held" "a non-zero exit, and the entry, PATH entry, shortcut and programs stay; after the file is released the uninstall succeeds" {
        Reset-SetupGuest
        try {
            $install = Invoke-GuestSetup -Arguments ($setupSilent + "/LOG=C:\Users\Public\cwvm-is3-install.log")
            if ($install.ExitCode -ne 0) { throw "the silent install failed first: exit $($install.ExitCode)" }
            Start-GuestFileHold "$guestTarget\codewiki-app.exe"
            $refusedRun = Invoke-GuestUninstall -Log "C:\Users\Public\cwvm-is3-refused.log"
            Wait-For 2
            $refusedShot = Shot "setup-uninstall-refused"
            $kept = Get-SetupState
            $problems = @()
            if ($refusedRun.ExitCode -eq 0) { $problems += "the uninstall exited with 0 while codewiki-app.exe was held" }
            if ($kept.entry -ne "True") { $problems += "the Apps entry is gone" }
            if ($kept.pathcount -ne "1") { $problems += "PATH entries $($kept.pathcount)" }
            if ($kept.lnk -ne "True") { $problems += "the shortcut is gone" }
            foreach ($need in "codewiki.exe", "codewiki-mcp.exe", "codewiki-app.exe") {
                if ($kept.names -notcontains $need) { $problems += "$need is gone" }
            }
            # Let go; the same command must now work.
            Stop-GuestFileHold
            $doneRun = Invoke-GuestUninstall -Log "C:\Users\Public\cwvm-is3-done.log"
            Wait-For 3
            $gone = Get-SetupState
            if ($doneRun.ExitCode -ne 0) { $problems += "the uninstall after the release exited with $($doneRun.ExitCode)" }
            if ($gone.folder -eq "True" -or $gone.entry -eq "True" -or $gone.pathcount -ne "0" -or $gone.lnk -eq "True") { $problems += "after the release: $(Format-SetupState $gone)" }
            Add-Result -Id "IS3" -Title "Uninstall refused while a program is held" -Expect "non-zero exit and everything kept while held; exit 0 and everything gone after" `
                -Status $(if ($problems) { "fail" } else { "pass" }) -Shots @($refusedShot) `
                -Output ("--- while held: $($refusedRun.Command) $($refusedRun.Arguments)`n" + $refusedRun.Log + "`n--- after the release`n" + $doneRun.Log).Trim() `
                -Observed "codewiki-app.exe held open with no sharing: uninstall exit $($refusedRun.ExitCode); then $(Format-SetupState $kept); released, uninstall exit $($doneRun.ExitCode); then $(Format-SetupState $gone); $(if ($problems) { $problems -join '; ' } else { 'refused and kept while held, removed after' }); read the screenshot for what, if anything, the uninstaller showed"
        } finally {
            Reset-SetupGuest
        }
    }

    Test-Case "IS4" "The setup wizard by mouse" "Ready to Install, a click on Install, the progress, the finish page with Start CodeWiki, a click on Finish; the app's window appears and the folder is as after a silent install" {
        Reset-SetupGuest
        try {
            Start-GuestExe -Exe $guestSetup
            $install = $null
            for ($try = 0; $try -lt 15 -and -not $install; $try++) {
                Wait-For 4
                $install = Find-SetupButton '^&?Install$'
            }
            $ready = Shot "setup-wizard-ready"
            if (-not $install) { throw "the wizard showed no Install button within a minute; read the screenshot; $(Format-SetupWizard $script:setupWizardSeen)" }
            $readyText = Format-SetupWizard $script:setupWizardSeen
            # What UI Automation sees of the same window, by the class Windows
            # gives it: recorded, to explain the first run's failure.
            $wizardClass = @($script:setupWizardSeen.Tops | Where-Object { $_ -match ';Setup - CodeWiki' } | ForEach-Object { ($_ -split ';')[0] }) | Select-Object -First 1
            if (-not $wizardClass) { $wizardClass = "TWizardForm" }
            $readyUia = @(Get-GuestElements -Class $wizardClass | Where-Object { $_.Name })
            $readyText += "`nUI Automation under class '$wizardClass': $($readyUia.Count) named elements: $(($readyUia | ForEach-Object { "$($_.Kind) '$($_.Name)'$(if (-not $_.OnScreen) { ' (off screen)' })" }) -join ', ')"
            Send-RestedClick -X $install.X -Y $install.Y
            Wait-For 1
            $progress = Shot "setup-wizard-progress"
            $finish = $null
            for ($try = 0; $try -lt 40 -and -not $finish; $try++) {
                Wait-For 4
                $finish = Find-SetupButton '^&?Finish$'
            }
            $done = Shot "setup-wizard-finish"
            if (-not $finish) { throw "the wizard showed no Finish button within three minutes; read the screenshots; $(Format-SetupWizard $script:setupWizardSeen)" }
            $finishText = Format-SetupWizard $script:setupWizardSeen
            # The Start CodeWiki box is an item of a check list, not a window
            # of its own: only UI Automation names it. Recorded, not judged; the
            # app's window appearing after Finish is what shows it was checked.
            $finishElements = @(Get-GuestElements -Class $wizardClass | Where-Object { $_.OnScreen -and $_.Name })
            $finishText += "`nUI Automation under class '$wizardClass': $(($finishElements | ForEach-Object { "$($_.Kind) '$($_.Name)'" }) -join ', ')"
            $startBox = @($finishElements | Where-Object { $_.Name -like "*Start CodeWiki*" }).Count
            Send-RestedClick -X $finish.X -Y $finish.Y
            $problems = @()
            try {
                [void](Show-GuestWindow -ProcessName "codewiki-app" -TimeoutSec 90)
            } catch {
                $problems += "no app window appeared after Finish"
            }
            Wait-For 8
            $shot = Shot "setup-wizard-app"
            $state = Get-SetupState
            $problems += @(Test-SetupInstalled $state $setupVersion)
            $probe = Get-Probe "setup-wizard" -Quick
            if (@(Get-App $probe | Where-Object { $_.hasWindow }).Count -ne 1) { $problems += "the app has $(@(Get-App $probe | Where-Object { $_.hasWindow }).Count) windows" }
            Add-Result -Id "IS4" -Title "The setup wizard by mouse" -Expect "Install and Finish by click; the app starts by itself from the checked Start CodeWiki; the install is complete" `
                -Status $(if ($problems) { "fail" } else { "pass" }) -Shots @($ready, $progress, $done, $shot) `
                -Output ("ready page: $readyText`nfinish page: $finishText") `
                -Observed "run with no switches; clicked Install at $($install.X),$($install.Y) and Finish at $($finish.X),$($finish.Y); Start CodeWiki box named by UI Automation on the finish page: $($startBox -gt 0) (it was left as it came: the app's window appearing shows it was checked); $(Format-SetupState $state); $(if ($problems) { $problems -join '; ' } else { 'all in place' }); read the four screenshots for the page order, the wording and the box"
        } finally {
            Reset-SetupGuest
        }
    }

    Test-Case "IS5" "Silent install over an install.cmd installation, app running" "exit 0 within 120 s with no prompt; the PATH entry once; the settings folder's marker and the Run value stay; the Apps entry appears; codewiki --version answers" {
        Reset-SetupGuest
        try {
            $first = Install-GuestWithCmd
            if ($first.ExitCode -ne 0) { throw "install.cmd failed first: exit $($first.ExitCode)" }
            Invoke-GuestScript -Script (@'
New-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'CodeWiki' -Value '{{RUN}}' -PropertyType String -Force | Out-Null
New-Item -ItemType Directory -Force (Join-Path $env:APPDATA 'CodeWiki') | Out-Null
Set-Content -Path (Join-Path $env:APPDATA 'CodeWiki\marker.txt') -Value 'settings'
'@).Replace('{{RUN}}', $autostartCommand) | Out-Null
            Start-GuestApp -Arguments $guestRepo
            Wait-For 15
            $before = Get-SetupState
            $log = "C:\Users\Public\cwvm-is5.log"
            $clock = [System.Diagnostics.Stopwatch]::StartNew()
            $run = Invoke-GuestSetup -Arguments ($setupSilent + "/LOG=$log") -TimeoutSec 150
            $seconds = [math]::Round($clock.Elapsed.TotalSeconds, 1)
            $shot = Shot "setup-over-install-cmd"
            $state = Get-SetupState
            $problems = @()
            if ($before.procs -eq "0") { $problems += "the app was not running before the setup" }
            if ($before.entry -eq "True") { $problems += "an Apps entry existed before the setup" }
            if ($run.ExitCode -ne 0) { $problems += "setup exited with $($run.ExitCode)" }
            if ($seconds -ge 120) { $problems += "setup took $seconds s" }
            if ($state.prefsmarker -ne "True") { $problems += "the settings folder's marker is gone" }
            if ($state.run -ne $autostartCommand) { $problems += "the Run value is '$($state.run)'" }
            $problems += @(Test-SetupInstalled $state $setupVersion)
            Add-Result -Id "IS5" -Title "Silent install over an install.cmd installation, app running" -Expect "exit 0 in under 120 s; PATH once; marker and Run value kept; Apps entry present; the version answers" `
                -Status $(if ($problems) { "fail" } else { "pass" }) -Shots @($shot) -Output ("setup log $log`n" + (Get-GuestLog $log) + "`n--- stdout/stderr`n" + ($run.Stdout + "`n" + $run.Stderr).Trim()).Trim() `
                -Observed "install.cmd first (exit $($first.ExitCode)), the app running ($($before.procs) processes), no Apps entry ($($before.entry)); then setup silent: exit $($run.ExitCode) after $seconds s; $(Format-SetupState $state); app processes after: $($state.procs); codewiki --version: '$($state.version)'; $(if ($problems) { $problems -join '; ' } else { 'all in place' })"
        } finally {
            Reset-SetupGuest
        }
    }

    Test-Case "IS6" "A failing install says why" "exit 7; the setup log holds 'CodeWiki could not be installed'; no Apps entry is written" {
        Reset-SetupGuest
        try {
            $first = Install-GuestWithCmd
            if ($first.ExitCode -ne 0) { throw "install.cmd failed first: exit $($first.ExitCode)" }
            # A copy over a file that is held open with no sharing fails.
            Start-GuestFileHold "$guestTarget\codewiki.exe"
            $log = "C:\Users\Public\cwvm-is6.log"
            $run = Invoke-GuestSetup -Arguments ($setupSilent + "/LOG=$log")
            $shot = Shot "setup-failed"
            $state = Get-SetupState
            $text = Get-GuestLog $log
            $problems = @()
            if ($run.ExitCode -ne 7) { $problems += "setup exited with $($run.ExitCode), not 7" }
            if ($text -notmatch "CodeWiki could not be installed") { $problems += "the setup log does not say 'CodeWiki could not be installed'" }
            if ($state.entry -eq "True") { $problems += "an Apps entry was written" }
            Add-Result -Id "IS6" -Title "A failing install says why" -Expect "exit 7, the reason in the log, no Apps entry" `
                -Status $(if ($problems) { "fail" } else { "pass" }) -Shots @($shot) -Output ("setup log $log`n" + $text + "`n--- stdout/stderr`n" + ($run.Stdout + "`n" + $run.Stderr).Trim()).Trim() `
                -Observed "install.cmd first (exit $($first.ExitCode)), codewiki.exe held open with no sharing, then setup silent: exit $($run.ExitCode); log says 'could not be installed': $($text -match 'CodeWiki could not be installed'); Apps entry: $($state.entry); $(if ($problems) { $problems -join '; ' } else { 'failed as it should' })"
        } finally {
            Reset-SetupGuest
        }
    }

    Test-Case "IS7" "Another folder is refused" "exit 7; the setup log holds 'CodeWiki installs only to'; no CodeWiki programs in the other folder, nothing in the default one, no Apps entry" {
        Reset-SetupGuest
        $elsewhere = "C:\Users\Public\cwvm-elsewhere"
        try {
            $log = "C:\Users\Public\cwvm-is7.log"
            # The folder has no space, so it needs no quotes; the harness would
            # hand Setup a quote inside an argument as \".
            $run = Invoke-GuestSetup -Arguments ($setupSilent + "/DIR=$elsewhere" + "/LOG=$log")
            $shot = Shot "setup-elsewhere-refused"
            $state = Get-SetupState
            $text = Get-GuestLog $log
            $other = (Invoke-GuestScript -Script (@'
$d = '{{DIR}}'
"exists=$(Test-Path -LiteralPath $d)"
$names = @()
if (Test-Path -LiteralPath $d) { $names = @(Get-ChildItem -LiteralPath $d -Force | ForEach-Object { $_.Name }) }
"names=$($names -join '|')"
$user = [Environment]::GetEnvironmentVariable('Path', 'User')
"inpath=$(@($user -split ';' | Where-Object { $_ -and $_.Trim().TrimEnd('\') -ieq $d }).Count)"
'@).Replace('{{DIR}}', $elsewhere)).Output
            if ($other -notmatch '(?m)^exists=(True|False)') { throw "could not read $elsewhere in the guest: $other" }
            $otherExists = $Matches[1]
            $otherNames = @(if ($other -match '(?m)^names=(.*)$') { $Matches[1].Trim() -split '\|' | Where-Object { $_ } })
            $otherInPath = if ($other -match '(?m)^inpath=(\d+)') { $Matches[1] } else { "?" }
            $problems = @()
            if ($run.ExitCode -ne 7) { $problems += "setup exited with $($run.ExitCode), not 7" }
            if ($text -notmatch "CodeWiki installs only to") { $problems += "the setup log does not say 'CodeWiki installs only to'" }
            $programs = @($otherNames | Where-Object { "codewiki.exe", "codewiki-mcp.exe", "codewiki-app.exe", "plugins", "uninstall.ps1", "unins000.exe" -contains $_ })
            if ($programs) { $problems += "$elsewhere holds $($programs -join ', ')" }
            if ($otherInPath -ne "0") { $problems += "the user PATH holds $elsewhere $otherInPath times" }
            if ($state.entry -eq "True") { $problems += "an Apps entry was written" }
            if ($state.folder -eq "True" -and $state.names.Count) { $problems += "the default folder holds $($state.names -join ', ')" }
            if ($state.pathcount -ne "0") { $problems += "the user PATH holds the default folder $($state.pathcount) times" }
            Add-Result -Id "IS7" -Title "Another folder is refused" -Expect "exit 7, the reason in the log, nothing installed in either folder, no Apps entry" `
                -Status $(if ($problems) { "fail" } else { "pass" }) -Shots @($shot) -Output ("setup log $log`n" + $text + "`n--- stdout/stderr`n" + ($run.Stdout + "`n" + $run.Stderr).Trim()).Trim() `
                -Observed "setup silent with /DIR=$elsewhere`: exit $($run.ExitCode); log says 'installs only to': $($text -match 'CodeWiki installs only to'); $elsewhere exists: $otherExists [$($otherNames -join ', ')], in the user PATH $otherInPath times; default folder: $(Format-SetupState $state); $(if ($problems) { $problems -join '; ' } else { 'refused, nothing written' })"
        } finally {
            Invoke-GuestScript -Script "Remove-Item -LiteralPath '$elsewhere' -Recurse -Force -ErrorAction SilentlyContinue" | Out-Null
            Reset-SetupGuest
        }
    }

    Test-Case "IS8" "Setup over a setup installation" "the second silent run exits 0; the PATH entry once; one Apps entry, with the version; codewiki --version answers; then the uninstall from the entry exits 0 and leaves nothing" {
        Reset-SetupGuest
        try {
            $first = Invoke-GuestSetup -Arguments ($setupSilent + "/LOG=C:\Users\Public\cwvm-is8-first.log")
            if ($first.ExitCode -ne 0) { throw "the first silent install failed: exit $($first.ExitCode)" }
            $log = "C:\Users\Public\cwvm-is8.log"
            $run = Invoke-GuestSetup -Arguments ($setupSilent + "/LOG=$log")
            $shot = Shot "setup-over-setup"
            $state = Get-SetupState
            $entries = @(Get-SetupEntryKey)
            $problems = @()
            if ($run.ExitCode -ne 0) { $problems += "the second setup exited with $($run.ExitCode)" }
            $problems += @(Test-SetupInstalled $state $setupVersion)
            if ($entries.Count -ne 1 -or $entries[0] -ne $setupEntryKey) { $problems += "Apps entries named CodeWiki: $($entries.Count) ($($entries -join ', '))" }
            $uninstall = Invoke-GuestUninstall -Log "C:\Users\Public\cwvm-is8-uninstall.log" -Quiet
            Wait-For 3
            $gone = Get-SetupState
            $left = @(Get-SetupEntryKey)
            if ($uninstall.ExitCode -ne 0) { $problems += "the uninstall exited with $($uninstall.ExitCode)" }
            if ($gone.folder -eq "True") { $problems += "the folder is left: $($gone.names -join ', ')" }
            if ($gone.pathcount -ne "0") { $problems += "PATH entry left" }
            if ($gone.lnk -eq "True") { $problems += "shortcut left" }
            if ($left.Count) { $problems += "Apps entries left: $($left -join ', ')" }
            if ($gone.procs -ne "0") { $problems += "$($gone.procs) CodeWiki processes still run" }
            Add-Result -Id "IS8" -Title "Setup over a setup installation" -Expect "second exit 0; PATH once; one Apps entry with the version; the version answers; then uninstall exit 0 and nothing left" `
                -Status $(if ($problems) { "fail" } else { "pass" }) -Shots @($shot) `
                -Output ("setup log $log`n" + (Get-GuestLog $log) + "`n--- stdout/stderr`n" + ($run.Stdout + "`n" + $run.Stderr).Trim() + "`n--- uninstall: $($uninstall.Command) $($uninstall.Arguments)`n" + $uninstall.Log).Trim() `
                -Observed "setup silent twice: exit $($first.ExitCode), then $($run.ExitCode); then $(Format-SetupState $state); Apps entries named CodeWiki: $($entries.Count); codewiki --version: '$($state.version)'; uninstall from QuietUninstallString: exit $($uninstall.ExitCode); then $(Format-SetupState $gone), Apps entries $($left.Count); $(if ($problems) { $problems -join '; ' } else { 'one installation, removed' })"
        } finally {
            Reset-SetupGuest
        }
    }

    # Since 0.4.0 an uninstall the user watches asks whether to delete the
    # data and settings too (No is the default), and /PURGEDATA says Yes for a
    # silent one. Both start from a setup installation with data and settings
    # to delete (marker files) and the app's notification key.
    function Initialize-PurgeCase {
        param([string]$Name)
        Reset-SetupGuest
        $install = Invoke-GuestSetup -Arguments ($setupSilent + "/LOG=C:\Users\Public\cwvm-$Name-install.log")
        if ($install.ExitCode -ne 0) { throw "the silent install failed first: exit $($install.ExitCode)" }
        Invoke-GuestScript -Script @'
New-Item -ItemType Directory -Force (Join-Path $env:LOCALAPPDATA 'CodeWiki'), (Join-Path $env:APPDATA 'CodeWiki') | Out-Null
Set-Content -Path (Join-Path $env:LOCALAPPDATA 'CodeWiki\marker.txt') -Value 'data'
Set-Content -Path (Join-Path $env:APPDATA 'CodeWiki\marker.txt') -Value 'settings'
'@ | Out-Null
    }

    Test-Case "IS9" "Uninstall from Settings > Apps, deleting data and settings" "the uninstall asks whether to delete the data and settings; Yes deletes both folders with the program" {
        Initialize-PurgeCase "is9"
        try {
            $before = Get-SetupState
            # UninstallString as Settings > Apps runs it: no switches, so the
            # uninstaller shows its questions.
            $command = $before.e_UninstallString
            if ($command -notmatch '^"([^"]+)"\s*(.*)$') { throw "the Apps entry has no usable uninstall command: '$command'" }
            Start-GuestExe -Exe $Matches[1] -Arguments @($Matches[2] -split '\s+' | Where-Object { $_ })
            $answered = @()
            # Not $shots: Shot reads the screenshots folder by that name.
            $pictures = @()
            # Inno's own "Are you sure ...", then CodeWiki's question, then
            # "... was successfully removed": Yes, Yes, OK.
            foreach ($step in @(@{ Name = "confirm"; Button = '^Yes$' }, @{ Name = "purge-question"; Button = '^Yes$' }, @{ Name = "removed"; Button = '^OK$' })) {
                $button = $null
                $deadline = (Get-Date).AddSeconds(90 * $Pace)
                while (-not $button -and (Get-Date) -lt $deadline) {
                    Start-Sleep -Seconds 2
                    $button = Find-SetupButton $step.Button
                }
                $pictures += Shot "setup-uninstall-$($step.Name)"
                $texts = @($script:setupWizardSeen.Controls | Where-Object { $_.Visible -and $_.Text -and $_.Class -notmatch 'Button' } | ForEach-Object { $_.Text })
                $answered += "$($step.Name): $(if ($button) { "'$($button.Text -replace '&', '')' clicked" } else { 'no button' }); text '$(($texts -join ' ') -replace '\s+', ' ')'"
                if (-not $button) { break }
                Send-RestedClick -X $button.X -Y $button.Y
            }
            Wait-GuestUninstaller
            $after = Get-SetupState
            $asked = [bool]($answered | Where-Object { $_ -match "^purge-question: 'Yes' clicked; text '.*Delete your CodeWiki data and settings" })
            $problems = @()
            if (-not $asked) { $problems += "the question about data and settings was not seen or not answered" }
            if ($before.datamarker -ne "True" -or $before.prefsmarker -ne "True") { $problems += "no data and settings before the uninstall" }
            if ($after.datadir -eq "True") { $problems += "the data folder is left" }
            if ($after.prefsdir -eq "True") { $problems += "the settings folder is left" }
            if ($after.folder -eq "True") { $problems += "the install folder is left: $($after.names -join ', ')" }
            if ($after.entry -eq "True") { $problems += "Apps entry left" }
            Add-Result -Id "IS9" -Title "Uninstall from Settings > Apps, deleting data and settings" -Expect "the question about data and settings; after Yes, no install folder, no Apps entry, no data folder, no settings folder" `
                -Status $(if ($problems) { "fail" } else { "pass" }) -Shots $pictures `
                -Observed "ran UninstallString '$command' as Settings > Apps does; $($answered -join '; '); after: $(Format-SetupState $after), data folder $($after.datadir), settings folder $($after.prefsdir); $(if ($problems) { $problems -join '; ' } else { 'all removed' })"
        } finally {
            Reset-SetupGuest
        }
    }

    Test-Case "IS10" "Silent uninstall with /PURGEDATA" "QuietUninstallString plus /PURGEDATA exits 0 and deletes the data and settings folders with the program" {
        Initialize-PurgeCase "is10"
        try {
            $before = Get-SetupState
            $run = Invoke-GuestUninstall -Log "C:\Users\Public\cwvm-is10.log" -Quiet -Extra "/PURGEDATA"
            $after = Get-SetupState
            $problems = @()
            if ($before.datamarker -ne "True" -or $before.prefsmarker -ne "True") { $problems += "no data and settings before the uninstall" }
            if ($run.ExitCode -ne 0) { $problems += "uninstall exited with $($run.ExitCode)" }
            if ($after.datadir -eq "True") { $problems += "the data folder is left" }
            if ($after.prefsdir -eq "True") { $problems += "the settings folder is left" }
            if ($after.folder -eq "True") { $problems += "the install folder is left: $($after.names -join ', ')" }
            if ($after.entry -eq "True") { $problems += "Apps entry left" }
            Add-Result -Id "IS10" -Title "Silent uninstall with /PURGEDATA" -Expect "exit 0; no install folder, no Apps entry, no data folder, no settings folder" `
                -Status $(if ($problems) { "fail" } else { "pass" }) -Output ("$($run.Command) $($run.Arguments)`n" + $run.Log).Trim() `
                -Observed "ran QuietUninstallString plus $($run.Arguments): exit $($run.ExitCode); after: $(Format-SetupState $after), data folder $($after.datadir), settings folder $($after.prefsdir); $(if ($problems) { $problems -join '; ' } else { 'all removed' })"
        } finally {
            Reset-SetupGuest
        }
    }
}

# ---------------------------------------------------------------- U2, self-update
# Ends the app and removes the installation U2 made, data included, and the
# Apps entry U2 made to stand for a setup-installed old copy.
function Remove-UpdateTestInstall {
    Stop-GuestApp
    Invoke-GuestExe -Exe "C:\Windows\System32\cmd.exe" -TimeoutSec 120 -Arguments "/c", "$guestTarget\uninstall.cmd", "-PurgeData" | Out-Null
    Invoke-GuestScript -Script "Remove-Item -LiteralPath '$setupEntryKey' -Recurse -Force -ErrorAction SilentlyContinue" | Out-Null
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
        # The Apps entry a setup-installed old copy would have; the update must
        # move its DisplayVersion to the new version, and never creates it.
        Invoke-GuestScript -Script (@'
New-Item -Path '{{KEY}}' -Force | Out-Null
New-ItemProperty -LiteralPath '{{KEY}}' -Name 'DisplayVersion' -Value '{{OLD}}' -PropertyType String -Force | Out-Null
'@).Replace('{{KEY}}', $setupEntryKey).Replace('{{OLD}}', $oldVersion) | Out-Null
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
        # Only an updater from 0.3.1 on writes the entry (cw-update apps_entry); an older
        # old app leaves it as seeded, which is recorded and not judged.
        $entryChecked = [version]($oldVersion -split '[-+]')[0] -ge [version]'0.3.1'
        $entryNow = (Invoke-GuestScript -Script "(Get-ItemProperty -LiteralPath '$setupEntryKey' -ErrorAction SilentlyContinue).DisplayVersion").Output.Trim()
        $ok = $install.ExitCode -eq 0 -and $was -match [regex]::Escape($oldVersion) -and $now -match [regex]::Escape($newVersion) -and $window.Count -eq 1 -and (Get-App $probe).Count -eq 1 -and -not $left -and (-not $entryChecked -or $entryNow -eq $newVersion)
        Add-Result -Id "U2" -Title "The app updates itself" -Expect "codewiki --version changes from $oldVersion to $newVersion; the app is running again; no .old file or .update folder is left; the Apps entry's DisplayVersion changes to $newVersion (checked only when the old version is 0.3.1 or later)" `
            -Status $(if ($ok) { "pass" } else { "fail" }) -Shots @($a, $b, $c) -Output ("requests the stand-in saw:`n" + $asked + "`n--- app.log`n" + $log).Trim() `
            -Observed "installed $oldVersion (exit $($install.ExitCode))$(if ($oldStart -and $oldStart.Status -ne 'started') { ", which Windows refused to start until the rules $($oldStart.Rules -join '; ') were in audit" }), codewiki --version said '$was'; update source: $(if ($RealRelease) { 'GitHub' } else { 'the stand-in in the guest' }); palette: 'update codewiki', Enter; 75 s later codewiki --version said '$now', app processes: $((Get-App $probe).Count), app windows: $($window.Count), left over in the install folder: $(if ($left) { $left } else { 'nothing' }); the Apps entry made before the update held DisplayVersion '$oldVersion' and holds '$entryNow' now $(if ($entryChecked) { '(checked)' } else { '(not checked: the old app''s updater predates the version write)' })"
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
