# Puts the parts of NVIDIA's CUDA toolkit that the CUDA worker needs, to be
# built and to run, into one folder:
#   pwsh scripts\cuda-toolkit.ps1 [-Target <folder>]
#
# The parts come from NVIDIA's redistributable archives. No installer runs,
# no administrator rights are needed, nothing is added to Windows, and
# deleting the folder removes it all. Each archive's sha256 is taken from
# NVIDIA's manifest and checked. A second run fetches only what is missing.
#
# scripts\package.ps1 calls this. The folder is about 2 GB; 0.9 GB is
# downloaded.
param(
    # cudarc 0.19.10, which candle builds on, accepts CUDA 11.4 to 13.4.
    [string]$Version = "13.4.3",
    # Where the toolkit goes. CODEWIKI_CUDA_TOOLKIT names a folder to keep it
    # in across checkouts; the default is inside target\, which git ignores.
    [string]$Target = $(if ($env:CODEWIKI_CUDA_TOOLKIT) { $env:CODEWIKI_CUDA_TOOLKIT } else { Join-Path (Split-Path -Parent $PSScriptRoot) "target\cuda-toolkit\$Version" })
)

$ErrorActionPreference = "Stop"
$base = "https://developer.download.nvidia.com/compute/cuda/redist"
# To build: nvcc with its compiler back end (libnvvm) and its device headers
# (cuda_crt, and cccl, from which cuda_fp16.h includes nv/target), and the
# import libraries of the runtime. To run: cublas (with cublasLt) and curand.
# candle links nvrtc too, though the worker never loads it.
$parts = "cuda_nvcc", "libnvvm", "cuda_crt", "cccl", "cuda_cudart", "libcublas", "libcurand", "cuda_nvrtc"
# The parts whose DLLs are shipped; NVIDIA's licence for them goes with them.
$shipped = "libcublas", "libcurand"

$downloads = Join-Path $Target ".downloads"
$licenses = Join-Path $Target "licenses"
New-Item -ItemType Directory -Force $downloads, $licenses | Out-Null
$manifest = Invoke-RestMethod "$base/redistrib_$Version.json"

foreach ($part in $parts) {
    $entry = $manifest.$part.'windows-x86_64'
    if (-not $entry) { throw "NVIDIA's manifest has no Windows archive for $part" }
    $zip = Join-Path $downloads (Split-Path -Leaf $entry.relative_path)
    $done = "$zip.unpacked"
    if (Test-Path $done) { continue }
    "$part $($manifest.$part.version): $([math]::Round($entry.size / 1MB)) MB"
    # curl goes on with a download that was cut short.
    & curl.exe -sSL --fail -C - -o $zip "$base/$($entry.relative_path)"
    if ($LASTEXITCODE -ne 0) { throw "download failed: $part" }
    $got = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLower()
    if ($got -ne $entry.sha256) {
        Remove-Item -Force $zip
        throw "${part}: sha256 is $got, NVIDIA's manifest says $($entry.sha256)"
    }
    # Every archive holds one folder with bin, include, lib and the rest.
    $scratch = Join-Path $downloads "unpack-$part"
    Remove-Item -Recurse -Force $scratch -ErrorAction SilentlyContinue
    Expand-Archive -Path $zip -DestinationPath $scratch
    $root = Get-ChildItem $scratch -Directory | Select-Object -First 1
    Copy-Item -Path (Join-Path $root.FullName "*") -Destination $Target -Recurse -Force
    Remove-Item -Recurse -Force $scratch
    Remove-Item -Force $zip
    New-Item -ItemType File -Force $done | Out-Null
}

# The licence of each shipped part, as NVIDIA publishes it beside the
# archives. The archives' own LICENSE files overwrite one another above.
foreach ($part in $shipped) {
    $file = Join-Path $licenses "$part.txt"
    if (Test-Path $file) { continue }
    & curl.exe -sSL --fail -o $file "$base/$($manifest.$part.license_path)"
    if ($LASTEXITCODE -ne 0) { throw "could not fetch NVIDIA's licence for $part" }
}

if (-not (Test-Path (Join-Path $Target "bin\nvcc.exe"))) { throw "no nvcc in $Target" }
$Target
