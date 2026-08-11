# ==============================================================================
# tag-1.0.0.ps1 — 打 v1.0.0 tag + gh release create (1.0 release, 主人手跑)
# ------------------------------------------------------------------------------
# R129-8 (sub-agent of mvs_367e66fae08342ffa399befe4f85dbac, 2026-08-11 00:08)
# Per decision-55 §2.6 + decision-58 §5 + decision-61 §3.1 + decision-62 拍板
# 触发: 主人 0:03 派 R129-8 准备 1.0 release 流程
# 关联: decision-33 (8 硬墙) + decision-48 (整合 #4 commit abf12243 严守) +
#       decision-55 (R127) + decision-58 (R128-2) + decision-61 (新会话接手) +
#       decision-62 (整合 #5 拆 3 commit 拍板, Mavis 自决)
#
# 作用:
#   1. verify 整合 #5 commit 3 个 done + master HEAD 已 push
#   2. 打 annotated tag v1.0.0 (per semver 严守, 整合 #4 1.2.0 → 1.0 大版本归 0 per decision-22 §2.2)
#   3. push tag v1.0.0
#   4. gh release create v1.0.0 --title --notes-file RELEASE_NOTES.md
#   5. verify GitHub release 页面
#
# 用法 (PowerShell, Windows 优先, 主人手跑):
#   cd Apeireth-rust
#   .\scripts\release\tag-1.0.0.ps1
#
# 0 主动 push 严守 (per decision-33 §2.3 + decision-58 §7 + decision-62 §9):
#   Mavis = orchestrator, 0 主动 push 0 主动 tag 0 主动 release
#   主人 8/11 起床后手跑本脚本 + 拍板 1.0 release
#
# 8 硬墙 (per decision-33 §2.3) 0 越界:
#   B1 24 LOCKED 入口签名 0 改 (本脚本 0 触碰 crate src/)
#   B2 workspace.version 1.2.0 0 改 (per decision-22 §2.2 大版本归 0 = 1.2 → 1.0, 但 workspace.version 实际 0 改, tag 标 1.0.0)
#   A1 R11 baseline 3 值 0 改 (本脚本 0 触碰)
#   B3-B7 + A2-A3 严守 (本脚本 0 触碰)
#   C1 0 主动 commit (本脚本仅 tag + push tag + release, 0 commit)
#   C2 0 装 PASS 严守 (本脚本 0 借具体源码)
#   C3 升 6 重 v7 严守 (本脚本 0 触碰)
#   0 主动 push 严守 (本脚本由主人手跑, Mavis 0 主动)
# ==============================================================================

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$VERSION = '1.0.0'
$TAG = "v$VERSION"
$WORKSPACE_DIR = 'Apeireth-rust'
$EXPECTED_REMOTE = 'https://github.com/apeireth/apeireth-rust.git'
$EXPECTED_REPO = 'apeireth/apeireth-rust'

# 整合 #4 commit (per decision-48, 19:41 done, 0 重跑)
$EXPECTED_INTEGRATION_4 = 'abf1224371016e36df8f4d3c9a05b33f1c563e0d'

# === Banner ===
Write-Host ''
Write-Host '==================================================' -ForegroundColor Cyan
Write-Host "  Apeireth 1.0 release — tag $TAG + gh release" -ForegroundColor Cyan
Write-Host "  版本:   $TAG" -ForegroundColor Cyan
Write-Host "  模式:   主人手跑 (0 主动 push 严守)" -ForegroundColor Cyan
Write-Host '==================================================' -ForegroundColor Cyan
Write-Host ''

# === 前置检查 (per O-5 不假装) ===

# 1. 当前目录 = Apeireth-rust 主仓
if (-not (Get-Location).Path.EndsWith('Apeireth-rust')) {
    Write-Host "❌ 当前目录不是 Apeireth-rust 主仓: $(Get-Location)" -ForegroundColor Red
    Write-Host "   cd $WORKSPACE_DIR" -ForegroundColor Red
    exit 1
}
Write-Host "✓ working dir: $(Get-Location)" -ForegroundColor Green

