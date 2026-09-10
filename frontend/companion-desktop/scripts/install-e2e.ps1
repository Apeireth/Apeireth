# Installed-product E2E (NSIS, Windows).
# Silently installs the built NSIS package, probes the INSTALLED sidecar
# (real chat via DeepSeek + gateway /health), smoke-tests the desktop app,
# reproduces the "running sidecar blocks uninstall" scenario, and verifies
# the uninstaller hook (src-tauri/installer.nsh) kills the orphaned sidecar
# and leaves zero residue (dir + registry).
#
# Usage:
#   $env:OPENAI_API_KEY='sk-...'; $env:APEIRETH_OPENAI_URL='https://api.deepseek.com/v1'; $env:APEIRETH_OPENAI_MODELS='deepseek-v4-flash'
#   pwsh scripts/install-e2e.ps1
#   pwsh scripts/install-e2e.ps1 -SetupPath '...\Apeireth Companion_2.0.0-rc.1_x64-setup.exe'
#
# Requires: a built NSIS setup (pnpm tauri build --bundles nsis), the
# DeepSeek-compatible env above, and a user session that can elevate
# (the package installs perMachine under Program Files).

[CmdletBinding()]
param(
    [string]$SetupPath = "",
    [string]$InstDir = "C:\Program Files\Apeireth Companion"
)

$ErrorActionPreference = 'Continue'
Set-Location $PSScriptRoot\..\..

if ($SetupPath -eq "") {
    $SetupPath = Get-ChildItem 'target\desktop-nsis' -Filter '*_x64-setup.exe' -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1 -ExpandProperty FullName
    if (-not $SetupPath) {
        Write-Error 'no setup found under target\desktop-nsis; pass -SetupPath'
        exit 1
    }
}
if (-not (Test-Path $SetupPath)) { Write-Error "setup missing: $SetupPath"; exit 1 }

$sidecar = Join-Path $InstDir 'apeireth.exe'
$app = Join-Path $InstDir 'companion-desktop.exe'
$dbdir = Join-Path $env:TEMP 'apx-install-e2e'
if (Test-Path $dbdir) { Remove-Item -Recurse -Force $dbdir }
New-Item -ItemType Directory $dbdir | Out-Null
$env:APEIRETH_COGNITIVE_DB = Join-Path $dbdir 'cognitive.sqlite3'
$fails = 0

function Check([bool]$Ok, [string]$Message) {
    if ($Ok) { Write-Host "  [PASS] $Message" -ForegroundColor Green }
    else { Write-Host "  [FAIL] $Message" -ForegroundColor Red; $script:fails++ }
}

Write-Host "=== install (silent, perMachine) ==="
$p = Start-Process -FilePath $SetupPath -ArgumentList '/S' -PassThru -Wait
Start-Sleep -Seconds 3
Check ($p.ExitCode -eq 0) "installer exit 0 (got $($p.ExitCode))"
Check (Test-Path $sidecar) "installed sidecar present: $sidecar"
Check (Test-Path $app) "installed app present: $app"

Write-Host '=== installed sidecar chat probe (real provider) ==='
$sid = [guid]::NewGuid().ToString()
$r = & $sidecar chat "你好" --model $env:APEIRETH_OPENAI_MODELS --session $sid 2>&1 | Out-String
Check ($r -match 'provider=provider.openai-compatible') "installed chat via openai-compatible provider"

Write-Host '=== gateway serve /health (installed binary) ==='
$gp = Start-Process -FilePath $sidecar -ArgumentList @('gateway', 'serve', '--port', '18124') -PassThru -WindowStyle Hidden
try {
    $ok = $false
    for ($i = 0; $i -lt 20; $i++) {
        Start-Sleep -Milliseconds 500
        try {
            $resp = Invoke-WebRequest -Uri 'http://127.0.0.1:18124/health' -UseBasicParsing -TimeoutSec 2
            if ($resp.StatusCode -eq 200) { $ok = $true; break }
        } catch { }
    }
    Check $ok 'installed gateway /health 200'
} finally {
    if (-not $gp.HasExited) { Stop-Process -Id $gp.Id -Force }
}

Write-Host '=== desktop app smoke (alive 6s; force-kill parent leaves orphan sidecar) ==='
$dp = Start-Process -FilePath $app -PassThru
Start-Sleep -Seconds 6
if ($dp.HasExited) {
    Check $false "desktop exited early (code $($dp.ExitCode))"
} else {
    Check $true "desktop app alive (pid $($dp.Id))"
    Stop-Process -Id $dp.Id -Force
}
Start-Sleep -Seconds 2
$orphan = Get-Process apeireth -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $sidecar }
Check ($null -ne $orphan) "defect precondition reproduced: orphaned sidecar running (pid $($orphan.Id -join ','))"

Write-Host '=== uninstall while sidecar running (installer.nsh hook must kill it) ==='
$up = Start-Process -FilePath (Join-Path $InstDir 'uninstall.exe') -ArgumentList '/S' -PassThru -Wait
Start-Sleep -Seconds 4
Check ($up.ExitCode -eq 0) "uninstall exit 0 (got $($up.ExitCode))"
Check (-not (Test-Path $InstDir)) 'install dir fully removed'
$reg = Get-ChildItem 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall' -ErrorAction SilentlyContinue |
    ForEach-Object { $q = Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue; if ($q.DisplayName -like '*Apeireth*') { $q.DisplayName } }
Check (-not $reg) 'uninstall registry entry removed'
$left = Get-Process apeireth -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $sidecar }
Check ($null -eq $left) 'orphaned sidecar killed by uninstaller'

if ($fails -gt 0) {
    Write-Host "FAILED $fails check(s)" -ForegroundColor Red
    exit 1
}
Write-Host 'install-e2e: PASS (install / chat / gateway / desktop / uninstall-with-running-sidecar)'
exit 0
