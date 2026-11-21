# Build release binaries and assemble a Windows package:
#   dist\CodeWiki-<version>-windows-x64.zip
#
# The zip holds the three programs, the agent plugins, the licence files,
# install.ps1 / uninstall.ps1 and the install.cmd / uninstall.cmd launchers
# that run them past Windows's default script policy.
# Run from the repository root:
#   pwsh scripts\package.ps1
#
# Error reports: a build made while CODEWIKI_SENTRY_DSN is set carries that
# DSN, and sends reports once the user turns them on (crates/cw-telemetry).
# The programs' debug files go to dist\symbols-<version>\, never into the
# zip; the release workflow uploads them to Sentry, which needs them to name
# the functions of a release build's stack trace.
param(
    # Skip `cargo build` and package what is already in target\release.
    [switch]$NoBuild
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

# Line tables and a PDB file beside each program. `strip = true` in the
# release profile would link with /DEBUG:NONE and write none. The Docker
# image, built by the same profile on Linux, keeps it as it is.
$env:CARGO_PROFILE_RELEASE_DEBUG = "line-tables-only"
$env:CARGO_PROFILE_RELEASE_STRIP = "none"

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

# Each program and its PDB, for Sentry. Kept out of the zip: the PDBs are
# large and of no use on the user's machine.
$symbols = Join-Path $dist "symbols-$version"
if (Test-Path $symbols) { Remove-Item -Recurse -Force $symbols }
New-Item -ItemType Directory -Force $symbols | Out-Null
$programs = @("codewiki", "codewiki-mcp", "codewiki-app")
foreach ($program in $programs) {
    $pdb = Join-Path $root "target\release\$($program.Replace('-', '_')).pdb"
    if (-not (Test-Path $pdb)) { throw "missing $pdb; the release build wrote no debug file" }
    Copy-Item $pdb $symbols
    Copy-Item (Join-Path $root "target\release\$program.exe") $symbols
}
$reports = if ($env:CODEWIKI_SENTRY_DSN) { "carries a DSN" } else { "carries no DSN" }
"error reports: this build $reports; debug files in $symbols"

foreach ($file in "LICENSE", "NOTICE") { Copy-Item (Join-Path $root $file) $stage }
Copy-Item -Recurse (Join-Path $root "plugins") (Join-Path $stage "plugins")
foreach ($script in "install.ps1", "uninstall.ps1", "install.cmd", "uninstall.cmd") {
    Copy-Item (Join-Path $PSScriptRoot $script) $stage
}

$zip = Join-Path $dist "$name.zip"
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip
$size = [math]::Round((Get-Item $zip).Length / 1MB, 1)
"packaged $zip ($size MB)"
