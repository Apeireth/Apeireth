# =============================================================================
# packaging/scoop/uninstall-scoop.ps1
#
# Scoop manifest 的 user-side uninstall helper (per task spec 1.0 release #4)
# vs packaging/scoop/install-scoop.ps1: 后者装, 本脚本卸
#
# 决策: D-06 (8 包齐发)
# Manifest: packaging/scoop/apeireth.json
# 兄弟: packaging/scoop/install-scoop.ps1 (装) / scripts/install/uninstall-all.sh (跨包)
#
# 6 哲学锚穿透:
#   1. 不假装已卸 — scoop list 验证
#   2. 守门 — Windows + scoop 命令守门, 确认提示
#   3. 真理 — scoop uninstall 真清, NSSM 服务 stop
#   4. 进化 — 保留数据选项 (-KeepData), 保留 bucket (-KeepBucket)
#   5. 择善 — 用 scoop 而非手 rm ~/scoop/apps
#   6. 不从 — 用户确认前不动手
#
# 8 项不修改承诺:
#   - 0 改 24 LOCKED crate
#   - 0 改 workspace version 1.0.0
#   - 0 引 NewAPI (用系统 scoop/nssm/schtasks)
#   - 不假装: 缺 scoop / 未装时显式报
#   - 编译期 hardcode: BUCKET=apeireth/scoop-bucket, APEIRETH_HOME=%USERPROFILE%\.apeireth
#   - 6 哲学锚穿透
#   - 不重复造轮子: 用 scoop uninstall + manifest 的 pre_uninstall 钩子
#   - 诚实标缺: NSSM 服务不在 manifest 默认注册, 标 TODO
#
# 用法 (PowerShell):
#   .\packaging\scoop\uninstall-scoop.ps1                 # 卸 + 删 bucket + 清数据
#   .\packaging\scoop\uninstall-scoop.ps1 -KeepData       # 卸, 保留数据
#   .\packaging\scoop\uninstall-scoop.ps1 -KeepBucket     # 卸, 保留 bucket
#   .\packaging\scoop\uninstall-scoop.ps1 -Force          # 跳过 y/N 确认
# =============================================================================

[CmdletBinding()]
param(
    [switch]$KeepData,
    [switch]$KeepBucket,
    [switch]$Force
)

$ErrorActionPreference = 'Stop'

# === 0. Windows + scoop 守门 ===
if ($env:OS -ne "Windows_NT") {
    Write-Host "❌ 此脚本仅在 Windows 跑"
    exit 1
}
if (-not (Get-Command scoop -ErrorAction SilentlyContinue)) {
    Write-Host "❌ scoop 未装, 无需卸载"
    exit 1
}

$BUCKET = $env:APEIRETH_BUCKET
if (-not $BUCKET) { $BUCKET = "apeireth/scoop-bucket" }

Write-Host "=== apeireth scoop uninstall ==="
Write-Host "    BUCKET:     ${BUCKET}"
Write-Host "    KeepData:   ${KeepData}"
Write-Host "    KeepBucket: ${KeepBucket}"
Write-Host "    Force:      ${Force}"

# === 2. 检查是否装了 ===
$installed = scoop list apeireth 2>$null
if (-not $installed -or $installed -notmatch "apeireth") {
    Write-Host "❌ scoop 未装 apeireth"
    Write-Host "   手动查: scoop list | findstr apeireth"
    exit 1
}

# === 3. 确认 (除非 -Force) ===
if (-not $Force) {
    $confirm = Read-Host "确认卸载 apeireth (scoop)? (y/N)"
    if ($confirm -ne "y" -and $confirm -ne "Y") {
        Write-Host "已取消"
        exit 0
    }
}

# === 4. 停 NSSM 服务 (如有) ===
Write-Host "[1/5] 停 NSSM 服务 (如有)..."
if (Get-Command nssm -ErrorAction SilentlyContinue) {
    $nssmStatus = nssm status Apeireth 2>$null
    if ($nssmStatus -match "SERVICE_RUNNING") {
        nssm stop Apeireth 2>$null
    }
    nssm remove Apeireth confirm 2>$null
    Write-Host "    ✅ NSSM 服务已清"
} else {
    Write-Host "    (无 nssm, 跳过 — 服务未注册为 NSSM)"
}

# === 5. 停 Task Scheduler 任务 (如有) ===
Write-Host "[2/5] 停 Task Scheduler 任务 (如有)..."
$task = Get-ScheduledTask -TaskName "Apeireth" -ErrorAction SilentlyContinue
if ($task) {
    Unregister-ScheduledTask -TaskName "Apeireth" -Confirm:$false
    Write-Host "    ✅ Task Scheduler 任务已清"
} else {
    Write-Host "    (无 Task Scheduler 任务, 跳过)"
}

# === 6. 停进程 (防 file lock) ===
Write-Host "[3/5] 停 apeireth 进程..."
Get-Process apeireth -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1

# === 7. scoop uninstall ===
Write-Host "[4/5] scoop uninstall apeireth..."
scoop uninstall apeireth

# === 8. bucket 清理 ===
if (-not $KeepBucket) {
    Write-Host "[5a/5] scoop bucket rm apeireth..."
    $buckets = scoop bucket list 2>$null
    if ($buckets -match "apeireth") {
        scoop bucket rm apeireth 2>$null
    } else {
        Write-Host "    (bucket 不存在, 跳过)"
    }
} else {
    Write-Host "[5a/5] ⚠️  保留 bucket (-KeepBucket): ${BUCKET}"
}

# === 9. 数据清理 ===
if (-not $KeepData) {
    Write-Host "[5b/5] drop data..."
    $apeirethHome = "$env:USERPROFILE\.apeireth"
    if (Test-Path $apeirethHome) {
        Remove-Item -Recurse -Force $apeirethHome
    }
    # 清 APEIRETH_HOME 环境变量
    [Environment]::SetEnvironmentVariable('APEIRETH_HOME', $null, 'User')
} else {
    Write-Host "[5b/5] ⚠️  保留数据 (-KeepData): $env:USERPROFILE\.apeireth"
}

# === 10. 验证 0 残留 ===
Write-Host ""
Write-Host "=== 验证 0 残留 ==="
$residue = 0

$stillInstalled = scoop list apeireth 2>$null
if ($stillInstalled -and $stillInstalled -match "apeireth") {
    Write-Host "    ❌ scoop list 仍有 apeireth"
    $residue++
}

if (-not $KeepData) {
    $apeirethHome = "$env:USERPROFILE\.apeireth"
    if (Test-Path $apeirethHome) {
        Write-Host "    ❌ $apeirethHome 残留"
        $residue++
    }
}

if ($residue -eq 0) {
    Write-Host "    ✅ 0 残留, 卸载完成"
    Write-Host ""
    Write-Host "重装: .\packaging\scoop\install-scoop.ps1"
    exit 0
} else {
    Write-Host ""
    Write-Host ("⚠️  {0} 项残留, 手动清: scoop uninstall --purge apeireth; Remove-Item -Recurse -Force `$env:USERPROFILE\.apeireth" -f $residue)
    exit 1
}
