# Helpers for driving a Hyper-V guest from the host.
# Dot-source this file, then call Initialize-Vm once:
#   . scripts\vm\Vm.ps1
#   Initialize-Vm -Name "Software Tester" -User tester -Password codewiki
#
# Needs PowerShell 7, the Hyper-V PowerShell module and membership of the
# local group "Hyper-V Administrators" (or an elevated shell). Nothing here
# needs elevation once the account is in that group.
#
# What is used for what:
#   files and commands   PowerShell Direct (New-PSSession -VMName), no network
#   a program on the     a scheduled task that runs in the logged-in session;
#   guest's desktop      a PowerShell Direct session has no desktop
#   screenshots          the thumbnail call of the virtualization WMI provider
#   keyboard and mouse   Msvm_Keyboard and Msvm_SyntheticMouse
#   recording            record.ps1, which pipes screenshots to ffmpeg
#
# Microsoft's pages for these interfaces:
#   PowerShell Direct    https://learn.microsoft.com/windows-server/virtualization/hyper-v/powershell-direct
#   screenshot           https://learn.microsoft.com/windows/win32/hyperv_v2/getvirtualsystemthumbnailimage-msvm-virtualsystemmanagementservice
#   keyboard             https://learn.microsoft.com/windows/win32/hyperv_v2/msvm-keyboard
#   key codes            https://learn.microsoft.com/windows/win32/inputdev/virtual-key-codes
#   mouse                https://learn.microsoft.com/windows/win32/hyperv_v2/msvm-syntheticmouse
#   graphics card        https://learn.microsoft.com/windows-hardware/drivers/display/gpu-paravirtualization

$ErrorActionPreference = "Stop"

$script:Vm = $null
$script:VirtualizationNamespace = "root\virtualization\v2"

function Initialize-Vm {
    param(
        [Parameter(Mandatory)][string]$Name,
        [string]$User = "tester",
        [string]$Password = "codewiki",
        # Folder in the guest that holds everything the test copies in.
        [string]$GuestDir = "C:\vmtest",
        # Run every guest script the old way, as a scheduled task of its own,
        # and not through the helper (guest\agent.ps1).
        [switch]$NoAgent
    )
    if (-not (Get-Command Get-VM -ErrorAction SilentlyContinue)) {
        throw "the Hyper-V PowerShell module is missing; turn on the Windows feature 'Hyper-V'"
    }
    # A guest account may have no password; ConvertTo-SecureString refuses an
    # empty text. Such a guest must also allow a blank password over
    # PowerShell Direct: HKLM\SYSTEM\CurrentControlSet\Control\Lsa,
    # LimitBlankPasswordUse = 0.
    $secret = if ($Password) { ConvertTo-SecureString $Password -AsPlainText -Force } else { [securestring]::new() }
    $script:Vm = @{
        Name       = $Name
        User       = $User
        Password   = $Password
        Credential = [pscredential]::new($User, $secret)
        GuestDir   = $GuestDir
        Sequence   = 0
        Recorder   = $null
        # When the recording began, for the timing log's "rec" column.
        RecStart   = $null
        # The one PowerShell Direct session that calls with -Reuse share.
        Session    = $null
        # The helper that runs guest scripts (Start-GuestAgent): its folder in
        # the guest. AgentFailedAt: when it last failed to start.
        Agent         = $null
        AgentFailedAt = $null
        NoAgent       = [bool]$NoAgent
        # Whether the guest draws on a shared graphics card; widens the screen
        # compare's tolerance (Get-ScreenTolerance). The caller sets it.
        Gpu        = $false
        # WMI objects of the running VM; cleared whenever the VM stops.
        Devices    = $null
    }
}

# ------------------------------------------------------------ timing and waits
# What takes the time in a run is written down as it happens, one JSON line per
# event, in logs\timing.jsonl: t (host time, UTC), rec (seconds into the film,
# null before it), case, kind (case, step, wait, hold, guest), what, sec,
# deadline and met for a wait, lane and exit for a guest call. Never the text
# of a script: it can hold a token.
$script:Timing = @{ Path = $null; Pace = 1.0; Case = "" }

function Initialize-Timing {
    param([string]$Path, [double]$Pace = 1.0)
    $script:Timing = @{ Path = $Path; Pace = $Pace; Case = "" }
    if ($Path) { Remove-Item $Path -Force -ErrorAction SilentlyContinue }
}

function Write-Timing {
    param(
        [Parameter(Mandatory)][string]$Kind,
        [Parameter(Mandatory)][string]$What,
        [double]$Sec = 0,
        $Deadline = $null,
        $Met = $null,
        [string]$Lane = "",
        $Exit = $null
    )
    if (-not $script:Timing.Path) { return }
    $record = [ordered]@{
        t    = [DateTime]::UtcNow.ToString("o")
        rec  = if ($script:Vm -and $script:Vm.RecStart) { [math]::Round(([DateTime]::UtcNow - $script:Vm.RecStart).TotalSeconds, 1) } else { $null }
        case = $script:Timing.Case
        kind = $Kind
        what = $What
        sec  = [math]::Round($Sec, 2)
    }
    if ($null -ne $Deadline) { $record.deadline = [math]::Round([double]$Deadline, 2) }
    if ($null -ne $Met) { $record.met = [bool]$Met }
    if ($Lane) { $record.lane = $Lane }
    if ($null -ne $Exit) { $record.exit = $Exit }
    try {
        [System.IO.File]::AppendAllText($script:Timing.Path, ((ConvertTo-Json $record -Compress) + "`n"))
    } catch {
        # A log that cannot be written must not stop the run.
    }
}

# Run a block and write down how long it took. For the steps of the setup:
# restore, boot, copies, install. The block runs in a scope of its own, so it
# sets what it hands on with $script:.
function Measure-Step {
    param([Parameter(Mandatory, Position = 0)][string]$Name, [Parameter(Mandatory, Position = 1)][scriptblock]$Block)
    $clock = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        & $Block
    } finally {
        Write-Timing -Kind step -What $Name -Sec $clock.Elapsed.TotalSeconds
    }
}

# Wait until the condition gives a value, and return that value; $null when
# -TimeoutSec has passed. It does not throw by default: the fixed sleep it
# replaces went on after its time, and the case's own checks judge what is
# there. -Pace multiplies the deadline, never -IntervalMs.
function Wait-Until {
    param(
        [Parameter(Mandatory)][string]$What,
        [Parameter(Mandatory)][double]$TimeoutSec,
        [int]$IntervalMs = 250,
        [switch]$Throw,
        [Parameter(Mandatory, Position = 0)][scriptblock]$Condition
    )
    $limit = $TimeoutSec * $script:Timing.Pace
    $clock = [System.Diagnostics.Stopwatch]::StartNew()
    $value = $null
    while ($true) {
        $value = & $Condition
        if ($value) { break }
        $left = $limit - $clock.Elapsed.TotalSeconds
        if ($left -le 0) { $value = $null; break }
        Start-Sleep -Milliseconds ([int][math]::Max(1, [math]::Min($IntervalMs, $left * 1000)))
    }
    Write-Timing -Kind wait -What $What -Sec $clock.Elapsed.TotalSeconds -Deadline $limit -Met ([bool]$value)
    if ($Throw -and -not $value) { throw "gave up waiting for '$What' after $([math]::Round($limit, 1)) seconds" }
    $value
}

# A pause that is part of the test: a window that must stay quiet while it is
# watched, a pointer that rests, an animation that must have finished. It says
# why, and it stays as long as it is. -Exact is not stretched by -Pace.
function Hold-For {
    param(
        [Parameter(Mandatory)][double]$Seconds,
        [Parameter(Mandatory)][string]$Reason,
        [switch]$Exact
    )
    $length = if ($Exact) { $Seconds } else { $Seconds * $script:Timing.Pace }
    $clock = [System.Diagnostics.Stopwatch]::StartNew()
    Start-Sleep -Milliseconds ([int]($length * 1000))
    Write-Timing -Kind hold -What $Reason -Sec $clock.Elapsed.TotalSeconds -Deadline $length
}

# Read logs\timing.jsonl of a report folder; an empty list when there is none.
function Read-TimingLog {
    param([Parameter(Mandatory)][string]$Report)
    $file = Join-Path $Report "logs\timing.jsonl"
    if (-not (Test-Path $file)) { return @() }
    @(Get-Content $file | Where-Object { $_.Trim() } | ForEach-Object { $_ | ConvertFrom-Json })
}

# What a timing log says about the run, in minutes: from its first event's
# start to its last event, and how long the setup before the film took.
function Get-TimingTotal {
    param($Events)
    $events = @($Events)
    if (-not $events.Count) { return $null }
    $first = $events | Sort-Object { [DateTime]$_.t } | Select-Object -First 1
    $last = $events | Sort-Object { [DateTime]$_.t } | Select-Object -Last 1
    $begin = ([DateTime]$first.t).AddSeconds(-[double]$first.sec)
    $end = [DateTime]$last.t
    $filmed = $events | Where-Object { $null -ne $_.rec -and $_.rec -ge 0 } | Select-Object -First 1
    $setup = $null
    if ($filmed) { $setup = (([DateTime]$filmed.t).AddSeconds(-[double]$filmed.rec) - $begin).TotalMinutes }
    [pscustomobject]@{ Total = ($end - $begin).TotalMinutes; Setup = $setup; Film = if ($null -ne $setup) { ($end - $begin).TotalMinutes - $setup } else { $null } }
}

