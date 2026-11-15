# Stands in for GitHub's release API inside the guest, so that the app's
# self-update can be driven before a release exists. The app is pointed at it
# with CODEWIKI_UPDATE_URL=http://localhost:<port>.
#
# It answers three things:
#   /repos/<owner>/<repo>/releases/latest   the release, as GitHub describes one
#   /download/<file>                        the package and its .sha256 file
#   /stop                                   ends the server
# Every request is written to requests.log beside the package.
param(
    # Folder with CodeWiki-<version>-windows-x64.zip and its .sha256 file.
    [Parameter(Mandatory)][string]$Folder,
    [int]$Port = 8765
)
$ErrorActionPreference = "Stop"
$zip = Get-ChildItem $Folder -Filter "CodeWiki-*-windows-x64.zip" | Select-Object -First 1
if (-not $zip) { throw "no package in $Folder" }
if ($zip.Name -notmatch '^CodeWiki-(.+)-windows-x64\.zip$') { throw "cannot read the version from $($zip.Name)" }
$version = $Matches[1]
$base = "http://localhost:$Port"
$log = Join-Path $Folder "requests.log"
$release = [ordered]@{
    tag_name   = "v$version"
    draft      = $false
    prerelease = $false
    assets     = @(
        [ordered]@{ name = $zip.Name; browser_download_url = "$base/download/$($zip.Name)" }
        [ordered]@{ name = "$($zip.Name).sha256"; browser_download_url = "$base/download/$($zip.Name).sha256" }
    )
} | ConvertTo-Json -Depth 4

$listener = [System.Net.HttpListener]::new()
$listener.Prefixes.Add("$base/")
$listener.Start()
Add-Content $log "$(Get-Date -Format o) listening on $base, serving $($zip.Name)"
try {
    while ($listener.IsListening) {
        $context = $listener.GetContext()
        $path = $context.Request.Url.AbsolutePath
        $status = 200
        $body = $null
        if ($path -eq "/stop") {
            $body = [System.Text.Encoding]::UTF8.GetBytes("stopping")
        } elseif ($path -like "/repos/*/releases/latest") {
            $context.Response.ContentType = "application/json"
            $body = [System.Text.Encoding]::UTF8.GetBytes($release)
        } elseif ($path -like "/download/*") {
            $file = Join-Path $Folder ([System.IO.Path]::GetFileName($path))
            if (Test-Path $file -PathType Leaf) {
                $context.Response.ContentType = "application/octet-stream"
                $body = [System.IO.File]::ReadAllBytes($file)
            } else { $status = 404 }
        } else { $status = 404 }
        if ($null -eq $body) { $body = [System.Text.Encoding]::UTF8.GetBytes("not found") }
        $context.Response.StatusCode = $status
        $context.Response.ContentLength64 = $body.Length
        $context.Response.OutputStream.Write($body, 0, $body.Length)
        $context.Response.Close()
        Add-Content $log "$(Get-Date -Format o) $($context.Request.HttpMethod) $path -> $status, $($body.Length) bytes, agent '$($context.Request.UserAgent)'"
        if ($path -eq "/stop") { break }
    }
} finally {
    $listener.Stop()
    Add-Content $log "$(Get-Date -Format o) stopped"
}
