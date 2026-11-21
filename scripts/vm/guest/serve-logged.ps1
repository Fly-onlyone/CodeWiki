# Runs a long model server in the guest and keeps what it prints, which a
# program started straight from Task Scheduler would lose. The test starts this
# script (test-release.ps1, Start-LoggedServer) behind conhost --headless; it
# ends when the server does. The server's output goes to <Log>.out and
# <Log>.err.
param(
    [Parameter(Mandatory)][string]$Exe,
    [Parameter(Mandatory)][string]$Log,
    # The server's arguments as one text, as they would be typed after its name.
    [string]$ArgLine = ''
)
$ErrorActionPreference = 'Continue'
New-Item -ItemType Directory -Force (Split-Path -Parent $Log) | Out-Null
$options = @{ FilePath = $Exe; NoNewWindow = $true; Wait = $true; RedirectStandardOutput = "$Log.out"; RedirectStandardError = "$Log.err" }
if ($ArgLine) { $options.ArgumentList = $ArgLine }
Start-Process @options