# Write logs\timing.md for a finished run: the total, the slowest cases and
# waits, what each kind of wait added up to, the waits that gave up at their
# deadline, the fallbacks taken, and each case against an earlier run's.
function Write-TimingSummary {
    param(
        [Parameter(Mandatory)][string]$Report,
        # The report folder of an earlier run to compare with.
        [string]$Previous
    )
    $events = @(Read-TimingLog $Report)
    if (-not $events.Count) { return }
    $text = [System.Collections.Generic.List[string]]::new()
    $minutes = { param($value) if ($null -eq $value) { "n/a" } else { "{0:n1} min" -f $value } }
    $totals = Get-TimingTotal $events
    $text.Add("# Timing")
    $text.Add("")
    $text.Add("- Run: $(& $minutes $totals.Total) from the first step to the end; setup before the film: $(& $minutes $totals.Setup); the film: $(& $minutes $totals.Film).")
    if ($Previous) {
        $before = @(Read-TimingLog $Previous)
        if ($before.Count) {
            $old = Get-TimingTotal $before
            $text.Add("- Earlier run ($(Split-Path -Leaf $Previous)): $(& $minutes $old.Total), setup $(& $minutes $old.Setup), film $(& $minutes $old.Film).")
        } else {
            $film = Join-Path $Previous "recording\session.mp4"
            $length = $null
            if ((Test-Path $film) -and (Get-Command ffprobe -ErrorAction SilentlyContinue)) {
                $length = [double](& ffprobe -v error -show_entries format=duration -of csv=p=0 $film) / 60
            }
            $text.Add("- Earlier run ($(Split-Path -Leaf $Previous)) has no timing log; its film is $(& $minutes $length).")
        }
    }
    $text.Add("")

    $text.Add("## Slowest cases")
    $text.Add("")
    $text.Add("| Case | Seconds |")
    $text.Add("|---|---|")
    foreach ($row in @($events | Where-Object { $_.kind -eq "case" } | Sort-Object sec -Descending | Select-Object -First 15)) {
        $text.Add("| $($row.what) | $($row.sec) |")
    }
    $text.Add("")

    $text.Add("## Slowest waits and pauses")
    $text.Add("")
    $text.Add("| Case | Kind | What | Seconds | Deadline | Met |")
    $text.Add("|---|---|---|---|---|---|")
    foreach ($row in @($events | Where-Object { $_.kind -in "wait", "hold" } | Sort-Object sec -Descending | Select-Object -First 15)) {
        $text.Add("| $($row.case) | $($row.kind) | $($row.what) | $($row.sec) | $($row.deadline) | $(if ($null -eq $row.met) { '' } else { $row.met }) |")
    }
    $text.Add("")

    $text.Add("## Time by kind")
    $text.Add("")
    $text.Add("| Kind | Events | Seconds |")
    $text.Add("|---|---|---|")
    foreach ($group in $events | Group-Object { if ($_.kind -eq "wait" -and $null -eq $_.met) { "fixed sleep" } elseif ($_.kind -eq "guest") { "guest call, $($_.lane)" } else { $_.kind } } | Sort-Object Name) {
        $sum = ($group.Group | Measure-Object sec -Sum).Sum
        $text.Add("| $($group.Name) | $($group.Count) | $([math]::Round($sum, 1)) |")
    }
    $text.Add("")

    $text.Add("## Steps before the film")
    $text.Add("")
    $text.Add("| Step | Seconds |")
    $text.Add("|---|---|")
    foreach ($row in @($events | Where-Object { $_.kind -eq "step" })) { $text.Add("| $($row.what) | $($row.sec) |") }
    $text.Add("")

    $text.Add("## Waits that gave up at their deadline")
    $text.Add("")
    $gaveUp = @($events | Where-Object { $_.kind -eq "wait" -and $_.met -eq $false })
    if ($gaveUp) {
        $text.Add("| Case | What | Seconds |")
        $text.Add("|---|---|---|")
        foreach ($row in $gaveUp) { $text.Add("| $($row.case) | $($row.what) | $($row.sec) |") }
    } else {
        $text.Add("None.")
    }
    $text.Add("")

    $text.Add("## Fallbacks taken")
    $text.Add("")
    $fallbacks = @($events | Where-Object { $_.kind -eq "guest" -and $_.what -like "fallback*" })
    if ($fallbacks) {
        foreach ($row in $fallbacks) { $text.Add("- $($row.case): $($row.what)") }
    } else {
        $text.Add("None.")
    }
    $text.Add("")

    $text.Add("## Guest calls by lane")
    $text.Add("")
    $text.Add("| Lane | Calls | Seconds | Mean |")
    $text.Add("|---|---|---|---|")
    foreach ($group in $events | Where-Object { $_.kind -eq "guest" -and $_.what -notlike "fallback*" } | Group-Object lane | Sort-Object Name) {
        $sum = ($group.Group | Measure-Object sec -Sum).Sum
        $text.Add("| $($group.Name) | $($group.Count) | $([math]::Round($sum, 1)) | $([math]::Round($sum / $group.Count, 2)) |")
    }
    $text.Add("")

    if ($Previous) {
        $oldResults = Join-Path $Previous "logs\results.json"
        $oldCases = @{}
        if (Test-Path $oldResults) {
            foreach ($row in @(Get-Content $oldResults -Raw | ConvertFrom-Json)) { if ($null -ne $row.seconds) { $oldCases[$row.id] = [double]$row.seconds } }
        }
        $text.Add("## Cases against the earlier run")
        $text.Add("")
        if ($oldCases.Count) {
            $text.Add("| Case | Now | Earlier | Difference |")
            $text.Add("|---|---|---|---|")
            foreach ($row in @($events | Where-Object { $_.kind -eq "case" -and $oldCases.ContainsKey($_.what) })) {
                $text.Add("| $($row.what) | $($row.sec) | $($oldCases[$row.what]) | $([math]::Round($row.sec - $oldCases[$row.what], 1)) |")
            }
        } else {
            $text.Add("The earlier run has no per-case times; only the totals compare.")
        }
        $text.Add("")
    }
    Set-Content -Path (Join-Path $Report "logs\timing.md") -Value $text
}

function Get-VmState {
    $machine = Get-VM -Name $script:Vm.Name -ErrorAction SilentlyContinue
    if ($machine) { "$($machine.State)" } else { "Missing" }
}

# ------------------------------------------------------------ guest commands

# Open a PowerShell Direct session as the guest user. It fails while Windows
# starts; the callers that expect that use Wait-Guest first.
function New-GuestSession {
    param([int]$Attempts = 4)
    $problem = $null
    foreach ($attempt in 1..$Attempts) {
        try {
            return New-PSSession -VMName $script:Vm.Name -Credential $script:Vm.Credential -ErrorAction Stop
        } catch {
            $problem = $_
            if ($attempt -lt $Attempts) { Start-Sleep -Seconds 5 }
        }
    }
    throw "no PowerShell Direct session to '$($script:Vm.Name)': $($problem.Exception.Message)"
}

# The session that calls with -Reuse share. Opening one costs about a second
# and a half, and a run makes some five hundred calls. It is dropped whenever
# the guest stops or restarts, and when a call on it fails or times out.
function Get-GuestSession {
    param([int]$Attempts = 4)
    $session = $script:Vm.Session
    if ($session -and $session.State -eq "Opened") { return $session }
    Clear-GuestSession
    $script:Vm.Session = New-GuestSession -Attempts $Attempts
    $script:Vm.Session
}

function Clear-GuestSession {
    if ($script:Vm -and $script:Vm.Session) { Remove-PSSession $script:Vm.Session -ErrorAction SilentlyContinue }
    if ($script:Vm) { $script:Vm.Session = $null }
}

# What the host holds of a running guest: the shared session and the link to
# the helper. Dropped whenever the guest stops, starts or is restored; the
# next call opens them again.
function Clear-GuestLinks {
    Clear-GuestSession
    if ($script:Vm) { $script:Vm.Agent = $null; $script:Vm.AgentFailedAt = $null }
}

# Run a script block in the guest. By default in a session of its own, so
# that it sees the environment (PATH and the rest) as a newly started program
# would. With -Reuse, in the session that calls share: for work that does not
# depend on a fresh environment (the transport of a script, starting a
# program, copying, reading a log).
# Returns Completed (false after a timeout), Output and Errors.
function Invoke-InGuest {
    param(
        [Parameter(Mandatory)][scriptblock]$ScriptBlock,
        [object[]]$ArgumentList = @(),
        [int]$TimeoutSec = 300,
        # How often to try to open the session; 1 while Windows may not be up yet.
        [int]$Attempts = 4,
        # What the timing log calls this call; "none" for a call that a caller
        # of its own writes down (Invoke-GuestScript). Default: "direct", or
        # "session" with -Reuse.
        [string]$Lane = "",
        [switch]$Reuse
    )
    if (-not $Lane) { $Lane = if ($Reuse) { "session" } else { "direct" } }
    $clock = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        foreach ($try in 1..2) {
            $session = if ($Reuse) { Get-GuestSession -Attempts $Attempts } else { New-GuestSession -Attempts $Attempts }
            $job = $null
            $answer = $null
            $lost = $false
            try {
                $job = Invoke-Command -Session $session -ScriptBlock $ScriptBlock -ArgumentList $ArgumentList -AsJob
                if (-not (Wait-Job $job -Timeout $TimeoutSec)) {
                    # The command still runs in the session: the session goes.
                    $lost = $true
                    return [pscustomobject]@{ Completed = $false; Output = @(); Errors = @("no answer within $TimeoutSec seconds") }
                }
                $problems = @()
                $output = @(Receive-Job $job -ErrorAction SilentlyContinue -ErrorVariable problems)
                $errors = @($problems | ForEach-Object { "$_" })
                # A session that broke mid-way leaves its reason on the job, not in the output.
                $reason = $job.ChildJobs | ForEach-Object { $_.JobStateInfo.Reason } | Where-Object { $_ } | Select-Object -First 1
                if ($reason) { $errors += "$($reason.Message)" }
                $answer = [pscustomobject]@{
                    Completed = $job.State -eq "Completed"
                    Output    = $output
                    Errors    = $errors
                }
                $lost = $session.State -ne "Opened"
            } catch {
                if (-not $Reuse -or $try -eq 2) { throw }
                $lost = $true
            } finally {
                if ($Reuse) {
                    if ($lost -or $session.State -ne "Opened") { Clear-GuestSession }
                } else {
                    # Closing the session first ends a command that is still running.
                    Remove-PSSession $session -ErrorAction SilentlyContinue
                }
                if ($job) { Remove-Job $job -Force -ErrorAction SilentlyContinue }
            }
            # A shared session that broke under the call, with nothing to show
            # for it, is opened again once.
            if ($answer -and -not ($Reuse -and $lost -and $try -eq 1 -and $answer.Output.Count -eq 0)) { return $answer }
        }
    } finally {
        if ($Lane -ne "none") { Write-Timing -Kind guest -What "in-guest" -Sec $clock.Elapsed.TotalSeconds -Lane $Lane }
    }
}
# Guest-side code of Invoke-GuestExe: start the program named in {{SPEC}},
# collect what it prints, and print the result as one line.
$script:RunExeTemplate = @'
$spec = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String('{{SPEC}}')) | ConvertFrom-Json
$line = (@($spec.arguments) | Where-Object { $null -ne $_ } | ForEach-Object {
        if ($_ -match '[\s"]' -or $_ -eq '') { '"' + ($_ -replace '"', '\"') + '"' } else { $_ }
    }) -join ' '
$info = New-Object System.Diagnostics.ProcessStartInfo
$info.FileName = $spec.exe
$info.Arguments = $line
$info.UseShellExecute = $false
$info.RedirectStandardOutput = $true
$info.RedirectStandardError = $true
$info.CreateNoWindow = $true
$process = [System.Diagnostics.Process]::Start($info)
$err = $process.StandardError.ReadToEndAsync()
$out = $process.StandardOutput.ReadToEnd()
$process.WaitForExit()
$answer = @{ code = $process.ExitCode; out = $out; err = $err.Result } | ConvertTo-Json -Compress
"__RESULT=" + [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($answer))
$global:LASTEXITCODE = 0
'@

# Run a program in the guest and wait for it. Returns ExitCode (null when it
# did not finish in time), Stdout and Stderr. Where it runs: see
# Invoke-GuestScript. Use Start-GuestExe for a program that opens a window.
function Invoke-GuestExe {
    param(
        [Parameter(Mandatory)][string]$Exe,
        [string[]]$Arguments = @(),
        [int]$TimeoutSec = 300,
        [switch]$Background
    )
    $json = @{ exe = $Exe; arguments = @($Arguments) } | ConvertTo-Json -Compress
    $spec = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($json))
    $run = Invoke-GuestScript -TimeoutSec $TimeoutSec -Background:$Background -Script $script:RunExeTemplate.Replace("{{SPEC}}", $spec)
    if ($run.Output -match '(?m)^__RESULT=(\S+)') {
        $answer = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($Matches[1])) | ConvertFrom-Json
        return [pscustomobject]@{ ExitCode = $answer.code; Stdout = "$($answer.out)"; Stderr = "$($answer.err)" }
    }
    [pscustomobject]@{ ExitCode = $null; Stdout = ""; Stderr = ("$($run.Output)`n$($run.Stderr)").Trim() }
}

