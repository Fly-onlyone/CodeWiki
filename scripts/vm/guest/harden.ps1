# Runs inside the test guest, elevated, under Windows PowerShell 5.1. Turns
# Windows' own protection up as far as its settings go, then prints one JSON
# document that says what is in force and which step failed.
# scripts\vm\test-release.ps1 -Hardened copies it in and runs it before
# anything is installed.
#
# It only turns protection up. It adds no exclusion and allows no program:
# what Windows then stops is the finding.
#
# With -Audit it does one other thing instead: it sets the named attack
# surface reduction rules from "block" to "audit". The run uses that after it
# has recorded that those rules stop CodeWiki's programs, so that the cases
# after it can still be judged. Defender goes on writing down what each such
# rule would have stopped.
param(
    # Rule GUIDs, as one text with commas between them.
    [string]$Audit
)
$ErrorActionPreference = "Stop"
$failed = New-Object System.Collections.Generic.List[string]

function Invoke-Step {
    param([string]$Name, [scriptblock]$Body)
    try { & $Body } catch { $failed.Add("${Name}: $($_.Exception.Message)") }
}

# The highest value of the enumeration a Set-MpPreference parameter takes.
# Asked of the cmdlet, so that a new level in a later Windows is used too.
function Get-Strictest {
    param([string]$Parameter)
    $type = (Get-Command Set-MpPreference).Parameters[$Parameter].ParameterType
    [enum]::GetValues($type) | Sort-Object { [int]$_ } | Select-Object -Last 1
}

# Every attack surface reduction rule, by the GUID Microsoft gives it
# (learn.microsoft.com/defender-endpoint/attack-surface-reduction-rules-reference).
$rules = [ordered]@{
    "56a863a9-875e-4185-98a7-b882c64b5ce5" = "abuse of exploited vulnerable signed drivers"
    "9e6c4e1f-7d60-472f-ba1a-a39ef669e4b2" = "credential stealing from lsass"
    "e6db77e5-3df2-4cf1-b95a-636979351e5b" = "persistence through WMI event subscription"
    "7674ba52-37eb-4a4f-a9a1-f0f9a1619a2c" = "Adobe Reader creating child processes"
    "d4f940ab-401b-4efc-aadc-ad5f3c50688a" = "Office applications creating child processes"
    "be9ba2d9-53ea-4cdc-84e5-9b1eeee46550" = "executable content from email client and webmail"
    "01443614-cd74-433a-b99e-2ecdc07bfc25" = "executable files that do not meet a prevalence, age or trusted list criterion"
    "5beb7efe-fd9a-4556-801d-275e5ffc04cc" = "potentially obfuscated scripts"
    "d3e037e1-3eb8-44c8-a917-57927947596d" = "JavaScript or VBScript launching downloaded executable content"
    "3b576869-a4ec-4529-8536-b80a7769e899" = "Office applications creating executable content"
    "75668c1f-73b5-4cf0-bb93-3ecf5cb7cc84" = "Office applications injecting code into other processes"
    "26190899-1602-49e8-8b27-eb1d0a1ce869" = "Office communication application creating child processes"
    "d1e49aac-8f56-4280-b9ba-993a6d77406c" = "process creations from PSExec and WMI commands"
    "33ddedf1-c6e0-47cb-833e-de6133960387" = "rebooting the machine in Safe Mode"
    "b2b3f03d-6a65-4f7b-a9c7-1c7ef74a9ba4" = "untrusted and unsigned processes that run from USB"
    "c0033c00-d16d-4114-a5a0-dc9b3a7d2ceb" = "copied or impersonated system tools"
    "a8f5898e-1dc8-49a9-9878-85004b8a61e6" = "Webshell creation for servers"
    "92e97fa1-2edf-4476-bdd6-9dd0b4dddc7b" = "Win32 API calls from Office macros"
    "c1db55ab-c21a-4637-bb3f-a12568109d35" = "ransomware (advanced protection)"
}

