# Samples the guest's resource use while a model server works, for the test
# run's embedder cases (test-release.ps1, Measure-RealEmbedder). It writes one
# line every few seconds to samples.csv in the folder it is given:
#   phase,epoch seconds,server working set MB,guest CPU %,guest free memory MB,GPU 3D %,GPU compute %
# The phase is whatever the first word of phase.txt in the same folder says
# ("load" when the file is missing); the test changes it as the work moves on.
# A GPU value is "na" when the guest has no "GPU Engine" counter. The server's
# processes are those whose program path (or name, when the path cannot be
# read) matches -Pattern. The script ends when stop.flag appears, or after
# -MaxSeconds so that a forgotten run does not sample for ever.
param(
    [Parameter(Mandatory)][string]$Folder,
    [Parameter(Mandatory)][string]$Pattern,
    [int]$IntervalSec = 2,
    [int]$MaxSeconds = 7200
)
$ErrorActionPreference = 'Continue'
$csv = Join-Path $Folder 'samples.csv'
$phaseFile = Join-Path $Folder 'phase.txt'
$stopFile = Join-Path $Folder 'stop.flag'
$invariant = [Globalization.CultureInfo]::InvariantCulture
$end = (Get-Date).AddSeconds($MaxSeconds)

# One counter's value, "na" when the counter does not exist. -Mode Max takes
# the busiest instance (a GPU engine has one instance per process and engine).
function Get-CounterValue {
    param([string]$Path, [string]$Mode = 'Sum')
    try {
        $samples = (Get-Counter -Counter $Path -ErrorAction Stop).CounterSamples
        if ($Mode -eq 'Max') { $value = ($samples | Measure-Object CookedValue -Maximum).Maximum } else { $value = ($samples | Measure-Object CookedValue -Sum).Sum }
        if ($null -eq $value) { 'na' } else { ([double]$value).ToString('0.0', $invariant) }
    } catch { 'na' }
}

while (-not (Test-Path $stopFile) -and (Get-Date) -lt $end) {
    $started = Get-Date
    $phase = 'load'
    try { if (Test-Path $phaseFile) { $phase = ((Get-Content $phaseFile -Raw -ErrorAction Stop) -split '\s+')[0] } } catch { }
    $bytes = 0
    try {
        foreach ($p in @(Get-CimInstance Win32_Process -ErrorAction Stop)) {
            $target = if ($p.ExecutablePath) { $p.ExecutablePath } else { $p.Name }
            if ($target -match $Pattern) { $bytes += [double]$p.WorkingSetSize }
        }
    } catch { }
    $free = 'na'
    try { $free = ([double](Get-CimInstance Win32_OperatingSystem -ErrorAction Stop).FreePhysicalMemory / 1024).ToString('0', $invariant) } catch { }
    $cpu = Get-CounterValue '\Processor(_Total)\% Processor Time'
    $gpu3d = Get-CounterValue '\GPU Engine(*engtype_3D)\Utilization Percentage' 'Max'
    $gpuCompute = Get-CounterValue '\GPU Engine(*engtype_Compute)\Utilization Percentage' 'Max'
    $epoch = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    $line = '{0},{1},{2},{3},{4},{5},{6}' -f $phase, $epoch, ($bytes / 1MB).ToString('0.0', $invariant), $cpu, $free, $gpu3d, $gpuCompute
    try { Add-Content -Path $csv -Value $line -ErrorAction Stop } catch { }
    $left = ($IntervalSec * 1000) - [int]((Get-Date) - $started).TotalMilliseconds
    if ($left -gt 0) { Start-Sleep -Milliseconds $left }
}
