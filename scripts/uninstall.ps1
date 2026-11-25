# Remove a CodeWiki installation made by install.ps1. uninstall.cmd beside it
# runs this past Windows's default script policy.
#
# Removes the programs, the PATH entry, the Start Menu shortcut, the start
# with Windows and the name and icon Windows shows the app's notifications
# under. Wikis live
# inside each repository and are not touched. The index cache, the installed
# logs in %LOCALAPPDATA%\CodeWiki (or CODEWIKI_DATA_DIR), the embedding models
# an older version downloaded there (its folder models) and the settings in
# %APPDATA%\CodeWiki are kept unless -PurgeData is given. The folder cuda an
# older version put beside the programs is removed with them. The folders
# CodeWiki-Dev, which debug builds from the sources use, are never touched.
param(
    [string]$Target = $PSScriptRoot,
    # Also delete the index cache, the models an older version downloaded, the
    # logs and the settings.
    [switch]$PurgeData
)
$ErrorActionPreference = "Stop"
$Target = (Resolve-Path $Target).Path

# uninstall.ps1 alone: an earlier run stopped after deleting the programs.
# Each step below removes only what is there, so a second run finishes it.
if (-not (Test-Path (Join-Path $Target "codewiki-app.exe")) -and
    -not (Test-Path (Join-Path $Target "uninstall.ps1"))) {
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
# each process to end before deleting anything. codewiki-embed-cuda is the
# worker of an older version.
foreach ($process in "codewiki-app", "codewiki-mcp", "codewiki", "codewiki-embed-cuda") {
    Get-Process $process -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -and $_.Path.StartsWith($Target, [StringComparison]::OrdinalIgnoreCase) } |
        Stop-Process -Force -PassThru |
        Wait-Process -Timeout 10 -ErrorAction SilentlyContinue
}

# The programs go first. If one cannot be deleted, the script stops here and
# the PATH entry and the shortcut still lead to a working installation.
foreach ($item in "codewiki-app.exe", "codewiki-mcp.exe", "codewiki.exe", "cuda", "plugins", "LICENSE", "NOTICE") {
    $file = Join-Path $Target $item
    if (Test-Path $file) { Remove-Patiently $file }
}
# What an update by the app leaves until its next start: the files it
# replaced (`codewiki.exe.old`, `plugins.old2`) and its download folder.
Get-ChildItem -Force $Target |
    Where-Object { $_.Name -match '\.old\d*$' -or $_.Name -eq ".update" } |
    ForEach-Object { Remove-Patiently $_.FullName }

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

# "Start with Windows" in the app's settings is one value under the user's Run
# key (crates/codewiki-app/src/autostart.rs). Only the one that starts this
# installation is removed.
$run = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run"
$started = (Get-ItemProperty -Path $run -Name "CodeWiki" -ErrorAction SilentlyContinue).CodeWiki
if ($started -and $started.Trim() -ieq ('"{0}" --tray' -f (Join-Path $Target "codewiki-app.exe"))) {
    Remove-ItemProperty -Path $run -Name "CodeWiki"
}

# The name and the icon Windows shows above the app's notifications
# (APP_IDENTITY in crates/codewiki-app/src/main.rs, and identity.rs beside
# it). The icon is a copy of the app's logo in the data folder that the key
# names, so it goes with the key, and only by its own file name. A copy left
# behind harms nothing.
$identity = "HKCU:\Software\Classes\AppUserModelId\CodeWiki.App"
if (Test-Path $identity) {
    $icon = (Get-ItemProperty -Path $identity -Name "IconUri" -ErrorAction SilentlyContinue).IconUri
    if ($icon -and (Split-Path -Leaf $icon) -ieq "notification-icon.png" -and (Test-Path -LiteralPath $icon -PathType Leaf)) {
        Remove-Item -Force -LiteralPath $icon -ErrorAction SilentlyContinue
    }
    Remove-Item -Recurse -Force $identity
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
