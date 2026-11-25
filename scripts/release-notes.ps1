# Write the notes of a GitHub Release as Markdown:
#   - the file to download,
#   - the highlights, copied from docs\releases\<version>.md,
#   - the features and the fixes, from the conventional commit subjects
#     since the previous release,
#   - every other commit, in a folded list,
#   - how to install and how to check a download,
#   - the link to the full changelog.
#
# GitHub's generated notes list merged pull requests, and CodeWiki's work
# lands by local merges, so they would list nothing. The release workflow
# publishes what this script writes, and an older release gets the same
# notes with `gh release edit <tag> --notes-file <file>`.
#
# Run from anywhere in the repository:
#   pwsh scripts\release-notes.ps1 -Tag v0.3.1 -OutFile <file>
# Without -Tag, the notes cover HEAD under the version in Cargo.toml: a
# preview of the next release.
param(
    # The release tag, v<version>. Leave it out for HEAD.
    [string]$Tag,
    [string]$OutFile = "dist/release-notes.md",
    [string]$Repo = "Fly-onlyone/CodeWiki"
)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

if ($Tag) {
    if ($Tag -notmatch '^v(.+)$') { throw "a release tag starts with v: $Tag" }
    $version = $Matches[1]
    $ref = $Tag
    git -C $root rev-parse --verify --quiet "$Tag^{commit}" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "no tag $Tag in this repository" }
} else {
    # Same pattern as scripts/package.ps1.
    $manifest = Get-Content (Join-Path $root "Cargo.toml") -Raw
    if ($manifest -notmatch '(?m)^version\s*=\s*"([^"]+)"') { throw "no workspace version in Cargo.toml" }
    $version = $Matches[1]
    $Tag = "v$version"
    $ref = "HEAD"
}

# A release covers the commits since the release before it. A final release
# counts from the previous final release, so its notes also hold what its
# pre-releases brought; a pre-release counts from any earlier tag.
$describe = @("describe", "--tags", "--abbrev=0")
if (-not $version.Contains("-")) { $describe += @("--exclude", "*-*") }
$previous = git -C $root @describe "$ref^" 2>$null
if ($LASTEXITCODE -ne 0) { $previous = $null }
$global:LASTEXITCODE = 0
$range = if ($previous) { "$previous..$ref" } else { $ref }

# The setup program came with 0.3.1. A release without it has only the zip.
git -C $root cat-file -e "${ref}:scripts/setup" 2>$null
$hasSetup = $LASTEXITCODE -eq 0
$global:LASTEXITCODE = 0

$zip = "CodeWiki-$version-windows-x64.zip"
$setup = "CodeWiki-$version-windows-x64-setup.exe"
$download = "https://github.com/$Repo/releases/download/$Tag"

# Markdown in a commit subject would change its meaning: `*.old???` turns to
# italics, and `@name` notifies a GitHub user. Code spans stay as written.
function Format-Subject([string]$text) {
    $parts = $text -split '`'
    if ($parts.Count % 2 -eq 0) {
        # An unpaired backtick: no code span at all.
        $parts = @($text)
        $escapeTicks = $true
    } else {
        $escapeTicks = $false
    }
    $out = for ($i = 0; $i -lt $parts.Count; $i++) {
        if ($i % 2 -eq 1) { $parts[$i]; continue }
        [regex]::Replace($parts[$i], '(?<![\w`])@[\w./-]+|[\\*_\[\]<>~`]', {
                param($m)
                if ($m.Value.StartsWith("@")) { return "``$($m.Value)``" }
                if ($m.Value -eq '`' -and -not $escapeTicks) { return $m.Value }
                return "\$($m.Value)"
            })
    }
    $out -join '`'
}

function Format-Link([string]$sha) {
    "([``$($sha.Substring(0, 7))``](https://github.com/$Repo/commit/$sha))"
}

$features = [System.Collections.Generic.List[object]]::new()
$fixes = [System.Collections.Generic.List[object]]::new()
$others = [System.Collections.Generic.List[object]]::new()
# Oldest first, so the changes of one part read in the order they were made.
$lines = git -C $root log --no-merges --reverse --format="%H%x1f%s" $range
if ($LASTEXITCODE -ne 0) { throw "git log failed for $range" }
foreach ($line in $lines) {
    $sha, $subject = $line -split "\x1f", 2
    if ($subject -match '^(?<type>\w+)(\((?<scope>[^)]+)\))?(?<bang>!)?: (?<text>.+)$') {
        $commit = [pscustomobject]@{
            Sha = $sha; Type = $Matches.type; Scope = $Matches.scope
            Breaking = [bool]$Matches.bang; Text = $Matches.text
        }
    } else {
        $commit = [pscustomobject]@{ Sha = $sha; Type = ""; Scope = ""; Breaking = $false; Text = $subject }
    }
    switch ($commit.Type) {
        "feat" { $features.Add($commit) }
        { $_ -in "fix", "perf" } { $fixes.Add($commit) }
        default { $others.Add($commit) }
    }
}

# - **app:** show the new version ([`abc1234`](...))
function Format-Change($commit) {
    $scope = if ($commit.Scope) { "**$(Format-Subject $commit.Scope):** " } else { "" }
    $breaking = if ($commit.Breaking) { "**Breaking:** " } else { "" }
    "- $scope$breaking$(Format-Subject $commit.Text) $(Format-Link $commit.Sha)"
}

