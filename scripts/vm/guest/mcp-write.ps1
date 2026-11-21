# Writes one note's explanation through the MCP bridge over stdio, as a coding
# agent does, with no agent involved: list_waiting_notes, get_note_brief,
# submit_note. Prints one "key: value" line per step; the host reads them.
#   -Mcp   codewiki-mcp.exe
#   -Repo  the repository, the bridge's working folder
#   -Marker  a word that is not in the wiki; the submitted text holds it
param(
    [Parameter(Mandatory)][string]$Mcp,
    [Parameter(Mandatory)][string]$Repo,
    [string]$Marker = "vmexplained"
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

    $queue = Invoke-Tool "list_waiting_notes" @{ limit = 1 }
    "queue: $(($queue -split "`n" | Select-Object -First 2) -join ' | ')"
    if ($queue -notmatch '(?m)^1\. (.+?)\s{2}\(') { "result: no note waits"; return }
    $path = $Matches[1].Trim()
    "note: $path"

    $brief = Invoke-Tool "get_note_brief" @{ note = $path }
    if ($brief -notmatch '(?m)^base: (\S+)') { "result: no base in the brief"; return }
    $base = $Matches[1]
    "base: $base"
    if ($brief -notmatch '(?m)^- ([^:\r\n]+): ') { "result: no section of the agent's in the brief"; return }
    $heading = $Matches[1].Trim()
    "section: $heading"

    $text = "$Marker was written through the MCP bridge by the release test. It says in plain words what this part is for, and nothing more."
    # An old base, as when the note changed after the brief was read.
    $stale = Invoke-Tool "submit_note" @{ note = $path; base = "0000000000000000"; sections = @(@{ heading = $heading; text = $text }); model = "vm-test" }
    "stale base: $($stale -replace '\s+', ' ')"
    # A heading the agent does not own.
    $wrong = Invoke-Tool "submit_note" @{ note = $path; base = $base; sections = @(@{ heading = "Source"; text = $text }); model = "vm-test" }
    "wrong section: $(($wrong -split "`n" | Select-Object -First 2) -join ' ' )"
    $saved = Invoke-Tool "submit_note" @{ note = $path; base = $base; sections = @(@{ heading = $heading; text = $text }); model = "vm-test" }
    "submit: $($saved -replace '\s+', ' ')"

    $file = Join-Path $Repo ("obsidian\" + ($path -replace '/', '\'))
    $held = (Test-Path $file) -and ((Get-Content $file -Raw) -match [regex]::Escape($Marker))
    "file holds the text: $held"
    $again = Invoke-Tool "get_note_brief" @{ note = $path }
    "brief again, section written by an agent: $($again -match [regex]::Escape("- ${heading}: an agent wrote it"))"
} finally {
    $bridge.StandardInput.Close()
    if (-not $bridge.WaitForExit(10000)) { $bridge.Kill() }
}
