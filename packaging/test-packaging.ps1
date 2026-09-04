# packaging/test-packaging.ps1
# Automated audit and validation test suite for Apeireth Windows Packaging

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..

Write-Host "=================================================="
Write-Host "  Running Apeireth Windows Packaging Test Suite"
Write-Host "=================================================="

$failures = 0

function Assert-Check {
    param([bool]$Condition, [string]$Message)
    if ($Condition) {
        Write-Host "  [PASS] $Message" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] $Message" -ForegroundColor Red
        $script:failures++
    }
}

# === 1. Dev Machine Path Audit ===
Write-Host "`n--- 1. Auditing Dev Machine Paths (D:\, H:\, /home/...) ---"
$packagingFiles = Get-ChildItem -Path packaging, frontend\companion-desktop\src-tauri\tauri.conf.json, scripts\build-all-packages.* -Include *.ps1,*.json,*.wxs,*.snippet,*.md,*.sh -Recurse -File

$hardcodedMatches = @()
foreach ($file in $packagingFiles) {
    $matches = Get-Content $file.FullName | Select-String -Pattern "(?:[DH]:[\\/]|/home/[a-zA-Z0-9_-]+)"
    if ($matches) {
        foreach ($m in $matches) {
            # Exclude standard C:\ Program Files or valid URLs
            if ($m.Line -match "https?://" -or $m.Line -match "PUT-GUID") { continue }
            $hardcodedMatches += "$($file.FullName):$($m.LineNumber): $($m.Line.Trim())"
        }
    }
}
Assert-Check ($hardcodedMatches.Count -eq 0) "Zero dev-machine hardcoded paths found in packaging files ($($hardcodedMatches.Count) matches)"
if ($hardcodedMatches.Count -gt 0) {
    $hardcodedMatches | ForEach-Object { Write-Host "    Found: $_" -ForegroundColor Yellow }
}

# === 2. WiX MSI Configuration Audit ===
Write-Host "`n--- 2. Auditing WiX MSI Configuration (packaging/msi/apeireth.wxs) ---"
$wxsContent = Get-Content "packaging\msi\apeireth.wxs" -Raw
$wxsGuidPlaceholders = ([regex]::Matches($wxsContent, "PUT-GUID-[A-Z]+")).Count
Assert-Check ($wxsGuidPlaceholders -eq 0) "Zero PUT-GUID placeholders in WiX definition ($wxsGuidPlaceholders found)"

$hasUpgradeCode = $wxsContent -match 'UpgradeCode="\{[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}\}"'
Assert-Check $hasUpgradeCode "WiX UpgradeCode is valid RFC-4122 GUID format"

$hasNotice = $wxsContent -match 'Id="NoticeFile"'
Assert-Check $hasNotice "WiX includes NOTICE file component"

$hasLicense = $wxsContent -match 'Id="LicenseFile"'
Assert-Check $hasLicense "WiX includes LICENSE file component"

$hasReadme = $wxsContent -match 'Id="ReadmeFile"'
Assert-Check $hasReadme "WiX includes README.md file component"

$hasShortcuts = $wxsContent -match 'Id="ApplicationStartMenuShortcut"' -and $wxsContent -match 'Id="CleanUpShortCut"'
Assert-Check $hasShortcuts "WiX includes Start Menu shortcuts and clean uninstall handler"

$hasIcon = $wxsContent -match 'Id="ApeirethIcon.ico"' -and $wxsContent -match 'Id="ARPPRODUCTICON"'
Assert-Check $hasIcon "WiX includes Application Icon and ARPPRODUCTICON property"

$hasEnv = $wxsContent -match 'Id="APEIRETH_HOME"' -and $wxsContent -match 'Id="PATH"'
Assert-Check $hasEnv "WiX includes Environment variable configuration (APEIRETH_HOME, PATH)"

$hasCleanUninstall = $wxsContent -match 'RemoveFolder' -and $wxsContent -match 'On="uninstall"'
Assert-Check $hasCleanUninstall "WiX specifies clean directory removal on uninstall"

# === 3. Tauri Configuration Audit ===
Write-Host "`n--- 3. Auditing Tauri Desktop Configuration (tauri.conf.json) ---"
$tauriConf = Get-Content "frontend\companion-desktop\src-tauri\tauri.conf.json" -Raw | ConvertFrom-Json
Assert-Check ($tauriConf.version -eq "2.0.0-rc.1") "Tauri version is 2.0.0-rc.1 ($($tauriConf.version))"

$wixUpgradeCode = $tauriConf.bundle.windows.wix.upgradeCode
$validWixUpgrade = $wixUpgradeCode -match '^[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}$'
Assert-Check $validWixUpgrade "Tauri bundle.windows.wix.upgradeCode is valid GUID ($wixUpgradeCode)"

Assert-Check ($tauriConf.bundle.windows.nsis.installMode -eq "perMachine") "Tauri bundle.windows.nsis installMode is perMachine"

$iconExists = Test-Path "frontend\companion-desktop\src-tauri\icons\icon.ico"
Assert-Check $iconExists "Tauri icon.ico asset exists"

# === 4. Scoop Manifest Audit ===
Write-Host "`n--- 4. Auditing Scoop Manifest (packaging/scoop/apeireth.json) ---"
$scoopJson = Get-Content "packaging\scoop\apeireth.json" -Raw | ConvertFrom-Json
Assert-Check ($scoopJson.version -eq "2.0.0-rc.1") "Scoop manifest version is 2.0.0-rc.1 ($($scoopJson.version))"
Assert-Check ($scoopJson.architecture.'64bit'.url -like "*apeireth-2.0.0-rc.1-windows-x86_64.zip") "Scoop manifest 64bit URL targets apeireth-2.0.0-rc.1-windows-x86_64.zip"

# === 5. Packaging Scripts Existence ===
Write-Host "`n--- 5. Auditing Packaging Scripts ---"
$requiredScripts = @(
    "packaging\zip\build.ps1",
    "packaging\zip\install.ps1",
    "packaging\zip\uninstall.ps1",
    "packaging\msi\build.ps1",
    "packaging\msi\install-msi.ps1",
    "packaging\msi\uninstall-msi.ps1",
    "packaging\scoop\build.ps1",
    "packaging\scoop\install-scoop.ps1",
    "packaging\scoop\uninstall-scoop.ps1",
    "packaging\desktop\stage-desktop.ps1",
    "packaging\desktop\build-desktop-msi.ps1",
    "packaging\desktop\build-desktop-nsis.ps1",
    "packaging\desktop\build-desktop.ps1",
    "scripts\build-all-packages.ps1",
    "docs\packaging\windows-packaging-lifecycle.md"
)

foreach ($scriptPath in $requiredScripts) {
    Assert-Check (Test-Path $scriptPath) "Found: $scriptPath"
}

Write-Host "`n=================================================="
if ($failures -eq 0) {
    Write-Host "  All Packaging Audits & Tests PASSED! (0 Failures)" -ForegroundColor Green
    exit 0
} else {
    Write-Host "  $failures Checks FAILED." -ForegroundColor Red
    exit 1
}
