# packaging/desktop/build-desktop.ps1
# Master build script for Apeireth Desktop Companion (Portable ZIP + MSI + NSIS)
param(
    [string]$Version = "2.0.0-rc.1",
    [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = 'Continue'
Set-Location $PSScriptRoot\..\..

Write-Host "=================================================="
Write-Host "  Apeireth Desktop Companion Packaging v$Version"
Write-Host "=================================================="

# 1. Stage assets & frontend
& "$PSScriptRoot\stage-desktop.ps1" -Version $Version -Target $Target

# 2. Build Portable Desktop ZIP
Write-Host "[1/3] Creating Desktop Portable ZIP..."
$STAGE_DIR = "target\desktop-stage"
$PORTABLE_ZIP = "target\apeireth-companion-${Version}-windows-x86_64.zip"
if (Test-Path $PORTABLE_ZIP) { Remove-Item $PORTABLE_ZIP -Force }
Compress-Archive -Path "$STAGE_DIR\*" -DestinationPath $PORTABLE_ZIP -CompressionLevel Optimal
$ZIP_SHA256 = (Get-FileHash -Path $PORTABLE_ZIP -Algorithm SHA256).Hash
"${ZIP_SHA256}  apeireth-companion-${Version}-windows-x86_64.zip" | Out-File -FilePath "${PORTABLE_ZIP}.sha256" -Encoding UTF8
Write-Host "    Desktop ZIP: $PORTABLE_ZIP"
Write-Host "    SHA256:      $ZIP_SHA256"

# 3. Build Desktop MSI
Write-Host "[2/3] Building Desktop MSI..."
& "$PSScriptRoot\build-desktop-msi.ps1" -Version $Version -Target $Target

# 4. Build Desktop NSIS
Write-Host "[3/3] Building Desktop NSIS..."
& "$PSScriptRoot\build-desktop-nsis.ps1" -Version $Version -Target $Target

Write-Host "=================================================="
Write-Host "  Desktop Packaging Complete (v$Version)"
Write-Host "=================================================="
exit 0
