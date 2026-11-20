# Build release binaries and assemble a Windows package:
#   dist\CodeWiki-<version>-windows-x64.zip
#
# The zip holds the three programs, the agent plugins, the licence files,
# install.ps1 / uninstall.ps1 and the install.cmd / uninstall.cmd launchers
# that run them past Windows's default script policy. It also holds the
# folder cuda\: the worker that runs a built-in embedding model on an NVIDIA
# card, the three NVIDIA libraries it needs, and NVIDIA's licence for them.
# Run from the repository root:
#   pwsh scripts\package.ps1
#
# The CUDA worker needs NVIDIA's toolkit and Visual Studio's C++ tools to
# build. scripts\cuda-toolkit.ps1 fetches the toolkit (0.9 GB, once), and
# scripts\cuda-worker.ps1 builds the worker into target\release\cuda.
param(
    # Skip `cargo build` and package what is already in target\release.
    [switch]$NoBuild,
    # Leave the folder cuda\ out: a package that runs models on the processor
    # alone, built without NVIDIA's toolkit.
    [switch]$NoCuda,
    # The toolkit's folder. Without it, scripts\cuda-toolkit.ps1 names one,
    # and fetches what is missing there.
    [string]$CudaToolkit
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

. (Join-Path $PSScriptRoot "cuda-common.ps1")

if (-not $NoBuild) {
    cargo build --release --locked -p codewiki -p codewiki-mcp -p codewiki-app
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}

if (-not $NoCuda) {
    if (-not $CudaToolkit) {
        $CudaToolkit = & (Join-Path $PSScriptRoot "cuda-toolkit.ps1") | Select-Object -Last 1
    }
    # The worker and its libraries, in target\release\cuda. Its build runs
    # inside Visual Studio's developer environment, so it comes after the
    # build above, which needs nothing of it.
    $cudaDir = & (Join-Path $PSScriptRoot "cuda-worker.ps1") -Release -CudaToolkit $CudaToolkit -NoBuild:$NoBuild |
        Select-Object -Last 1
    Set-Location $root
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
# `codewiki` must start on a machine with no NVIDIA file on it.
$stray = Get-CudaImports (Join-Path $stage "codewiki.exe")
if ($stray) { throw "codewiki.exe imports $($stray -join ', '); it was built with the feature cuda" }

if (-not $NoCuda) {
    $cuda = Join-Path $stage "cuda"
    New-Item -ItemType Directory -Force $cuda | Out-Null
    # cuda-worker.ps1 checked that the worker needs no other library. The
    # libraries there may be hard links; copies of them are whole files.
    Copy-Item (Join-Path $cudaDir "*") $cuda
    # NVIDIA's licence lets these libraries be passed on with a program that
    # uses them (Attachment A), and its text goes with them.
    Copy-Item (Join-Path $CudaToolkit "licenses\libcublas.txt") (Join-Path $cuda "NVIDIA-CUDA-EULA.txt")
}

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
