# Put the computer into the power state the user chose for the end of a run:
#   pwsh .claude\skills\ship-release\scripts\power.ps1 -Action Shutdown
#   pwsh .claude\skills\ship-release\scripts\power.ps1 -Action Sleep -DryRun
#
# Run it only when the user asked for that action in the current session, and
# as the last thing the session does. It waits -DelaySeconds first, so that
# somebody at the machine can stop it: "shutdown /a" stops a shutdown, ending
# this script stops a sleep or a hibernation. The wait is inside the call, so
# give the call a timeout well above it (300 seconds for the default delay).
param(
    [Parameter(Mandatory)][ValidateSet("Shutdown", "Sleep", "Hibernate")][string]$Action,
    # Never less than half a minute: the wait is the only way to stop it.
    [ValidateRange(30, 3600)][int]$DelaySeconds = 120,
    # Say what would happen and change nothing.
    [switch]$DryRun
)
$ErrorActionPreference = "Stop"

# A guest that runs through a shutdown loses its work, and one that runs into
# a sleep is paused in the middle of a test. Starting, saving and paused
# guests count too: only "Off" is safe.
function Assert-NoGuest {
    if (-not (Get-Command Get-VM -ErrorAction SilentlyContinue)) { return }
    $busy = @(Get-VM | Where-Object { $_.State -ne "Off" })
    if ($busy) { throw "a Hyper-V guest is not off ($(($busy | ForEach-Object { "$($_.Name): $($_.State)" }) -join ', ')); stop it first" }
}

# Tell whoever sits at the machine. Windows shows its own notice for a
# shutdown; for the other two nothing would be on the screen otherwise.
function Send-Notice {
    param([string]$Text)
    $msg = Join-Path $env:SystemRoot "System32\msg.exe"
    if (Test-Path $msg) { & $msg * /TIME:$DelaySeconds $Text 2>$null }
}

Assert-NoGuest

if ($Action -eq "Hibernate") {
    $power = Get-ItemProperty "HKLM:\SYSTEM\CurrentControlSet\Control\Power" -ErrorAction SilentlyContinue
    # Without a value of its own the setting follows its default.
    $enabled = if ($null -ne $power.HibernateEnabled) { $power.HibernateEnabled } else { $power.HibernateEnabledDefault }
    if ($enabled -ne 1) {
        throw "hibernation is turned off on this machine ('powercfg /hibernate on' from an elevated prompt turns it on); choose Sleep or Shutdown"
    }
}

if ($DryRun) {
    "dry run: would $($Action.ToLowerInvariant()) after $DelaySeconds seconds; nothing was changed"
    return
}

switch ($Action) {
    "Shutdown" {
        # Windows counts down itself and shows the notice. With a delay it
        # closes every open program without asking: unsaved work is lost.
        & shutdown.exe /s /t $DelaySeconds /c "CodeWiki release run finished. 'shutdown /a' stops this."
        if ($LASTEXITCODE -ne 0) { throw "shutdown.exe failed with $LASTEXITCODE" }
        "shutting down in $DelaySeconds seconds; 'shutdown /a' stops it"
    }
    "Hibernate" {
        "hibernating in $DelaySeconds seconds; end this script to stop it"
        Send-Notice "CodeWiki release run finished. This computer hibernates in $DelaySeconds seconds."
        Start-Sleep -Seconds $DelaySeconds
        Assert-NoGuest
        & shutdown.exe /h
        if ($LASTEXITCODE -ne 0) { throw "shutdown.exe failed with $LASTEXITCODE" }
    }
    "Sleep" {
        "going to sleep in $DelaySeconds seconds; end this script to stop it"
        Send-Notice "CodeWiki release run finished. This computer goes to sleep in $DelaySeconds seconds."
        Start-Sleep -Seconds $DelaySeconds
        Assert-NoGuest
        # Not "rundll32 powrprof.dll,SetSuspendState": with hibernation turned
        # on, that hibernates instead.
        Add-Type -AssemblyName System.Windows.Forms
        $asleep = [System.Windows.Forms.Application]::SetSuspendState([System.Windows.Forms.PowerState]::Suspend, $false, $false)
        if (-not $asleep) { throw "Windows refused to sleep" }
    }
}
