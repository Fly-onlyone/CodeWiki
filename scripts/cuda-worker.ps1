# Builds the worker that runs a built-in embedding model on an NVIDIA card,
# and puts it where CodeWiki looks for it: the folder cuda\ beside the
# programs of a build, with the three NVIDIA libraries it loads.
#   pwsh scripts\cuda-worker.ps1 [-Release] [-CudaToolkit <folder>] [-NoBuild]
#
# A development build has no worker of its own and runs every model on the
# processor (`worker::cuda_program`). This gives it one:
#   pwsh scripts\cuda-worker.ps1 -CudaToolkit D:\packages\cuda\13.4.3
# Run it again after a change to the engine or to the worker's protocol: a
# worker older than its backend can fail, and after two failures in a row
# the backend goes on with the processor. scripts\package.ps1 runs it with
# -Release.
#
# The build needs NVIDIA's toolkit and Visual Studio's C++ tools. The
# libraries are hard links into the toolkit, which take no space, when both
# are on one drive, and copies otherwise. The last line printed is the
# folder.
param(
    # Build in the release profile, into target\release\cuda. Without it,
    # target\debug\cuda.
    [switch]$Release,
    # The toolkit's folder. Without it, CODEWIKI_CUDA_TOOLKIT; without that,
    # scripts\cuda-toolkit.ps1 names one and fetches what is missing there.
    [string]$CudaToolkit,
    # Skip the build and lay out the worker that is already built.
    [switch]$NoBuild
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root
. (Join-Path $PSScriptRoot "cuda-common.ps1")

if (-not $CudaToolkit) { $CudaToolkit = $env:CODEWIKI_CUDA_TOOLKIT }
if (-not $CudaToolkit) {
    $CudaToolkit = & (Join-Path $PSScriptRoot "cuda-toolkit.ps1") | Select-Object -Last 1
}
if (-not (Test-Path (Join-Path $CudaToolkit "bin\nvcc.exe"))) { throw "no CUDA toolkit in $CudaToolkit" }
$CudaToolkit = (Resolve-Path $CudaToolkit).Path

# The target folder cargo uses here; a worktree has its own.
$metadata = cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw "cargo metadata failed" }
$profileDir = Join-Path $metadata.target_directory $(if ($Release) { "release" } else { "debug" })
$worker = Join-Path $profileDir "codewiki-embed-cuda.exe"
$folder = Join-Path $profileDir "cuda"

# A worker that runs holds its program and its libraries open, and Windows
# refuses to replace them. Say so before anything is changed.
function Assert-NotInUse {
    if (-not (Test-Path $folder)) { return }
    foreach ($file in Get-ChildItem $folder -File) {
        try {
            [System.IO.File]::Open($file.FullName, "Open", "ReadWrite", "None").Dispose()
        } catch [System.IO.IOException] {
            throw "$($file.FullName) is in use; quit CodeWiki (its backend runs the worker) and run this again"
        }
    }
}

Assert-NotInUse
if (-not $NoBuild) {
    # nvcc starts cl.exe, so this build runs inside Visual Studio's
    # developer environment.
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
    $profileFlag = if ($Release) { @("--release") } else { @() }
    cargo build @profileFlag --locked -p codewiki --features cuda --bin codewiki-embed-cuda
    if ($LASTEXITCODE -ne 0) { throw "cargo build of the CUDA worker failed" }
}

if (-not (Test-Path $worker)) { throw "missing $worker; build it first" }
# A library the worker asks for and the folder does not hold would stop it
# before its first line. nvcuda.dll is not one of them: it is the graphics
# driver's, and is on every machine the worker is started on.
$shipped = @($cudaLibraries | ForEach-Object { $_.ToLower() }) + "nvcuda.dll"
$unmet = @(Get-CudaImports $worker | Where-Object { $shipped -notcontains $_ })
if ($unmet) { throw "codewiki-embed-cuda.exe imports $($unmet -join ', '), which the folder cuda\ does not hold" }

# The build took a while; a worker may have started meanwhile.
Assert-NotInUse
New-Item -ItemType Directory -Force $folder | Out-Null
Copy-Item -Force $worker $folder
foreach ($library in $cudaLibraries) {
    $source = Join-Path $CudaToolkit "bin\x64\$library"
    if (-not (Test-Path $source)) { throw "missing $source; the toolkit is not CUDA 13.4" }
    $target = Join-Path $folder $library
    if (Test-Path $target) { Remove-Item -Force $target }
    try {
        New-Item -ItemType HardLink -Path $target -Target $source | Out-Null
    } catch {
        # Another drive, or a file system without hard links.
        Copy-Item $source $target
    }
}
$folder
