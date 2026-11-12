# Free the disk space builds leave behind. Safe to run at any time: everything
# removed here is rebuilt on demand.
#
#   pwsh scripts\clean.ps1            debug artifacts and spike build folders
#   pwsh scripts\clean.ps1 -All       also release artifacts and dist\
#   pwsh scripts\clean.ps1 -Docker    also Docker's build cache (all projects)
param([switch]$All, [switch]$Docker)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Size([string]$path) {
    if (-not (Test-Path $path)) { return 0 }
    (Get-ChildItem $path -Recurse -File -Force -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum
}
$before = (Size "target") + (Size "spikes") + (Size "dist")

if ($All) { cargo clean } else { cargo clean --profile dev }
Get-ChildItem "spikes" -Directory | ForEach-Object {
    $target = Join-Path $_.FullName "target"
    if (Test-Path $target) { Remove-Item -Recurse -Force $target }
}
if ($All -and (Test-Path "dist")) { Remove-Item -Recurse -Force "dist" }
if ($Docker) { docker builder prune --force | Select-Object -Last 1 }

$after = (Size "target") + (Size "spikes") + (Size "dist")
"freed {0:N1} GB; build folders now use {1:N1} GB" -f (($before - $after) / 1GB), ($after / 1GB)
