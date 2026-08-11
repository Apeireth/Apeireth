# ==============================================================================
# setup-github-remote.ps1 — 配置 GitHub remote (1.0 release 准备, 主人手跑)
# ------------------------------------------------------------------------------
# R129-8 (sub-agent of mvs_367e66fae08342ffa399befe4f85dbac, 2026-08-11 00:08)
# Per decision-55 §2.6 + decision-58 §5 + decision-61 §3.1 + 主人 8/4 23:33
# 触发: 主人 0:03 派 R129-8 准备 1.0 release 流程
# 关联: decision-33 (8 硬墙) + decision-48 (整合 #4 commit abf12243 严守) +
#       decision-55 (R127) + decision-58 (R128-2) + decision-61 (新会话接手) +
#       decision-62 (整合 #5 拆 3 commit 拍板)
#
# 作用:
#   1. 创建 GitHub repo (主人浏览器 https://github.com/new)
#   2. 加 remote origin = https://github.com/apeireth/apeireth-rust.git
#   3. verify: git remote -v 显示 origin
#
# 用法 (PowerShell, Windows 优先, 主人手跑):
#   cd Apeireth-rust
#   .\scripts\release\setup-github-remote.ps1
#
# 0 主动 push 严守 (per decision-33 §2.3 + decision-58 §7 + decision-62 §9):
#   Mavis = orchestrator, 0 主动 push 0 主动 commit 0 主动配 remote
#   主人 8/11 起床后手跑本脚本 + 拍板 1.0 release
#
# 8 硬墙 (per decision-33 §2.3) 0 越界:
#   B1 24 LOCKED 入口签名 0 改 (本脚本 0 触碰 crate src/)
#   B2 workspace.version 1.2.0 0 改 (本脚本 0 改 Cargo.toml)
#   A1 R11 baseline 3 值 0 改 (本脚本 0 触碰 17 baseline 文件)
#   B3-B7 + A2-A3 严守 (本脚本 0 触碰)
#   C1 0 主动 commit (本脚本 0 git commit, 仅配 remote)
#   C2 0 装 PASS 严守 (本脚本 0 借具体源码)
#   C3 升 6 重 v7 严守 (本脚本 0 触碰)
#   0 主动 push 严守 (本脚本仅配 remote, 0 push, push 见 git-push-1.0.ps1)
# ==============================================================================

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

# === 0 主动 push 严守: 本脚本仅配 remote, 0 push ===
$REPO_URL = 'https://github.com/apeireth/apeireth-rust.git'
$EXPECTED_USER = 'apeireth'
$EXPECTED_REPO = 'apeireth-rust'
$VERSION = '1.0.0'

# === Banner ===
Write-Host ''
Write-Host '==================================================' -ForegroundColor Cyan
Write-Host "  Apeireth 1.0 release — GitHub remote 配置" -ForegroundColor Cyan
Write-Host "  仓库:   $REPO_URL" -ForegroundColor Cyan
Write-Host "  版本:   v$VERSION" -ForegroundColor Cyan
Write-Host "  模式:   主人手跑 (0 主动 push 严守)" -ForegroundColor Cyan
Write-Host '==================================================' -ForegroundColor Cyan
Write-Host ''

# === 前置检查 (per O-5 不假装) ===

# 1. 当前目录 = Apeireth-rust 主仓
$ExpectedPath = 'Apeireth-rust'
if (-not (Get-Location).Path.EndsWith($ExpectedPath)) {
    Write-Host "❌ 当前目录不是 Apeireth-rust 主仓: $(Get-Location)" -ForegroundColor Red
    Write-Host "   期望: Apeireth-rust\" -ForegroundColor Red
    Write-Host "   cd Apeireth-rust\" -ForegroundColor Red
    exit 1
}
Write-Host "✓ 当前目录: $(Get-Location)" -ForegroundColor Green

# 2. .git 存在 (整合 #4 commit 19:41 后 .git 在主仓根)
if (-not (Test-Path '.git')) {
    Write-Host "❌ .git 不在主仓根" -ForegroundColor Red
    Write-Host "   整合 #4 commit 19:41 后 .git 应该在 Apeireth-rust\.git\" -ForegroundColor Red
    Write-Host "   per decision-46 (git mv done) + decision-48 (整合 #4 commit abf12243)" -ForegroundColor Red
    exit 1
}
Write-Host "✓ .git 在主仓根" -ForegroundColor Green

