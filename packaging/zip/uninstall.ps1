# packaging/zip/uninstall.ps1
# User uninstall helper for Apeireth portable ZIP package
param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\Apeireth",
    [switch]$KeepData = $false
)

$ErrorActionPreference = 'Stop'
Write-Host "=== Uninstalling Apeireth Portable ZIP ==="

# Stop any running process
Get-Process apeireth -ErrorAction SilentlyContinue | Stop-Process -Force

# Remove directory
if (Test-Path $InstallDir) {
    Remove-Item -Path $InstallDir -Recurse -Force
    Write-Host "Removed directory: $InstallDir"
}

# Remove from user PATH
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -like "*$InstallDir*") {
    $parts = $userPath.Split(';') | Where-Object { $_ -and $_ -notlike "*$InstallDir*" }
    $newPath = $parts -join ';'
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "Cleaned up User PATH."
}

if (-not $KeepData) {
    $apeirethHome = "$env:USERPROFILE\.apeireth"
    if (Test-Path $apeirethHome) {
        Remove-Item -Path $apeirethHome -Recurse -Force
        Write-Host "Cleaned up data: $apeirethHome"
    }
}

Write-Host "Apeireth portable uninstallation complete."
