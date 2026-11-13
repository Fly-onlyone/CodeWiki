# Runs inside the test guest under Windows PowerShell 5.1 and prints one JSON
# document that says what the machine has and what CodeWiki left on it.
# scripts\vm\test-release.ps1 copies it in and calls it between test cases.
param(
    [string]$Target = (Join-Path $env:LOCALAPPDATA "Programs\CodeWiki"),
    # Leave out what is slow to ask Windows for (system facts, Defender, the
    # event log). For use while the app is running and the guest is busy.
    [switch]$Quick
)
$ErrorActionPreference = "SilentlyContinue"

function Get-Tree {
    param([string]$Path, [int]$Depth = 3)
    if (-not (Test-Path $Path)) { return $null }
    $root = (Resolve-Path $Path).Path
    @(Get-ChildItem -LiteralPath $root -Recurse -Depth $Depth -Force | ForEach-Object {
            [pscustomobject]@{
                path   = $_.FullName.Substring($root.Length).TrimStart("\")
                folder = $_.PSIsContainer
                bytes  = if ($_.PSIsContainer) { $null } else { $_.Length }
            }
        })
}

$os = if ($Quick) { $null } else { Get-CimInstance Win32_OperatingSystem }
$current = Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion"
$system32 = Join-Path $env:WINDIR "System32"

$runtime = foreach ($dll in "vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll", "ucrtbase.dll", "d3d11.dll", "dcomp.dll") {
    $file = Join-Path $system32 $dll
    [pscustomobject]@{
        name    = $dll
        present = Test-Path $file
        version = if (Test-Path $file) { (Get-Item $file).VersionInfo.FileVersion } else { $null }
    }
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$pathEntries = @($userPath -split ";" | Where-Object { $_ })

$link = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\CodeWiki.lnk"
$shortcut = $null
if (Test-Path $link) {
    $lnk = (New-Object -ComObject WScript.Shell).CreateShortcut($link)
    $shortcut = [pscustomobject]@{
        target           = $lnk.TargetPath
        arguments        = $lnk.Arguments
        workingDirectory = $lnk.WorkingDirectory
        icon             = $lnk.IconLocation
    }
}

$processes = @(Get-Process | Where-Object { $_.ProcessName -like "codewiki*" } | ForEach-Object {
        [pscustomobject]@{
            name       = $_.ProcessName
            id         = $_.Id
            session    = $_.SessionId
            path       = $_.Path
            title      = $_.MainWindowTitle
            hasWindow  = $_.MainWindowHandle -ne 0
            started    = $_.StartTime.ToString("o")
            cpuSeconds = [math]::Round($_.TotalProcessorTime.TotalSeconds, 2)
            memoryMb   = [math]::Round($_.WorkingSet64 / 1MB, 1)
        }
    })
$ids = @($processes | ForEach-Object { $_.id })
# A console host whose parent is a CodeWiki program means that program owns a console.
$consoles = @(Get-CimInstance Win32_Process -Filter "Name = 'conhost.exe' OR Name = 'OpenConsole.exe'" |
        Where-Object { $ids -contains $_.ParentProcessId } |
        ForEach-Object { [pscustomobject]@{ name = $_.Name; id = $_.ProcessId; parent = $_.ParentProcessId } })

$dataDir = Join-Path $env:LOCALAPPDATA "CodeWiki"
$prefsDir = Join-Path $env:APPDATA "CodeWiki"
$backend = $null
$backendFile = Join-Path $dataDir "backend.json"
if (Test-Path $backendFile) {
    $json = Get-Content $backendFile -Raw | ConvertFrom-Json
    $backend = [ordered]@{
        url         = $json.url
        pid         = $json.pid
        pidAlive    = [bool](Get-Process -Id $json.pid)
        tokenLength = "$($json.token)".Length
    }
    try {
        $health = Invoke-WebRequest -UseBasicParsing -TimeoutSec 5 "$($json.url)/v1/health"
        $backend.health = [int]$health.StatusCode
        $backend.healthBody = $health.Content
    } catch {
        $backend.health = "$($_.Exception.Message)"
    }
    try {
        Invoke-WebRequest -UseBasicParsing -TimeoutSec 5 "$($json.url)/v1/repos" | Out-Null
        $backend.reposWithoutToken = 200
    } catch {
        $backend.reposWithoutToken = [int]$_.Exception.Response.StatusCode
    }
    try {
        $repos = Invoke-WebRequest -UseBasicParsing -TimeoutSec 5 "$($json.url)/v1/repos" -Headers @{ Authorization = "Bearer $($json.token)" }
        $backend.repos = $repos.Content
    } catch {
        $backend.repos = "$($_.Exception.Message)"
    }
}

$events = @()
if (-not $Quick) {
    $crashLogs = @{
        LogName      = "Application"
        ProviderName = "Application Error", "Application Hang", "Windows Error Reporting"
        StartTime    = (Get-Date).AddHours(-12)
    }
    $events = @(Get-WinEvent -FilterHashtable $crashLogs |
            Where-Object { $_.Message -match "codewiki" } |
            Select-Object -First 20 | ForEach-Object {
                [pscustomobject]@{
                    time     = $_.TimeCreated.ToString("o")
                    provider = $_.ProviderName
                    id       = $_.Id
                    level    = $_.LevelDisplayName
                    message  = if ($_.Message.Length -gt 900) { $_.Message.Substring(0, 900) } else { $_.Message }
                }
            })
}

if ($Quick) {
    [ordered]@{
        install   = [ordered]@{ target = $Target; exists = Test-Path $Target; files = Get-Tree $Target 1 }
        userPath  = [ordered]@{
            entries     = $pathEntries
            targetCount = @($pathEntries | Where-Object { $_ -eq $Target }).Count
            processSees = @($env:Path -split ";" | Where-Object { $_ -eq $Target }).Count
        }
        shortcut  = $shortcut
        processes = $processes
        consoles  = $consoles
        backend   = $backend
        dataDir   = [ordered]@{ path = $dataDir; files = Get-Tree $dataDir 2 }
        prefsDir  = [ordered]@{ path = $prefsDir; files = Get-Tree $prefsDir 2 }
        events    = $events
    } | ConvertTo-Json -Depth 6
    return
}

[ordered]@{
    os          = [ordered]@{
        caption = $os.Caption
        version = $os.Version
        build   = "$($current.CurrentBuild).$($current.UBR)"
        display = $current.DisplayVersion
        edition = $current.EditionID
    }
    powershell  = [ordered]@{
        version = $PSVersionTable.PSVersion.ToString()
        pwsh    = (Get-Command pwsh).Source
        policy  = @(Get-ExecutionPolicy -List | ForEach-Object { [pscustomobject]@{ scope = "$($_.Scope)"; policy = "$($_.ExecutionPolicy)" } })
    }
    session     = [ordered]@{
        id          = (Get-Process -Id $PID).SessionId
        interactive = [Environment]::UserInteractive
        user        = "$env:USERDOMAIN\$env:USERNAME"
        elevated    = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    }
    runtime     = @($runtime)
    git         = (Get-Command git).Source
    adapters    = @(Get-CimInstance Win32_VideoController | ForEach-Object {
            [pscustomobject]@{ name = $_.Name; driver = $_.DriverVersion; vendor = $_.AdapterCompatibility; mode = $_.VideoModeDescription }
        })
    defender    = (Get-MpComputerStatus | Select-Object AntivirusEnabled, RealTimeProtectionEnabled)
    install     = [ordered]@{
        target = $Target
        exists = Test-Path $Target
        files  = Get-Tree $Target 1
    }
    userPath    = [ordered]@{
        entries      = $pathEntries
        targetCount  = @($pathEntries | Where-Object { $_ -eq $Target }).Count
        processSees  = @($env:Path -split ";" | Where-Object { $_ -eq $Target }).Count
    }
    shortcut    = $shortcut
    processes   = $processes
    consoles    = $consoles
    backend     = $backend
    dataDir     = [ordered]@{ path = $dataDir; files = Get-Tree $dataDir 2 }
    prefsDir    = [ordered]@{ path = $prefsDir; files = Get-Tree $prefsDir 2 }
    events      = $events
} | ConvertTo-Json -Depth 6
