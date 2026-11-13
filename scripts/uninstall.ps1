# Remove a CodeWiki installation made by install.ps1. uninstall.cmd beside it
# runs this past Windows's default script policy.
#
# Removes the programs, the PATH entry and the Start Menu shortcut. Wikis live
# inside each repository and are not touched. The index cache and logs in
# %LOCALAPPDATA%\CodeWiki (or CODEWIKI_DATA_DIR) and the settings in
# %APPDATA%\CodeWiki are kept unless -PurgeData is given.
param(
    [string]$Target = $PSScriptRoot,
    # Also delete the index cache, the logs and the settings.
    [switch]$PurgeData
)
$ErrorActionPreference = "Stop"
$Target = (Resolve-Path $Target).Path

if (-not (Test-Path (Join-Path $Target "codewiki-app.exe"))) {
    throw "$Target does not look like a CodeWiki installation"
}

# Delete a file or folder, trying again for a few seconds: Windows can hold a
# program file a moment after its process has ended.
function Remove-Patiently([string]$Path) {
    for ($attempt = 1; ; $attempt++) {
        try {
            Remove-Item -Recurse -Force $Path -ErrorAction Stop
            return
        } catch {
            if ($attempt -ge 20) { throw }
            Start-Sleep -Milliseconds 250
        }
    }
}

# Stop-Process returns before Windows lets go of the program file, so wait for
# each process to end before deleting anything.
foreach ($process in "codewiki-app", "codewiki-mcp", "codewiki") {
    Get-Process $process -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -and $_.Path.StartsWith($Target, [StringComparison]::OrdinalIgnoreCase) } |
        Stop-Process -Force -PassThru |
        Wait-Process -Timeout 10 -ErrorAction SilentlyContinue
}

# The programs go first. If one cannot be deleted, the script stops here and
# the PATH entry and the shortcut still lead to a working installation.
foreach ($item in "codewiki-app.exe", "codewiki-mcp.exe", "codewiki.exe", "plugins", "LICENSE", "NOTICE") {
    $file = Join-Path $Target $item
    if (Test-Path $file) { Remove-Patiently $file }
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

if ($PurgeData) {
    # The same folders the programs use: cw_core::paths::data_dir() for the
    # index, the backend's state and the logs, and the app's settings.
    $data = if ($env:CODEWIKI_DATA_DIR) { $env:CODEWIKI_DATA_DIR } else { Join-Path $env:LOCALAPPDATA "CodeWiki" }
    $settings = Join-Path $env:APPDATA "CodeWiki"
    foreach ($folder in $data, $settings) {
        if (Test-Path $folder) { Remove-Patiently $folder }
    }
}

# cmd.exe reads uninstall.cmd while it runs, so neither script can delete itself.
"removed CodeWiki from $Target (uninstall.ps1, uninstall.cmd and the folder remain; delete them by hand)"
