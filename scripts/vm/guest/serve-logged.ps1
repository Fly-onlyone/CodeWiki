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
# In the user's own folder, as a server typed into a terminal starts: started
# from Task Scheduler the folder is C:\Windows\System32, and mistral.rs ended
# there with "Access is denied. (os error 5)" right after loading its model.
$options = @{ FilePath = $Exe; NoNewWindow = $true; Wait = $true; RedirectStandardOutput = "$Log.out"; RedirectStandardError = "$Log.err"; WorkingDirectory = $env:USERPROFILE }
if ($ArgLine) { $options.ArgumentList = $ArgLine }
Start-Process @options