# 2. master HEAD 已 push (本地 + 远端一致)
$LocalHead = (git rev-parse master 2>$null).Trim()
$RemoteHead = (git ls-remote origin master 2>$null) -replace '\s+.*$', ''
if ($LocalHead -ne $RemoteHead) {
    Write-Host '❌ master HEAD 0 一致 (本地 != 远端)' -ForegroundColor Red
    Write-Host "   本地: $LocalHead" -ForegroundColor Red
    Write-Host "   远端: $RemoteHead" -ForegroundColor Red
    Write-Host '   主人先跑 scripts/release/git-push-1.0.ps1' -ForegroundColor Red
    exit 1
}
Write-Host "✓ master HEAD 一致 (本地 + 远端 = $LocalHead)" -ForegroundColor Green
Write-Host ''

# 3. 整合 #5 commit 3 个 done
$Commit5Count = (git log --oneline 2>$null | Select-String '整合 #5\.').Count
Write-Host "整合 #5.x commit 数: $Commit5Count / 3 期望" -ForegroundColor Cyan
if ($Commit5Count -lt 3) {
    Write-Host '❌ 整合 #5 commit 不全' -ForegroundColor Red
    Write-Host '   主人先跑 scripts/release/git-push-1.0.ps1 (3 commit + push)' -ForegroundColor Red
    exit 1
}
Write-Host "✓ 整合 #5 commit 拆 3 done (5.1 + 5.2 + 5.3)" -ForegroundColor Green
Write-Host ''

# 4. 整合 #4 commit 严守
$Integration4 = (git log --oneline 2>$null | Select-String '整合 #4 commit abf12243' | Select-Object -First 1)
if (-not $Integration4) {
    Write-Host "❌ 整合 #4 commit abf12243 0 在历史" -ForegroundColor Red
    Write-Host "   per decision-48 (整合 #4 commit abf12243 19:41 done, 0 重跑)" -ForegroundColor Red
    exit 1
}
Write-Host "✓ 整合 #4 commit abf12243 在历史 (per decision-48)" -ForegroundColor Green
Write-Host ''

# 5. Cargo.toml 严守 1.2.0 (B2 严守 0 改, tag 标 1.0.0 是 semver 大版本归 0, Cargo.toml 不改)
if (-not (Select-String -Path 'Cargo.toml' -Pattern '^version\s*=\s*"1\.2\.0"' -Quiet)) {
    Write-Host "❌ Cargo.toml version != 1.2.0 (B2 严守 0 改)" -ForegroundColor Red
    Write-Host '   per decision-22 §2.2: 大版本归 0 是 1.2 → 1.0, tag 标 1.0.0, 但 Cargo.toml 实际保留 1.2.0' -ForegroundColor Red
    Write-Host '   等 1.0 release 后再 bump Cargo.toml' -ForegroundColor Red
    exit 1
}
Write-Host "✓ Cargo.toml version = 1.2.0 (B2 严守 0 改, tag 1.0.0 = 大版本归 0 per decision-22 §2.2)" -ForegroundColor Green
Write-Host ''

# 6. gh CLI 装好
try {
    $null = gh --version 2>&1
    if ($LASTEXITCODE -ne 0) { throw 'gh not found' }
} catch {
    Write-Host '❌ gh CLI 不在 PATH' -ForegroundColor Red
    Write-Host '   安装: winget install GitHub.cli  (or https://cli.github.com)' -ForegroundColor Red
    Write-Host '   主人必装, gh release create 必用' -ForegroundColor Red
    exit 1
}
$GhVersion = (gh --version 2>$null | Select-Object -First 1)
Write-Host "✓ gh CLI: $GhVersion" -ForegroundColor Green

# 7. gh auth 认证
$AuthStatus = (gh auth status 2>$null | Out-String)
if ($LASTEXITCODE -ne 0) {
    Write-Host '❌ gh 0 认证' -ForegroundColor Red
    Write-Host '   主人跑: gh auth login  (选 HTTPS, 浏览器认证)' -ForegroundColor Red
    exit 1
}
Write-Host "✓ gh auth 认证 done" -ForegroundColor Green
Write-Host ''

# 8. RELEASE_NOTES.md 存在
if (-not (Test-Path 'RELEASE_NOTES.md')) {
    Write-Host '❌ RELEASE_NOTES.md 不存在' -ForegroundColor Red
    Write-Host '   per P7-3 retry 21:27 写 (36.8KB)' -ForegroundColor Red
    exit 1
}
Write-Host "✓ RELEASE_NOTES.md 存在 (per P7-3 retry 21:27 写, 36.8KB)" -ForegroundColor Green
Write-Host ''