# 3. git 可执行
try {
    $null = git --version 2>&1
    if ($LASTEXITCODE -ne 0) { throw 'git not found' }
} catch {
    Write-Host "❌ git 不在 PATH" -ForegroundColor Red
    Write-Host "   安装: https://git-scm.com/download/win" -ForegroundColor Red
    exit 1
}
$GitVersion = (git --version 2>$null | Select-Object -First 1)
Write-Host "✓ git: $GitVersion" -ForegroundColor Green

# 4. master HEAD = abf12243 (整合 #4 commit 严守, per decision-48)
$MasterHead = (Get-Content '.git\refs\heads\master' -Raw).Trim()
if ($MasterHead -ne 'abf1224371016e36df8f4d3c9a05b33f1c563e0d') {
    Write-Host "❌ master HEAD != abf12243 (整合 #4 commit 严守)" -ForegroundColor Red
    Write-Host "   当前: $MasterHead" -ForegroundColor Red
    Write-Host "   期望: abf1224371016e36df8f4d3c9a05b33f1c563e0d" -ForegroundColor Red
    Write-Host "   per decision-48 (整合 #4 commit abf12243 19:41 done, 0 重跑)" -ForegroundColor Red
    exit 1
}
Write-Host "✓ master HEAD = abf12243 (整合 #4 commit 严守)" -ForegroundColor Green

# 5. Cargo.toml 严守 1.2.0 (B2 严守, per decision-33 §2.3)
if (-not (Select-String -Path 'Cargo.toml' -Pattern '^version\s*=\s*"1\.2\.0"' -Quiet)) {
    Write-Host "❌ Cargo.toml version != 1.2.0 (B2 严守 0 改)" -ForegroundColor Red
    Write-Host "   per decision-33 §2.3 B2 + decision-48 (整合 #4 commit abf12243 严守)" -ForegroundColor Red
    exit 1
}
Write-Host "✓ Cargo.toml version = 1.2.0 (B2 严守 0 改)" -ForegroundColor Green

# === 当前 remote 状态 ===
Write-Host ''
Write-Host '=== 当前 git remote 状态 ===' -ForegroundColor Yellow
git remote -v
$OriginExists = (git remote 2>$null) -contains 'origin'
Write-Host ''

# === Step 1: 创建 GitHub repo (主人浏览器) ===
if ($OriginExists) {
    Write-Host "ℹ️  origin 已存在, 跳过 GitHub repo 创建步骤" -ForegroundColor Yellow
    $ExistingUrl = (git remote get-url origin 2>$null)
    Write-Host "   当前 origin: $ExistingUrl" -ForegroundColor Yellow
    if ($ExistingUrl -ne $REPO_URL) {
        Write-Host "⚠️  origin URL 跟期望不同: $REPO_URL" -ForegroundColor Yellow
        $Answer = Read-Host "   是否更新? (y/N)"
        if ($Answer -eq 'y') {
            git remote set-url origin $REPO_URL
            Write-Host "✓ origin URL 已更新: $REPO_URL" -ForegroundColor Green
        } else {
            Write-Host "ℹ️  保留当前 origin: $ExistingUrl" -ForegroundColor Yellow
        }
    }
} else {
    Write-Host '=== Step 1: 主人创建 GitHub repo ===' -ForegroundColor Yellow
    Write-Host ''
    Write-Host '主人需要浏览器手动创建 GitHub repo:' -ForegroundColor White
    Write-Host ''
    Write-Host '   1. 打开 https://github.com/new' -ForegroundColor White
    Write-Host "   2. Owner: $EXPECTED_USER" -ForegroundColor White
    Write-Host "   3. Repository name: $EXPECTED_REPO" -ForegroundColor White
    Write-Host '   4. Visibility: Public (推荐, 跟 Apache-2.0 开源一致)' -ForegroundColor White
    Write-Host '   5. ⚠️  0 勾选任何初始化 (README / .gitignore / license)' -ForegroundColor Red
    Write-Host '      主仓已有 README.md / .gitignore / LICENSE, 0 让 GitHub 覆盖' -ForegroundColor Red
    Write-Host '   6. 点 Create repository' -ForegroundColor White
    Write-Host ''
    $Ready = Read-Host "主人创建完 GitHub repo 后按 Enter 继续 (q = 退出)"
    if ($Ready -eq 'q') {
        Write-Host "⏸  退出, 主人创建后重跑本脚本" -ForegroundColor Yellow
        exit 0
    }
}

