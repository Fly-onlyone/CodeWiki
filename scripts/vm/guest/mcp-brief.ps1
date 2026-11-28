# Reads one note's brief through the MCP bridge over stdio, as a coding agent
# does, and says what the brief holds about the files under the note. With
# -Submit it also writes the note's first section, naming every file of the
# brief as read. Prints one "key: value" line per fact; the host reads them.
#   -Mcp     codewiki-mcp.exe
#   -Repo    the repository, the bridge's working folder
#   -Note    the note; without it, the first note that waits
#   -Submit  write an explanation after the brief
#   -Marker  a word that is not in the wiki; the submitted text holds it
param(
    [Parameter(Mandatory)][string]$Mcp,
    [Parameter(Mandatory)][string]$Repo,
    [string]$Note = "",
    [switch]$Submit,
    [string]$Marker = "vmbrief"
)
$ErrorActionPreference = "Stop"
$start = New-Object System.Diagnostics.ProcessStartInfo $Mcp
$start.RedirectStandardInput = $true
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
$start.WorkingDirectory = $Repo
$bridge = [System.Diagnostics.Process]::Start($start)
# Nothing reads standard error, so a full pipe must not stop the bridge.
[void]$bridge.StandardError.ReadToEndAsync()
$script:next = 1

# Send one request and return the message that answers it.
function Invoke-Rpc {
    param([string]$Method, $Params)
    $id = $script:next
    $script:next += 1
    $request = [ordered]@{ jsonrpc = "2.0"; id = $id; method = $Method }
    if ($null -ne $Params) { $request.params = $Params }
    $bridge.StandardInput.WriteLine(($request | ConvertTo-Json -Depth 10 -Compress))
    $bridge.StandardInput.Flush()
    $deadline = (Get-Date).AddSeconds(120)
    while ((Get-Date) -lt $deadline) {
        $reading = $bridge.StandardOutput.ReadLineAsync()
        if (-not $reading.Wait(120000) -or $null -eq $reading.Result) { break }
        try { $message = $reading.Result | ConvertFrom-Json } catch { continue }
        if ($message.id -eq $id) { return $message }
    }
    throw "no answer to $Method"
}

# The text of a tool call's answer.
function Invoke-Tool {
    param([string]$Name, $Arguments)
    $answer = Invoke-Rpc "tools/call" @{ name = $Name; arguments = $Arguments }
    if ($answer.error) { return "tool error: $($answer.error.message)" }
    (@($answer.result.content) | ForEach-Object { $_.text }) -join "`n"
}

try {
    [void](Invoke-Rpc "initialize" @{ protocolVersion = "2024-11-05"; capabilities = @{}; clientInfo = @{ name = "vm-test"; version = "0" } })
    $bridge.StandardInput.WriteLine('{"jsonrpc":"2.0","method":"notifications/initialized"}')
    $bridge.StandardInput.Flush()

    if (-not $Note) {
        $queue = Invoke-Tool "list_waiting_notes" @{ limit = 1 }
        "queue: $(($queue -split "`n" | Select-Object -First 2) -join ' | ')"
        if ($queue -notmatch '(?m)^1\. (.+?)\s{2}\(') { "result: no note waits"; return }
        $Note = $Matches[1].Trim()
    }
    "note: $Note"

    $brief = Invoke-Tool "get_note_brief" @{ note = $Note }
    if ($brief -notmatch '(?m)^base: (\S+)') { "result: no base in the brief: $(($brief -split "`n" | Select-Object -First 2) -join ' | ')"; return }
    $base = $Matches[1]
    "base: $base"
    # The line the bridge puts on top when a file under the note differs from
    # what the last sync recorded.
    "stop: $($brief -match '(?m)^STOP: ')"
    "behind: $($brief -match '(?m)^The explanation may be behind the code:')"
    # The files to read are the "- <path>" lines after their heading, up to
    # the first empty line.
    $files = @()
    $changed = @()
    $inFiles = $false
    foreach ($line in ($brief -split "`r?`n")) {
        if ($line -match '^Read these files before you write:') { $inFiles = $true; continue }
        if (-not $inFiles) { continue }
        if ($line -notmatch '^- (.+)$') { break }
        $entry = $Matches[1]
        $path = ($entry -replace '\s+\(changed since the note was explained\)\s*$', '').Trim()
        $files += $path
        if ($entry -match '\(changed since the note was explained\)') { $changed += $path }
    }
    "files: $($files -join ', ')"
    "changed files: $($changed.Count)$(if ($changed) { ' (' + ($changed -join ', ') + ')' })"

    if ($Submit) {
        if ($brief -notmatch '(?m)^- ([^:\r\n]+): ') { "result: no section of the agent's in the brief"; return }
        $heading = $Matches[1].Trim()
        "section: $heading"
        $text = "$Marker was written through the MCP bridge by the release test. It says in plain words what this part is for, and nothing more."
        $saved = Invoke-Tool "submit_note" @{ note = $Note; base = $base; sections = @(@{ heading = $heading; text = $text }); read = @($files); model = "vm-test" }
        "submit: $($saved -replace '\s+', ' ')"
    }
} finally {
    $bridge.StandardInput.Close()
    if (-not $bridge.WaitForExit(10000)) { $bridge.Kill() }
}
