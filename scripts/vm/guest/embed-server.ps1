# Stands in for an embedding server inside the guest, so that CodeWiki's
# OpenAI-compatible embedder can be driven without a real model. CodeWiki is
# pointed at it as the llama.cpp server (llamacpp.url = http://localhost:<port>),
# which reads /v1/models and refuses a model that list lacks.
#
# It answers:
#   GET  /v1/models       one model, as OpenAI-style servers list them
#   POST /v1/embeddings   {"model": ..., "input": text or [texts]} -> one
#                         vector per text, OpenAI's response shape
#   /stop                 ends the server
# A vector is deterministic: each word of the text adds one to a slot chosen
# by a hash of the word, and the result is scaled to length 1. Texts that share
# words are therefore near each other, which is all search needs here.
# Every request is written to requests.log beside this script's folder
# argument, one line each; a POST to /v1/embeddings also says how many texts it
# carried.
param(
    # Folder for requests.log.
    [Parameter(Mandatory)][string]$Folder,
    [int]$Port = 9931,
    [string]$Model = "vm-embed-stub",
    [int]$Dimensions = 16
)
$ErrorActionPreference = "Stop"
$base = "http://localhost:$Port"
$log = Join-Path $Folder "requests.log"

function Get-Vector {
    param([string]$Text, [int]$Size)
    $vector = New-Object 'double[]' $Size
    $words = @(([string]$Text).ToLowerInvariant() -split '[^a-z0-9_]+' | Where-Object { $_ })
    if ($words.Count -eq 0) { $vector[0] = 1.0 }
    foreach ($word in $words) {
        # FNV-1a, 32 bits.
        [long]$hash = 2166136261
        foreach ($character in $word.ToCharArray()) {
            $hash = (($hash -bxor [long][char]$character) * 16777619) % 4294967296
        }
        $vector[[int]($hash % $Size)] += 1.0
    }
    $length = [math]::Sqrt(($vector | ForEach-Object { $_ * $_ } | Measure-Object -Sum).Sum)
    if ($length -gt 0) { for ($i = 0; $i -lt $Size; $i++) { $vector[$i] = [math]::Round($vector[$i] / $length, 6) } }
    , $vector
}

$models = [ordered]@{
    object = "list"
    data   = @([ordered]@{ id = $Model; object = "model"; created = 0; owned_by = "vm-test" })
} | ConvertTo-Json -Depth 4

$listener = [System.Net.HttpListener]::new()
$listener.Prefixes.Add("$base/")
$listener.Start()
Add-Content $log "$(Get-Date -Format o) listening on $base, model $Model, $Dimensions dimensions"
try {
    while ($listener.IsListening) {
        $context = $listener.GetContext()
        $path = $context.Request.Url.AbsolutePath
        $method = $context.Request.HttpMethod
        $status = 200
        $note = ""
        $body = $null
        if ($path -eq "/stop") {
            $body = [System.Text.Encoding]::UTF8.GetBytes("stopping")
        } elseif ($method -eq "GET" -and ($path -eq "/v1/models" -or $path -eq "/models")) {
            $context.Response.ContentType = "application/json"
            $body = [System.Text.Encoding]::UTF8.GetBytes($models)
        } elseif ($method -eq "POST" -and $path -eq "/v1/embeddings") {
            try {
                $reader = New-Object System.IO.StreamReader($context.Request.InputStream, [System.Text.Encoding]::UTF8)
                $request = $reader.ReadToEnd() | ConvertFrom-Json
                $inputs = @($request.input)
                $rows = @()
                for ($i = 0; $i -lt $inputs.Count; $i++) {
                    $rows += [ordered]@{ object = "embedding"; index = $i; embedding = (Get-Vector -Text $inputs[$i] -Size $Dimensions) }
                }
                $answer = [ordered]@{
                    object = "list"
                    data   = $rows
                    model  = $Model
                    usage  = [ordered]@{ prompt_tokens = 0; total_tokens = 0 }
                } | ConvertTo-Json -Depth 6 -Compress
                $context.Response.ContentType = "application/json"
                $body = [System.Text.Encoding]::UTF8.GetBytes($answer)
                $note = " texts=$($inputs.Count) asked-for-model=$($request.model)"
            } catch {
                $status = 400
                $note = " error=$($_.Exception.Message)"
            }
        } else { $status = 404 }
        if ($null -eq $body) { $body = [System.Text.Encoding]::UTF8.GetBytes("not found") }
        $context.Response.StatusCode = $status
        $context.Response.ContentLength64 = $body.Length
        $context.Response.OutputStream.Write($body, 0, $body.Length)
        $context.Response.Close()
        Add-Content $log "$(Get-Date -Format o) $method $path -> $status$note"
        if ($path -eq "/stop") { break }
    }
} finally {
    $listener.Stop()
    Add-Content $log "$(Get-Date -Format o) stopped"
}
