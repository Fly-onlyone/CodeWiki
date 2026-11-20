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
# build. scripts\cuda-toolkit.ps1 fetches the toolkit (0.9 GB, once).
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

# The NVIDIA libraries the CUDA worker loads: the two it imports, and
# cublasLt, which cublas imports. They are named for CUDA 13.4.
$cudaLibraries = "curand64_10.dll", "cublas64_13.dll", "cublasLt64_13.dll"

# Names of the NVIDIA libraries a program asks Windows for when it starts,
# read from the file itself.
function Get-CudaImports([string]$Program) {
    $text = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($Program))
    @([regex]::Matches($text, '(?i)\b(cu[a-z]+64_\d+|nvcuda|nvrtc[\w-]*)\.dll') |
        ForEach-Object { $_.Value.ToLower() } | Sort-Object -Unique)
}

if (-not $NoBuild) {
    cargo build --release --locked -p codewiki -p codewiki-mcp -p codewiki-app
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}

if (-not $NoCuda) {
    if (-not $CudaToolkit) {
        $CudaToolkit = & (Join-Path $PSScriptRoot "cuda-toolkit.ps1") | Select-Object -Last 1
    }
    if (-not (Test-Path (Join-Path $CudaToolkit "bin\nvcc.exe"))) { throw "no CUDA toolkit in $CudaToolkit" }
    if (-not $NoBuild) {
        # nvcc starts cl.exe, so this build runs inside Visual Studio's
        # developer environment. It comes after the build above, which needs
        # nothing of it.
        $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
        $vs = & $vswhere -latest -products * -property installationPath
        if (-not $vs) { throw "no Visual Studio with the C++ tools found; the CUDA worker needs cl.exe" }
        $env:PATH = "$(Split-Path $vswhere);$env:PATH"
        & (Join-Path $vs "Common7\Tools\Launch-VsDevShell.ps1") -Arch amd64 -HostArch amd64 -SkipAutomaticLocation | Out-Null
        Set-Location $root
        # The headers of CUDA 13.2 and later refuse MSVC's traditional
        # preprocessor (candle issue 3686). cl.exe reads options from CL.
        $env:CL = "/Zc:preprocessor"
        $env:CUDA_PATH = $CudaToolkit
        $env:PATH = "$CudaToolkit\bin;$CudaToolkit\bin\x64;$env:PATH"
        # The kernels are compiled for a generation of cards and run on that
        # one and every later one. Left alone, the build takes the card of
        # the machine it runs on, and a package built beside a new card would
        # not run on an older one. 75 is the oldest CUDA 13 supports: GeForce
        # GTX 16 and RTX 20.
        if (-not $env:CUDA_COMPUTE_CAP) { $env:CUDA_COMPUTE_CAP = "75" }
        # Only this program: `codewiki` built with the feature would not
        # start where NVIDIA's libraries are missing.
        cargo build --release --locked -p codewiki --features cuda --bin codewiki-embed-cuda
        if ($LASTEXITCODE -ne 0) { throw "cargo build of the CUDA worker failed" }
    }
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
    $worker = Join-Path $root "target\release\codewiki-embed-cuda.exe"
    if (-not (Test-Path $worker)) { throw "missing $worker; build first" }
    Copy-Item $worker $cuda
    foreach ($library in $cudaLibraries) {
        $source = Join-Path $CudaToolkit "bin\x64\$library"
        if (-not (Test-Path $source)) { throw "missing $source; the toolkit is not CUDA 13.4" }
        Copy-Item $source $cuda
    }
    # A library the worker asks for and the package does not hold would stop
    # it before its first line. nvcuda.dll is not one of them: it is the
    # graphics driver's, and is on every machine the worker is started on.
    $shipped = @($cudaLibraries | ForEach-Object { $_.ToLower() }) + "nvcuda.dll"
    $unmet = @(Get-CudaImports $worker | Where-Object { $shipped -notcontains $_ })
    if ($unmet) { throw "codewiki-embed-cuda.exe imports $($unmet -join ', '), which the package does not hold" }
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
