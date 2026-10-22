#!/usr/bin/env pwsh
<#
.SYNOPSIS
  全量门禁（ADR-0005 R4a-1；ADR-0011 R6-A 单一执行器）：主 workspace 门禁（排除 vendored 图像家族）
  + 图像家族独立门禁。
.DESCRIPTION
  背景：图像家族 11 crate = 126k 行 = 全仓 .rs 22.3%，是"全量 >18 分钟"的最大单项。
  R4a-1 只做门禁隔离（可逆），不动源码目录；R4a-2（条件触发）才抽独立 workspace。
  vendored 清单唯一来源 = docs/architecture/contract.toml [vendored].image_family，
  经 `python scripts/arch_guard.py --print-vendored` 读取，禁止手抄第二份。

  R6-A（ADR-0011 D2）：
  - 进程级文件锁 logs/gate-full.lock：同一时刻只允许一次全量执行；默认排队等待，
    -FailIfBusy 时立即以 exit 2 退出（供 CI/并行线判定"已被占用"而非"失败"）。
  - 失败归因 FAILURE_CLASS（oom / env / flaky / regression）：结果落日志与 stdout，
    仍以 exit 1 表示失败；分类只是归因口径，不改变门禁结论。
.USAGE
  pwsh scripts/gate_full.ps1 -DryRun        # 只打印将执行的命令（不加锁）
  pwsh scripts/gate_full.ps1                # 主 + 图像全量（拿不到锁则排队）
  pwsh scripts/gate_full.ps1 -FailIfBusy    # 另一全量在跑时立即 exit 2
  pwsh scripts/gate_full.ps1 -MainOnly      # 只主 workspace（排除图像）
  pwsh scripts/gate_full.ps1 -ImageOnly     # 只图像家族
.NOTES
  退出码：0 = 通过；1 = 任一阶段失败（附 FAILURE_CLASS）；2 = 锁被占用/参数环境问题。
  日志落 logs/gate-full.log，按落盘退出码判定（不做管道直判）。
#>
param(
    [switch]$MainOnly,
    [switch]$ImageOnly,
    [switch]$DryRun,
    [switch]$FailIfBusy,
    [int]$LockTimeoutMinutes = 90,
    [string]$ClassifyLog = "",
    [string]$Log = "logs/gate-full.log"
)

$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot
$env:PYTHONIOENCODING = 'utf-8'

$LogPath = if ([IO.Path]::IsPathRooted($Log)) { $Log } else { Join-Path $RepoRoot $Log }
New-Item -ItemType Directory -Force (Split-Path -Parent $LogPath) | Out-Null

function Write-LogLine {
    param([string]$Line)
    $Line | Tee-Object -FilePath $LogPath -Append | Out-Null
}

function Get-FailureClass {
    param([string[]]$FailedStages)
    if ($FailedStages.Count -eq 0) { return 'none' }
    $text = ''
    try { $text = Get-Content -Raw -Path $ClassifyLog -ErrorAction SilentlyContinue } catch { }
    if (-not $text) {
        try { $text = Get-Content -Raw -Path $LogPath -ErrorAction SilentlyContinue } catch { }
    }
    if (-not $text) { $text = '' }
    # 证据判定（ADR-0011 D2）：
    # 1) oom = 分配失败 / OOM killer / 链接器内存错误
    if ($text -match 'memory allocation of|out of memory|failed to allocate|cannot allocate memory|Os \{ code: 12|signal: 9|killed|LNK1102|LNK1318') {
        return 'oom'
    }
    # 2) env = 构建目录锁 / 缺工具 / 磁盘空间 / 网络 / 权限
    if ($text -match 'Blocking waiting for file lock on build directory|could not find native static library|linker .* not found|failed to run custom build command|No space left on device|Access is denied|Permission denied|error: linker .* failed|failed to download|Connection refused|timed out') {
        return 'env'
    }
    # 3) flaky = 重跑通过且无代码改动（日志显式重试/重跑证据）
    if ($text -match 'retrying|flaky|passed on retry|FAILED \(retried\)|rerun.*passed') {
        return 'flaky'
    }
    # 4) 其余（测试 panic / clippy / fmt / 编译错误）= regression
    return 'regression'
}

# 校验/调试用途：对既有日志判定 FAILURE_CLASS，不执行门禁、不加锁。
if ($ClassifyLog) {
    $classifyPath = if ([IO.Path]::IsPathRooted($ClassifyLog)) { $ClassifyLog } else { Join-Path $RepoRoot $ClassifyLog }
    if (-not (Test-Path -LiteralPath $classifyPath)) { Write-Host "ClassifyLog 不存在: $classifyPath"; exit 2 }
    $class = Get-FailureClass -FailedStages @('manual.classify')
    Write-Host "FAILURE_CLASS=$class"
    exit 0
}

