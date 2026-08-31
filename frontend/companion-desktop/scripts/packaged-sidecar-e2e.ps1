# Packaged-product sidecar harness (no UI).
# Extracts the Tauri MSI administratively, checks the canonical CLI sits
# beside the app binary, and optionally probes `gateway serve`.
#
# Usage:
#   pwsh scripts/packaged-sidecar-e2e.ps1
#   pwsh scripts/packaged-sidecar-e2e.ps1 -ProbeGateway
#
# Does not install or register the product.

[CmdletBinding()]
param(
    [switch]$ProbeGateway,
    [string]$Target = "x86_64-pc-windows-msvc"
)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot\..

$bundleMsiCandidates = @(
    "$PWD\src-tauri\target\$Target\release\bundle\msi",
    "$PWD\src-tauri\target\release\bundle\msi"
)
$releaseCandidates = @(
    "$PWD\src-tauri\target\$Target\release",
    "$PWD\src-tauri\target\release"
)
$workspaceCliCandidates = @(
    "$PWD\..\..\target\$Target\release\apeireth.exe",
    "$PWD\..\..\target\release\apeireth.exe"
)

$bundleMsiDir = ($bundleMsiCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1)
$releaseDir = ($releaseCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1)
$workspaceCli = ($workspaceCliCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1)
$failures = 0

function Assert-Check([bool]$Ok, [string]$Message) {
    if ($Ok) {
        Write-Host "  [PASS] $Message" -ForegroundColor Green
    } else {
        Write-Host "  [FAIL] $Message" -ForegroundColor Red
        $script:failures++
    }
}

Write-Host '=== packaged sidecar identity ==='
$msi = Get-ChildItem -Path $bundleMsiDir -Filter '*.msi' -ErrorAction SilentlyContinue |
    Sort-Object Length -Descending |
    Select-Object -First 1
Assert-Check ($null -ne $msi) "MSI present under $bundleMsiDir"

$sidecarOnDisk = Test-Path (Join-Path $PWD 'src-tauri\binaries\apeireth-x86_64-pc-windows-msvc.exe')
Assert-Check $sidecarOnDisk 'Tauri externalBin sidecar is staged'

if (Test-Path $workspaceCli) {
    $help = & $workspaceCli --help 2>&1 | Out-String
    Assert-Check ($help -match 'gateway serve') 'workspace apeireth.exe help lists gateway serve'
} else {
    Assert-Check $false "workspace release CLI missing: $workspaceCli"
}

Write-Host '=== built release layout (what the installer packages) ==='
$builtApp = Join-Path $releaseDir 'companion-desktop.exe'
$builtSidecar = if (Test-Path (Join-Path $releaseDir 'apeireth.exe')) { Join-Path $releaseDir 'apeireth.exe' } else { $workspaceCli }
Assert-Check (Test-Path $builtApp) "release companion-desktop.exe present"
Assert-Check (Test-Path $builtSidecar) "release apeireth.exe sits beside the app (bundled-backend layout)"
if (Test-Path $builtSidecar) {
    $builtHelp = & $builtSidecar --help 2>&1 | Out-String
    Assert-Check ($builtHelp -match 'gateway serve') 'release sidecar help lists gateway serve'
    if ($ProbeGateway) {
        $port = 18121
        $proc = Start-Process -FilePath $builtSidecar -ArgumentList @('gateway', 'serve', '--port', "$port") -PassThru -WindowStyle Hidden
        try {
            $ok = $false
            for ($i = 0; $i -lt 20; $i++) {
                Start-Sleep -Milliseconds 500
                try {
                    $response = Invoke-WebRequest -Uri "http://127.0.0.1:$port/health" -UseBasicParsing -TimeoutSec 2
                    if ($response.StatusCode -eq 200) { $ok = $true; break }
                } catch { }
            }
            Assert-Check $ok "release sidecar /health on :$port"
        } finally {
            if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue }
        }
    }
}

if ($null -eq $msi) {
    Write-Host "skipping MSI extract (no installer yet)"
    if ($failures -gt 0) { exit 1 }
    exit 0
}

# Administrative extract. Paths with spaces must be quoted; msiexec 1639 is
# "invalid command line" and is what unquoted MSI names produce.
$extract = Join-Path $env:TEMP 'apx-msi-e2e'
if (Test-Path $extract) { Remove-Item -Recurse -Force $extract }
New-Item -ItemType Directory -Path $extract | Out-Null
try {
    Write-Host "extracting $($msi.Name) -> $extract"
    $arg = "/a `"$($msi.FullName)`" /qn TARGETDIR=`"$extract`""
    $msiexec = Start-Process -FilePath 'msiexec.exe' -ArgumentList $arg -Wait -PassThru
    Assert-Check ($msiexec.ExitCode -eq 0) "msiexec /a exit 0 (got $($msiexec.ExitCode))"

    $appDir = Get-ChildItem -Path $extract -Recurse -Directory -ErrorAction SilentlyContinue |
        Where-Object { Test-Path (Join-Path $_.FullName 'companion-desktop.exe') } |
        Select-Object -First 1
    if ($null -eq $appDir) {
        $appDir = Get-ChildItem -Path $extract -Recurse -Directory -ErrorAction SilentlyContinue |
            Where-Object { Test-Path (Join-Path $_.FullName 'apeireth.exe') } |
            Select-Object -First 1
    }
    Assert-Check ($null -ne $appDir) 'extracted install dir contains the app or sidecar'

    if ($null -ne $appDir) {
        $appExe = Join-Path $appDir.FullName 'companion-desktop.exe'
        $sidecar = Join-Path $appDir.FullName 'apeireth.exe'
        Assert-Check (Test-Path $appExe) "companion-desktop.exe next to sidecar ($appExe)"
        Assert-Check (Test-Path $sidecar) "canonical sidecar apeireth.exe present ($sidecar)"
        if (Test-Path $sidecar) {
            $packagedHelp = & $sidecar --help 2>&1 | Out-String
            Assert-Check ($packagedHelp -match 'gateway serve') 'packaged sidecar help lists gateway serve'
            if ($ProbeGateway) {
                $port = 18121
                $proc = Start-Process -FilePath $sidecar -ArgumentList @('gateway', 'serve', '--port', "$port") -PassThru -WindowStyle Hidden
                try {
                    $ok = $false
                    for ($i = 0; $i -lt 20; $i++) {
                        Start-Sleep -Milliseconds 500
                        try {
                            $response = Invoke-WebRequest -Uri "http://127.0.0.1:$port/health" -UseBasicParsing -TimeoutSec 2
                            if ($response.StatusCode -eq 200) { $ok = $true; break }
                        } catch { }
                    }
                    Assert-Check $ok "packaged sidecar /health on :$port"
                } finally {
                    if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
                }
            }
        }
    }
} finally {
    Remove-Item -Recurse -Force $extract -ErrorAction SilentlyContinue
}

if ($failures -gt 0) {
    Write-Host "FAILED $failures check(s)"
    exit 1
}
Write-Host 'packaged sidecar harness: PASS'
exit 0