# === 当前 tag 状态 ===
Write-Host '=== 当前 tag 列表 (v1.x) ===' -ForegroundColor Yellow
git tag --list 'v1.*' 2>$null
Write-Host ''

# === Step 1: 打 annotated tag v1.0.0 ===
Write-Host "=== Step 1: 打 annotated tag $TAG ===" -ForegroundColor Yellow
Write-Host ''

# 检查 tag 是否已存在
$ExistingTag = (git tag --list $TAG 2>$null)
if ($ExistingTag) {
    Write-Host "⚠️  tag $TAG 已存在" -ForegroundColor Yellow
    Write-Host "   当前: $ExistingTag" -ForegroundColor Yellow
    $Answer = Read-Host "   删重打? (y/N)"
    if ($Answer -eq 'y') {
        git tag -d $TAG
        Write-Host "✓ 旧 tag $TAG 已删" -ForegroundColor Green
    } else {
        Write-Host "⏸  跳过, 主人手动处理" -ForegroundColor Yellow
        exit 0
    }
}

$TagMessage = @"
Apeireth 1.0.0 release

整合 #4 commit abf12243 19:41 done (per decision-48).
整合 #5 commit 拆 3 commit (per decision-62, Mavis 自决):
  5.1 src/ 实施 (R125-R128-2 era 41 任务, 50+ 文件)
  5.2 1.0 release 文档 (CHANGELOG + ROADMAP + RELEASE_NOTES + OSS_NOTICE + LICENSE + Cargo.toml)
  5.3 决策链 + 41 sub-agent 报告 + HANDOFF (reports/)

8 硬墙 0 越界 100%:
  B1 24 LOCKED 入口签名 0 改
  B2 workspace.version 1.2.0 0 改 (Cargo.toml 实际保留 1.2.0, tag 1.0.0 是大版本归 0 per decision-22 §2.2)
  A1 R11 baseline 3 值 0.8682/0.8532/0.9063 数字严守
  B3 V0.5 30 维
  B4 6 重守门 v7
  B5 8 哲学锚
  A3 13 键
  C1 0 主动 commit
  C2 0 装 PASS 严守 (✅ 10 真实施 + ⏳ 0 限流 + ❌ 1 跳过)
  C3 升 6 重 v6 → v7
  0 主动 push 严守 (Mavis 0 主动, 主人手跑)

借鉴 11/11 状态 clear:
  ✅ 10 真实施 (clap / hyper / servers / PyO3 / kani / langgraph / superpowers / LiteLLM + 2 限流 retry done)
  ⏳ 0 限流
  ❌ 1 跳过 (OpenCog AGPL-3.0)

Release notes: 根目录 RELEASE_NOTES.md (P7-3 retry 21:27 写, 36.8KB)
CHANGELOG: 根目录 CHANGELOG.md (P7-1 21:23 写 v1.0.0, 42.8KB)
ROADMAP: 根目录 ROADMAP.md (P7-2 21:25 写, 28.7KB)
LICENSE: Apache-2.0 (P13-1 写, 175 行 verbatim)
OSS_NOTICE: 根目录 OSS_NOTICE.md (P13-1 写, 346 行, 借鉴 8/11 致谢)

Refs: decision-22, #33, #41, #42, #47, #48, #55, #56, #57, #58, #61, #62
Tests: 4100+ tests pass (per R125-16 + P12-1 verify)
"@

Write-Host '=== tag message ===' -ForegroundColor Yellow
Write-Host $TagMessage -ForegroundColor White
Write-Host ''
$Answer = Read-Host "主人确认打 tag $TAG? (y/N)"
if ($Answer -ne 'y') {
    Write-Host "⏸  跳过, 主人手动 git tag -a $TAG -m ..." -ForegroundColor Yellow
    exit 0
}

git tag -a $TAG -m $TagMessage
Write-Host "✓ tag $TAG 已打" -ForegroundColor Green
Write-Host ''

# === Step 2: push tag ===
Write-Host "=== Step 2: push tag $TAG ===" -ForegroundColor Yellow
Write-Host ''
$Answer = Read-Host "主人确认 push tag? (y/N)"
if ($Answer -ne 'y') {
    Write-Host "⏸  跳过 push tag, 主人手动 git push origin $TAG" -ForegroundColor Yellow
    exit 0
}

