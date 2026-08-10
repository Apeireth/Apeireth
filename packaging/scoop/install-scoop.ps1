# =============================================================================
# packaging/scoop/install-scoop.ps1
#
# Scoop manifest 的 user-side install helper (per task spec 1.0 release #4)
# vs packaging/scoop/build.ps1: 后者是 release engineer 用的 (推 manifest 到 bucket)
#                                 前者是 end user 用的 (从 bucket 装)
#
# 决策: D-06 (8 包齐发)
# Manifest: packaging/scoop/apeireth.json
# 兄弟: scripts/install/install-scoop.ps1 (跨包统一入口)
#
# 用法 (PowerShell):
#   .\packaging\scoop\install-scoop.ps1                                # 装正式版
#   $env:APEIRETH_BUCKET = "apeireth/scoop-bucket"
#   .\packaging\scoop\install-scoop.ps1
# 卸载: scoop uninstall apeireth
# =============================================================================

$ErrorActionPreference = 'Stop'

$VERSION = $env:APEIRETH_VERSION
if (-not $VERSION) { $VERSION = "1.0.0" }
$BUCKET = $env:APEIRETH_BUCKET
if (-not $BUCKET) { $BUCKET = "apeireth/scoop-bucket" }

Write-Host "=== apeireth scoop install v${VERSION} (bucket=${BUCKET}) ==="

# 1. scoop 检测
if (-not (Get-Command scoop -ErrorAction SilentlyContinue)) {
    Write-Host "❌ scoop 未装, 先装: irm get.scoop.sh | iex"
    exit 1
}

# 2. bucket add
$scoopBuckets = scoop bucket list 2>$null
if ($scoopBuckets -notmatch "apeireth") {
    Write-Host "[1/3] scoop bucket add apeireth https://github.com/${BUCKET}..."
    scoop bucket add apeireth "https://github.com/${BUCKET}"
}

# 3. install
Write-Host "[2/3] scoop install apeireth..."
scoop install apeireth

# 4. 配置 APEIRETH_HOME
Write-Host "[3/3] 配置 APEIRETH_HOME..."
$env:APEIRETH_HOME = "$env:USERPROFILE\.apeireth"
[Environment]::SetEnvironmentVariable('APEIRETH_HOME', $env:APEIRETH_HOME, 'User')
if (-not (Test-Path $env:APEIRETH_HOME)) {
    New-Item -ItemType Directory -Path $env:APEIRETH_HOME -Force | Out-Null
}

# 5. 报告
try {
    $response = Invoke-WebRequest -Uri "http://localhost:8080/health" -Method GET -UseBasicParsing -TimeoutSec 5
    Write-Host "    ✅ /health: $($response.Content)"
} catch {
    Write-Host "    ⚠️  /health 未响应, 看: scoop which apeireth, scoop status apeireth"
}

Write-Host "✅ 安装完成 (详细见 docs/installation/windows-scoop-install.md)"
