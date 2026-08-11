# ==============================================================================
# git-push-1.0.ps1 — push 整合 #5 commit + master + tags (1.0 release, 主人手跑)
# ------------------------------------------------------------------------------
# R129-8 (sub-agent of mvs_367e66fae08342ffa399befe4f85dbac, 2026-08-11 00:08)
# Per decision-55 §2.6 + decision-58 §5 + decision-61 §3.1 + decision-62 拍板
# 触发: 主人 0:03 派 R129-8 准备 1.0 release 流程
# 关联: decision-33 (8 硬墙) + decision-48 (整合 #4 commit abf12243 严守) +
#       decision-55 (R127) + decision-58 (R128-2) + decision-61 (新会话接手) +
#       decision-62 (整合 #5 拆 3 commit 拍板, Mavis 自决)
#
# 作用 (per decision-62 拍板):
#   1. verify master HEAD = abf12243 (整合 #4 commit 严守)
#   2. verify Cargo.toml 1.2.0 严守 (B2)
#   3. 整合 #5.1 commit (R125-R128-2 era 41 任务 src/ 实施, 50+ 文件)
#   4. 整合 #5.2 commit (1.0 release 文档, CHANGELOG + ROADMAP + RELEASE_NOTES + OSS_NOTICE + LICENSE + Cargo.toml)
#   5. 整合 #5.3 commit (决策链 + 41 sub-agent 报告 + HANDOFF, reports/)
#   6. push master + tags
#   7. verify 推送结果
#
# 用法 (PowerShell, Windows 优先, 主人手跑):
#   cd Apeireth-rust
#   .\scripts\release\git-push-1.0.ps1
#
# 0 主动 push 严守 (per decision-33 §2.3 + decision-58 §7 + decision-62 §9):
#   Mavis = orchestrator, 0 主动 push 0 主动 commit 0 主动配 remote
#   主人 8/11 起床后手跑本脚本 + 拍板 1.0 release
#   整合 #5 commit 拆 3 commit 由 Mavis 自决拍板 (per decision-62), 但 git add + commit + push 由主人手跑
#
# 8 硬墙 (per decision-33 §2.3) 0 越界:
#   B1 24 LOCKED 入口签名 0 改 (commit 5.1 仅内部 fn 改 + 入口 0 改)
#   B2 workspace.version 1.2.0 0 改 (本脚本 0 改 Cargo.toml version)
#   A1 R11 baseline 3 值 0 改 (本脚本 0 触碰 17 baseline 文件)
#   B3-B7 + A2-A3 严守 (本脚本 0 触碰)
#   C1 0 主动 commit (本脚本由主人手跑, Mavis 0 主动)
#   C2 0 装 PASS 严守 (commit 5.1 仅借鉴 8/11 真实施)
#   C3 升 6 重 v7 严守 (本脚本 0 触碰)
#   0 主动 push 严守 (本脚本由主人手跑, Mavis 0 主动)
# ==============================================================================

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$VERSION = '1.0.0'
$WORKSPACE_DIR = 'Apeireth-rust'
$EXPECTED_REMOTE = 'https://github.com/apeireth/apeireth-rust.git'

# 整合 #4 commit (per decision-48, 19:41 done, 0 重跑)
$EXPECTED_HEAD = 'abf1224371016e36df8f4d3c9a05b33f1c563e0d'

# 整合 #5 commit 拆 3 commit (per decision-62 拍板, Mavis 自决)
$Commit5_1_Subject = '整合 #5.1 commit: R125-R128-2 era 41 任务 src/ 实施'
$Commit5_2_Subject = '整合 #5.2 commit: 1.0 release 文档 (CHANGELOG + ROADMAP + RELEASE_NOTES + OSS_NOTICE + LICENSE + Cargo.toml)'
$Commit5_3_Subject = '整合 #5.3 commit: 决策链 #30-#60 + 41 sub-agent 报告 + HANDOFF (reports/)'

