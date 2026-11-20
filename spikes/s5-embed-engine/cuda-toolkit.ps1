# Puts the parts of NVIDIA's CUDA toolkit that a candle `cuda` build needs
# into one folder, from NVIDIA's redistributable archives. No installer, no
# administrator rights, nothing added to Windows; deleting the folder removes
# it all.
#
# Each archive's sha256 is taken from NVIDIA's manifest and checked.
# cudarc 0.19.10 accepts CUDA 11.4 to 13.4, so the version is pinned below 13.5.
param(
    [string]$Version = "13.4.3",
    [string]$Target = "D:\packages\cuda\$Version"
)

$ErrorActionPreference = "Stop"
$base = "https://developer.download.nvidia.com/compute/cuda/redist"
# nvcc with its compiler back end and device headers (cuda_fp16.h includes
# nv/target from cccl), the runtime, and the
# three libraries candle links: cublas (with cublasLt), curand, nvrtc.
$parts = "cuda_nvcc", "libnvvm", "cuda_crt", "cccl", "cuda_cudart", "libcublas", "libcurand", "cuda_nvrtc"

$downloads = Join-Path $Target ".downloads"
New-Item -ItemType Directory -Force $downloads | Out-Null
$manifest = Invoke-RestMethod "$base/redistrib_$Version.json"

foreach ($part in $parts) {
    $entry = $manifest.$part.'windows-x86_64'
    if (-not $entry) { throw "NVIDIA's manifest has no Windows archive for $part" }
    $zip = Join-Path $downloads (Split-Path -Leaf $entry.relative_path)
    $done = "$zip.unpacked"
    if (Test-Path $done) { "$part $($manifest.$part.version): already unpacked"; continue }
    "$part $($manifest.$part.version): $([math]::Round($entry.size / 1MB)) MB"
    & curl.exe -sSL --fail -C - -o $zip "$base/$($entry.relative_path)"
    if ($LASTEXITCODE -ne 0) { throw "download failed: $part" }
    $got = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLower()
    if ($got -ne $entry.sha256) { throw "${part}: sha256 is $got, NVIDIA's manifest says $($entry.sha256)" }
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

"toolkit in $Target"
& (Join-Path $Target "bin\nvcc.exe") --version | Select-Object -Last 2
