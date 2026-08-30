<#
.SYNOPSIS
    Apeireth Release Version Consistency Validator (PowerShell)
.DESCRIPTION
    Validates version consistency across all authoritative version sources in Apeireth 2.0:
      1. Root Cargo.toml ([workspace.package].version)
      2. 16 Workspace Crates (crates/*/*/Cargo.toml -> version.workspace = true)
         - CLI (crates/adapters/cli)
         - Gateway (crates/adapters/gateway)
         - SDK (crates/adapters/sdk)
         - Core, Runtime, Storage, Memory, Perception, Organ, Tools, etc.
      3. Desktop Frontend (frontend/companion-desktop/package.json)
      4. Desktop Tauri Shell (frontend/companion-desktop/src-tauri/Cargo.toml)
      5. Desktop Tauri Config (frontend/companion-desktop/src-tauri/tauri.conf.json)

.PARAMETER ExpectedVersion
    Optional expected version string (e.g. "1.2.0" or "2.0.0-rc.1").
.PARAMETER Strict
    If true, requires companion-desktop to match the root workspace version exactly.
    If false (default), validates internal consistency within workspace and within desktop.
.PARAMETER Json
    Outputs result in JSON format.
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$ExpectedVersion,

    [Parameter()]
    [switch]$Strict,

    [Parameter()]
    [switch]$Json
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path

$Results = [System.Collections.Generic.List[PSCustomObject]]::new()
$HasErrors = $false

function Add-Result {
    param(
        [string]$Component,
        [string]$Category,
        [string]$FilePath,
        [string]$DetectedVersion,
        [string]$Inheritance,
        [string]$Status,
        [string]$Message
    )
    $obj = [PSCustomObject]@{
        Component       = $Component
        Category        = $Category
        FilePath        = $FilePath
        DetectedVersion = $DetectedVersion
        Inheritance     = $Inheritance
        Status          = $Status
        Message         = $Message
    }
    $Results.Add($obj)
    if ($Status -eq "FAIL") {
        $script:HasErrors = $true
    }
}

# 1. Root Cargo.toml
$RootCargoPath = Join-Path $RepoRoot "Cargo.toml"
$RootWorkspaceVersion = $null

if (Test-Path $RootCargoPath) {
    $RootContent = Get-Content $RootCargoPath -Raw
    # Find [workspace.package] version
    if ($RootContent -match '\[workspace\.package\][\s\S]*?version\s*=\s*"([^"]+)"') {
        $RootWorkspaceVersion = $Matches[1]
        $status = "PASS"
        $msg = "Root workspace package version defined"
        if ($ExpectedVersion -and ($RootWorkspaceVersion -ne $ExpectedVersion)) {
            $status = "FAIL"
            $msg = "Mismatch with expected version '$ExpectedVersion'"
        }
        Add-Result -Component "Root Workspace" -Category "Workspace" -FilePath "Cargo.toml" `
            -DetectedVersion $RootWorkspaceVersion -Inheritance "Authority ([workspace.package])" -Status $status -Message $msg
    } else {
        Add-Result -Component "Root Workspace" -Category "Workspace" -FilePath "Cargo.toml" `
            -DetectedVersion "NOT_FOUND" -Inheritance "Authority" -Status "FAIL" -Message "Missing [workspace.package].version"
    }
} else {
    Add-Result -Component "Root Workspace" -Category "Workspace" -FilePath "Cargo.toml" `
        -DetectedVersion "FILE_MISSING" -Inheritance "Authority" -Status "FAIL" -Message "Root Cargo.toml not found"
}

# 2. Workspace Crates (All 16 crates in crates/)
$CratesPath = Join-Path $RepoRoot "crates"
if (Test-Path $CratesPath) {
    $CrateTomls = Get-ChildItem -Path $CratesPath -Filter "Cargo.toml" -Recurse | Sort-Object FullName
    foreach ($toml in $CrateTomls) {
        $relPath = $toml.FullName.Substring($RepoRoot.Length + 1).Replace("\", "/")
        $content = Get-Content $toml.FullName -Raw
        
        $crateName = "unknown"
        if ($content -match '\[package\][\s\S]*?name\s*=\s*"([^"]+)"') {
            $crateName = $Matches[1]
        }

        $verMode = "NONE"
        $crateVer = $null

        if ($content -match 'version\.workspace\s*=\s*true') {
            $verMode = "version.workspace = true"
            $crateVer = $RootWorkspaceVersion
            Add-Result -Component $crateName -Category "Workspace Crate" -FilePath $relPath `
                -DetectedVersion $crateVer -Inheritance "Workspace Inherited" -Status "PASS" -Message "Inherits [workspace.package].version ($crateVer)"
        } elseif ($content -match '\[package\][\s\S]*?version\s*=\s*"([^"]+)"') {
            $crateVer = $Matches[1]
            $verMode = "Explicit"
            $status = if ($crateVer -eq $RootWorkspaceVersion) { "PASS" } else { "FAIL" }
            $msg = if ($status -eq "PASS") { "Matches workspace version" } else { "Explicit version differs from workspace ($RootWorkspaceVersion)" }
            Add-Result -Component $crateName -Category "Workspace Crate" -FilePath $relPath `
                -DetectedVersion $crateVer -Inheritance "Explicit ($crateVer)" -Status $status -Message $msg
        } else {
            Add-Result -Component $crateName -Category "Workspace Crate" -FilePath $relPath `
                -DetectedVersion "NONE" -Inheritance "None" -Status "FAIL" -Message "No version specified in Cargo.toml"
        }
    }
}