# === Step 2: 加 origin remote (0 push) ===
Write-Host ''
Write-Host '=== Step 2: 加 origin remote (0 push) ===' -ForegroundColor Yellow
if (-not $OriginExists) {
    git remote add origin $REPO_URL
    Write-Host "✓ origin 已加: $REPO_URL" -ForegroundColor Green
} else {
    Write-Host "ℹ️  origin 已存在, 跳过 add" -ForegroundColor Yellow
}

# === Step 3: verify remote ===
Write-Host ''
Write-Host '=== Step 3: verify remote ===' -ForegroundColor Yellow
Write-Host ''
git remote -v
Write-Host ''

$VerifyUrl = (git remote get-url origin 2>$null)
if ($VerifyUrl -ne $REPO_URL) {
    Write-Host "❌ origin URL verify 失败" -ForegroundColor Red
    Write-Host "   当前: $VerifyUrl" -ForegroundColor Red
    Write-Host "   期望: $REPO_URL" -ForegroundColor Red
    exit 1
}
Write-Host "✓ origin URL verify PASS: $VerifyUrl" -ForegroundColor Green

# === Step 4: 准备 git push 认证 ===
Write-Host ''
Write-Host '=== Step 4: git push 认证 (主人手配) ===' -ForegroundColor Yellow
Write-Host ''
Write-Host '主人需要 git push 认证, 推荐 2 选 1:' -ForegroundColor White
Write-Host ''
Write-Host '   方式 A: GitHub CLI (推荐, 主人 8/10 跑过 Windows PowerShell 熟悉)' -ForegroundColor White
Write-Host '     1. winget install GitHub.cli  (or https://cli.github.com)' -ForegroundColor White
Write-Host "     2. gh auth login  (选 HTTPS, 浏览器认证, 0 配 SSH key)" -ForegroundColor White
Write-Host '     3. gh auth status  (verify 已认证)' -ForegroundColor White
Write-Host ''
Write-Host '   方式 B: GitHub Personal Access Token (PAT)' -ForegroundColor White
Write-Host '     1. https://github.com/settings/tokens/new 生成 PAT (scopes: repo + workflow)' -ForegroundColor White
Write-Host '     2. git credential-manager store (Windows 凭据管理器存 PAT)' -ForegroundColor White
Write-Host '     3. 下次 push 自动用 PAT' -ForegroundColor White
Write-Host ''
Write-Host '   方式 C: SSH key (0 推荐, 主人 0 必配)' -ForegroundColor White
Write-Host '     ssh-keygen + 贴公钥到 GitHub' -ForegroundColor White
Write-Host ''
$AuthReady = Read-Host "主人认证配好按 Enter 继续 (q = 退出)"
if ($AuthReady -eq 'q') {
    Write-Host "⏸  退出, 主人认证后重跑本脚本" -ForegroundColor Yellow
    exit 0
}

# === Done ===
Write-Host ''
Write-Host '==================================================' -ForegroundColor Green
Write-Host '  GitHub remote 配置 done' -ForegroundColor Green
Write-Host "  origin: $REPO_URL" -ForegroundColor Green
Write-Host "  master HEAD: abf12243" -ForegroundColor Green
Write-Host '  0 push 严守 (push 见 git-push-1.0.ps1)' -ForegroundColor Green
Write-Host '==================================================' -ForegroundColor Green
Write-Host ''
Write-Host '下一步: 跑 scripts/release/verify-1.0-pre-tag.ps1 (8 步 verify)' -ForegroundColor Cyan
Write-Host '   8 步全 PASS → 拍板整合 #5 commit → 跑 scripts/release/git-push-1.0.ps1' -ForegroundColor Cyan
Write-Host '   整合 #5 commit done → 跑 scripts/release/tag-1.0.0.ps1' -ForegroundColor Cyan
Write-Host ''
Write-Host '0 主动 push 严守 (per decision-33 §2.3 + decision-62 §9):' -ForegroundColor Yellow
Write-Host '   Mavis = orchestrator, 0 push 0 commit 0 配 remote' -ForegroundColor Yellow
Write-Host '   主人 8/11 起床后手跑本脚本 + 拍板 1.0 release' -ForegroundColor Yellow
Write-Host ''
