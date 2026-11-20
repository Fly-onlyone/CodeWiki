# Spike S5 in the guest.
#
# Without -Gpu (gate A): does the processor build start and embed on a clean
# Windows, with no C++ runtime, no NVIDIA driver and no graphics card?
# With -Gpu (gate B): does the CUDA build, with its three NVIDIA DLLs beside
# it, embed on the host's card shared into the guest?
#
# Restores the checkpoint, copies the release exe and the small model in,
# runs it, compares its vectors with the reference, and restores the
# checkpoint again so nothing of the run stays in the guest. Build the exe
# that fits first: `cargo build --release`, or cuda-build.ps1 for -Gpu.
param(
    [string]$VmName = "Tiny 11",
    [string]$User = "Software Tester",
    [string]$Password = "",
    [string]$Snapshot,
    [int]$MemoryMb = 2048,
    [switch]$Gpu,
    [string]$Toolkit = "D:\packages\cuda\13.4.3"
)

$ErrorActionPreference = "Stop"
if (-not $Snapshot) { $Snapshot = if ($Gpu) { "Clean GPU" } else { "Clean" } }
$spike = $PSScriptRoot
. (Join-Path $spike "..\..\scripts\vm\Vm.ps1")
Initialize-Vm -Name $VmName -User $User -Password $Password
Start-KeepAwake

$stage = Join-Path $spike "models\s5"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force (Join-Path $stage "granite-small\1_Pooling") | Out-Null
Copy-Item (Join-Path $spike "target\release\spike-s5-embed-engine.exe") (Join-Path $stage "spike.exe")
Copy-Item (Join-Path $spike "texts.json") $stage
foreach ($file in "config.json", "tokenizer.json", "model.safetensors", "1_Pooling\config.json") {
    Copy-Item (Join-Path $spike "models\granite-small\$file") (Join-Path $stage "granite-small\$file")
}
if ($Gpu) {
    # What the CUDA build imports, and what cublas imports in turn.
    foreach ($dll in "curand64_10.dll", "cublas64_13.dll", "cublasLt64_13.dll") {
        Copy-Item (Join-Path $Toolkit "bin\x64\$dll") $stage
    }
}

"restoring checkpoint '$Snapshot'"
Restore-Guest -Snapshot $Snapshot
Set-VMMemory -VMName $VmName -StartupBytes ($MemoryMb * 1MB) -MaximumBytes ($MemoryMb * 1MB)
# A checkpoint does not keep the card, so it is attached after every restore.
if ($Gpu) { Enable-GuestGpu }
Start-Guest
Wait-Guest -TimeoutSec 900
"guest is up with $MemoryMb MB"

try {
    $facts = Invoke-GuestScript -TimeoutSec 120 -Script @'
"vcruntime140.dll in System32: " + (Test-Path "$env:WINDIR\System32\vcruntime140.dll")
"msvcp140.dll in System32: " + (Test-Path "$env:WINDIR\System32\msvcp140.dll")
"nvml.dll in System32: " + (Test-Path "$env:WINDIR\System32\nvml.dll")
"nvcuda.dll in System32: " + (Test-Path "$env:WINDIR\System32\nvcuda.dll")
"display: " + ((Get-CimInstance Win32_VideoController | ForEach-Object Name) -join ", ")
"build: " + (Get-CimInstance Win32_OperatingSystem).BuildNumber
'@
    $facts.Output

    Copy-ToGuest -Path $stage -GuestDirectory "C:\vmtest"
    $exe = "C:\vmtest\s5\spike.exe"
    $model = "C:\vmtest\s5\granite-small"
    $device = if ($Gpu) { @("--device", "cuda") } else { @() }
    $runs = @(
        @{ Name = "probe"; Arguments = @("probe") },
        @{ Name = "embed"; Arguments = $device + @("embed", $model, "C:\vmtest\s5\texts.json", "C:\vmtest\s5\candle.json") },
        @{ Name = "bench"; Arguments = $device + @("bench", $model, "16", "500", "8") }
    )
    foreach ($run in $runs) {
        "--- $($run.Name)"
        $result = Invoke-GuestExe -Exe $exe -Arguments $run.Arguments -TimeoutSec 400
        "exit code: $($result.ExitCode)"
        $result.Stdout.Trim()
        if ($result.Stderr.Trim()) { "stderr: " + $result.Stderr.Trim() }
    }
    $out = Join-Path $spike "models\vm-candle-granite-small.json"
    Remove-Item $out -ErrorAction SilentlyContinue
    Copy-FromGuest -GuestPath "C:\vmtest\s5\candle.json" -Destination $out
    "--- compare with the reference"
    & python -I (Join-Path $spike "compare.py") (Join-Path $spike "models\ref-granite-small.json") $out
} catch {
    "failed: $_"
} finally {
    "stopping the guest and restoring '$Snapshot'"
    Stop-Guest
    Restore-Guest -Snapshot $Snapshot
    Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
}
