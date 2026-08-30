# packaging/zip/build-zip.ps1
# Alias wrapper for packaging/zip/build.ps1
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..\..
& "$PSScriptRoot\build.ps1" @args
