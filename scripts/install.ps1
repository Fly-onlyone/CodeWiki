# Install CodeWiki for the current user. No administrator rights needed.
#
# Copies the programs next to this script to %LOCALAPPDATA%\Programs\CodeWiki,
# adds that folder to the user PATH (so agents can start `codewiki-mcp`), and
# creates a Start Menu shortcut for the desktop app.
param(
    # Install folder.
    [string]$Target = (Join-Path $env:LOCALAPPDATA "Programs\CodeWiki"),
    # Leave the user PATH alone.
    [switch]$NoPath,
    # Do not create the Start Menu shortcut.
    [switch]$NoShortcut
)
$ErrorActionPreference = "Stop"
$source = $PSScriptRoot

foreach ($exe in "codewiki.exe", "codewiki-mcp.exe", "codewiki-app.exe") {
    if (-not (Test-Path (Join-Path $source $exe))) { throw "$exe is not next to install.ps1" }
}

# A running backend or app keeps its file locked.
foreach ($process in "codewiki-app", "codewiki-mcp", "codewiki") {
    Get-Process $process -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -and $_.Path.StartsWith($Target, [StringComparison]::OrdinalIgnoreCase) } |
        Stop-Process -Force
}

New-Item -ItemType Directory -Force $Target | Out-Null
foreach ($item in "codewiki.exe", "codewiki-mcp.exe", "codewiki-app.exe", "LICENSE", "NOTICE", "uninstall.ps1") {
    Copy-Item (Join-Path $source $item) $Target -Force
}
if (Test-Path (Join-Path $source "plugins")) {
    Copy-Item -Recurse -Force (Join-Path $source "plugins") $Target
}

if (-not $NoPath) {
    $path = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @($path -split ";" | Where-Object { $_ })
    if ($parts -notcontains $Target) {
        [Environment]::SetEnvironmentVariable("Path", (($parts + $Target) -join ";"), "User")
        "added $Target to the user PATH; open a new terminal to use it"
    }
}

if (-not $NoShortcut) {
    $menu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut((Join-Path $menu "CodeWiki.lnk"))
    $shortcut.TargetPath = Join-Path $Target "codewiki-app.exe"
    $shortcut.WorkingDirectory = $env:USERPROFILE
    $shortcut.Description = "CodeWiki desktop app"
    $shortcut.Save()
}

"installed CodeWiki to $Target"
