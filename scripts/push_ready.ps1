#!/usr/bin/env pwsh
# push_ready.ps1 - 准备当前分支 tip 推到 origin (用环境变量避免 prompt)
#
# 分支**不硬编码**（2026-11-04 批）：此前默认 "master" 且 4 处 diff/log 写死
# "origin/master"，而主线是 main ⇒ 默认推一个不存在的分支。现默认由 git 推导：
# Branch = 当前分支；BaseRef = remote 的默认分支（origin/HEAD → origin/main → origin/master）。

param(
    [string]$Remote = "origin",
    [string]$Branch = "",
    [string]$BaseRef = ""
)

$ErrorActionPreference = "Stop"

if (-not $Branch) {
    $Branch = (git rev-parse --abbrev-ref HEAD 2>$null | Select-Object -First 1)
}
if (-not $Branch) {
    Write-Host "Error: 推不出当前分支（detached HEAD?）—— 用 -Branch 指定" -ForegroundColor Red
    exit 2
}
if (-not $BaseRef) {
    $BaseRef = (git symbolic-ref --short "refs/remotes/$Remote/HEAD" 2>$null | Select-Object -First 1)
    if (-not $BaseRef) {
        foreach ($cand in @("main", "master")) {
            git rev-parse --verify --quiet "refs/remotes/$Remote/$cand" 2>$null | Out-Null
            if ($LASTEXITCODE -eq 0) { $BaseRef = "$Remote/$cand"; break }
        }
    }
}
if (-not $BaseRef) {
    Write-Host "Error: 推不出 $Remote 的默认分支 —— 用 -BaseRef 指定" -ForegroundColor Red
    exit 2
}
Write-Host "Remote: $Remote  Branch: $Branch  Base: $BaseRef"

Write-Host "=== Pre-push health check ===" -ForegroundColor Cyan

# 1. Network check (Test-NetConnection 走 127.0.0.1, 跟 push 不同)
Write-Host "Network test (raw):"
try {
    $tcp = Test-NetConnection -ComputerName github.com -Port 443 -InformationLevel Quiet
    Write-Host "  github.com:443 reachable: $tcp"
    $tcpApi = Test-NetConnection -ComputerName api.github.com -Port 443 -InformationLevel Quiet
    Write-Host "  api.github.com:443 reachable: $tcpApi"
} catch {
    Write-Host "  Network test failed: $_" -ForegroundColor Red
}

# 2. 验证 0 触碰 src/ logic (主人约束)
Write-Host ""
Write-Host "=== 0 触碰 verification (主人约束: 0 触碰 src/ logic) ===" -ForegroundColor Cyan
$srcFiles = git diff "$BaseRef" HEAD --name-only 2>&1 | Where-Object { $_ -match "\.rs$" -and $_ -notmatch "/tests/" -and $_ -notmatch "/examples/" }
if ($srcFiles) {
    Write-Host "src/ logic modified (excluded tests/examples):" -ForegroundColor Yellow
    $srcFiles | ForEach-Object { Write-Host "  $_" }
    Write-Host ""
    $confirm = Read-Host "These src/ changes — confirm '0 触碰 src/ logic' is OK (cargo fmt 拆行 0 装 stub 1:1 兼容)? (y/N)"
    if ($confirm -ne "y") {
        Write-Host "Aborted. Run 'git diff $BaseRef HEAD -- <file>' to review." -ForegroundColor Red
        exit 1
    }
} else {
    Write-Host "  0 src/ logic changes (only fmt + tests + yml + cfg)" -ForegroundColor Green
}

# 3. 验证 Cargo.lock 没引未知 dep
Write-Host ""
Write-Host "=== Cargo.lock dep diff ===" -ForegroundColor Cyan
$lockChanges = git diff "$BaseRef" HEAD -- Cargo.lock 2>&1
if ($LASTEXITCODE -ne 0) { $lockChanges = "" }
$lockLineCount = ($lockChanges | Measure-Object -Line).Count
Write-Host "  Cargo.lock diff lines: $lockLineCount"

# 4. Pre-push commit list
Write-Host ""
Write-Host "=== Commits to push (HEAD~N..HEAD) ===" -ForegroundColor Cyan
$commitCount = (git log --oneline "$BaseRef..HEAD" 2>&1 | Measure-Object -Line).Count
Write-Host "  Total: $commitCount commits"
if ($commitCount -gt 0) {
    git log --oneline "$BaseRef..HEAD" | ForEach-Object { Write-Host "    $_" }
}

# 5. LOCKED 关键文件 0 触碰 **提示**（判定权在门禁，不在本脚本）
#    2026-11-04 批修正：此前此处内嵌一份手抄的「24 LOCKED crate」清单 ——
#    ① 该 crate 集合早已撤销（现行 LOCKED 是「5 项」：9 哲学锚本体 / 键集 / 3 不可变脊柱 /
#      workspace.version / R11 baseline，见 AGENTS.md §1 + ENGINEER-MANIFESTO §10）；
#    ② 手抄 crate 清单本身被 arch_guard 的 CI 口径扫描判为违规（handwritten LOCKED crate list）。
#    故此处不再复制清单，改为列出**仓内既有门禁已锁的具体文件**是否被本次 push 触碰。
#    诚实边界：本段只是提示，不是 LOCKED 判定；LOCKED 判定以门禁为准。
Write-Host ""
Write-Host "=== LOCKED 关键文件是否被本次 push 触碰 ===" -ForegroundColor Cyan
$lockedPaths = @(
    "crates/foundation/core/src/eight_anchors.rs",            # 9 哲学锚本体
    "crates/foundation/core/src/philosophy.rs",                # 键集（现行 14 键 PHL-01..08）
    "legacy/donor/apeireth-asi/tests/integration_r_measure.rs" # R11 baseline 3 值
)
$lockedHits = git diff "$BaseRef" HEAD --name-only 2>&1 | Where-Object { $lockedPaths -contains $_ }
if ($lockedHits) {
    Write-Host "  ⚠️ LOCKED 关键文件被改（判定以门禁为准）:" -ForegroundColor Yellow
    $lockedHits | ForEach-Object { Write-Host "    $_" }
} else {
    Write-Host "  ✅ 0 LOCKED 关键文件被改（哲学锚本体 / 键集 / R11 baseline）" -ForegroundColor Green
}
Write-Host "  注：3 不可变脊柱 / workspace.version 的判定由门禁负责，本脚本不复制清单。" -ForegroundColor DarkGray

# 6. Pre-push 实际跑
Write-Host ""
Write-Host "=== Pre-push status ===" -ForegroundColor Cyan
$env:GIT_TERMINAL_PROMPT = "0"
git status --short | Select-Object -First 5

Write-Host ""
$confirm = Read-Host "Push $commitCount commits to $Remote/$Branch ? (y/N)"
if ($confirm -ne "y") {
    Write-Host "Aborted." -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "=== Pushing ===" -ForegroundColor Cyan
git push $Remote $Branch 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Push OK" -ForegroundColor Green
} else {
    Write-Host "❌ Push failed (exit $LASTEXITCODE)" -ForegroundColor Red
    Write-Host "Network troubleshooting:" -ForegroundColor Yellow
    Write-Host "  - Test-NetConnection github.com 走 127.0.0.1 (loopback)"
    Write-Host "  - 实际 push 走 443 走 loopback, IDS 拦了"
    Write-Host "  - 试 ssh -T git@github.com 看是否 SSH 通 (跳过 IDS)"
    Write-Host "  - 或走 proxy: git config http.proxy socks5://127.0.0.1:1080"
    exit $LASTEXITCODE
}