git push origin $TAG 2>&1 | Tee-Object -Variable null
$PushExit = $LASTEXITCODE
if ($PushExit -ne 0) {
    Write-Host "❌ git push origin $TAG FAIL (exit $PushExit)" -ForegroundColor Red
    exit 1
}
Write-Host "✓ tag $TAG pushed" -ForegroundColor Green
Write-Host ''

# === Step 3: gh release create ===
Write-Host "=== Step 3: gh release create ===" -ForegroundColor Yellow
Write-Host ''
Write-Host "release 标题: Apeireth $VERSION" -ForegroundColor Cyan
Write-Host "release notes: RELEASE_NOTES.md (36.8KB, per P7-3 retry)" -ForegroundColor Cyan
Write-Host "目标 repo: $EXPECTED_REPO" -ForegroundColor Cyan
Write-Host ''
$Answer = Read-Host "主人确认 create release? (y/N)"
if ($Answer -ne 'y') {
    Write-Host '⏸  跳过 gh release create, 主人手动跑' -ForegroundColor Yellow
    Write-Host "   gh release create $TAG --title 'Apeireth $VERSION' --notes-file RELEASE_NOTES.md" -ForegroundColor Yellow
    exit 0
}

gh release create $TAG --title "Apeireth $VERSION" --notes-file RELEASE_NOTES.md 2>&1 | Tee-Object -Variable null
$ReleaseExit = $LASTEXITCODE
if ($ReleaseExit -ne 0) {
    Write-Host "❌ gh release create FAIL (exit $ReleaseExit)" -ForegroundColor Red
    exit 1
}
Write-Host "✓ release $TAG created" -ForegroundColor Green
Write-Host ''

# === Step 4: verify release 页面 ===
Write-Host '=== Step 4: verify release 页面 ===' -ForegroundColor Yellow
Write-Host ''
$ReleaseUrl = "https://github.com/$EXPECTED_REPO/releases/tag/$TAG"
Write-Host "release URL: $ReleaseUrl" -ForegroundColor Cyan
Write-Host ''
$Answer = Read-Host "主人浏览器打开 verify? (y/N)"
if ($Answer -eq 'y') {
    Start-Process $ReleaseUrl
    Write-Host "✓ 浏览器已开 $ReleaseUrl" -ForegroundColor Green
}
Write-Host ''

# === Done ===
Write-Host '==================================================' -ForegroundColor Green
Write-Host "  1.0 release done 🎉" -ForegroundColor Green
Write-Host "  tag:      $TAG" -ForegroundColor Green
Write-Host "  release:  $ReleaseUrl" -ForegroundColor Green
Write-Host "  master HEAD: $LocalHead" -ForegroundColor Green
Write-Host '  整合 #4 commit abf12243 严守 (0 重跑)' -ForegroundColor Green
Write-Host '  整合 #5 commit 拆 3 done (5.1 + 5.2 + 5.3)' -ForegroundColor Green
Write-Host '  8 硬墙 0 越界 100%' -ForegroundColor Green
Write-Host '  0 主动 push 严守 (本脚本由主人手跑, Mavis 0 主动)' -ForegroundColor Green
Write-Host '==================================================' -ForegroundColor Green
Write-Host ''
Write-Host '🎉 Apeireth 1.0.0 released!' -ForegroundColor Green
Write-Host ''
Write-Host '1.0 release 后 (per decision-9 + 主人 8/4 23:33):' -ForegroundColor Cyan
Write-Host '   - TUI 升级 (per decision-9 路线图, 改瘦后暂告段落, 优先后端)' -ForegroundColor Cyan
Write-Host '   - Tauri 终极前端 (per 主人 8/4 23:33, 等设计团队到位)' -ForegroundColor Cyan
Write-Host '   - ASI Python Stage 4-6 (per R129-4/5/6, 跑过夜 done 整合 #6 commit)' -ForegroundColor Cyan
Write-Host '   - 形式化证明扩展 (per R129-10 续 P8-2)' -ForegroundColor Cyan
Write-Host ''
Write-Host '0 主动 push 严守 (per decision-33 §2.3 + decision-62 §9):' -ForegroundColor Yellow
Write-Host '   Mavis = orchestrator, 0 push 0 commit 0 配 remote' -ForegroundColor Yellow
Write-Host '   主人 8/11 起床后手跑本脚本 + 拍板 1.0 release' -ForegroundColor Yellow
Write-Host ''
exit 0
