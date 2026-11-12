# Remove a CodeWiki installation made by install.ps1.
#
# Removes the programs, the PATH entry and the Start Menu shortcut. Wikis live
# inside each repository and are not touched. The index cache in
# %LOCALAPPDATA%\CodeWiki is kept unless -PurgeData is given.
param(
    [string]$Target = $PSScriptRoot,
    # Also delete the index cache and settings in %LOCALAPPDATA%\CodeWiki.
    [switch]$PurgeData
)
$ErrorActionPreference = "Stop"
$Target = (Resolve-Path $Target).Path

if (-not (Test-Path (Join-Path $Target "codewiki-app.exe"))) {
    throw "$Target does not look like a CodeWiki installation"
}

foreach ($process in "codewiki-app", "codewiki-mcp", "codewiki") {
    Get-Process $process -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -and $_.Path.StartsWith($Target, [StringComparison]::OrdinalIgnoreCase) } |
        Stop-Process -Force
}

$path = [Environment]::GetEnvironmentVariable("Path", "User")
$parts = @($path -split ";" | Where-Object { $_ })
if ($parts -contains $Target) {
    [Environment]::SetEnvironmentVariable("Path", (($parts | Where-Object { $_ -ne $Target }) -join ";"), "User")
}

$shortcut = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\CodeWiki.lnk"
if (Test-Path $shortcut) {
    $shell = New-Object -ComObject WScript.Shell
    # Only remove the shortcut that points at this installation.
    if ($shell.CreateShortcut($shortcut).TargetPath -eq (Join-Path $Target "codewiki-app.exe")) {
        Remove-Item -Force $shortcut
    }
}

foreach ($item in "codewiki.exe", "codewiki-mcp.exe", "codewiki-app.exe", "LICENSE", "NOTICE") {
    $file = Join-Path $Target $item
    if (Test-Path $file) { Remove-Item -Force $file }
}
$plugins = Join-Path $Target "plugins"
if (Test-Path $plugins) { Remove-Item -Recurse -Force $plugins }

if ($PurgeData) {
    $data = Join-Path $env:LOCALAPPDATA "CodeWiki"
    if (Test-Path $data) { Remove-Item -Recurse -Force $data }
}

"removed CodeWiki from $Target (this script and the folder remain; delete them by hand)"
