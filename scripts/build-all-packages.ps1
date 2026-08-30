# scripts/build-all-packages.ps1
# Apeireth 2.0 RC - Windows Packaging Master Orchestrator
# Builds: Portable ZIP, WiX MSI, Scoop Manifest, Desktop MSI/NSIS/ZIP

param(
    [string]$Version = "2.0.0-rc.1",
    [string]$Target = "x86_64-pc-windows-msvc",
    [switch]$DryRun = $false
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..

Write-Host "=================================================="
Write-Host "  Apeireth OS v$Version — Windows Packaging Suite"
Write-Host "  Target: $Target"
Write-Host "  DryRun: $DryRun"
Write-Host "=================================================="
Write-Host ""

$env:APEIRETH_VERSION = $Version
$env:APEIRETH_TARGET = $Target

# 1. CLI Portable ZIP
Write-Host "[1/4] Building CLI Portable ZIP (apeireth-$Version-windows-x86_64.zip)..."
if (-not $DryRun) {
    & powershell -ExecutionPolicy Bypass -File packaging\zip\build.ps1
}

# 2. CLI MSI Installer
Write-Host "[2/4] Building CLI Windows MSI (WiX)..."
if (-not $DryRun) {
    & powershell -ExecutionPolicy Bypass -File packaging\msi\build.ps1
}

# 3. Scoop Manifest & Bucket Staging
Write-Host "[3/4] Updating Scoop Manifest..."
if (-not $DryRun) {
    & powershell -ExecutionPolicy Bypass -File packaging\scoop\build.ps1
}

# 4. Tauri Desktop Packaging (MSI / NSIS / Portable)
Write-Host "[4/4] Building Desktop Packaging Suite..."
if (-not $DryRun) {
    & powershell -ExecutionPolicy Bypass -File packaging\desktop\build-desktop.ps1
}

Write-Host ""
Write-Host "=================================================="
Write-Host "  Windows Packaging Suite Completed (v$Version)"
Write-Host "=================================================="