# 3. Companion Desktop Frontend (package.json)
$DesktopPackagePath = Join-Path $RepoRoot "frontend/companion-desktop/package.json"
$DesktopPackageVer = $null
if (Test-Path $DesktopPackagePath) {
    $pkgJson = Get-Content $DesktopPackagePath -Raw | ConvertFrom-Json
    $DesktopPackageVer = $pkgJson.version
    $status = "PASS"
    $msg = "Desktop UI package version"
    if ($Strict -and $RootWorkspaceVersion -and ($DesktopPackageVer -ne $RootWorkspaceVersion)) {
        $status = "FAIL"
        $msg = "Strict mode: Desktop version ($DesktopPackageVer) != Workspace version ($RootWorkspaceVersion)"
    }
    Add-Result -Component "companion-desktop (UI)" -Category "Desktop App" -FilePath "frontend/companion-desktop/package.json" `
        -DetectedVersion $DesktopPackageVer -Inheritance "Independent App" -Status $status -Message $msg
} else {
    Add-Result -Component "companion-desktop (UI)" -Category "Desktop App" -FilePath "frontend/companion-desktop/package.json" `
        -DetectedVersion "FILE_MISSING" -Inheritance "Independent App" -Status "WARN" -Message "File not found"
}

# 4. Companion Desktop Tauri Shell (src-tauri/Cargo.toml)
$DesktopCargoPath = Join-Path $RepoRoot "frontend/companion-desktop/src-tauri/Cargo.toml"
$DesktopCargoVer = $null
if (Test-Path $DesktopCargoPath) {
    $cargoContent = Get-Content $DesktopCargoPath -Raw
    if ($cargoContent -match '\[package\][\s\S]*?version\s*=\s*"([^"]+)"') {
        $DesktopCargoVer = $Matches[1]
        $status = "PASS"
        $msg = "Desktop Tauri shell crate version"
        if ($DesktopPackageVer -and ($DesktopCargoVer -ne $DesktopPackageVer)) {
            $status = "FAIL"
            $msg = "Mismatch with desktop package.json ($DesktopPackageVer)"
        }
        if ($Strict -and $RootWorkspaceVersion -and ($DesktopCargoVer -ne $RootWorkspaceVersion)) {
            $status = "FAIL"
            $msg = "Strict mode: Desktop shell ($DesktopCargoVer) != Workspace ($RootWorkspaceVersion)"
        }
        Add-Result -Component "companion-desktop (Tauri Shell)" -Category "Desktop App" -FilePath "frontend/companion-desktop/src-tauri/Cargo.toml" `
            -DetectedVersion $DesktopCargoVer -Inheritance "Independent [workspace]" -Status $status -Message $msg
    } else {
        Add-Result -Component "companion-desktop (Tauri Shell)" -Category "Desktop App" -FilePath "frontend/companion-desktop/src-tauri/Cargo.toml" `
            -DetectedVersion "NOT_FOUND" -Inheritance "Independent [workspace]" -Status "FAIL" -Message "Missing package.version"
    }
} else {
    Add-Result -Component "companion-desktop (Tauri Shell)" -Category "Desktop App" -FilePath "frontend/companion-desktop/src-tauri/Cargo.toml" `
        -DetectedVersion "FILE_MISSING" -Inheritance "Independent [workspace]" -Status "WARN" -Message "File not found"
}

# 5. Companion Desktop Tauri Config (tauri.conf.json)
$DesktopTauriConfigPath = Join-Path $RepoRoot "frontend/companion-desktop/src-tauri/tauri.conf.json"
$DesktopTauriVer = $null
if (Test-Path $DesktopTauriConfigPath) {
    $tauriJson = Get-Content $DesktopTauriConfigPath -Raw | ConvertFrom-Json
    $DesktopTauriVer = $tauriJson.version
    $status = "PASS"
    $msg = "Desktop Tauri app bundle version"
    if ($DesktopPackageVer -and ($DesktopTauriVer -ne $DesktopPackageVer)) {
        $status = "FAIL"
        $msg = "Mismatch with desktop package.json ($DesktopPackageVer)"
    }
    if ($DesktopCargoVer -and ($DesktopTauriVer -ne $DesktopCargoVer)) {
        $status = "FAIL"
        $msg = "Mismatch with desktop src-tauri Cargo.toml ($DesktopCargoVer)"
    }
    if ($Strict -and $RootWorkspaceVersion -and ($DesktopTauriVer -ne $RootWorkspaceVersion)) {
        $status = "FAIL"
        $msg = "Strict mode: Desktop tauri.conf.json ($DesktopTauriVer) != Workspace ($RootWorkspaceVersion)"
    }
    Add-Result -Component "companion-desktop (Tauri Config)" -Category "Desktop App" -FilePath "frontend/companion-desktop/src-tauri/tauri.conf.json" `
        -DetectedVersion $DesktopTauriVer -Inheritance "Tauri App Config" -Status $status -Message $msg
} else {
    Add-Result -Component "companion-desktop (Tauri Config)" -Category "Desktop App" -FilePath "frontend/companion-desktop/src-tauri/tauri.conf.json" `
        -DetectedVersion "FILE_MISSING" -Inheritance "Tauri App Config" -Status "WARN" -Message "File not found"
}

# Output Results
if ($Json) {
    $outputObj = [PSCustomObject]@{
        timestamp         = (Get-Date).ToUniversalTime().ToString("o")
        expected_version  = $ExpectedVersion
        strict_mode       = $Strict.IsPresent
        workspace_version = $RootWorkspaceVersion
        desktop_version   = $DesktopPackageVer
        passed            = (-not $HasErrors)
        results           = $Results
    }
    $outputObj | ConvertTo-Json -Depth 5
} else {
    $wsDisplay = if ($RootWorkspaceVersion) { $RootWorkspaceVersion } else { "UNKNOWN" }
    $dtDisplay = if ($DesktopPackageVer) { $DesktopPackageVer } else { "UNKNOWN" }

    Write-Host ""
    Write-Host "================================================================================" -ForegroundColor Cyan
    Write-Host "                  APEIRETH RELEASE VERSION CONSISTENCY REPORT                   " -ForegroundColor Cyan
    Write-Host "================================================================================" -ForegroundColor Cyan
    Write-Host "  Repository Root:    $RepoRoot"
    Write-Host "  Workspace Version:  $wsDisplay" -ForegroundColor Yellow
    Write-Host "  Desktop Version:    $dtDisplay" -ForegroundColor Yellow
    if ($ExpectedVersion) {
        Write-Host "  Expected Version:   $ExpectedVersion" -ForegroundColor Magenta
    }
    Write-Host "  Strict Mode:        $($Strict.IsPresent)"
    Write-Host "--------------------------------------------------------------------------------"

    foreach ($r in $Results) {
        $color = switch ($r.Status) {
            "PASS" { "Green" }
            "WARN" { "Yellow" }
            "FAIL" { "Red" }
            Default { "White" }
        }
        $statusTag = "[$($r.Status)]".PadRight(7)
        $compTag = $r.Component.PadRight(34)
        $verTag = "$($r.DetectedVersion)".PadRight(12)
        Write-Host "$statusTag $compTag $verTag" -ForegroundColor $color -NoNewline
        Write-Host " ($($r.FilePath))" -ForegroundColor Gray
        if ($r.Status -ne "PASS") {
            Write-Host "        -> $($r.Message)" -ForegroundColor $color
        }
    }

    Write-Host "================================================================================" -ForegroundColor Cyan
    if ($HasErrors) {
        Write-Host "  RESULT: FAILED - Version inconsistencies detected!" -ForegroundColor Red
        Write-Host "================================================================================" -ForegroundColor Cyan
        exit 1
    } else {
        Write-Host "  RESULT: SUCCESS - All checked version sources are consistent." -ForegroundColor Green
        Write-Host "================================================================================" -ForegroundColor Cyan
        exit 0
    }
}