# - **test(vm):** add IS7 ([`abc1234`](...)), with the type, since the list mixes them.
function Format-Other($commit) {
    $label = if ($commit.Type) {
        $type = if ($commit.Scope) { "$($commit.Type)($($commit.Scope))" } else { $commit.Type }
        "**$(Format-Subject $type)$(if ($commit.Breaking) { '!' }):** "
    } else { "" }
    "- $label$(Format-Subject $commit.Text) $(Format-Link $commit.Sha)"
}

$notes = [System.Collections.Generic.List[string]]::new()
# Settings -> Apps, kept out of the source so it stays ASCII.
$arrow = [char]0x2192

if ($hasSetup) {
    $notes.Add("**Download [``$setup``]($download/$setup)** and double-click it. The zip is for self-update and for installing by hand.")
} else {
    $notes.Add("**Download [``$zip``]($download/$zip)**, unpack it and double-click ``install.cmd``.")
}

$highlights = Join-Path $root "docs/releases/$version.md"
if (Test-Path $highlights) {
    $notes.Add("")
    $notes.Add("## Highlights")
    $notes.Add("")
    $notes.Add((Get-Content $highlights -Raw).Trim())
} else {
    $message = "no highlights for $version (docs/releases/$version.md): the notes leave them out"
    if ($env:GITHUB_ACTIONS -eq "true") { Write-Output "::warning::$message" } else { Write-Warning $message }
}

# Sort-Object -Stable keeps the oldest-first order inside one scope.
foreach ($section in @(@("New", $features), @("Fixes", $fixes))) {
    $title, $commits = $section
    if ($commits.Count -eq 0) { continue }
    $notes.Add("")
    $notes.Add("## $title")
    $notes.Add("")
    foreach ($commit in ($commits | Sort-Object -Stable -Property Scope)) { $notes.Add((Format-Change $commit)) }
}

if ($others.Count -gt 0) {
    $notes.Add("")
    $notes.Add("<details><summary>Other changes ($($others.Count)): refactoring, tests, docs, build</summary>")
    $notes.Add("")
    foreach ($commit in $others) { $notes.Add((Format-Other $commit)) }
    $notes.Add("")
    $notes.Add("</details>")
}

$notes.Add("")
$notes.Add("## Install")
$notes.Add("")
if ($hasSetup) {
    $notes.Add("Requires Windows 11; no administrator rights.")
    $notes.Add("")
    $notes.Add("1. Download ``$setup`` and double-click it.")
    $notes.Add("2. The setup program is not signed, so Windows SmartScreen shows ""Windows protected your PC"": click **More info**, then **Run anyway**.")
    $notes.Add("3. Follow the wizard. ""Start CodeWiki"" is checked on the last page. Open a new terminal before you start a coding agent: the install changed your ``PATH``.")
    $notes.Add("")
    $notes.Add("CodeWiki goes to ``%LOCALAPPDATA%\Programs\CodeWiki``, with a Start Menu shortcut and an entry in Settings $arrow Apps. Uninstall it there: that keeps your data (``%LOCALAPPDATA%\CodeWiki``) and settings (``%APPDATA%\CodeWiki``); delete those two folders by hand to remove them as well.")
    $notes.Add("")
    $notes.Add("**The zip** installs the same without the Settings $arrow Apps entry: unpack ``$zip`` and double-click ``install.cmd``. ``uninstall.cmd`` in the install folder removes a zip installation; ``uninstall.cmd -PurgeData`` also deletes the index, the logs and the settings.")
    $check = $setup
} else {
    $notes.Add("Requires Windows 11; no administrator rights.")
    $notes.Add("")
    $notes.Add("1. Download ``$zip`` and unpack it.")
    $notes.Add("2. Double-click ``install.cmd``. It copies the programs to ``%LOCALAPPDATA%\Programs\CodeWiki``, adds that folder to your ``PATH`` and creates a Start Menu shortcut.")
    $notes.Add("")
    $notes.Add("``uninstall.cmd`` in that folder reverses it; ``uninstall.cmd -PurgeData`` also deletes the index, the logs and the settings. Newer releases also come as a setup program: see the [latest release](https://github.com/$Repo/releases/latest).")
    $check = $zip
}
$notes.Add("")
$notes.Add("To check a download, put it beside its ``.sha256`` file and run this in PowerShell; it prints ``True`` when they match:")
$notes.Add("")
$notes.Add('```powershell')
$notes.Add("`$f = ""$check""")
$notes.Add('(Get-FileHash $f).Hash -eq (Get-Content "$f.sha256").Split(" ")[0]')
$notes.Add('```')

$notes.Add("")
# A preview has no tag yet: its link ends at the commit.
$end = if ($ref -eq "HEAD") { git -C $root rev-parse HEAD } else { $Tag }
if ($previous) {
    $notes.Add("**Full Changelog**: https://github.com/$Repo/compare/$previous...$end")
} else {
    $notes.Add("**Full Changelog**: https://github.com/$Repo/commits/$end")
}

$dir = Split-Path -Parent $OutFile
if ($dir) { New-Item -ItemType Directory -Force $dir | Out-Null }
# UTF-8 without a byte order mark, LF line ends.
[System.IO.File]::WriteAllText(
    $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutFile),
    ($notes -join "`n") + "`n",
    [System.Text.UTF8Encoding]::new($false))
$since = if ($previous) { "since $previous" } else { "first release" }
"$Tag ($since): $($features.Count) new, $($fixes.Count) fixes, $($others.Count) other -> $OutFile"