if ($Audit) {
    Invoke-Step "Rules to audit" {
        $wanted = @($Audit -split "," | ForEach-Object { $_.Trim().ToLowerInvariant() } | Where-Object { $_ })
        $names = @{ 0 = "Disabled"; 1 = "Enabled"; 2 = "AuditMode"; 6 = "Warn" }
        $now = Get-MpPreference
        $ids = @($now.AttackSurfaceReductionRules_Ids)
        $actions = @($now.AttackSurfaceReductionRules_Actions)
        $unknown = @($wanted | Where-Object { $ids -notcontains $_ })
        if ($unknown) { throw "not among the rules that are set: $($unknown -join ', ')" }
        $new = for ($i = 0; $i -lt $ids.Count; $i++) {
            if ($wanted -contains "$($ids[$i])".ToLowerInvariant()) { "AuditMode" } else { $names[[int]$actions[$i]] }
        }
        Set-MpPreference -AttackSurfaceReductionRules_Ids $ids -AttackSurfaceReductionRules_Actions $new
    }
} else {
# Protection is only as good as what it knows: fetch today's definitions.
Invoke-Step "Defender definitions" { Update-MpSignature }
Invoke-Step "Defender scanning" {
    Set-MpPreference -DisableRealtimeMonitoring $false -DisableBehaviorMonitoring $false -DisableIOAVProtection $false `
        -DisableScriptScanning $false -DisableBlockAtFirstSeen $false -DisableArchiveScanning $false
}
Invoke-Step "Defender cloud protection" {
    Set-MpPreference -MAPSReporting (Get-Strictest "MAPSReporting") -SubmitSamplesConsent SendAllSamples `
        -CloudBlockLevel (Get-Strictest "CloudBlockLevel") -CloudExtendedTimeout 50
}
Invoke-Step "Unwanted applications" { Set-MpPreference -PUAProtection Enabled }
Invoke-Step "Network protection" { Set-MpPreference -EnableNetworkProtection Enabled }
Invoke-Step "Controlled folder access" { Set-MpPreference -EnableControlledFolderAccess Enabled }
Invoke-Step "Attack surface reduction" {
    $ids = @($rules.Keys)
    Set-MpPreference -AttackSurfaceReductionRules_Ids $ids -AttackSurfaceReductionRules_Actions @($ids | ForEach-Object { "Enabled" })
}
# User Account Control on "Always notify", on the secure desktop.
Invoke-Step "User Account Control" {
    $key = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System"
    Set-ItemProperty $key -Name EnableLUA -Value 1 -Type DWord
    Set-ItemProperty $key -Name ConsentPromptBehaviorAdmin -Value 2 -Type DWord
    Set-ItemProperty $key -Name PromptOnSecureDesktop -Value 1 -Type DWord
}
# SmartScreen for programs: block what it does not know, with no "Run anyway"
# (the policy "Configure Windows Defender SmartScreen").
Invoke-Step "SmartScreen" {
    $key = "HKLM:\SOFTWARE\Policies\Microsoft\Windows\System"
    New-Item $key -Force | Out-Null
    Set-ItemProperty $key -Name EnableSmartScreen -Value 1 -Type DWord
    Set-ItemProperty $key -Name ShellSmartScreenLevel -Value "Block" -Type String
}
Invoke-Step "Firewall" { Set-NetFirewallProfile -All -Enabled True -DefaultInboundAction Block }
# 1 and 2 both mean "on" (2: without a UEFI lock); Windows 11 ships with 2.
# Turned on here it is in force after the next restart of Windows.
Invoke-Step "LSA protection" {
    $key = "HKLM:\SYSTEM\CurrentControlSet\Control\Lsa"
    if ((Get-ItemProperty $key).RunAsPPL -notin 1, 2) { Set-ItemProperty $key -Name RunAsPPL -Value 2 -Type DWord }
}
}

$ErrorActionPreference = "SilentlyContinue"
$status = Get-MpComputerStatus
$preference = Get-MpPreference
$actions = @($preference.AttackSurfaceReductionRules_Actions)
$uac = Get-ItemProperty "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System"
$smartScreen = Get-ItemProperty "HKLM:\SOFTWARE\Policies\Microsoft\Windows\System"
[pscustomobject]@{
    failed                 = @($failed)
    defenderMode           = $status.AMRunningMode
    realTimeProtection     = $status.RealTimeProtectionEnabled
    behaviorMonitor        = $status.BehaviorMonitorEnabled
    tamperProtection       = $status.IsTamperProtected
    signatures             = "$($status.AntivirusSignatureVersion) of $($status.AntivirusSignatureLastUpdated)"
    cloudReporting         = [int]$preference.MAPSReporting
    sampleSubmission       = [int]$preference.SubmitSamplesConsent
    cloudBlockLevel        = [int]$preference.CloudBlockLevel
    cloudTimeout           = [int]$preference.CloudExtendedTimeout
    unwantedApplications   = [int]$preference.PUAProtection
    networkProtection      = [int]$preference.EnableNetworkProtection
    controlledFolderAccess = [int]$preference.EnableControlledFolderAccess
    reductionRules         = @($preference.AttackSurfaceReductionRules_Ids).Count
    reductionRulesBlocking = @($actions | Where-Object { [int]$_ -eq 1 }).Count
    reductionRulesAuditing = @(for ($i = 0; $i -lt $actions.Count; $i++) { if ([int]$actions[$i] -eq 2) { "$(@($preference.AttackSurfaceReductionRules_Ids)[$i])" } })
    exclusions             = @(@($preference.ExclusionPath) + @($preference.ExclusionProcess) | Where-Object { $_ })
    uacPrompt              = $uac.ConsentPromptBehaviorAdmin
    uacSecureDesktop       = $uac.PromptOnSecureDesktop
    smartScreen            = "$($smartScreen.EnableSmartScreen) $($smartScreen.ShellSmartScreenLevel)"
    firewall               = @(Get-NetFirewallProfile | ForEach-Object { "$($_.Name)=$($_.Enabled)/$($_.DefaultInboundAction)" })
    lsaProtection          = (Get-ItemProperty "HKLM:\SYSTEM\CurrentControlSet\Control\Lsa").RunAsPPL
    # Without the process scope: that one is how this script was started.
    executionPolicy        = @(Get-ExecutionPolicy -List | Where-Object { $_.Scope -ne "Process" } | ForEach-Object { "$($_.Scope)=$($_.ExecutionPolicy)" })
} | ConvertTo-Json -Depth 4
