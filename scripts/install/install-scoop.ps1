# =============================================================================
# scripts/install/install-scoop.ps1
#
# Apeireth OS — Windows Scoop 安装入口
# (1.0 release checklist #4 install, D-06 8 包齐发)
#
# 蓝图: docs/stage4/v09021-rust-translation-blueprint-2026-08-05.md §3.4
# 决策: D-06 (主人 2026-08-05 20:53 拍 A: 8 包齐发)
# Manifest: packaging/scoop/apeireth.json
#
# 5 步标准安装流 (per 蓝图 §3.4):
#   1. 检测 scoop 是否装
#   2. bucket add apeireth (per packaging/scoop/build.ps1 推送到 scoop-bucket 仓库)
#   3. scoop install apeireth
#   4. scoop update 路径 + 创建 APEIRETH_HOME 目录
#   5. 健康检查 Invoke-WebRequest /health
#
# 8 项不修改承诺 (同 install-deb.sh, 不重述):
#   - 0 改 24 LOCKED, 0 改 workspace version, 0 引 NewAPI
#   - 编译期 hardcode VERSION=1.0.0, BUCKET_REPO=apeireth/scoop-bucket
#   - 不重复造轮子: 调 packaging/scoop/apeireth.json, 不重写 manifest
#
# 用法 (PowerShell):
#   .\scripts\install\install-scoop.ps1                                # 装正式版
#   $env:APEIRETH_BUCKET_REPO = "apeireth/scoop-bucket-test"
#   .\scripts\install\install-scoop.ps1
# 卸载: scoop uninstall apeireth
# =============================================================================

$ErrorActionPreference = 'Stop'

Set-Location $PSScriptRoot\..\..

$VERSION = $env:APEIRETH_VERSION
if (-not $VERSION) { $VERSION = "1.0.0" }
$BUCKET_REPO = $env:APEIRETH_BUCKET_REPO
if (-not $BUCKET_REPO) { $BUCKET_REPO = "apeireth/scoop-bucket" }

Write-Host "=== apeireth scoop install v${VERSION} ==="
Write-Host "    bucket: ${BUCKET_REPO}"

# === 0. Windows 守门 ===
if ($env:OS -ne "Windows_NT") {
    Write-Host "❌ 此脚本仅在 Windows 跑"
    exit 1
}

# === 1. scoop 检测 ===
if (-not (Get-Command scoop -ErrorAction SilentlyContinue)) {
    Write-Host "❌ scoop 未装, 先装: Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser"
    Write-Host "   irm get.scoop.sh | iex"
    exit 1
}
Write-Host "[1/5] ✅ scoop 已装: $(scoop --version)"

# === 2. bucket add apeireth ===
Write-Host "[2/5] scoop bucket add apeireth https://github.com/${BUCKET_REPO}..."
$scoopBuckets = scoop bucket list 2>$null
if ($scoopBuckets -match "apeireth") {
    Write-Host "    ✅ bucket 已存在"
} else {
    scoop bucket add apeireth "https://github.com/${BUCKET_REPO}"
}

# === 3. scoop install ===
Write-Host "[3/5] scoop install apeireth..."
scoop install apeireth
# 注: 装过的会自动跳过, 如需重装: scoop uninstall apeireth; scoop install apeireth

# === 4. 配置 APEIRETH_HOME + PATH ===
Write-Host "[4/5] 配置 APEIRETH_HOME + 刷新 PATH..."
# post_install 已在 manifest 处理, 这里做补充
$env:APEIRETH_HOME = "$env:USERPROFILE\.apeireth"
if (-not (Test-Path $env:APEIRETH_HOME)) {
    New-Item -ItemType Directory -Path $env:APEIRETH_HOME -Force | Out-Null
}
# PATH 刷新 (scoop shims 已在 user PATH)
$env:Path = [System.Environment]::GetEnvironmentVariable("Path", "User")

# === 5. 健康检查 ===
Write-Host "[5/5] 健康检查 /health (期望 200)..."
try {
    $response = Invoke-WebRequest -Uri "http://localhost:8080/health" -Method GET -UseBasicParsing -TimeoutSec 5
    $health = $response.Content
    Write-Host "    ✅ /health: ${health}"
} catch {
    Write-Host "⚠️  /health 未响应: $($_.Exception.Message)"
    Write-Host "    安装完成, 服务可能未启动 (请人工: scoop which apeireth, apeireth serve)"
    exit 0
}

Write-Host ""
Write-Host "✅ apeireth ${VERSION} 安装完成"
Write-Host "    状态: scoop status apeireth"
Write-Host "    日志: $env:USERPROFILE\.apeireth\logs\"
Write-Host "    卸载: scoop uninstall apeireth"
Write-Host "          或: scripts\uninstall\uninstall.ps1 --channel scoop"
