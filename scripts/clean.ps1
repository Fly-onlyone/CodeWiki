# Free the disk space builds leave behind. Safe to run at any time: everything
# removed here is rebuilt on demand.
#
#   pwsh scripts\clean.ps1            prune debug artifacts, remove spike build folders
#   pwsh scripts\clean.ps1 -DryRun    list what the prune would remove; remove nothing
#   pwsh scripts\clean.ps1 -All       remove every build artifact, release ones and dist\ too
#   pwsh scripts\clean.ps1 -Docker    also Docker's build cache (all projects)
#
# The prune keeps what the RustRover run configurations in .run\ build in the
# dev profile, so that starting the app afterwards compiles nothing:
# `codewiki backend` (cargo build -p codewiki) and `codewiki-app`
# (cargo run -p codewiki-app). It makes those two builds first, reads the
# artifacts cargo reports for them, and deletes every other debug artifact:
# test binaries, clippy output, stale versions, builds made from worktrees.
# If either build fails, nothing is deleted. Release artifacts are left alone.
#
# Cargo's build-dir layout is internal and may change, so nothing here names
# its folders. Every intermediate artifact carries a 16-digit hex unit hash in
# its name, and an entry is kept when its hash belongs to one of the two
# builds. A hash missed here costs a recompile of that one unit, never a broken
# build.
param([switch]$All, [switch]$DryRun, [switch]$Docker)
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if ($All -and $DryRun) { throw "-DryRun lists what the prune would remove; -All prunes nothing" }

# One cargo invocation per run configuration: features are unified per
# invocation, so building both packages at once could hash differently.
$RunConfigBuilds = @(
    @("build", "-p", "codewiki"),
    @("build", "-p", "codewiki-app")
)

# A unit hash at the start of a name or after a dash, not inside a longer run
# of hex digits.
$HashPattern = [regex]'(?:^|-)([0-9a-f]{16})(?=$|[^0-9a-f])'

function Size([string]$path) {
    if (-not (Test-Path $path)) { return 0 }
    (Get-ChildItem $path -Recurse -File -Force -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum
}

# The unit hashes in each segment of `path`.
function Get-Hashes([string]$path) {
    foreach ($segment in $path -split '[\\/]') {
        foreach ($match in $HashPattern.Matches($segment)) { $match.Groups[1].Value }
    }
}

function Get-Metadata {
    $metadata = cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw "cargo metadata failed" }
    $metadata
}

# The debug folders of the target dir and of the build dir, when they exist.
function Get-BuildDirs($metadata) {
    @($metadata.target_directory, $metadata.build_directory) |
        Where-Object { $_ } |
        ForEach-Object { Join-Path $_ "debug" } |
        Sort-Object -Unique |
        Where-Object { Test-Path $_ }
}

# Make the run configurations' builds and return the unit hashes they use,
# with the targets whose reported files carry no hash.
function Get-KeepHashes($metadata) {
    $names = @{}
    foreach ($package in $metadata.packages) { $names[$package.id] = $package.name }
    $keep = [System.Collections.Generic.HashSet[string]]::new()
    $roots = [System.Collections.Generic.List[object]]::new()
    foreach ($build in $RunConfigBuilds) {
        # Progress and rendered diagnostics go to the console; stdout is JSON.
        $lines = & cargo @build --message-format=json-render-diagnostics
        if ($LASTEXITCODE -ne 0) { throw "cargo $($build -join ' ') failed; nothing was removed" }
        foreach ($line in $lines) {
            if (-not $line.StartsWith("{")) { continue }
            $message = $line | ConvertFrom-Json
            # Both messages are emitted for units that were already fresh.
            $paths = switch ($message.reason) {
                "compiler-artifact" { @($message.filenames) + @($message.executable) }
                "build-script-executed" { @($message.out_dir) }
                default { @() }
            }
            foreach ($path in $paths | Where-Object { $_ }) {
                $hashes = @(Get-Hashes $path)
                if ($hashes.Count -gt 0) {
                    $hashes | ForEach-Object { [void]$keep.Add($_) }
                } elseif ($names.ContainsKey($message.package_id)) {
                    $roots.Add([pscustomobject]@{
                        Package = $names[$message.package_id]
                        Kinds = @($message.target.kind)
                        Target = $message.target.name
                    })
                }
            }
        }
    }
    # One object: PowerShell would unroll the collections themselves.
    [pscustomobject]@{ Keep = $keep; Roots = $roots }
}

