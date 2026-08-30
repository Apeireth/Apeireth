# packaging/msi/uninstall-msi.ps1
# User uninstall helper for Apeireth MSI package
param(
    [string]$MsiFile = "",
    [switch]$Quiet = $false,
    [switch]$KeepData = $false
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..\..

Write-Host "=== Uninstalling Apeireth MSI Package ==="

# 1. Stop service if running
Write-Host "[1/4] Stopping service (if running)..."
if (Get-Service -Name "ApeirethOS" -ErrorAction SilentlyContinue) {
    Stop-Service -Name "ApeirethOS" -Force -ErrorAction SilentlyContinue
}

# 2. Stop running processes
Write-Host "[2/4] Stopping apeireth processes..."
Get-Process apeireth -ErrorAction SilentlyContinue | Stop-Process -Force

# 3. Trigger MSI uninstallation
Write-Host "[3/4] Triggering MSI uninstallation..."
if (-not $MsiFile) {
    $candidates = Get-ChildItem -Path "target\msi\*.msi", "target\wix\*.msi" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending
    if ($candidates.Count -gt 0) {
        $MsiFile = $candidates[0].FullName
    }
}

if ($MsiFile -and (Test-Path $MsiFile)) {
    $args = @("/x", """")
    if ($Quiet) { $args += "/qn"; $args += "/norestart" }
    $process = Start-Process msiexec.exe -ArgumentList $args -Wait -PassThru
} else {
    # Uninstall by UpgradeCode or Product Name via WMI/Registry search
    Write-Host "Searching installed Apeireth product..."
    $installed = Get-ItemProperty HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\* | Where-Object { $_.DisplayName -like "*Apeireth*" }
    if ($installed) {
        foreach ($app in $installed) {
            $uninst = $app.UninstallString
            if ($uninst) {
                Write-Host "Running: $uninst"
                Start-Process cmd.exe -ArgumentList "/c $uninst /qn" -Wait
            }
        }
    } else {
        Write-Host "No installed Apeireth MSI product found in registry."
    }
}

# 4. Data directory cleanup
if (-not $KeepData) {
    Write-Host "[4/4] Cleaning user data directory..."
    $dataPath = "$env:USERPROFILE\.apeireth"
    if (Test-Path $dataPath) {
        Remove-Item -Path $dataPath -Recurse -Force
        Write-Host "Removed: $dataPath"
    }
}

Write-Host "Apeireth MSI uninstallation complete."