# Start a program on the desktop of the logged-in guest user and do not wait
# for it. It runs as a scheduled task with normal priority and the user's
# everyday rights, which is as close to a double-click as a script gets.
function Start-GuestExe {
    param(
        [Parameter(Mandatory)][string]$Exe,
        [string[]]$Arguments = @(),
        [string]$WorkingDirectory
    )
    $script:Vm.Sequence += 1
    $task = "vmtest-app-{0:d3}" -f $script:Vm.Sequence
    $run = Invoke-InGuest -Reuse -TimeoutSec 120 -ArgumentList $Exe, $Arguments, $WorkingDirectory, $task -ScriptBlock {
        param($exe, $arguments, $directory, $task)
        Get-ScheduledTask -TaskName "vmtest-app-*" -ErrorAction SilentlyContinue | Where-Object { $_.State -ne "Running" } |
            Unregister-ScheduledTask -Confirm:$false -ErrorAction SilentlyContinue
        $line = (@($arguments) | Where-Object { $null -ne $_ } | ForEach-Object {
                if ($_ -match '[\s"]') { '"' + ($_ -replace '"', '\"') + '"' } else { $_ }
            }) -join " "
        # Task Scheduler cuts an unquoted path at its first space
        # (C:\Users\Software Tester\...).
        $options = @{ Execute = if ($exe -match '\s') { '"' + $exe + '"' } else { $exe } }
        if ($line) { $options.Argument = $line }
        if ($directory) { $options.WorkingDirectory = $directory }
        $action = New-ScheduledTaskAction @options
        $principal = New-ScheduledTaskPrincipal -UserId "$env:COMPUTERNAME\$env:USERNAME" -LogonType Interactive -RunLevel Limited
        # A task runs below normal priority unless told otherwise.
        $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -Priority 4 -ExecutionTimeLimit ([TimeSpan]::Zero)
        Register-ScheduledTask -TaskName $task -Action $action -Principal $principal -Settings $settings -Force | Out-Null
        Start-ScheduledTask -TaskName $task
        "started"
    }
    if ($run.Output -notcontains "started") { throw "could not start $Exe on the guest's desktop: $($run.Errors -join '; ')" }
}