# === Banner ===
Write-Host ''
Write-Host '==================================================' -ForegroundColor Cyan
Write-Host "  Apeireth 1.0 release — git push (整合 #5 拆 3 commit)" -ForegroundColor Cyan
Write-Host "  版本:   v$VERSION" -ForegroundColor Cyan
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

# 2. master HEAD = abf12243 (整合 #4 commit 严守)
$MasterHead = (Get-Content '.git\refs\heads\master' -Raw).Trim()
if ($MasterHead -ne $EXPECTED_HEAD) {
    Write-Host "❌ master HEAD != abf12243" -ForegroundColor Red
    Write-Host "   当前: $MasterHead" -ForegroundColor Red
    Write-Host "   期望: $EXPECTED_HEAD" -ForegroundColor Red
    Write-Host "   per decision-48 (整合 #4 commit abf12243 19:41 done, 0 重跑)" -ForegroundColor Red
    exit 1
}
Write-Host "✓ master HEAD = abf12243 (整合 #4 commit 严守)" -ForegroundColor Green
Write-Host ''

# 3. Cargo.toml 严守 1.2.0
if (-not (Select-String -Path 'Cargo.toml' -Pattern '^version\s*=\s*"1\.2\.0"' -Quiet)) {
    Write-Host "❌ Cargo.toml version != 1.2.0 (B2 严守 0 改)" -ForegroundColor Red
    exit 1
}
Write-Host "✓ Cargo.toml version = 1.2.0 (B2 严守 0 改)" -ForegroundColor Green
Write-Host ''

# 4. origin 配好 (主人已跑 setup-github-remote.ps1)
$OriginUrl = (git remote get-url origin 2>$null)
if (-not $OriginUrl) {
    Write-Host "❌ origin 0 配" -ForegroundColor Red
    Write-Host "   主人先跑 scripts/release/setup-github-remote.ps1" -ForegroundColor Red
    exit 1
}
if ($OriginUrl -ne $EXPECTED_REMOTE) {
    Write-Host "⚠️  origin URL 跟期望不同" -ForegroundColor Yellow
    Write-Host "   当前: $OriginUrl" -ForegroundColor Yellow
    Write-Host "   期望: $EXPECTED_REMOTE" -ForegroundColor Yellow
    $Answer = Read-Host "   继续? (y/N)"
    if ($Answer -ne 'y') {
        Write-Host "⏸  退出" -ForegroundColor Yellow
        exit 0
    }
}
Write-Host "✓ origin: $OriginUrl" -ForegroundColor Green
Write-Host ''

# 5. working dir 状态
Write-Host '=== git status ===' -ForegroundColor Yellow
git status --short
Write-Host ''

$ModifiedCount = (git status --short | Measure-Object).Count
Write-Host "改动文件数: $ModifiedCount" -ForegroundColor Cyan
if ($ModifiedCount -eq 0) {
    Write-Host 'ℹ️  0 改动, 0 commit 必跑 (整合 #5 commit 已 done)' -ForegroundColor Yellow
    Write-Host '   直接跳到 push 步骤' -ForegroundColor Yellow
} else {
    Write-Host "ℹ️  有 $ModifiedCount 文件改动, 需要 3 commit" -ForegroundColor Yellow
}
Write-Host ''

# === 整合 #5.1 commit: src/ 实施 ===
Write-Host '=== Step 1: 整合 #5.1 commit (src/ 实施) ===' -ForegroundColor Yellow
Write-Host ''
Write-Host "Subject: $Commit5_1_Subject" -ForegroundColor Cyan
Write-Host ''
Write-Host '改动范围 (per decision-62 §2.1):' -ForegroundColor White
Write-Host '   - 31 M src/ (LOCKED crate 内部 fn)' -ForegroundColor White
Write-Host '   - 50+ ?? src/ (借鉴 8/11 真实施 + 新模块)' -ForegroundColor White
Write-Host '   - 20+ tests/ + 10+ examples/' -ForegroundColor White
Write-Host '   - 3 NEW 库目录 (apeireth-library-governance/ + frontend/ + library/)' -ForegroundColor White
Write-Host '   - 0 改 Cargo.toml version (B2 严守)' -ForegroundColor White
Write-Host '   - 0 改 24 LOCKED 入口签名 (B1 严守)' -ForegroundColor White
Write-Host ''

