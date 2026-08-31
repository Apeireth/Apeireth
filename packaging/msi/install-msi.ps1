# packaging/msi/install-msi.ps1
# User install helper for Apeireth MSI package
param(
    [string]$MsiFile = "",
    [switch]$Quiet = $false
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..\..

if (-not $MsiFile) {
    $candidates = Get-ChildItem -Path "target\msi\*.msi", "target\wix\*.msi" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending
    if ($candidates.Count -gt 0) {
        $MsiFile = $candidates[0].FullName
    } else {
        throw "No MSI installer package found in target/msi/ or target/wix/. Run packaging\msi\build.ps1 first."
    }
}

Write-Host "=== Installing Apeireth MSI Package ==="
Write-Host "MSI File: $MsiFile"

$args = @('/i', $MsiFile)
if ($Quiet) {
    $args += "/qn"
    $args += "/norestart"
}

$process = Start-Process msiexec.exe -ArgumentList $args -Wait -PassThru
if ($process.ExitCode -eq 0 -or $process.ExitCode -eq 3010) {
    Write-Host "Apeireth MSI installed successfully (ExitCode: $($process.ExitCode))."
} else {
    throw "msiexec failed with exit code: $($process.ExitCode)"
}