# The C# the window helpers use, in the guest: one type, VmTest.Windows, with
# the window functions of user32 and a way to find a program's main window.
# Not the process's MainWindowHandle: that is whichever of its top-level
# windows Windows lists first, and when the pointer rests where the app's close
# button appears, it is the tooltip Windows shows for that button, 35 pixels
# wide. The main window is the largest one on screen.
#
# The text begins every script that needs it. The type is compiled only when
# the process does not have it yet: the helper in the guest (guest\agent.ps1)
# compiles it once, at its start, and a fresh process, as on the fallback
# path, once per call. In memory, never as a DLL on disk.
$script:GuestTypes = @'
if (-not ('VmTest.Windows' -as [type])) {
Add-Type -Namespace VmTest -Name Windows -MemberDefinition '
[StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
[StructLayout(LayoutKind.Sequential)] public struct Point { public int X, Y; }
public delegate bool EachWindow(IntPtr window, IntPtr data);
[DllImport("user32.dll")] public static extern bool EnumWindows(EachWindow each, IntPtr data);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
[DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr window, out Rect rect);
[DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr window, ref Point point);
[DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr window, int command);
[DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
[DllImport("user32.dll")] public static extern bool IsIconic(IntPtr window);
[DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr window);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr window);
[DllImport("user32.dll")] public static extern bool SystemParametersInfo(uint action, uint param, out Rect rect, uint winIni);
[DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);
public static IntPtr Largest(int process) {
    IntPtr best = IntPtr.Zero;
    long most = 0;
    EnumWindows(delegate(IntPtr window, IntPtr data) {
        uint owner;
        GetWindowThreadProcessId(window, out owner);
        if (owner == (uint)process && IsWindowVisible(window)) {
            Rect rect;
            GetWindowRect(window, out rect);
            long area = (long)(rect.Right - rect.Left) * (rect.Bottom - rect.Top);
            if (area > most) { most = area; best = window; }
        }
        return true;
    }, IntPtr.Zero);
    return best;
}'
}
'@

# The main window of the first process of that name that has one, or zero.
$script:GuestWindowOf = @'
function Get-MainWindow($name) {
    foreach ($process in @(Get-Process $name -ErrorAction SilentlyContinue)) {
        $window = [VmTest.Windows]::Largest($process.Id)
        if ($window -ne [IntPtr]::Zero) { return $window }
    }
    [IntPtr]::Zero
}
'@

# Wait for a program's window, click its top edge so that it has the keyboard
# focus, and return where it is on the guest's screen (Left, Top, Right,
# Bottom, in pixels). A window that Start-GuestExe opened does not get the
# focus by itself: Windows gives it only to what the user started, and keys
# sent before this call go to the desktop instead. The returned position
# lets a test click at a place relative to the window.
function Show-GuestWindow {
    param(
        [Parameter(Mandatory)][string]$ProcessName,
        [int]$TimeoutSec = 60
    )
    $run = Invoke-GuestScript -InProcess -TimeoutSec ($TimeoutSec + 60) -Script @"
$script:GuestTypes
$script:GuestWindowOf
[void][VmTest.Windows]::SetProcessDPIAware()
`$deadline = (Get-Date).AddSeconds($TimeoutSec)
do {
    `$window = Get-MainWindow '$ProcessName'
    if (`$window -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 250 }
} while (`$window -eq [IntPtr]::Zero -and (Get-Date) -lt `$deadline)
if (`$window -ne [IntPtr]::Zero) {
    `$rect = New-Object VmTest.Windows+Rect
    [void][VmTest.Windows]::GetWindowRect(`$window, [ref]`$rect)
    "rect=`$(`$rect.Left),`$(`$rect.Top),`$(`$rect.Right),`$(`$rect.Bottom)"
}
"@
    if ($run.Output -notmatch 'rect=(-?\d+),(-?\d+),(-?\d+),(-?\d+)') {
        throw "no window of '$ProcessName' appeared within $TimeoutSec seconds: $($run.Output) $($run.Stderr)"
    }
    $window = [pscustomobject]@{ Left = [int]$Matches[1]; Top = [int]$Matches[2]; Right = [int]$Matches[3]; Bottom = [int]$Matches[4] }
    Send-Click -X ([int](($window.Left + $window.Right) / 2)) -Y ($window.Top + 10)
    $window
}

# Where a program's main window is and what state it is in, without touching
# it: the client area in screen pixels (what the app draws in; a click point
# is measured from it), whether the window is minimized or maximized, and the
# part of the screen the taskbar leaves free. Returns null when the program
# has no window.
function Get-GuestWindowInfo {
    param([Parameter(Mandatory)][string]$ProcessName)
    $run = Invoke-GuestScript -InProcess -TimeoutSec 60 -Script @"
$script:GuestTypes
$script:GuestWindowOf
# Without this, a scaled screen gives scaled numbers, and clicks miss.
[void][VmTest.Windows]::SetProcessDPIAware()
`$h = Get-MainWindow '$ProcessName'
if (`$h -ne [IntPtr]::Zero) {
    `$client = New-Object VmTest.Windows+Rect
    `$outer = New-Object VmTest.Windows+Rect
    `$origin = New-Object VmTest.Windows+Point
    `$free = New-Object VmTest.Windows+Rect
    [void][VmTest.Windows]::GetClientRect(`$h, [ref]`$client)
    [void][VmTest.Windows]::GetWindowRect(`$h, [ref]`$outer)
    [void][VmTest.Windows]::ClientToScreen(`$h, [ref]`$origin)
    # SPI_GETWORKAREA: what the taskbar leaves free. Not
    # Screen.PrimaryScreen, which a process without a message pump caches.
    [void][VmTest.Windows]::SystemParametersInfo(0x30, 0, [ref]`$free, 0)
    `$screenWidth = [VmTest.Windows]::GetSystemMetrics(0)
    `$screenHeight = [VmTest.Windows]::GetSystemMetrics(1)
    "info=`$(`$origin.X),`$(`$origin.Y),`$(`$client.Right),`$(`$client.Bottom);`$(`$outer.Left),`$(`$outer.Top),`$(`$outer.Right),`$(`$outer.Bottom);`$([int][VmTest.Windows]::IsIconic(`$h)),`$([int][VmTest.Windows]::IsZoomed(`$h)),`$([int]([VmTest.Windows]::GetForegroundWindow() -eq `$h));`$([VmTest.Windows]::GetDpiForWindow(`$h));`$(`$free.Left),`$(`$free.Top),`$(`$free.Right),`$(`$free.Bottom);`$screenWidth,`$screenHeight"
} else { "info=none" }
"@
    if ($run.Output -notmatch 'info=(-?\d+),(-?\d+),(-?\d+),(-?\d+);(-?\d+),(-?\d+),(-?\d+),(-?\d+);(\d),(\d),(\d);(\d+);(-?\d+),(-?\d+),(-?\d+),(-?\d+);(\d+),(\d+)') {
        if ($run.Output -match 'info=none') { return $null }
        throw "could not read the window of '$ProcessName': $($run.Output) $($run.Stderr)"
    }
    $m = $Matches
    [pscustomobject]@{
        # The client area, in screen pixels.
        Left = [int]$m[1]; Top = [int]$m[2]; Width = [int]$m[3]; Height = [int]$m[4]
        Right = [int]$m[1] + [int]$m[3]; Bottom = [int]$m[2] + [int]$m[4]
        # The whole window, borders that are not drawn included.
        Outer = [pscustomobject]@{ Left = [int]$m[5]; Top = [int]$m[6]; Right = [int]$m[7]; Bottom = [int]$m[8] }
        Minimized = $m[9] -eq "1"; Maximized = $m[10] -eq "1"; Foreground = $m[11] -eq "1"
        Scale = [int]$m[12] / 96.0
        # What the taskbar leaves free, and the whole screen.
        Free = [pscustomobject]@{ Left = [int]$m[13]; Top = [int]$m[14]; Right = [int]$m[15]; Bottom = [int]$m[16] }
        Screen = [pscustomobject]@{ Width = [int]$m[17]; Height = [int]$m[18] }
    }
}

# Move and size a program's main window so that its client area (what the app
# draws in) is at -Left, -Top with the given size, in screen pixels: the same
# layout on screens of different sizes. Returns the window as Get-GuestWindowInfo does.
function Set-GuestWindowClient {
    param(
        [Parameter(Mandatory)][string]$ProcessName,
        [int]$Left = 16, [int]$Top = 12, [int]$Width = 992, [int]$Height = 688
    )
    # The same call that moves the window waits until its client area has the
    # asked size (the app answers a resize on its own time) and reports it, so
    # that no fixed sleep and no second call are needed.
    $run = Invoke-GuestScript -InProcess -TimeoutSec 60 -Script @"
$script:GuestTypes
$script:GuestWindowOf
[void][VmTest.Windows]::SetProcessDPIAware()
`$h = Get-MainWindow '$ProcessName'
if (`$h -eq [IntPtr]::Zero) { 'placed=none'; return }
# A maximized window ignores a new size: restore it first (SW_RESTORE).
[void][VmTest.Windows]::ShowWindow(`$h, 9)
`$outer = New-Object VmTest.Windows+Rect
`$client = New-Object VmTest.Windows+Rect
`$origin = New-Object VmTest.Windows+Point
[void][VmTest.Windows]::GetWindowRect(`$h, [ref]`$outer)
[void][VmTest.Windows]::GetClientRect(`$h, [ref]`$client)
[void][VmTest.Windows]::ClientToScreen(`$h, [ref]`$origin)
`$dx = `$origin.X - `$outer.Left
`$dy = `$origin.Y - `$outer.Top
`$extraWidth = (`$outer.Right - `$outer.Left) - `$client.Right
`$extraHeight = (`$outer.Bottom - `$outer.Top) - `$client.Bottom
# SWP_NOZORDER | SWP_NOACTIVATE
[void][VmTest.Windows]::SetWindowPos(`$h, [IntPtr]::Zero, $Left - `$dx, $Top - `$dy, $Width + `$extraWidth, $Height + `$extraHeight, 0x0014)
'placed=done'
`$deadline = (Get-Date).AddSeconds(2)
do {
    Start-Sleep -Milliseconds 100
    [void][VmTest.Windows]::GetClientRect(`$h, [ref]`$client)
    [void][VmTest.Windows]::ClientToScreen(`$h, [ref]`$origin)
} while ((`$client.Right -ne $Width -or `$client.Bottom -ne $Height -or `$origin.X -ne $Left -or `$origin.Y -ne $Top) -and (Get-Date) -lt `$deadline)
"@
    if ($run.Output -notmatch 'placed=done') { throw "could not place the window of '$ProcessName': $($run.Output) $($run.Stderr)" }
    Get-GuestWindowInfo $ProcessName
}

# Click the way a person does who has stopped the pointer on a control first:
# move there, rest, then press without moving. A click that follows its own
# pointer move at once reaches a control that has only just been redrawn for
# the hover, and so cannot show a control that stops answering while the
# pointer rests on it (the title bar's buttons once did).
function Send-RestedClick {
    param(
        [Parameter(Mandatory)][int]$X,
        [Parameter(Mandatory)][int]$Y,
        [double]$RestSeconds = 1.5,
        [switch]$Double
    )
    Move-Pointer -X $X -Y $Y
    Start-Sleep -Milliseconds ([int]($RestSeconds * 1000))
    Invoke-Device Mouse ClickButton @{ ButtonIndex = [uint32]1 }
    if ($Double) {
        Start-Sleep -Milliseconds 80
        Invoke-Device Mouse ClickButton @{ ButtonIndex = [uint32]1 }
    }
    Start-Sleep -Milliseconds 300
}

function New-GuestDirectory {
    param([Parameter(Mandatory)][string]$GuestPath)
    Invoke-InGuest -Reuse -ArgumentList $GuestPath -ScriptBlock {
        param($path) New-Item -ItemType Directory -Force $path | Out-Null
    } | Out-Null
}

# Copy a file or a folder into a guest folder; a folder arrives under its own
# name. The content travels as one zip file: that is quick for many small
# files, and it leaves behind file attributes the guest does not know (a
# cloud-synced folder marks its files in a way that makes a plain copy fail).
function Copy-ToGuest {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$GuestDirectory
    )
    $source = (Resolve-Path $Path).Path
    $script:Vm.Sequence += 1
    $name = "vmtest-copy-$PID-$($script:Vm.Sequence).zip"
    $package = Join-Path ([System.IO.Path]::GetTempPath()) $name
    Remove-Item $package -Force -ErrorAction SilentlyContinue
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    if (Test-Path $source -PathType Container) {
        [System.IO.Compression.ZipFile]::CreateFromDirectory($source, $package, [System.IO.Compression.CompressionLevel]::Fastest, $true)
    } else {
        $archive = [System.IO.Compression.ZipFile]::Open($package, [System.IO.Compression.ZipArchiveMode]::Create)
        try {
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, $source, (Split-Path -Leaf $source), [System.IO.Compression.CompressionLevel]::Fastest) | Out-Null
        } finally {
            $archive.Dispose()
        }
    }
    $session = Get-GuestSession
    try {
        $guestPackage = Join-Path $script:Vm.GuestDir $name
        Invoke-Command -Session $session -ArgumentList $script:Vm.GuestDir -ScriptBlock {
            param($staging) New-Item -ItemType Directory -Force $staging | Out-Null
        }
        Copy-Item -ToSession $session -Path $package -Destination $guestPackage -Force
    } finally {
        # The shared session stays; one that broke under the copy goes.
        if ($session.State -ne "Opened") { Clear-GuestSession }
        Remove-Item $package -Force -ErrorAction SilentlyContinue
    }
    # Unpacked by the user, so that the files belong to the user as downloaded files do.
    $run = Invoke-GuestScript -TimeoutSec 600 -Script @"
New-Item -ItemType Directory -Force '$GuestDirectory' | Out-Null
Expand-Archive -Path '$guestPackage' -DestinationPath '$GuestDirectory' -Force
Remove-Item '$guestPackage' -Force
"@
    if (-not $run.Completed -or $run.ExitCode -ne 0) { throw "could not unpack $Path in the guest: $($run.Output) $($run.Stderr)" }
}
# Copy a file or a folder out of the guest.
function Copy-FromGuest {
    param(
        [Parameter(Mandatory)][string]$GuestPath,
        [Parameter(Mandatory)][string]$Destination
    )
    $session = Get-GuestSession
    try {
        Copy-Item -FromSession $session -Path $GuestPath -Destination $Destination -Recurse -Force
    } finally {
        if ($session.State -ne "Opened") { Clear-GuestSession }
    }
}

# What a guest script runs: the code, wrapped so that its output, its errors
# and the exit code it leaves land in "<file>.out", one line "__EXIT=<code>" last.
function Get-GuestScriptBody {
    param([Parameter(Mandatory)][string]$Script)
    @"
# This file runs with -ExecutionPolicy Bypass, which travels to every child
# process in this variable. Without it, what the code starts sees the policy
# the guest really has.
Remove-Item Env:PSExecutionPolicyPreference -ErrorAction SilentlyContinue
& {
`$ErrorActionPreference = 'Continue'
`$global:LASTEXITCODE = 0
try {
$Script
    `$__code = `$global:LASTEXITCODE
} catch {
    `$_ | Out-String | Write-Output
    `$__code = 997
}
Write-Output "__EXIT=`$__code"
} 2>&1 | ForEach-Object { "`$_" } | Out-File -FilePath "`$PSCommandPath.out" -Encoding utf8 -Width 8192
"@
}

# The same for the helper's in-process lane: no output file (the helper
# collects what the runspace writes), and nothing that relies on being a file.
function Get-GuestInProcBody {
    param([Parameter(Mandatory)][string]$Script)
    @"
`$ErrorActionPreference = 'Continue'
`$global:LASTEXITCODE = 0
& {
try {
$Script
    `$__code = `$global:LASTEXITCODE
} catch {
    `$_ | Out-String | Write-Output
    `$__code = 997
}
Write-Output "__EXIT=`$__code"
} 2>&1 | ForEach-Object { "`$_" }
"@
}

# The result of a guest script from the lines the guest sent back: the code
# after "__EXIT=" and the rest as output.
function ConvertTo-GuestScriptResult {
    param($Lines, $Errors)
    $code = $null
    $text = foreach ($line in $Lines) {
        if ("$line" -match '^__EXIT=(-?\d+)$') { $code = [int]$Matches[1] } else { "$line" }
    }
    [pscustomobject]@{
        ExitCode  = $code
        Output    = ($text -join "`n").Trim()
        Stderr    = (@($Errors) -join "`n").Trim()
        Completed = $null -ne $code
    }
}

# Run PowerShell code in the guest as a new Windows PowerShell 5.1 process,
# which is what a clean Windows has. The code travels as a file, so it needs
# no quoting. Returns the output, the exit code the code left in
# $LASTEXITCODE (0 when it ran no program), and Completed, which is false
# when the code did not run to its end.
#
# By default the code runs where the user's own programs run: in the
# logged-in desktop session, with the user's everyday rights, without a
# window of its own. It can see the windows on the desktop, and a dialog it
# raises shows in a screenshot. With -Background it runs in the PowerShell
# Direct session instead: elevated and without a desktop. Use that for
# housekeeping, not for what a user would do.
#
# The desktop lane goes through the helper in the guest (guest\agent.ps1),
# which starts the script's own powershell.exe at once; it falls back to the
# scheduled task of Invoke-GuestScriptByTask when the helper cannot be had,
# and with -NoAgent (Initialize-Vm's too) goes that way throughout.
function Invoke-GuestScript {
    param(
        [Parameter(Mandatory)][string]$Script,
        [int]$TimeoutSec = 300,
        [switch]$Background,
        # For the rig's own instruments that read the screen, the log or the
        # processes and start nothing of their own: the helper runs the code
        # in a runspace of its own process, which saves the start of a
        # powershell.exe (0.1 to 0.25 s a call against 0.6 to 1.1). The code
        # sees the helper's environment, not a fresh one, and nothing it
        # starts ends with it. Without the helper it runs as any script.
        [switch]$InProcess,
        [switch]$NoAgent
    )
    if (-not $Background -and -not $NoAgent -and -not $script:Vm.NoAgent) {
        $answer = Invoke-GuestAgent -Script $Script -TimeoutSec $TimeoutSec -InProcess:$InProcess
        if ($answer) { return $answer }
    }
    Invoke-GuestScriptByTask -Script $Script -TimeoutSec $TimeoutSec -Background:$Background
}

# Start the helper in the guest: guest\agent.ps1 as the scheduled task
# "vmtest-agent", in the desktop session, with the same principal and settings
# as the task of a script. It has no logon trigger: nothing may start by
# itself at boot (I1 and AS2 judge what does). Returns false when it did not
# come up; that is remembered for a minute so that each call does not pay for
# it again.
function Start-GuestAgent {
    if ($script:Vm.AgentFailedAt -and ((Get-Date) - $script:Vm.AgentFailedAt).TotalSeconds -lt 60) { return $false }
    $clock = [System.Diagnostics.Stopwatch]::StartNew()
    $source = Get-Content (Join-Path $PSScriptRoot "guest\agent.ps1") -Raw
    $dir = Join-Path (Join-Path $script:Vm.GuestDir "agent") ([guid]::NewGuid().ToString("N").Substring(0, 8))
    $ready = $false
    try {
        $run = Invoke-InGuest -Reuse -Lane none -TimeoutSec 120 -ArgumentList $source, $dir, $script:Vm.GuestDir, $script:GuestTypes -ScriptBlock {
            param($source, $dir, $base, $types)
            New-Item -ItemType Directory -Force (Join-Path $dir "in") | Out-Null
            [System.IO.File]::WriteAllText((Join-Path $dir "types.ps1"), $types, (New-Object System.Text.UTF8Encoding $true))
            $file = Join-Path $base "agent.ps1"
            [System.IO.File]::WriteAllText($file, $source, (New-Object System.Text.UTF8Encoding $true))
            $shell = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
            $action = New-ScheduledTaskAction -Execute "$env:SystemRoot\System32\conhost.exe" `
                -Argument "--headless `"$shell`" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$file`" -Dir `"$dir`""
            $principal = New-ScheduledTaskPrincipal -UserId "$env:COMPUTERNAME\$env:USERNAME" -LogonType Interactive -RunLevel Limited
            $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -Priority 4 -ExecutionTimeLimit ([TimeSpan]::Zero)
            Register-ScheduledTask -TaskName "vmtest-agent" -Action $action -Principal $principal -Settings $settings -Force | Out-Null
            Start-ScheduledTask -TaskName "vmtest-agent"
            $deadline = (Get-Date).AddSeconds(40)
            while ((Get-Date) -lt $deadline) {
                if (Test-Path (Join-Path $dir "pid")) { "agent=ready"; return }
                Start-Sleep -Milliseconds 100
            }
            "agent=not-ready"
        }
        $ready = [bool]($run.Output -contains "agent=ready")
    } catch {
        $ready = $false
    }
    Write-Timing -Kind guest -What $(if ($ready) { "agent start" } else { "fallback: the helper did not start" }) -Sec $clock.Elapsed.TotalSeconds -Lane agent
    if ($ready) {
        $script:Vm.Agent = @{ Dir = $dir }
    } else {
        $script:Vm.AgentFailedAt = Get-Date
    }
    $ready
}

# Run a script through the helper. Returns the result of Invoke-GuestScript,
# or $null when the script was not run and the caller should run it the old
# way: the helper is not there, never took the request, or could not start the
# script. A request the helper took is never run a second time.
function Invoke-GuestAgent {
    param(
        [Parameter(Mandatory)][string]$Script,
        [int]$TimeoutSec = 300,
        [switch]$InProcess
    )
    if (-not $script:Vm.Agent -and -not (Start-GuestAgent)) { return $null }
    $dir = $script:Vm.Agent.Dir
    $script:Vm.Sequence += 1
    $id = "vmtest-{0:d3}" -f $script:Vm.Sequence
    $clock = [System.Diagnostics.Stopwatch]::StartNew()
    $kind = if ($InProcess) { "inproc" } else { "script" }
    $request = @{ kind = $kind; timeoutSec = $TimeoutSec } | ConvertTo-Json -Compress
    $body = if ($InProcess) { Get-GuestInProcBody $Script } else { Get-GuestScriptBody $Script }
    try {
        $run = Invoke-InGuest -Reuse -Lane none -TimeoutSec ($TimeoutSec + 90) -ArgumentList $dir, $id, $body, $request, $TimeoutSec -ScriptBlock {
            param($dir, $id, $body, $request, $timeout)
            $in = Join-Path $dir "in"
            $file = Join-Path $in "$id.ps1"
            $json = Join-Path $in "$id.json"
            $done = Join-Path $in "$id.done"
            $alive = Join-Path $dir "alive"
            # A call that is made again (a broken session is opened once more)
            # must not hand the helper the same request twice.
            $known = ($json, "$json.run", "$json.cancelled", $done | Where-Object { [System.IO.File]::Exists($_) })
            if (-not $known) {
                [System.IO.File]::WriteAllText($file, $body, (New-Object System.Text.UTF8Encoding $true))
                [System.IO.File]::WriteAllText("$json.tmp", $request)
                [System.IO.File]::Move("$json.tmp", $json)
            }
            $started = [DateTime]::UtcNow
            $deadline = $started.AddSeconds($timeout + 60)
            $checked = [System.Diagnostics.Stopwatch]::StartNew()
            $status = $null
            while ([DateTime]::UtcNow -lt $deadline) {
                if ([System.IO.File]::Exists($done)) { $status = "done"; break }
                if ($checked.ElapsedMilliseconds -ge 1000) {
                    $checked.Restart()
                    $silent = try { ([DateTime]::UtcNow - [System.IO.File]::GetLastWriteTimeUtc($alive)).TotalSeconds } catch { 999 }
                    $taken = [System.IO.File]::Exists("$json.run")
                    if (-not $taken -and $silent -gt 5) {
                        # Never taken, and the helper is quiet: cancel by rename. If
                        # that fails the helper took it just now; go on waiting.
                        try { [System.IO.File]::Move($json, "$json.cancelled"); $status = "cancelled"; break } catch { }
                    }
                    if ($taken -and $silent -gt 15) { $status = "lost"; break }
                }
                Start-Sleep -Milliseconds 20
            }
            if (-not $status) { $status = "timeout" }
            "__STATUS=$status"
            if ($status -eq "done") {
                $first = @([System.IO.File]::ReadAllLines($done))
                "__DONE=" + $first[0]
                if (Test-Path "$file.out") { Get-Content "$file.out" }
            }
            Remove-Item "$in\$id.*" -Force -ErrorAction SilentlyContinue
        }
    } catch {
        $run = $null
    }
    $lines = @($run.Output | ForEach-Object { "$_" })
    $status = ($lines | Where-Object { $_ -like "__STATUS=*" } | Select-Object -First 1)
    $finished = ($lines | Where-Object { $_ -like "__DONE=*" } | Select-Object -First 1)
    if ($status -eq "__STATUS=cancelled" -or ($finished -and $finished -like "__DONE=notstarted*")) {
        # The script never ran: the old way runs it, once.
        $why = if ($finished) { $finished.Substring(7) } else { "the helper did not take the request" }
        Write-Timing -Kind guest -What "fallback: $why" -Sec $clock.Elapsed.TotalSeconds -Lane agent
        if (-not $finished) { $script:Vm.Agent = $null }
        return $null
    }
    $text = @($lines | Where-Object { $_ -notlike "__STATUS=*" -and $_ -notlike "__DONE=*" })
    $result = ConvertTo-GuestScriptResult -Lines $text -Errors $(if ($run) { $run.Errors } else { "the call to the guest failed" })
    # A helper that lost a request, or had to stop a runspace, is gone or about
    # to be: the next call starts a new one.
    if ($status -eq "__STATUS=lost" -or ($InProcess -and $finished -like "__DONE=timeout")) { $script:Vm.Agent = $null }
    Write-Timing -Kind guest -What $id -Sec $clock.Elapsed.TotalSeconds -Lane $(if ($InProcess) { "inproc" } else { "script" }) -Exit $result.ExitCode
    $result
}

# Run a guest script as a scheduled task of its own: a new conhost and
# powershell.exe each time, with the output polled every half second. This is
# how every script ran before the helper, and what -NoAgent and the fallback do.
function Invoke-GuestScriptByTask {
    param(
        [Parameter(Mandatory)][string]$Script,
        [int]$TimeoutSec = 300,
        [switch]$Background
    )
    $script:Vm.Sequence += 1
    $task = "vmtest-{0:d3}" -f $script:Vm.Sequence
    $callClock = [System.Diagnostics.Stopwatch]::StartNew()
    $file = Join-Path $script:Vm.GuestDir "$task.ps1"
    $body = Get-GuestScriptBody $Script
    $inBackground = [bool]$Background
    # The transport may share a session; a background script runs as a child of
    # it and so needs one of its own, with a fresh environment.
    $run = Invoke-InGuest -Reuse:(-not $inBackground) -TimeoutSec ($TimeoutSec + 60) -Lane none -ArgumentList $file, $body, $TimeoutSec, $inBackground, $task -ScriptBlock {
        param($file, $body, $timeout, $background, $task)
        $ErrorActionPreference = "Continue"
        New-Item -ItemType Directory -Force (Split-Path -Parent $file) | Out-Null
        [System.IO.File]::WriteAllText($file, $body, (New-Object System.Text.UTF8Encoding $true))
        $out = "$file.out"
        Remove-Item $out -Force -ErrorAction SilentlyContinue
        $shell = "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe"
        if ($background) {
            & $shell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File $file | Out-Null
        } else {
            # conhost --headless gives the console program no window, so nothing
            # flashes on the desktop and the program under test keeps the focus.
            $action = New-ScheduledTaskAction -Execute "$env:SystemRoot\System32\conhost.exe" `
                -Argument "--headless `"$shell`" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$file`""
            $principal = New-ScheduledTaskPrincipal -UserId "$env:COMPUTERNAME\$env:USERNAME" -LogonType Interactive -RunLevel Limited
            $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -Priority 4 -ExecutionTimeLimit ([TimeSpan]::Zero)
            Register-ScheduledTask -TaskName $task -Action $action -Principal $principal -Settings $settings -Force | Out-Null
            Start-ScheduledTask -TaskName $task
            $deadline = (Get-Date).AddSeconds($timeout)
            $idle = 0
            while ((Get-Date) -lt $deadline) {
                Start-Sleep -Milliseconds 500
                if ((Test-Path $out) -and (Select-String -Path $out -Pattern '^__EXIT=' -Quiet)) { break }
                # A task that stopped without the last line has crashed or was closed.
                if ((Get-ScheduledTask -TaskName $task).State -eq "Running") { $idle = 0 } else { $idle += 1 }
                if ($idle -ge 20) { break }
            }
            Stop-ScheduledTask -TaskName $task -ErrorAction SilentlyContinue
            Unregister-ScheduledTask -TaskName $task -Confirm:$false -ErrorAction SilentlyContinue
        }
        if (Test-Path $out) { Get-Content $out }
    }
    $result = ConvertTo-GuestScriptResult -Lines $run.Output -Errors $run.Errors
    Write-Timing -Kind guest -What $task -Sec $callClock.Elapsed.TotalSeconds -Lane $(if ($inBackground) { "background" } else { "task" }) -Exit $result.ExitCode
    $result
}
# ------------------------------------------------------------ power

# Wait until the guest user is logged in at the desktop and commands answer.
function Wait-Guest {
    param([int]$TimeoutSec = 900)
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        if ((Get-VmState) -eq "Running") {
            try {
                $user = $script:Vm.User
                $run = Invoke-InGuest -TimeoutSec 60 -Attempts 1 -ArgumentList $user -ScriptBlock {
                    param($user)
                    $shell = Get-CimInstance Win32_Process -Filter "Name='explorer.exe'" |
                        Where-Object { (Invoke-CimMethod -InputObject $_ -MethodName GetOwner).User -eq $user }
                    if ($shell) { "desktop" }
                }
                if ($run.Output -contains "desktop") { return }
            } catch {
                # Windows is still starting.
            }
        }
        Start-Sleep -Seconds 5
    }
    throw "the guest did not reach a logged-in desktop within $TimeoutSec seconds"
}

function Wait-VmState {
    param(
        [Parameter(Mandatory)][string]$State,
        [int]$TimeoutSec = 300
    )
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        if ((Get-VmState) -eq $State) { return $true }
        Start-Sleep -Seconds 2
    }
    $false
}

# Ask Windows not to put the host to sleep for as long as this PowerShell
# process lives. No power setting is changed: the request ends with the
# process, as a video player's does. A pass takes forty minutes with nobody at
# the keyboard; a host that slept half a minute after the guest had booted
# once cost a run.
function Start-KeepAwake {
    if (-not ("VmTest.Power" -as [type])) {
        Add-Type -Namespace VmTest -Name Power -MemberDefinition '[DllImport("kernel32.dll")] public static extern uint SetThreadExecutionState(uint flags);'
    }
    # ES_CONTINUOUS | ES_SYSTEM_REQUIRED
    [void][VmTest.Power]::SetThreadExecutionState([uint32]"0x80000001")
}

# Megabytes of memory the host has free right now.
function Get-HostFreeMemoryMb {
    [int]((Get-CimInstance Win32_OperatingSystem).FreePhysicalMemory / 1024)
}

# Ask Windows to page out what the host's programs are not using right now.
# Nothing is closed, and a program takes its memory back as it needs it.
function Clear-HostMemory {
    if (-not ("VmTest.HostMemory" -as [type])) {
        Add-Type -Namespace VmTest -Name HostMemory -MemberDefinition '[DllImport("psapi.dll")] public static extern bool EmptyWorkingSet(IntPtr process);'
    }
    foreach ($process in Get-Process) {
        try { [void][VmTest.HostMemory]::EmptyWorkingSet($process.Handle) } catch { }
    }
    Start-Sleep -Seconds 3
}

# Hyper-V starts a guest only when all its memory is free on the host at that
# moment. On a host that is in use that is often not so, least of all when
# the guest is started again in the middle of a run: the memory it gave back
# a minute ago has been taken. So free memory first when there is too little,
# and try once more when the start still fails.
function Start-Guest {
    $script:Vm.Devices = $null
    Clear-GuestLinks
    if ((Get-VmState) -eq "Running") { return }
    $needed = [int]((Get-VM -Name $script:Vm.Name).MemoryStartup / 1MB) + 512
    if ((Get-HostFreeMemoryMb) -lt $needed) { Clear-HostMemory }
    try {
        Start-VM -Name $script:Vm.Name -ErrorAction Stop
    } catch {
        Clear-HostMemory
        Start-Sleep -Seconds 5
        if ((Get-VmState) -ne "Running") { Start-VM -Name $script:Vm.Name -ErrorAction Stop }
    }
}

# Ask Windows to shut down; pull the plug if it does not within the timeout.
function Stop-Guest {
    param([int]$TimeoutSec = 180)
    $script:Vm.Devices = $null
    Clear-GuestLinks
    if ((Get-VmState) -eq "Off") { return }
    try {
        Stop-VM -Name $script:Vm.Name -Force -AsJob | Out-Null
    } catch {
        # Falls through to the plug.
    }
    if (-not (Wait-VmState -State "Off" -TimeoutSec $TimeoutSec)) {
        Stop-VM -Name $script:Vm.Name -TurnOff -Force
        Wait-VmState -State "Off" -TimeoutSec 60 | Out-Null
    }
}

# Shut Windows down and start it again.
function Restart-Guest {
    Stop-Guest
    Start-Guest
}

# Put the VM back to the named checkpoint. A checkpoint taken while the VM
# was off leaves it off; Start-Guest then boots it.
function Restore-Guest {
    param([string]$Snapshot = "clean")
    $script:Vm.Devices = $null
    Clear-GuestLinks
    if ((Get-VmState) -ne "Off") {
        Stop-VM -Name $script:Vm.Name -TurnOff -Force
        Wait-VmState -State "Off" -TimeoutSec 60 | Out-Null
    }
    Restore-VMCheckpoint -VMName $script:Vm.Name -Name $Snapshot -Confirm:$false
}

# How busy the guest's processors are, in percent of all of them, averaged
# over the given time. Measured by the host, so it works when the guest is too
# busy to answer a command, which is exactly when the number matters.
function Get-GuestProcessorLoad {
    param([int]$Seconds = 10)
    $samples = Get-Counter '\Hyper-V Hypervisor Virtual Processor(*)\% Guest Run Time' -SampleInterval 1 -MaxSamples $Seconds
    $prefix = $script:Vm.Name.ToLowerInvariant() + ":"
    $values = @($samples | ForEach-Object { $_.CounterSamples } |
            Where-Object { $_.InstanceName.StartsWith($prefix) } | ForEach-Object { $_.CookedValue })
    if (-not $values.Count) { throw "the host has no processor counters for '$($script:Vm.Name)'; is it running?" }
    [math]::Round(($values | Measure-Object -Average).Average, 1)
}

# Wait until the guest's processors are quiet and return their load. Right
# after a start or an install Windows is busy by itself for a minute or two,
# and a measurement taken then says nothing about the program under test.
function Wait-GuestIdle {
    param([int]$BelowPercent = 20, [int]$TimeoutSec = 180)
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    do { $load = Get-GuestProcessorLoad -Seconds 5 } while ($load -ge $BelowPercent -and (Get-Date) -lt $deadline)
    $load
}
# ------------------------------------------------------------ screen, keys, mouse

function Get-VmDevices {
    if ($script:Vm.Devices) { return $script:Vm.Devices }
    $namespace = $script:VirtualizationNamespace
    $name = $script:Vm.Name -replace "'", "''"
    $system = Get-CimInstance -Namespace $namespace -ClassName Msvm_ComputerSystem -Filter "ElementName='$name'"
    if (-not $system) { throw "no virtual machine named '$($script:Vm.Name)'" }
    $script:Vm.Devices = @{
        System   = $system
        Settings = Get-CimAssociatedInstance -InputObject $system -ResultClassName Msvm_VirtualSystemSettingData -Association Msvm_SettingsDefineState
        Service  = Get-CimInstance -Namespace $namespace -ClassName Msvm_VirtualSystemManagementService
        Keyboard = Get-CimAssociatedInstance -InputObject $system -ResultClassName Msvm_Keyboard
        Mouse    = Get-CimAssociatedInstance -InputObject $system -ResultClassName Msvm_SyntheticMouse | Select-Object -First 1
    }
    $script:Vm.Devices
}

# Width and height of the guest's screen in pixels; null while the VM is off.
function Get-GuestScreenSize {
    $devices = Get-VmDevices
    $head = Get-CimInstance -Namespace $script:VirtualizationNamespace -ClassName Msvm_VideoHead -Filter "SystemName='$($devices.System.Name)'" |
        Select-Object -First 1
    if (-not $head -or -not $head.CurrentHorizontalResolution) { return $null }
    [pscustomobject]@{ Width = [int]$head.CurrentHorizontalResolution; Height = [int]$head.CurrentVerticalResolution }
}

# The guest's screen as raw pixels, two bytes each (RGB565), scaled to the
# given size. Null when the VM shows nothing.
function Get-GuestScreenBytes {
    param([Parameter(Mandatory)][int]$Width, [Parameter(Mandatory)][int]$Height)
    $devices = Get-VmDevices
    $answer = Invoke-CimMethod -InputObject $devices.Service -MethodName GetVirtualSystemThumbnailImage -Arguments @{
        TargetSystem = $devices.Settings
        WidthPixels  = [uint16]$Width
        HeightPixels = [uint16]$Height
    }
    if ($answer.ReturnValue -ne 0 -or -not $answer.ImageData) { return $null }
    $bytes = [byte[]]$answer.ImageData
    # The WMI layer puts the array's length in front of the pixels.
    $pixels = $Width * $Height * 2
    if ($bytes.Length -eq $pixels + 4) {
        $picture = [byte[]]::new($pixels)
        [Array]::Copy($bytes, 4, $picture, 0, $pixels)
        $bytes = $picture
    }
    , $bytes
}

# ------------------------------------------------------------ waiting on the screen
# What a program that has no log line to wait for shows on the screen is read
# from the host's thumbnail (480x270, as the film is), compared in C# that is
# compiled once. A region is given in thumbnail pixels: left, top, right,
# bottom. The recorder reads the same WMI service; the film must still hold
# about four frames a second after these run (check with ffprobe).
function Initialize-ScreenCompare {
    if ('VmTest.ScreenDiff' -as [type]) { return }
    Add-Type -Namespace VmTest -Name ScreenDiff -MemberDefinition @'
public static double Share(byte[] a, byte[] b, int width, int left, int top, int right, int bottom, int threshold) {
    long differing = 0;
    long area = (long)(right - left) * (bottom - top);
    if (area <= 0 || a == null || b == null || a.Length != b.Length) { return 1.0; }
    for (int y = top; y < bottom; y++) {
        for (int x = left; x < right; x++) {
            int i = (y * width + x) * 2;
            int p = a[i] | (a[i + 1] << 8);
            int q = b[i] | (b[i + 1] << 8);
            if (p == q) { continue; }
            int dr = Math.Abs(((p >> 11) & 31) - ((q >> 11) & 31)) << 3;
            int dg = Math.Abs(((p >> 5) & 63) - ((q >> 5) & 63)) << 2;
            int db = Math.Abs((p & 31) - (q & 31)) << 3;
            if (Math.Max(dr, Math.Max(dg, db)) > threshold) { differing++; }
        }
    }
    return (double)differing / area;
}
'@
}

$script:ThumbnailWidth = 480
$script:ThumbnailHeight = 270

# The screen as a small picture to compare; null when the VM shows nothing.
function Get-ScreenFrame {
    Get-GuestScreenBytes -Width $script:ThumbnailWidth -Height $script:ThumbnailHeight
}

# The share (0 to 1) of a region's pixels that differ clearly between two frames.
function Get-ScreenShare {
    param($A, $B, [int[]]$Region = @(0, 0, 480, 270))
    Initialize-ScreenCompare
    if (-not $A -or -not $B) { return 1.0 }
    [VmTest.ScreenDiff]::Share([byte[]]$A, [byte[]]$B, $script:ThumbnailWidth, $Region[0], $Region[1], $Region[2], $Region[3], 24)
}

# Share of a region that may differ between two frames and still count as the
# same picture. With a graphics card the backdrop drifts in a few percent of
# the pixels all the time (Vm.Gpu, set by Initialize-Vm's caller).
function Get-ScreenTolerance {
    param([double]$Tolerance = -1)
    if ($Tolerance -ge 0) { return $Tolerance }
    if ($script:Vm.Gpu) { 0.05 } else { 0.005 }
}

# Wait until the screen (or a region of it) has stood still for -QuietMs, and
# at least -MinSec has passed. $false at the deadline: the caller goes on, as it
# did after the fixed sleep this replaces.
function Wait-ScreenStill {
    param(
        [Parameter(Mandatory)][double]$TimeoutSec,
        [int]$QuietMs = 400,
        [double]$MinSec = 0,
        [int[]]$Region = @(0, 0, 480, 270),
        [double]$Tolerance = -1,
        [string]$What = "screen still",
        [switch]$PassThru
    )
    $tolerance = Get-ScreenTolerance $Tolerance
    $limit = $TimeoutSec * $script:Timing.Pace
    $floor = $MinSec * $script:Timing.Pace
    $clock = [System.Diagnostics.Stopwatch]::StartNew()
    $previous = Get-ScreenFrame
    $lastChange = 0.0
    $met = $false
    while ($true) {
        Start-Sleep -Milliseconds 150
        $frame = Get-ScreenFrame
        $now = $clock.Elapsed.TotalSeconds
        if ((Get-ScreenShare $previous $frame $Region) -gt $tolerance) { $lastChange = $now }
        $previous = $frame
        if ($now -ge $floor -and ($now - $lastChange) * 1000 -ge $QuietMs) { $met = $true; break }
        if ($now -ge $limit) { break }
    }
    Write-Timing -Kind wait -What $What -Sec $clock.Elapsed.TotalSeconds -Deadline $limit -Met $met
    if ($PassThru) { $met }
}

# Wait until the screen (or a region) differs from -Since, a frame taken before
# the input. $false at the deadline.
function Wait-ScreenChange {
    param(
        [Parameter(Mandatory)][double]$TimeoutSec,
        $Since,
        [int[]]$Region = @(0, 0, 480, 270),
        [double]$Tolerance = -1,
        [string]$What = "screen change",
        [switch]$PassThru
    )
    $tolerance = Get-ScreenTolerance $Tolerance
    $limit = $TimeoutSec * $script:Timing.Pace
    $clock = [System.Diagnostics.Stopwatch]::StartNew()
    if (-not $Since) { $Since = Get-ScreenFrame }
    $met = $false
    while ($true) {
        if ((Get-ScreenShare $Since (Get-ScreenFrame) $Region) -gt $tolerance) { $met = $true; break }
        if ($clock.Elapsed.TotalSeconds -ge $limit) { break }
        Start-Sleep -Milliseconds 150
    }
    Write-Timing -Kind wait -What $What -Sec $clock.Elapsed.TotalSeconds -Deadline $limit -Met $met
    if ($PassThru) { $met }
}

# Save the guest's screen as a PNG at its own size. Colors come from the
# host as 16 bits per pixel, so smooth gradients show faint bands.
function Save-Screenshot {
    param([Parameter(Mandatory)][string]$Path)
    New-Item -ItemType Directory -Force (Split-Path -Parent $Path) | Out-Null
    $size = Get-GuestScreenSize
    if (-not $size) { throw "the guest shows no screen (state $(Get-VmState))" }
    $bytes = Get-GuestScreenBytes -Width $size.Width -Height $size.Height
    if (-not $bytes) { throw "the host returned no picture of the guest's screen" }
    Add-Type -AssemblyName System.Drawing
    $bitmap = [System.Drawing.Bitmap]::new($size.Width, $size.Height, [System.Drawing.Imaging.PixelFormat]::Format16bppRgb565)
    try {
        $area = [System.Drawing.Rectangle]::new(0, 0, $size.Width, $size.Height)
        $data = $bitmap.LockBits($area, [System.Drawing.Imaging.ImageLockMode]::WriteOnly, $bitmap.PixelFormat)
        $rowBytes = $size.Width * 2
        foreach ($row in 0..($size.Height - 1)) {
            [System.Runtime.InteropServices.Marshal]::Copy($bytes, $row * $rowBytes, [IntPtr]::Add($data.Scan0, $row * $data.Stride), $rowBytes)
        }
        $bitmap.UnlockBits($data)
        $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $bitmap.Dispose()
    }
    $Path
}

function Invoke-Device {
    param([string]$Device, [string]$Method, [hashtable]$Arguments = @{})
    $target = (Get-VmDevices)[$Device]
    if (-not $target) { throw "the VM has no $Device device; is it running?" }
    $answer = Invoke-CimMethod -InputObject $target -MethodName $Method -Arguments $Arguments
    if ($answer.ReturnValue -ne 0) { throw "$Device.$Method failed with $($answer.ReturnValue)" }
}

# Windows virtual-key codes. Letters and digits are their own character codes.
$script:KeyCodes = @{
    "backspace" = 0x08; "tab" = 0x09; "enter" = 0x0D; "shift" = 0x10; "ctrl" = 0x11; "alt" = 0x12
    "esc" = 0x1B; "space" = 0x20; "pgup" = 0x21; "pgdn" = 0x22; "end" = 0x23; "home" = 0x24
    "left" = 0x25; "up" = 0x26; "right" = 0x27; "down" = 0x28; "insert" = 0x2D; "delete" = 0x2E
    "win" = 0x5B; "f1" = 0x70; "f2" = 0x71; "f3" = 0x72; "f4" = 0x73; "f5" = 0x74; "f6" = 0x75
    "f7" = 0x76; "f8" = 0x77; "f9" = 0x78; "f10" = 0x79; "f11" = 0x7A; "f12" = 0x7B
    "equals" = 0xBB; "minus" = 0xBD
}

# Press keys on the guest's keyboard. Each argument is one key or a chord
# joined with "+": Send-Keys "ctrl+k", "down", "enter". A chord holds its keys
# down in order and releases them in reverse, as a hand does.
function Send-Keys {
    param(
        [Parameter(Mandatory, Position = 0)][string[]]$Keys,
        [int]$DelayMs = 150
    )
    foreach ($chord in $Keys) {
        $codes = foreach ($name in ($chord.ToLowerInvariant() -split "\+")) {
            if ($script:KeyCodes.ContainsKey($name)) { [uint32]$script:KeyCodes[$name] }
            elseif ($name -match '^[a-z0-9]$') { [uint32][char]$name.ToUpperInvariant() }
            else { throw "unknown key '$name'" }
        }
        $codes = @($codes)
        foreach ($code in $codes) { Invoke-Device Keyboard PressKey @{ KeyCode = $code }; Start-Sleep -Milliseconds 30 }
        [array]::Reverse($codes)
        foreach ($code in $codes) { Invoke-Device Keyboard ReleaseKey @{ KeyCode = $code }; Start-Sleep -Milliseconds 30 }
        Start-Sleep -Milliseconds $DelayMs
    }
}

# Type plain ASCII text on the guest's keyboard, one character at a time.
# Handed over as one string, or faster than this, the guest loses characters.
function Send-Text {
    param(
        [Parameter(Mandatory)][string]$Text,
        [int]$DelayMs = 60
    )
    foreach ($character in $Text.ToCharArray()) {
        Invoke-Device Keyboard TypeText @{ AsciiText = [string]$character }
        Start-Sleep -Milliseconds $DelayMs
    }
    Start-Sleep -Milliseconds 300
}

# Move the guest's pointer to a point of its screen, in pixels from the top left.
function Move-Pointer {
    param([Parameter(Mandatory)][int]$X, [Parameter(Mandatory)][int]$Y)
    Invoke-Device Mouse SetAbsolutePosition @{ HorizontalPosition = [int32]$X; VerticalPosition = [int32]$Y }
}

# Click at a point of the guest's screen. Read the point off a screenshot.
function Send-Click {
    param(
        [Parameter(Mandatory)][int]$X,
        [Parameter(Mandatory)][int]$Y,
        [ValidateSet("left", "right", "middle")][string]$Button = "left",
        [switch]$Double,
        # Keys held down for the click, as in a Ctrl+click: "ctrl", "shift".
        [string[]]$Hold = @()
    )
    $index = @{ left = 1; right = 2; middle = 3 }[$Button]
    Move-Pointer -X $X -Y $Y
    Start-Sleep -Milliseconds 150
    foreach ($key in $Hold) { Invoke-Device Keyboard PressKey @{ KeyCode = [uint32]$script:KeyCodes[$key] }; Start-Sleep -Milliseconds 50 }
    try {
        Invoke-Device Mouse ClickButton @{ ButtonIndex = [uint32]$index }
        if ($Double) {
            Start-Sleep -Milliseconds 80
            Invoke-Device Mouse ClickButton @{ ButtonIndex = [uint32]$index }
        }
    } finally {
        foreach ($key in $Hold) { Start-Sleep -Milliseconds 50; Invoke-Device Keyboard ReleaseKey @{ KeyCode = [uint32]$script:KeyCodes[$key] } }
    }
    Start-Sleep -Milliseconds 300
}

# Hold the left mouse button down, or let it go, where the pointer is: to see
# what a control looks like while it is pressed.
function Set-MouseButton {
    param([Parameter(Mandatory)][bool]$Down)
    Invoke-Device Mouse SetButtonState @{ ButtonIndex = [uint32]1; IsDown = $Down }
}

# Turn the mouse wheel; a positive number scrolls up.
function Send-Scroll {
    param([Parameter(Mandatory)][int]$Notches)
    Invoke-Device Mouse SetScrollPosition @{ ScrollPositionDelta = [int32]($Notches * 120) }
    Start-Sleep -Milliseconds 300
}

# ------------------------------------------------------- notification area
# Windows names each button of the notification area (the tray) by its
# tooltip, and each row of a menu by its text, and says where each is on the
# screen. That is read in the guest through UI Automation; the pointer and
# the keys still come from the VM's devices, as a user's would.

# Runs in the guest: every element under the windows of the given classes
# whose name is like $like, one line each.
$script:GuestElementsScript = @'
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
Add-Type -Namespace VmTest -Name Dpi -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
# Without this, a scaled screen gives scaled numbers, and clicks miss.
[void][VmTest.Dpi]::SetProcessDPIAware()
$automation = [System.Windows.Automation.AutomationElement]
$scope = [System.Windows.Automation.TreeScope]
foreach ($class in $classes) {
    # "*" asks every top-level window, for a window whose class is not known.
    $ofClass = if ($class -eq "*") { [System.Windows.Automation.Condition]::TrueCondition } else { New-Object System.Windows.Automation.PropertyCondition($automation::ClassNameProperty, $class) }
    foreach ($window in $automation::RootElement.FindAll($scope::Children, $ofClass)) {
        $label = if ($class -eq "*") { $window.Current.ClassName -replace ';', ' ' } else { $class }
        foreach ($element in $window.FindAll($scope::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) {
            $current = $element.Current
            if ($current.Name -notlike $like) { continue }
            $box = $current.BoundingRectangle
            $place = if ($box.IsEmpty) { "0,0,0,0" } else { "$([int]$box.X),$([int]$box.Y),$([int]$box.Width),$([int]$box.Height)" }
            $kind = $current.ControlType.ProgrammaticName -replace "^ControlType\.", ""
            "element=$label;$kind;$place;$([int]$current.IsOffscreen);$([int]$current.IsEnabled);$($current.ClassName -replace ';', ' ');$($current.Name -replace '\s+', ' ')"
        }
    }
}
'@

# The elements of the guest's screen, under top-level windows of the given
# window classes ("*" for every top-level window), whose name is like -Like. Each has Class (of the window),
# Kind, UiClass (of the element itself), Name, its place (Left, Top, Width,
# Height), the point in its middle (X, Y), and whether it is on screen
# (OnScreen) and can be chosen (Enabled).
function Get-GuestElements {
    param(
        [Parameter(Mandatory)][string[]]$Class,
        [string]$Like = "*"
    )
    $classes = ($Class | ForEach-Object { "'$_'" }) -join ", "
    $run = Invoke-GuestScript -TimeoutSec 90 -Script "`$classes = @($classes)`n`$like = '$($Like -replace "'", "''")'`n$script:GuestElementsScript"
    @($run.Output -split "`r?`n" | ForEach-Object {
            if ($_ -match '^element=([^;]*);([^;]*);(-?\d+),(-?\d+),(-?\d+),(-?\d+);(\d);(\d);([^;]*);(.*)$') {
                [pscustomobject]@{
                    Class = $Matches[1]; Kind = $Matches[2]; UiClass = $Matches[9]; Name = $Matches[10].Trim()
                    Left = [int]$Matches[3]; Top = [int]$Matches[4]; Width = [int]$Matches[5]; Height = [int]$Matches[6]
                    X = [int]$Matches[3] + [int]([int]$Matches[5] / 2); Y = [int]$Matches[4] + [int]([int]$Matches[6] / 2)
                    OnScreen = $Matches[7] -eq "0" -and [int]$Matches[5] -gt 0
                    Enabled = $Matches[8] -eq "1"
                }
            }
        })
}

# The taskbar's notification area, and the flyout that holds the icons Windows
# does not show on the taskbar. A new icon starts in the flyout.
$script:TrayClasses = "Shell_TrayWnd", "TopLevelWindowForOverflowXamlIsland"

# The taskbar's arrow that opens and closes the flyout ("Show Hidden Icons";
# while the flyout is open its name also holds "Hide"). Asked for up to three
# times: while Windows draws a notification the taskbar can be missing from
# what UI Automation lists.
function Get-GuestTrayArrow {
    foreach ($try in 1..3) {
        $arrow = Get-GuestElements -Class "Shell_TrayWnd" -Like "*Hidden Icons*" | Where-Object { $_.OnScreen } | Select-Object -First 1
        if ($arrow) { return $arrow }
        Start-Sleep -Seconds 2
    }
}

# Open the flyout of hidden icons when it is closed. Says whether it had to.
function Open-GuestTrayFlyout {
    $arrow = Get-GuestTrayArrow
    if (-not $arrow -or $arrow.Name -match "Hide") { return $false }
    Send-Click -X $arrow.X -Y $arrow.Y
    Start-Sleep -Milliseconds 1500
    $true
}

# Close the flyout of hidden icons when it is open.
function Close-GuestTrayFlyout {
    $arrow = Get-GuestTrayArrow
    if ($arrow -and $arrow.Name -match "Hide") {
        Send-Click -X $arrow.X -Y $arrow.Y
        Start-Sleep -Milliseconds 800
    }
}

# The tray icons whose tooltip is like -Like, on the taskbar and in the
# flyout. The flyout's window exists only while it is open, and a new icon
# starts in it: when the taskbar shows no such icon the flyout is opened to
# look, and closed again unless -KeepOpen. An icon's place is good for a click
# only while what holds it is open.
function Get-GuestTrayIcons {
    param([string]$Like = "*CodeWiki*", [switch]$KeepOpen)
    # The taskbar also has a button for each running program, named after it
    # ("codewiki-app - 1 running window"); that one is not a tray icon.
    $isIcon = { $_.Kind -eq "Button" -and $_.UiClass -notlike "Taskbar.*" }
    $icons = @(Get-GuestElements -Class $script:TrayClasses -Like $Like | Where-Object $isIcon)
    if ($icons) { return $icons }
    $opened = Open-GuestTrayFlyout
    $icons = @(Get-GuestElements -Class $script:TrayClasses -Like $Like | Where-Object $isIcon)
    if ($opened -and -not $KeepOpen) { Close-GuestTrayFlyout }
    $icons
}

# One tray icon that can be clicked: on the taskbar, or in the flyout, which
# is opened for it and left open. Null when there is none.
function Get-GuestTrayIcon {
    param([string]$Like = "*CodeWiki*")
    Get-GuestTrayIcons $Like -KeepOpen | Where-Object { $_.OnScreen } | Select-Object -First 1
}

# Click a tray icon, as a user does: the left button for what the icon does,
# the right one for its menu. Returns the icon, with where it was clicked.
function Send-GuestTrayClick {
    param(
        [string]$Like = "*CodeWiki*",
        [ValidateSet("left", "right")][string]$Button = "left"
    )
    $icon = Get-GuestTrayIcon $Like
    if (-not $icon) { throw "no tray icon named like '$Like' can be reached" }
    Send-Click -X $icon.X -Y $icon.Y -Button $Button
    Start-Sleep -Milliseconds 1500
    $icon
}

# A tray icon's menu cannot be read this way: it closes as soon as another
# program starts, and every question to the guest starts one. Take a picture
# of it instead (the host does that without the guest), and choose a row with
# the keyboard: Up from a newly opened menu is its last row.

# ------------------------------------------------------------ Windows Update
# Windows version of the running guest as "build.revision", for example 26200.6584.
function Get-GuestBuild {
    $run = Invoke-InGuest -TimeoutSec 120 -ScriptBlock {
        $current = Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion"
        "$($current.CurrentBuild).$($current.UBR)"
    }
    "$($run.Output | Select-Object -Last 1)"
}

# Install the updates whose title matches the pattern in the running guest,
# through Windows Update, and return the log. The guest needs internet. The
# work is done by guest\update-windows.ps1 as a SYSTEM task; restart the
# guest afterwards. Takes from ten minutes to over an hour.
function Update-GuestWindows {
    param(
        # The monthly Windows update: "2026-11 Security Update (KB...)", earlier
    # "... Cumulative Update for Windows 11 ...". Not the .NET or Defender ones.
    [string]$Pattern = '^\d{4}-\d{2} (Security|Cumulative) Update',
        [int]$TimeoutMin = 120
    )
    $script = Get-Content (Join-Path $PSScriptRoot "guest\update-windows.ps1") -Raw
    $log = Join-Path $script:Vm.GuestDir "update.log"
    $start = Invoke-InGuest -TimeoutSec 120 -ArgumentList $script, $script:Vm.GuestDir, $Pattern, $log -ScriptBlock {
        param($script, $directory, $pattern, $log)
        New-Item -ItemType Directory -Force $directory | Out-Null
        $file = Join-Path $directory "update-windows.ps1"
        [System.IO.File]::WriteAllText($file, $script, (New-Object System.Text.UTF8Encoding $true))
        Remove-Item $log -Force -ErrorAction SilentlyContinue
        $action = New-ScheduledTaskAction -Execute "$env:SystemRoot\System32\WindowsPowerShell\v1.0\powershell.exe" `
            -Argument "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$file`" -Pattern `"$pattern`" -Log `"$log`""
        $principal = New-ScheduledTaskPrincipal -UserId "SYSTEM" -LogonType ServiceAccount -RunLevel Highest
        $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero)
        Register-ScheduledTask -TaskName "vmtest-update" -Action $action -Principal $principal -Settings $settings -Force | Out-Null
        Start-ScheduledTask -TaskName "vmtest-update"
        "started"
    }
    if ($start.Output -notcontains "started") { throw "could not start the update in the guest: $($start.Errors -join '; ')" }
    $deadline = (Get-Date).AddMinutes($TimeoutMin)
    $lines = @()
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Seconds 30
        try {
            $read = Invoke-InGuest -TimeoutSec 60 -Attempts 1 -ArgumentList $log -ScriptBlock { param($log) if (Test-Path $log) { Get-Content $log } }
            $lines = @($read.Output | ForEach-Object { "$_" })
        } catch {
            # The guest is busy installing; ask again.
        }
        if ($lines | Where-Object { $_ -match '^\S+ (DONE|FAILED)' }) { break }
    }
    Invoke-InGuest -TimeoutSec 60 -ScriptBlock { Unregister-ScheduledTask -TaskName "vmtest-update" -Confirm:$false -ErrorAction SilentlyContinue } | Out-Null
    $lines
}
# ------------------------------------------------------------ graphics card
# A guest normally has the "Microsoft Hyper-V Video" adapter only and draws
# everything on the processor. These helpers give it a share of the host's
# graphics card (GPU paravirtualization, the mechanism Windows Sandbox and
# WSL use). Microsoft documents GPU partitioning for Windows Server hosts and
# data-center cards; on a Windows 11 host with a desktop card it works when
# Get-VMHostPartitionableGpu lists the card, but it is not a supported setup.
#   https://learn.microsoft.com/windows-server/virtualization/hyper-v/gpu-partitioning
#   https://learn.microsoft.com/windows-hardware/drivers/display/gpu-paravirtualization

# Folder in the host's driver store that holds the driver of the first
# graphics card Hyper-V can share.
function Get-HostGpuDriverFolder {
    $gpu = Get-VMHostPartitionableGpu | Select-Object -First 1
    if (-not $gpu) { throw "this host has no graphics card that Hyper-V can share (Get-VMHostPartitionableGpu is empty)" }
    # \\?\PCI#VEN_...#4&22f77229&0&0008#{guid}\GPUPARAV  ->  PCI\VEN_...\4&22f77229&0&0008
    if ($gpu.Name -notmatch '^\\\\\?\\(.+?)#\{') { throw "cannot read a device id from '$($gpu.Name)'" }
    $device = $Matches[1] -replace '#', '\'
    $service = (Get-PnpDeviceProperty -InstanceId $device -KeyName DEVPKEY_Device_Service).Data
    $image = (Get-ItemProperty "HKLM:\SYSTEM\CurrentControlSet\Services\$service").ImagePath
    if ($image -notmatch '(?i)DriverStore\\FileRepository\\([^\\]+)\\') { throw "the driver of $device is not in the driver store: $image" }
    Join-Path $env:SystemRoot "System32\DriverStore\FileRepository\$($Matches[1])"
}

# Copy the host's graphics driver into the running guest. The guest loads its
# user-mode half from System32\HostDriverStore; Windows does not copy it there
# by itself. About 2 GB for an NVIDIA driver. Repeat after a driver update on
# the host: guest and host must hold the same version.
function Copy-GpuDriverToGuest {
    $folder = Get-HostGpuDriverFolder
    $name = Split-Path -Leaf $folder
    $package = Join-Path ([System.IO.Path]::GetTempPath()) "vmtest-gpu-$PID.zip"
    Remove-Item $package -Force -ErrorAction SilentlyContinue
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [System.IO.Compression.ZipFile]::Open($package, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        # Without the Linux libraries the driver carries for WSL.
        foreach ($file in Get-ChildItem $folder -Recurse -File | Where-Object { $_.Name -notmatch '\.so(\.|$)' }) {
            $entry = $file.FullName.Substring($folder.Length + 1) -replace '\\', '/'
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, $file.FullName, $entry, [System.IO.Compression.CompressionLevel]::Fastest) | Out-Null
        }
    } finally {
        $archive.Dispose()
    }
    $session = New-GuestSession
    try {
        $guestPackage = "C:\vmtest-gpu.zip"
        Copy-Item -ToSession $session -Path $package -Destination $guestPackage -Force
        # The PowerShell Direct session is elevated, which System32 needs.
        Invoke-Command -Session $session -ArgumentList $guestPackage, $name -ScriptBlock {
            param($package, $name)
            $target = Join-Path $env:SystemRoot "System32\HostDriverStore\FileRepository\$name"
            if (Test-Path $target) { Remove-Item $target -Recurse -Force }
            New-Item -ItemType Directory -Force $target | Out-Null
            Add-Type -AssemblyName System.IO.Compression.FileSystem
            [System.IO.Compression.ZipFile]::ExtractToDirectory($package, $target)
            Remove-Item $package -Force
        }
    } finally {
        Remove-PSSession $session -ErrorAction SilentlyContinue
        Remove-Item $package -Force -ErrorAction SilentlyContinue
    }
    $name
}

# Attach a share of the host's graphics card to the VM, which must be off.
# The settings are those of Microsoft's GPU paravirtualization sample. A
# checkpoint does not keep the adapter, so call this after every Restore-Guest.
function Enable-GuestGpu {
    if ((Get-VmState) -ne "Off") { throw "turn the VM off before attaching the graphics card" }
    if (-not (Get-VMGpuPartitionAdapter -VMName $script:Vm.Name -ErrorAction SilentlyContinue)) {
        Add-VMGpuPartitionAdapter -VMName $script:Vm.Name
    }
    Set-VM -Name $script:Vm.Name -GuestControlledCacheTypes $true -LowMemoryMappedIoSpace 1GB -HighMemoryMappedIoSpace 16GB
}

function Disable-GuestGpu {
    if ((Get-VmState) -ne "Off") { throw "turn the VM off before removing the graphics card" }
    Get-VMGpuPartitionAdapter -VMName $script:Vm.Name -ErrorAction SilentlyContinue | Remove-VMGpuPartitionAdapter
}

# Whether the running guest holds the same graphics driver as the host. The
# driver's folder name changes with every driver version, so after a driver
# update on the host the guest's copy is stale and the card may not start
# there. provision-gpu.ps1 -Force copies the new one and retakes the checkpoint.
function Test-GuestGpuDriver {
    $expected = Split-Path -Leaf (Get-HostGpuDriverFolder)
    $run = Invoke-InGuest -TimeoutSec 120 -ScriptBlock {
        Get-ChildItem "$env:SystemRoot\System32\HostDriverStore\FileRepository" -Directory -ErrorAction SilentlyContinue | ForEach-Object { $_.Name }
    }
    $found = @($run.Output | ForEach-Object { "$_" })
    [pscustomobject]@{ Match = $found -contains $expected; HostDriver = $expected; GuestDrivers = $found }
}
# Whether the logged-in desktop has a screen. With a graphics card attached,
# Windows 11 builds 26100.4484 to 26100.7170 (and the same revisions of
# 26200) lose it: the session's display device is then "WinDisc", the host's
# screenshots stay on the boot logo, and nothing is drawn at all, which makes
# an idle-load measurement look perfect. Microsoft fixed it in KB5068861
# (revision 7171); Update-GuestWindows brings a guest past it.
function Test-GuestDisplay {
    $run = Invoke-GuestScript -TimeoutSec 120 -Script 'Add-Type -AssemblyName System.Windows.Forms; "device=" + [System.Windows.Forms.Screen]::PrimaryScreen.DeviceName'
    $run.Output -match 'device=\\\\\.\\DISPLAY'
}
# Display adapters as the guest sees them: name, driver version, and whether
# Windows started the device (problem code 0).
function Get-GuestDisplayAdapters {
    $run = Invoke-InGuest -TimeoutSec 120 -ScriptBlock {
        Get-CimInstance Win32_VideoController | ForEach-Object { "$($_.Name) | driver $($_.DriverVersion) | problem code $($_.ConfigManagerErrorCode)" }
    }
    $run.Output | ForEach-Object { "$_" }
}
# ------------------------------------------------------------ recording

function Find-Ffmpeg {
    $found = Get-Command ffmpeg -ErrorAction SilentlyContinue
    if ($found) { $found.Source } else { $null }
}

# Record the guest's screen to an MP4 file until Stop-Recording. Hyper-V has
# no recorder, so record.ps1 takes screenshots and hands them to ffmpeg.
# Returns false, and records nothing, when ffmpeg is not on PATH.
function Start-Recording {
    param(
        [Parameter(Mandatory)][string]$Path,
        [int]$Fps = 4,
        # Width of the film in pixels. A full-size picture takes the host over
        # half a second to hand over, one of 1280 pixels a fifth of a second.
        [int]$Width = 1280
    )
    $ffmpeg = Find-Ffmpeg
    if (-not $ffmpeg) {
        Write-Warning "ffmpeg is not on PATH; the screen is not recorded"
        return $false
    }
    New-Item -ItemType Directory -Force (Split-Path -Parent $Path) | Out-Null
    $stop = "$Path.stop"
    Remove-Item $stop, $Path -Force -ErrorAction SilentlyContinue
    $arguments = "-NoProfile", "-File", (Join-Path $PSScriptRoot "record.ps1"),
        "-VmName", $script:Vm.Name, "-Path", $Path, "-StopFile", $stop, "-Fps", $Fps, "-Width", $Width, "-Ffmpeg", $ffmpeg
    $process = Start-Process -FilePath (Get-Process -Id $PID).Path -ArgumentList ($arguments | ForEach-Object { if ("$_" -match '\s') { "`"$_`"" } else { "$_" } }) `
        -WindowStyle Hidden -PassThru
    $script:Vm.Recorder = @{ Process = $process; Stop = $stop }
    $script:Vm.RecStart = [DateTime]::UtcNow
    $true
}

function Stop-Recording {
    $recorder = $script:Vm.Recorder
    if (-not $recorder) { return }
    Set-Content -Path $recorder.Stop -Value "stop"
    if (-not $recorder.Process.WaitForExit(60000)) { $recorder.Process.Kill() }
    Remove-Item $recorder.Stop -Force -ErrorAction SilentlyContinue
    $script:Vm.Recorder = $null
}