# A requested package's own files are reported where cargo copies them for
# the user, without a hash, and this cargo keeps no hashed copy of a binary at
# all. Its unit is found by its fingerprint instead: a `<package>-<hash>`
# folder holding a `<kind>-<target>` file, such as `bin-codewiki-app`; a test
# of the same target holds `test-bin-codewiki-app` and is not kept. The check
# that clippy makes of the target matches as well and is kept; it is small.
function Add-RootHashes($keep, $roots, $dirs) {
    foreach ($root in $roots | Sort-Object Package, Target -Unique) {
        $pattern = [regex]('^' + [regex]::Escape($root.Package) + '-([0-9a-f]{16})$')
        $units = $dirs | ForEach-Object {
            Get-ChildItem $_ -Recurse -Directory -Force -Filter "$($root.Package)-*"
        }
        foreach ($unit in $units) {
            $match = $pattern.Match($unit.Name)
            if (-not $match.Success) { continue }
            foreach ($kind in $root.Kinds) {
                if (Test-Path -LiteralPath (Join-Path $unit.FullName "$kind-$($root.Target)")) {
                    [void]$keep.Add($match.Groups[1].Value)
                }
            }
        }
    }
}

# Every entry under `dir` whose unit hash is not kept. A kept entry is left
# whole; an entry with no hash in its name is looked into.
function Get-Unkept([string]$dir, $keep) {
    foreach ($entry in Get-ChildItem $dir -Force) {
        $hashes = @(Get-Hashes $entry.Name)
        if ($hashes.Count -gt 0) {
            if (-not ($hashes | Where-Object { $keep.Contains($_) })) { $entry }
        } elseif ($entry.PSIsContainer) {
            Get-Unkept $entry.FullName $keep
        }
    }
}

function Remove-Unkept {
    $metadata = Get-Metadata
    $dirs = @(Get-BuildDirs $metadata)
    if ($dirs.Count -eq 0) { return }
    $marked = Get-KeepHashes $metadata
    $keep = $marked.Keep
    Add-RootHashes $keep $marked.Roots $dirs
    # Proton Drive leaves "(# Name clash ... #)" copies that rustc picks over the
    # real artifact, which breaks linking. They go whatever their hash.
    $clashes = $dirs | ForEach-Object { Get-ChildItem $_ -Recurse -Force -Filter "*Name clash*" }
    $doomed = @($clashes) + @($dirs | ForEach-Object { Get-Unkept $_ $keep }) |
        Where-Object { $_ } |
        Sort-Object FullName -Unique
    foreach ($entry in $doomed) {
        if (-not (Test-Path -LiteralPath $entry.FullName)) { continue }
        if ($DryRun) {
            "{0,8:N1} MB  {1}" -f ((Size $entry.FullName) / 1MB), (Resolve-Path -Relative $entry.FullName)
        } else {
            Remove-Item -LiteralPath $entry.FullName -Recurse -Force
        }
    }
    if ($DryRun) {
        $total = ($doomed | ForEach-Object { Size $_.FullName } | Measure-Object -Sum).Sum
        "would free {0:N1} GB from {1} entries; keeping {2} unit hashes" -f ($total / 1GB), $doomed.Count, $keep.Count
    }
}

$before = (Size "target") + (Size "spikes") + (Size "dist")

if ($All) { cargo clean } else { Remove-Unkept }
if ($DryRun) { return }
Get-ChildItem "spikes" -Directory | ForEach-Object {
    $target = Join-Path $_.FullName "target"
    if (Test-Path $target) { Remove-Item -Recurse -Force $target }
}
if ($All -and (Test-Path "dist")) { Remove-Item -Recurse -Force "dist" }
if ($Docker) { docker builder prune --force | Select-Object -Last 1 }

$after = (Size "target") + (Size "spikes") + (Size "dist")
"freed {0:N1} GB; build folders now use {1:N1} GB" -f (($before - $after) / 1GB), ($after / 1GB)
