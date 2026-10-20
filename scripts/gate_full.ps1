#!/usr/bin/env pwsh
<#
.SYNOPSIS
  全量门禁（ADR-0005 R4a-1）：主 workspace 门禁（排除 vendored 图像家族）+ 图像家族独立门禁。
.DESCRIPTION
  背景：图像家族 11 crate = 126k 行 = 全仓 .rs 22.3%，是"全量 >18 分钟"的最大单项。
  R4a-1 只做门禁隔离（可逆），不动源码目录；R4a-2（条件触发）才抽独立 workspace。
  vendored 清单唯一来源 = docs/architecture/contract.toml [vendored].image_family，
  经 `python scripts/arch_guard.py --print-vendored` 读取，禁止手抄第二份。
.USAGE
  pwsh scripts/gate_full.ps1 -DryRun        # 只打印将执行的命令
  pwsh scripts/gate_full.ps1                # 主 + 图像全量
  pwsh scripts/gate_full.ps1 -MainOnly      # 只主 workspace（排除图像）
  pwsh scripts/gate_full.ps1 -ImageOnly     # 只图像家族
.NOTES
  退出码：任一阶段失败即 1；日志落 logs/gate-full.log，按落盘退出码判定（不做管道直判）。
#>
param(
    [switch]$MainOnly,
    [switch]$ImageOnly,
    [switch]$DryRun,
    [string]$Log = "logs/gate-full.log"
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot
$env:PYTHONIOENCODING = 'utf-8'

$LogPath = if ([IO.Path]::IsPathRooted($Log)) { $Log } else { Join-Path $RepoRoot $Log }
New-Item -ItemType Directory -Force (Split-Path -Parent $LogPath) | Out-Null
Set-Content -Path $LogPath -Value "gate_full run: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" -Encoding UTF8

$vendored = @(& python scripts/arch_guard.py --print-vendored 2>$null | Where-Object { $_ -and $_ -notmatch '^#' })
if ($LASTEXITCODE -ne 0 -or $vendored.Count -eq 0) {
    throw "无法从契约读取 vendored 清单（python scripts/arch_guard.py --print-vendored）"
}
Write-Host "vendored image family: $($vendored.Count) crates（来源 contract.toml [vendored]）"
$excludeArgs = @(); foreach ($p in $vendored) { $excludeArgs += @('--exclude', $p) }
$imagePkgArgs = @(); foreach ($p in $vendored) { $imagePkgArgs += @('-p', $p) }

function Invoke-Stage {
    param([string]$Name, [string[]]$ArgsList)
    Write-Host "== $Name"
    Write-Host ("   " + ($ArgsList -join ' '))
    if ($DryRun) { return 0 }
    $exe = $ArgsList[0]
    $rest = @($ArgsList | Select-Object -Skip 1)
    & $exe @rest 2>&1 | Tee-Object -FilePath $LogPath -Append | Out-Null
    $code = $LASTEXITCODE
    if ($code -ne 0) { Write-Host "FAIL(exit=$code): $Name"; return $code }
    Write-Host "OK: $Name"
    return 0
}

$failed = @()
if (-not $ImageOnly) {
    $stages = @(
        @{ n = 'main.cargo-test';        a = @('cargo', 'test', '--workspace', '--all-targets', '--locked', '--no-fail-fast', '-j', '2') + $excludeArgs },
        @{ n = 'main.clippy';            a = @('cargo', 'clippy', '--workspace', '--all-targets', '--locked') + $excludeArgs + @('--', '-D', 'warnings') },
        @{ n = 'main.fmt';               a = @('cargo', 'fmt', '--all', '--', '--check') }
    )
    foreach ($s in $stages) {
        $code = Invoke-Stage -Name $s.n -ArgsList $s.a
        if ($code -ne 0) { $failed += $s.n; break }
    }
}

if (-not $MainOnly -and $failed.Count -eq 0) {
    $stages = @(
        @{ n = 'image.cargo-test';       a = @('cargo', 'test', '--locked', '--all-targets') + $imagePkgArgs },
        @{ n = 'image.clippy';           a = @('cargo', 'clippy', '--locked', '--all-targets') + $imagePkgArgs + @('--', '-D', 'warnings') }
    )
    foreach ($s in $stages) {
        $code = Invoke-Stage -Name $s.n -ArgsList $s.a
        if ($code -ne 0) { $failed += $s.n; break }
    }
}

Write-Host ""
Write-Host "log: $LogPath"
if ($failed.Count -gt 0) {
    Write-Host ("GATE FULL FAILED: " + ($failed -join ', '))
    exit 1
}
Write-Host "GATE FULL PASSED"
exit 0
