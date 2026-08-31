<#
.SYNOPSIS
    Stage the canonical Apeireth backend as a Tauri sidecar before packaging.

.DESCRIPTION
    `tauri.conf.json` declares bundle.externalBin = ["binaries/apeireth"].
    Tauri resolves that to `binaries/apeireth-<target-triple><exe-suffix>`, so
    the release build of the canonical CLI has to be copied there under the
    triple-suffixed name before `pnpm tauri build` runs.

    The staged file is a build artifact, not a source file: packaging/../
    frontend/companion-desktop/src-tauri/binaries/ is gitignored.

    The bundled backend is the SAME canonical `apeireth` binary the CLI and
    gateway use. This script never builds a second backend implementation.

.PARAMETER Profile
    Cargo profile to stage from. Defaults to release.

.PARAMETER SkipBuild
    Stage an existing binary without rebuilding.

.EXAMPLE
    ./packaging/stage-sidecar.ps1
    ./packaging/stage-sidecar.ps1 -SkipBuild
#>
[CmdletBinding()]
param(
    [ValidateSet('release', 'debug')]
    [string]$Profile = 'release',
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$sidecarDir = Join-Path $repoRoot 'frontend/companion-desktop/src-tauri/binaries'

# Ask rustc for the host triple rather than assuming one, so this script works
# on every platform Tauri targets.
$hostLine = (rustc -vV | Select-String -Pattern '^host:').Line
if (-not $hostLine) { throw 'Could not determine host target triple from rustc -vV.' }
$targetTriple = $hostLine.Split(':')[1].Trim()
$exeSuffix = if ($IsWindows -or $env:OS -eq 'Windows_NT') { '.exe' } else { '' }

if (-not $SkipBuild) {
    Write-Host "Building canonical CLI (profile: $Profile)..." -ForegroundColor Cyan
    Push-Location $repoRoot
    try {
        if ($Profile -eq 'release') {
            cargo build --release -p apeireth-cli
        } else {
            cargo build -p apeireth-cli
        }
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }
    } finally {
        Pop-Location
    }
}

$source = Join-Path $repoRoot "target/$Profile/apeireth$exeSuffix"
if (-not (Test-Path $source)) {
    throw "Canonical backend not found at $source. Run without -SkipBuild first."
}

# Fail loudly if the artifact is not the binary we expect: a silently wrong
# sidecar would only surface as a broken installed app.
$versionOutput = & $source --version 2>&1
if ($LASTEXITCODE -ne 0) { throw "Staged backend did not run: $versionOutput" }
if ($versionOutput -notmatch '^apeireth ') {
    throw "Unexpected --version output from ${source}: $versionOutput"
}

if (-not (Test-Path $sidecarDir)) {
    New-Item -ItemType Directory -Path $sidecarDir | Out-Null
}

$destination = Join-Path $sidecarDir "apeireth-$targetTriple$exeSuffix"
Copy-Item -Path $source -Destination $destination -Force

$sizeMb = [math]::Round((Get-Item $destination).Length / 1MB, 2)
Write-Host ''
Write-Host 'Sidecar staged for Tauri packaging.' -ForegroundColor Green
Write-Host "  backend : $versionOutput"
Write-Host "  source  : $source"
Write-Host "  staged  : $destination"
Write-Host "  triple  : $targetTriple"
Write-Host "  size    : $sizeMb MB"
Write-Host ''
Write-Host 'Next: cd frontend/companion-desktop && pnpm tauri build'
