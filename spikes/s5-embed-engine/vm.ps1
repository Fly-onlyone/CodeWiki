# Gate A of spike S5: does the spike program start and embed on a clean
# Windows, with no C++ runtime, no NVIDIA driver and no graphics card?
#
# Restores the checkpoint, copies the release exe and the small model in,
# runs it, compares its vectors with the reference, and restores the
# checkpoint again so nothing of the run stays in the guest.
param(
    [string]$VmName = "Tiny 11",
    [string]$User = "Software Tester",
    [string]$Password = "",
    [string]$Snapshot = "Clean",
    [int]$MemoryMb = 2048
)

$ErrorActionPreference = "Stop"
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

"restoring checkpoint '$Snapshot'"
Restore-Guest -Snapshot $Snapshot
Set-VMMemory -VMName $VmName -StartupBytes ($MemoryMb * 1MB) -MaximumBytes ($MemoryMb * 1MB)
Start-Guest
Wait-Guest -TimeoutSec 900
"guest is up with $MemoryMb MB"

try {
    $facts = Invoke-GuestScript -TimeoutSec 120 -Script @'
"vcruntime140.dll in System32: " + (Test-Path "$env:WINDIR\System32\vcruntime140.dll")
"msvcp140.dll in System32: " + (Test-Path "$env:WINDIR\System32\msvcp140.dll")
"nvml.dll in System32: " + (Test-Path "$env:WINDIR\System32\nvml.dll")
"display: " + ((Get-CimInstance Win32_VideoController | ForEach-Object Name) -join ", ")
"build: " + (Get-CimInstance Win32_OperatingSystem).BuildNumber
'@
    $facts.Output

    Copy-ToGuest -Path $stage -GuestDirectory "C:\vmtest"
    $exe = "C:\vmtest\s5\spike.exe"
    $model = "C:\vmtest\s5\granite-small"
    $runs = @(
        @{ Name = "probe"; Arguments = @("probe") },
        @{ Name = "embed"; Arguments = @("embed", $model, "C:\vmtest\s5\texts.json", "C:\vmtest\s5\candle.json") },
        @{ Name = "bench"; Arguments = @("bench", $model, "16", "500", "8") }
    )
    foreach ($run in $runs) {
        "--- $($run.Name)"
        $result = Invoke-GuestExe -Exe $exe -Arguments $run.Arguments -TimeoutSec 400
        "exit code: $($result.ExitCode)"
        $result.Stdout.Trim()
        if ($result.Stderr.Trim()) { "stderr: " + $result.Stderr.Trim() }
    }
    Copy-FromGuest -GuestPath "C:\vmtest\s5\candle.json" -Destination (Join-Path $spike "models\vm-candle-granite-small.json")
    "--- compare with the reference"
    & python -I (Join-Path $spike "compare.py") (Join-Path $spike "models\ref-granite-small.json") (Join-Path $spike "models\vm-candle-granite-small.json")
} finally {
    "stopping the guest and restoring '$Snapshot'"
    Stop-Guest
    Restore-Guest -Snapshot $Snapshot
    Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
}
