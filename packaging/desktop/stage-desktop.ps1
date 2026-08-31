# packaging/desktop/stage-desktop.ps1
# Prepares and stages Tauri Desktop frontend and binaries for MSI/NSIS packaging
param(
    [string]$Version = "2.0.0-rc.1",
    [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..\..

$RepoRoot = (Get-Location).Path

Write-Host "=== Staging Apeireth Desktop Companion v$Version ==="

$DesktopDir = "frontend\companion-desktop"
$StageDir = "target\desktop-stage"

if (Test-Path $StageDir) { Remove-Item $StageDir -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $StageDir "bin") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $StageDir "share") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $StageDir "icons") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $StageDir "dist") -Force | Out-Null

# 1. Build the real frontend. A placeholder UI is not a release artifact.
Write-Host "[1/3] Checking frontend dist..."
$distPath = Join-Path $DesktopDir "dist"
if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {
    throw 'pnpm is required to build the RC frontend; refusing placeholder staging.'
}
Push-Location $DesktopDir
try {
    $env:CI = "true"
    cmd.exe /c pnpm install --frozen-lockfile
    if ($LASTEXITCODE -ne 0) { throw "pnpm install failed with exit code $LASTEXITCODE" }
    cmd.exe /c pnpm build
    if ($LASTEXITCODE -ne 0) { throw "pnpm build failed with exit code $LASTEXITCODE" }
} finally {
    Pop-Location
}
if (-not (Test-Path (Join-Path $distPath 'index.html'))) {
    throw "Frontend build did not produce $distPath/index.html"
}
Copy-Item (Join-Path $distPath "*") (Join-Path $StageDir "dist") -Recurse -Force

# 2. Copy legal and icon assets
Write-Host "[2/3] Copying legal docs and icons..."
if (Test-Path "LICENSE") { Copy-Item "LICENSE" (Join-Path $StageDir "share\LICENSE") }
if (Test-Path "NOTICE") { Copy-Item "NOTICE" (Join-Path $StageDir "share\NOTICE") }
if (Test-Path "README.md") { Copy-Item "README.md" (Join-Path $StageDir "share\README.md") }
if (Test-Path "$DesktopDir\src-tauri\icons") {
    Copy-Item (Join-Path "$DesktopDir\src-tauri\icons" "*") (Join-Path $StageDir "icons") -Force
}

# 3. Build and stage the canonical backend sidecar
Write-Host "[3/3] Staging desktop binaries..."
$sidecarScript = Join-Path $RepoRoot 'packaging\stage-sidecar.ps1'
& $sidecarScript -Profile release -Target $Target
if ($LASTEXITCODE -ne 0) { throw "Sidecar staging failed with exit code $LASTEXITCODE" }

$tauriBinCandidates = @(
    "$DesktopDir\src-tauri\target\$Target\release\companion-desktop.exe",
    "$DesktopDir\src-tauri\target\release\companion-desktop.exe",
    "target\$Target\release\companion-desktop.exe",
    "target\release\companion-desktop.exe"
)

$foundBin = $false
foreach ($cand in $tauriBinCandidates) {
    if (Test-Path $cand) {
        Copy-Item $cand (Join-Path $StageDir "bin\apeireth-companion.exe") -Force
        $foundBin = $true
        Write-Host "    Found binary: $cand"
        break
    }
}

if (-not $foundBin) {
    Write-Host "    Companion desktop binary will be produced by the Tauri build phase."
}

Write-Host "Desktop staging completed at: $StageDir"
