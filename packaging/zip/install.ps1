# packaging/zip/install.ps1
# User install helper for Apeireth portable ZIP package
param(
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\Apeireth",
    [string]$ZipFile = ""
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..\..

if (-not $ZipFile) {
    $candidates = Get-ChildItem -Path "target\apeireth-*.zip" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending
    if ($candidates.Count -gt 0) {
        $ZipFile = $candidates[0].FullName
    } else {
        throw "No ZIP package found in target/. Run packaging\zip\build.ps1 first."
    }
}

Write-Host "=== Installing Apeireth Portable ZIP ==="
Write-Host "Source:      $ZipFile"
Write-Host "Destination: $InstallDir"

if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}

Expand-Archive -Path $ZipFile -DestinationPath $InstallDir -Force
Write-Host "Files extracted successfully."

# Add to user PATH if not present
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$InstallDir*") {
    $newPath = "$userPath;$InstallDir;$InstallDir\bin"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "Added $InstallDir to User PATH."
}

Write-Host "Apeireth portable installation complete."