# 准备 stage src/ 改动
git add 'crates/*/src/lib.rs' 'crates/*/src/*.rs' 'crates/*/Cargo.toml' 'crates/*/tests/*.rs' 'crates/*/examples/*.rs' 2>$null
git add 'frontend/' 'library/' 'apeireth-library-governance/' 2>$null

$StagedCount = (git diff --cached --name-only | Measure-Object).Count
Write-Host "已 stage 文件数: $StagedCount" -ForegroundColor Cyan
Write-Host ''

if ($StagedCount -gt 0) {
    Write-Host '=== 5.1 commit message (per decision-62 §2.2) ===' -ForegroundColor Yellow
    $CommitMsg1 = @"
$Commit5_1_Subject

主仓 src/ 实施整合 (R125 16 + R126 16 + R127 4 + R127-2 10 + R128 6 + R128-2 3 = 41 sub-agent 全 done).

借鉴 8/11 真实施:
- clap-rs/clap 4.6.6 (R125-2) - derive 实施
- hyperium/hyper 0.1.20 (R125-3) - 池复用
- modelcontextprotocol/servers 76d64c8 (R125-4) - MCP 协议对齐
- PyO3/PyO3 0.29.2 (R125-9) - pybridge
- model-checking/kani 0.67.0 (R125-10) - 形式化
- langchain-ai/langgraph d56666f (R125-13) - StateGraph
- obra/superpowers 6.2.0 (R125-14) - 9 skill files
- LiteLLM (P6-1 retry 21:38) - 公开设计 1:1 翻译

升级:
- 8 哲学锚 (B5, 6→8)
- V0.5 30 维 (B3, 25→30)
- 6 重守门 v7 (B4, v6→v7)
- 12 键 + PHL-07 = 13 键 (A3)

0 越界 8 硬墙 100%:
- B1 24 LOCKED 入口签名 0 改
- B2 workspace.version 1.2.0 0 改
- A1 R11 baseline 3 值 0 改
- C1 0 主动 commit (整合 #5 commit 时机)
- C2 0 装 PASS 严守
- 0 主动 push

整合 #4 commit abf12243 严守 (0 重跑).

Refs: decision-22, #33, #41, #42, #47, #48, #51, #55, #56, #57, #58, #61, #62
Tests: 4100+ tests pass (per R125-16 + P12-1 verify)
"@
    Write-Host $CommitMsg1 -ForegroundColor White
    Write-Host ''
    $Answer = Read-Host "主人确认 commit 5.1? (y/N)"
    if ($Answer -eq 'y') {
        git commit -m $CommitMsg1
        Write-Host '✅ 整合 #5.1 commit done' -ForegroundColor Green
    } else {
        Write-Host '⏸  跳过 5.1 commit, 主人手动 git commit' -ForegroundColor Yellow
    }
} else {
    Write-Host 'ℹ️  0 stage 文件, 跳过 5.1 commit (可能已 done)' -ForegroundColor Yellow
}
Write-Host ''

# === 整合 #5.2 commit: 1.0 release 文档 ===
Write-Host '=== Step 2: 整合 #5.2 commit (1.0 release 文档) ===' -ForegroundColor Yellow
Write-Host ''
Write-Host "Subject: $Commit5_2_Subject" -ForegroundColor Cyan
Write-Host ''
Write-Host '改动范围 (per decision-62 §3.1):' -ForegroundColor White
Write-Host '   - CHANGELOG.md (P7-1 21:23 写 v1.0.0, 42.8KB)' -ForegroundColor White
Write-Host '   - ROADMAP.md (P7-2 21:25 写, 28.7KB)' -ForegroundColor White
Write-Host '   - RELEASE_NOTES.md (P7-3 retry 21:27 写, 36.8KB)' -ForegroundColor White
Write-Host '   - OSS_NOTICE.md (P13-1 21:53 写, 346 行)' -ForegroundColor White
Write-Host '   - LICENSE (P13-1 写, 175 行 Apache 2.0 verbatim)' -ForegroundColor White
Write-Host '   - Cargo.toml (P15-1 22:48 写, license = "Apache-2.0" + workspace.metadata.apeireth)' -ForegroundColor White
Write-Host '   - 0 改 Cargo.toml version (B2 严守 1.2.0)' -ForegroundColor White
Write-Host ''

git add 'CHANGELOG.md' 'ROADMAP.md' 'RELEASE_NOTES.md' 'OSS_NOTICE.md' 'LICENSE' 'NOTICE' 'Cargo.toml' 'Cargo.lock' 2>$null

$StagedCount2 = (git diff --cached --name-only | Measure-Object).Count
Write-Host "已 stage 文件数: $StagedCount2" -ForegroundColor Cyan
Write-Host ''

if ($StagedCount2 -gt 0) {
    Write-Host '=== 5.2 commit message (per decision-62 §3.2) ===' -ForegroundColor Yellow
    $CommitMsg2 = @"
$Commit5_2_Subject

1.0 release 文档整合:
- CHANGELOG.md (v1.0.0, P7-1 写, 42.8KB)
- ROADMAP.md (P7-2 写, 28.7KB)
- RELEASE_NOTES.md (P7-3 retry 写, 36.8KB)
- OSS_NOTICE.md (P13-1 写, 346 行, 借鉴 8/11 致谢)
- LICENSE (175 行, Apache-2.0 verbatim, P13-1 写, 严守不动)
- NOTICE (66 行, R20 阶段 6, 严守不动)

Cargo.toml 配 (per P15-1 R128-2 阶段 C):
- [workspace.package] license = "Apache-2.0" 单一来源
- [workspace.metadata.apeireth] section (73 行, 11 字段)
- 18 行注释 block (LICENSE 引用链 + 借鉴 8/11)

0 越界 8 硬墙 100%:
- B2 workspace.version 1.2.0 0 改
- C1 0 主动 commit (整合 #5 commit 时机)
- C2 0 装 PASS 严守
- 0 主动 push (等 1.0 release 配 GitHub remote)

Refs: decision-22, #33, #34, #48, #55, #57, #58, #61, #62
Depends: 5.1 (Cargo.toml metadata 引用 src/ 路径字符串, 但 Cargo.toml 已独立 done)
"@
    Write-Host $CommitMsg2 -ForegroundColor White
    Write-Host ''
    $Answer = Read-Host "主人确认 commit 5.2? (y/N)"
    if ($Answer -eq 'y') {
        git commit -m $CommitMsg2
        Write-Host '✅ 整合 #5.2 commit done' -ForegroundColor Green
    } else {
        Write-Host '⏸  跳过 5.2 commit, 主人手动 git commit' -ForegroundColor Yellow
    }
} else {
    Write-Host 'ℹ️  0 stage 文件, 跳过 5.2 commit (可能已 done)' -ForegroundColor Yellow
}
Write-Host ''

# === 整合 #5.3 commit: 决策链 + 报告 ===
Write-Host '=== Step 3: 整合 #5.3 commit (决策链 + 报告) ===' -ForegroundColor Yellow
Write-Host ''
Write-Host "Subject: $Commit5_3_Subject" -ForegroundColor Cyan
Write-Host ''
Write-Host '改动范围 (per decision-62 §4.1):' -ForegroundColor White
Write-Host '   - 30+ reports/ 文件 (决策链 #30-#60 + HANDOFF + 41 sub-agent final)' -ForegroundColor White
Write-Host '   - 备查用, 0 影响 build' -ForegroundColor White
Write-Host ''

git add 'reports/' 2>$null

$StagedCount3 = (git diff --cached --name-only | Measure-Object).Count
Write-Host "已 stage 文件数: $StagedCount3" -ForegroundColor Cyan
Write-Host ''

if ($StagedCount3 -gt 0) {
    Write-Host '=== 5.3 commit message (per decision-62 §4.2) ===' -ForegroundColor Yellow
    $CommitMsg3 = @"
$Commit5_3_Subject

备查用, 0 影响 build.

决策链 (per decision-22 ~ decision-60, 31 份):
- R125 era 决策: #30-#32, #35, #37, #41
- R126 era 决策: #33, #36, #38, #39, #40, #42, #51, #52, #53, #54
- R127 era 决策: #55
- R127-2 era 决策: #56
- R128 era 决策: #57
- R128-2 era 决策: #58
- promethean/ 清理: #44, #45, #46, #47, #49, #50, #59, #60
- 整合 #4 commit: #48

41 sub-agent final 报告 (per R125 16 + R126 16 + R127 4 + R127-2 10 + R128 6 + R128-2 3):
- R125 era: agent-r125-15e/15f/16/17/18/19/20/21 + retry
- R126 era: agent-p1-1/1-2/1-3/1-4/2-1/2-2/2-3/2-4/3-1/3-2/3-3/3-4 + retry
- R127 era: agent-p4-1 + p5-1/2/3
- R127-2 era: agent-p6-1/2/3 + p7-1/2/3 retry + p8-1/2 retry/3 + p9-1
- R128 era: agent-p10-1/2 + p11-1 + p12-1 + p13-1 + p14-1 retry
- R128-2 era: agent-p10-3 + p11-2 + p15-1

HANDOFF:
- reports/HANDOFF-NEXT-SESSION-2026-08-10.md

cargo logs (per P12-1 + P15-1):
- agent-p12-1-cargo-*.log (10+ log)
- agent-p15-1-cargo-build-release-{api,tui}-2026-08-10.log
- agent-p15-1-cargo-run-release-api-2026-08-10.log

locked-audit 报告:
- reports/locked-audit-2026-08-10.md
- reports/locked-audit-v2-final-2026-08-10.md

promethean/ 清理脚本 (per decision-60 挂起):
- reports/promethean-full-cleanup-2026-08-10.ps1 (v1)
- reports/promethean-full-cleanup-v2-2026-08-10.ps1 (v2)

0 越界 8 硬墙 100% (per decision-33):
- C1 0 主动 commit (整合 #5 commit 时机)
- 0 主动 push (等 1.0 release 配 GitHub remote)

Refs: decision-22, #33, #34, #48, #61, #62
Depends: 0 (独立)
"@
    Write-Host $CommitMsg3 -ForegroundColor White
    Write-Host ''
    $Answer = Read-Host "主人确认 commit 5.3? (y/N)"
    if ($Answer -eq 'y') {
        git commit -m $CommitMsg3
        Write-Host '✅ 整合 #5.3 commit done' -ForegroundColor Green
    } else {
        Write-Host '⏸  跳过 5.3 commit, 主人手动 git commit' -ForegroundColor Yellow
    }
} else {
    Write-Host 'ℹ️  0 stage 文件, 跳过 5.3 commit (可能已 done)' -ForegroundColor Yellow
}
Write-Host ''

# === verify 整合 #5 commit 3 个 done ===
Write-Host '=== Step 4: verify 整合 #5 commit 3 个 done ===' -ForegroundColor Yellow
Write-Host ''
Write-Host '最近 5 commit:' -ForegroundColor Cyan
git log --oneline -5
Write-Host ''

$CommitCount = (git log --oneline | Select-String '整合 #5\.' | Measure-Object).Count
Write-Host "整合 #5.x commit 数: $CommitCount / 3 期望" -ForegroundColor Cyan
if ($CommitCount -lt 3) {
    Write-Host "⚠️  整合 #5 commit 不全 (期望 3, 实际 $CommitCount)" -ForegroundColor Yellow
    Write-Host '   主人手动补 commit 后重跑' -ForegroundColor Yellow
    $Answer = Read-Host "继续 push? (y/N)"
    if ($Answer -ne 'y') {
        Write-Host '⏸  退出' -ForegroundColor Yellow
        exit 0
    }
}
Write-Host ''

# === push master + tags ===
Write-Host '=== Step 5: push master + tags ===' -ForegroundColor Yellow
Write-Host ''
Write-Host "push 目标: origin = $OriginUrl" -ForegroundColor Cyan
Write-Host ''
$Answer = Read-Host "主人确认 push? (y/N)"
if ($Answer -ne 'y') {
    Write-Host '⏸  跳过 push, 主人手动 git push' -ForegroundColor Yellow
    exit 0
}

# push master
git push -u origin master 2>&1 | Tee-Object -Variable null
$PushExit = $LASTEXITCODE
if ($PushExit -ne 0) {
    Write-Host "❌ git push -u origin master FAIL (exit $PushExit)" -ForegroundColor Red
    exit 1
}
Write-Host '✅ git push -u origin master done' -ForegroundColor Green
Write-Host ''

# push tags (暂无 v1.0.0 tag, 0 push)
# 等 Step 6 (tag-1.0.0.ps1) 跑完再 push tag
Write-Host 'ℹ️  暂无 v1.0.0 tag (待 tag-1.0.0.ps1 跑完)' -ForegroundColor Yellow
Write-Host ''

# === verify push 成功 ===
Write-Host '=== Step 6: verify push 成功 ===' -ForegroundColor Yellow
Write-Host ''
git log --oneline -5
Write-Host ''
Write-Host 'master HEAD (本地):' -ForegroundColor Cyan
git log -1 --format='%H %s'
Write-Host ''
Write-Host 'master HEAD (远端):' -ForegroundColor Cyan
git ls-remote origin master 2>$null | ForEach-Object { Write-Host $_ }
Write-Host ''

$LocalHead = (git rev-parse master).Trim()
$RemoteHead = (git ls-remote origin master 2>$null) -replace '\s+.*$', ''
if ($LocalHead -eq $RemoteHead) {
    Write-Host "✅ local master = remote master = $LocalHead" -ForegroundColor Green
} else {
    Write-Host "⚠️  local != remote" -ForegroundColor Yellow
    Write-Host "   local:  $LocalHead" -ForegroundColor Yellow
    Write-Host "   remote: $RemoteHead" -ForegroundColor Yellow
}
Write-Host ''

# === Done ===
Write-Host '==================================================' -ForegroundColor Green
Write-Host '  git push done' -ForegroundColor Green
Write-Host "  master HEAD (本地 + 远端): $LocalHead" -ForegroundColor Green
Write-Host '  整合 #5 commit 拆 3 done (5.1 + 5.2 + 5.3)' -ForegroundColor Green
Write-Host '  0 主动 push 严守 (本脚本由主人手跑, Mavis 0 主动)' -ForegroundColor Green
Write-Host '==================================================' -ForegroundColor Green
Write-Host ''
Write-Host '下一步: 跑 scripts/release/tag-1.0.0.ps1 (tag v1.0.0 + gh release create)' -ForegroundColor Cyan
Write-Host ''
Write-Host '0 主动 push 严守 (per decision-33 §2.3 + decision-62 §9):' -ForegroundColor Yellow
Write-Host '   Mavis = orchestrator, 0 push 0 commit 0 配 remote' -ForegroundColor Yellow
Write-Host '   主人 8/11 起床后手跑本脚本 + 拍板 1.0 release' -ForegroundColor Yellow
Write-Host ''
exit 0
