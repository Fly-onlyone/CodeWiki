# Build release binaries and assemble a Windows package:
#   dist\CodeWiki-<version>-windows-x64.zip
#
# The zip holds the three programs, the agent plugins, the licence files and
# install.ps1 / uninstall.ps1. Run from the repository root:
#   pwsh scripts\package.ps1
param(
    # Skip `cargo build` and package what is already in target\release.
    [switch]$NoBuild
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not $NoBuild) {
    cargo build --release --locked -p codewiki -p codewiki-mcp -p codewiki-app
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}

$manifest = Get-Content (Join-Path $root "Cargo.toml") -Raw
if ($manifest -notmatch '(?m)^version\s*=\s*"([^"]+)"') { throw "no workspace version in Cargo.toml" }
$version = $Matches[1]

$name = "CodeWiki-$version-windows-x64"
$dist = Join-Path $root "dist"
$stage = Join-Path $dist $name
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force $stage | Out-Null

foreach ($exe in "codewiki.exe", "codewiki-mcp.exe", "codewiki-app.exe") {
    $source = Join-Path $root "target\release\$exe"
    if (-not (Test-Path $source)) { throw "missing $source; build first" }
    Copy-Item $source $stage
}
foreach ($file in "LICENSE", "NOTICE") { Copy-Item (Join-Path $root $file) $stage }
Copy-Item -Recurse (Join-Path $root "plugins") (Join-Path $stage "plugins")
Copy-Item (Join-Path $PSScriptRoot "install.ps1") $stage
Copy-Item (Join-Path $PSScriptRoot "uninstall.ps1") $stage

$zip = Join-Path $dist "$name.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip
$size = [math]::Round((Get-Item $zip).Length / 1MB, 1)
"packaged $zip ($size MB)"
