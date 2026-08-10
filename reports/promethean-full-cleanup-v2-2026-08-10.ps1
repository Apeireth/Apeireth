# promethean-full-cleanup-v2-2026-08-10.ps1
# 主人 22:02 自执行, per 决策 #59 v2 (处理 locked file/folder)
# 跳过被 lock 的项 + 用 cmd.exe rmdir 兜底 + 多次重试 + 强制删除
# 不动: borrowed-repos / apeireth-debug / 新主仓 `Apeireth-rust\`

$ErrorActionPreference = 'Continue'  # 改 Continue, 0 让 locked 中断
$promethean = '.openclaw\workspace\promethean'
$borrowedRepos = '.openclaw\workspace\borrowed-repos'
$apeirethDebug = '.openclaw\workspace\apeireth-debug'
$newMaster = 'Apeireth-rust'

function Remove-ItemIgnoreLock {
    param([string]$Path, [int]$MaxRetry = 3, [int]$WaitSec = 2)
    for ($i = 1; $i -le $MaxRetry; $i++) {
        try {
            if (Test-Path -LiteralPath $Path) {
                Remove-Item -LiteralPath $Path -Recurse -Force -ErrorAction Stop
            }
            return $true
        } catch {
            $err = $_.Exception.Message
            Write-Host "  [retry $i/$MaxRetry] $Path locked: $err"
            if ($i -lt $MaxRetry) {
                Start-Sleep -Seconds $WaitSec
            }
        }
    }
    return $false
}

function Remove-ItemCmdRmdir {
    param([string]$Path)
    Write-Host "  [fallback cmd rmdir] $Path"
    try {
        $p = $Path -replace '/', '\'
        cmd /c "rmdir /s /q `"$p`"" 2>&1 | Out-Null
        return $true
    } catch {
        Write-Host "  [fallback failed] $_"
        return $false
    }
}

Write-Host '=== Before ==='
Write-Host "promethean exists: $(Test-Path -LiteralPath $promethean -PathType Container)"

if (-not (Test-Path -LiteralPath $promethean -PathType Container)) {
    Write-Host "promethean/ 0 存在, 0 必删, exit"
    exit 0
}

Write-Host '=== 1st pass: PowerShell Remove-Item 跳过 lock + cmd rmdir 兜底 ==='
$items = Get-ChildItem -LiteralPath $promethean -Force
$count = 0
$locked = @()
foreach ($item in $items) {
    $count++
    $ok = Remove-ItemIgnoreLock -Path $item.FullName
    if (-not $ok) {
        $ok2 = Remove-ItemCmdRmdir -Path $item.FullName
        if (-not $ok2) {
            $locked += $item.FullName
        }
    }
    if ($count % 10 -eq 0) {
        Write-Host "processed $count items..."
    }
}
Write-Host "1st pass: $count processed, $($locked.Count) still locked"

if ($locked.Count -gt 0) {
    Write-Host '=== Locked items ==='
    $locked | ForEach-Object { Write-Host "  $_" }
}

Write-Host '=== 2nd pass: 重试 locked 项 ==='
$stillLocked = @()
foreach ($path in $locked) {
    if (Test-Path -LiteralPath $path) {
        $ok = Remove-ItemIgnoreLock -Path $path -MaxRetry 5 -WaitSec 5
        if (-not $ok) {
            $ok2 = Remove-ItemCmdRmdir -Path $path
            if (-not $ok2) {
                $stillLocked += $path
            }
        }
    }
}
Write-Host "2nd pass: $($stillLocked.Count) still locked after 5 retry + cmd rmdir"

if ($stillLocked.Count -gt 0) {
    Write-Host '=== Still locked items (3rd pass 5s wait + 5 retry) ==='
    foreach ($path in $stillLocked) {
        if (Test-Path -LiteralPath $path) {
            $ok = Remove-ItemIgnoreLock -Path $path -MaxRetry 5 -WaitSec 5
            if (-not $ok) {
                Write-Host "  [FINAL FAIL] $path - 主人需要手动处理 (可能是 stale handle, 关闭 IDE/编辑器/IDE 后重试)"
            }
        }
    }
}

Write-Host '=== Try remove promethean/ itself ==='
if (Test-Path -LiteralPath $promethean -PathType Container) {
    $itemsLeft = Get-ChildItem -LiteralPath $promethean -Force
    Write-Host "promethean/ 还剩 $($itemsLeft.Count) 项"
    if ($itemsLeft.Count -eq 0) {
        Remove-Item -LiteralPath $promethean -Force
        Write-Host "promethean/ 删了"
    } else {
        Write-Host "promethean/ 还有内容, 0 删顶层, 主人手动处理"
    }
}

Write-Host '=== After ==='
Write-Host "promethean exists: $(Test-Path -LiteralPath $promethean -PathType Container)"
Write-Host "borrowed-repos exists: $(Test-Path -LiteralPath $borrowedRepos -PathType Container)"
Write-Host "apeireth-debug exists: $(Test-Path -LiteralPath $apeirethDebug -PathType Container)"
Write-Host "new master HEAD: $((Get-Content (Join-Path $newMaster '.git\refs\heads\master')).Trim())"

Write-Host '=== promethean/ 全删 done (or partial) ==='
