# packaging/desktop/stage-desktop.ps1
# Prepares and stages Tauri Desktop frontend and binaries for MSI/NSIS packaging
param(
    [string]$Version = "2.0.0-rc.1",
    [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..\..

Write-Host "=== Staging Apeireth Desktop Companion v$Version ==="

$DesktopDir = "frontend\companion-desktop"
$StageDir = "target\desktop-stage"

if (Test-Path $StageDir) { Remove-Item $StageDir -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $StageDir "bin") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $StageDir "share") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $StageDir "icons") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $StageDir "dist") -Force | Out-Null

# 1. Build frontend dist if pnpm is available, or ensure valid dist
Write-Host "[1/3] Checking frontend dist..."
$distPath = Join-Path $DesktopDir "dist"
if (-not (Test-Path $distPath) -or (Get-ChildItem $distPath).Count -eq 0) {
    if (Get-Command pnpm -ErrorAction SilentlyContinue) {
        Write-Host "    Building Svelte 5 frontend via pnpm build..."
        Push-Location $DesktopDir
        pnpm install --frozen-lockfile
        pnpm build
        Pop-Location
    } else {
        Write-Host "    [STAGING FALLBACK] Creating fallback placeholder dist/index.html..."
        New-Item -ItemType Directory -Path $distPath -Force | Out-Null
        "<!DOCTYPE html><html><head><title>Apeireth Companion</title></head><body><h1>Apeireth Companion Desktop</h1></body></html>" | Out-File -FilePath (Join-Path $distPath "index.html") -Encoding UTF8
    }
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

# 3. Check/Stage binaries
Write-Host "[3/3] Staging desktop binaries..."
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
    Write-Host "    [NOTE] Pre-compiled companion-desktop binary not found in target. Staged for build phase."
}

Write-Host "Desktop staging completed at: $StageDir"
