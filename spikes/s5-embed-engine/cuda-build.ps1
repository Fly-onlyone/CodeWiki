# Gate B of spike S5: build the spike with candle's `cuda` feature against
# the toolkit that cuda-toolkit.ps1 unpacked. nvcc needs cl.exe, so the build
# runs inside Visual Studio's developer environment. Nothing is set outside
# this process.
param(
    [string]$Toolkit = "D:\packages\cuda\13.4.3",
    [string[]]$CargoArgs = @("build", "--release", "--features", "cuda")
)

$ErrorActionPreference = "Stop"
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vs = & $vswhere -latest -products * -property installationPath
if (-not $vs) { throw "no Visual Studio with the C++ tools found" }
# The developer shell looks for vswhere on PATH.
$env:PATH = "$(Split-Path $vswhere);$env:PATH"
& (Join-Path $vs "Common7\Tools\Launch-VsDevShell.ps1") -Arch amd64 -HostArch amd64 -SkipAutomaticLocation | Out-Null

# The headers of CUDA 13.2 and later refuse MSVC's traditional preprocessor
# (candle issue 3686). cl.exe reads extra options from the CL variable, and
# nvcc starts cl.exe, so this reaches the kernels' compilation.
$env:CL = "/Zc:preprocessor"
$env:CUDA_PATH = $Toolkit
$env:PATH = "$Toolkit\bin;$Toolkit\bin\x64;$env:PATH"
# The kernels are compiled for the card that nvidia-smi reports; a machine
# without one (a CI runner) sets CUDA_COMPUTE_CAP instead.
"nvcc: " + ((& nvcc --version) | Select-Object -Last 2 | Select-Object -First 1)
"cl:   " + (Get-Command cl.exe).Source

Push-Location $PSScriptRoot
try {
    $started = Get-Date
    & cargo @CargoArgs
    "cargo exit code: $LASTEXITCODE, $([int]((Get-Date) - $started).TotalSeconds) s"
} finally {
    Pop-Location
}