# ---- R6-A 进程级文件锁（同一时刻只允许一次全量） ------------------------
$LockPath = Join-Path $RepoRoot 'logs/gate-full.lock'
$LockStream = $null
$LockOwned = $false
if (-not $DryRun) {
    $deadline = (Get-Date).AddMinutes($LockTimeoutMinutes)
    $waited = $false
    while ($true) {
        try {
            $LockStream = [System.IO.File]::Open(
                $LockPath,
                [System.IO.FileMode]::OpenOrCreate,
                [System.IO.FileAccess]::ReadWrite,
                [System.IO.FileShare]::None)
            $LockOwned = $true
            $info = "pid=$PID started=$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') host=$env:COMPUTERNAME"
            $bytes = [System.Text.Encoding]::UTF8.GetBytes($info)
            $LockStream.SetLength(0)
            $LockStream.Write($bytes, 0, $bytes.Length)
            $LockStream.Flush()
            break
        }
        catch {
            if ($FailIfBusy) {
                Write-Host "GATE FULL BUSY: logs/gate-full.lock 被另一全量执行占用（-FailIfBusy）；未运行本批门禁。"
                Write-Host "  lock: $LockPath"
                exit 2
            }
            if ((Get-Date) -gt $deadline) {
                Write-Host "GATE FULL BUSY: 等待 logs/gate-full.lock 超时（${LockTimeoutMinutes} 分钟）；未运行本批门禁。"
                exit 2
            }
            if (-not $waited) {
                $owner = ''
                try { $owner = (Get-Content -Raw -Path $LockPath -ErrorAction SilentlyContinue) } catch { }
                Write-Host "== 等待全量门禁锁（另一执行中: $owner）；每 5 秒重试，最长 ${LockTimeoutMinutes} 分钟"
                $waited = $true
            }
            Start-Sleep -Seconds 5
        }
    }
}

Write-LogLine "gate_full run: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') script=$PSCommandPath"

$vendored = @(& python scripts/arch_guard.py --print-vendored 2>$null | Where-Object { $_ -and $_ -notmatch '^#' })
if ($LASTEXITCODE -ne 0 -or $vendored.Count -eq 0) {
    if ($LockOwned -and $LockStream) { $LockStream.Dispose(); $LockStream = $null; Remove-Item -Path $LockPath -Force -ErrorAction SilentlyContinue }
    throw "无法从契约读取 vendored 清单（python scripts/arch_guard.py --print-vendored）"
}
Write-LogLine "vendored image family: $($vendored.Count) crates（来源 contract.toml [vendored]）"
Write-Host "vendored image family: $($vendored.Count) crates（来源 contract.toml [vendored]）"
$excludeArgs = @(); foreach ($p in $vendored) { $excludeArgs += @('--exclude', $p) }
$imagePkgArgs = @(); foreach ($p in $vendored) { $imagePkgArgs += @('-p', $p) }

function Invoke-Stage {
    param([string]$Name, [string[]]$ArgsList)
    Write-LogLine "== $Name"
    Write-LogLine ("   " + ($ArgsList -join ' '))
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
$failedDetail = @{}
try {
    if (-not $ImageOnly) {
        $stages = @(
            @{ n = 'main.cargo-test';        a = @('cargo', 'test', '--workspace', '--all-targets', '--locked', '--no-fail-fast', '-j', '2') + $excludeArgs },
            @{ n = 'main.clippy';            a = @('cargo', 'clippy', '--workspace', '--all-targets', '--locked') + $excludeArgs + @('--', '-D', 'warnings') },
            @{ n = 'main.fmt';               a = @('cargo', 'fmt', '--all', '--', '--check') }
        )
        foreach ($s in $stages) {
            $code = Invoke-Stage -Name $s.n -ArgsList $s.a
            if ($code -ne 0) { $failed += $s.n; $failedDetail[$s.n] = $code; break }
        }
    }

    if (-not $MainOnly -and $failed.Count -eq 0) {
        $stages = @(
            @{ n = 'image.cargo-test';       a = @('cargo', 'test', '--locked', '--all-targets') + $imagePkgArgs },
            @{ n = 'image.clippy';           a = @('cargo', 'clippy', '--locked', '--all-targets') + $imagePkgArgs + @('--', '-D', 'warnings') }
        )
        foreach ($s in $stages) {
            $code = Invoke-Stage -Name $s.n -ArgsList $s.a
            if ($code -ne 0) { $failed += $s.n; $failedDetail[$s.n] = $code; break }
        }
    }

    Write-Host ""
    Write-Host "log: $LogPath"
    $finishedAt = Get-Date -Format 'yyyy-MM-dd HH:mm:ss'
    if ($failed.Count -gt 0) {
        $class = Get-FailureClass -FailedStages $failed
        $detail = ($failed | ForEach-Object { "$_=$($failedDetail[$_])" }) -join ', '
        Write-LogLine "GATE FULL FAILED ($finishedAt): $detail"
        Write-LogLine "FAILURE_CLASS=$class"
        Write-Host ("GATE FULL FAILED: " + ($failed -join ', '))
        Write-Host "FAILURE_CLASS=$class（归因口径；修复后须重跑，未定性不放行）"
        exit 1
    }
    Write-LogLine "GATE FULL PASSED ($finishedAt)"
    Write-Host "GATE FULL PASSED"
    exit 0
}
finally {
    if ($LockOwned -and $LockStream) {
        try { $LockStream.Dispose() } catch { }
        $LockStream = $null
        Remove-Item -Path $LockPath -Force -ErrorAction SilentlyContinue
    }
}
